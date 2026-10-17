# WBS Historical Notes (Implementation Record · Audit · Adjudication Trace)

> This document is the **historical notes** for [09-execution-wbs.md](./09-execution-wbs.md)
> (construction task table): implementation records, audit findings, adjudication traces, and errata
> for each phase. **Division of labor**: 09 only describes the **plan** (kanban, three-level task
> table, dependencies, risks, acceptance); this document only records **history** (what was done,
> what was found, how adjudicated), appended chronologically without rewriting history.

## P2 Establishing Equivalence Criterion Baseline

> **P2 Implementation (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green.
> **D38 prophecy fulfilled**—the green run empirically discovered and fixed three real defects in
> ir_gen (see 06 §G6 for details): method locals table truncated to parameter count (local_count
> underestimated, runtime relied on Frame::set_slot tolerance), unannotated parameters dropped by
> filter_map (three places, params metadata misaligned with frame layout), match result slot
> depended on frame's implicit Void initialization (now explicitly pre-initialized). The actual
> semantics of jump targets is flattened instruction index (not the label-keyed form in 07 text),
> and CFG is built at instruction level—documented in the verify.rs file header. **Commit
> cross-reference**: ir_gen fixes = `a9ea199b`; verify main body = `d5b18390` (⚠ parallel stream
> COMMIT_EDITMSG collision caused its message to be mislabeled as "docs(rfc): RFC-027a…", but the
> content is actually verify.rs + six test files, buried in history and unamendable, recorded here
> for the record).
>
> **P2 narrowing partial implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> hemorrhage channel preconditions unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (4 behavior + 1 compile-error) + runner
>   `tests/yx_multifile_runner.rs`; the judgment contract uses the same `TestFileSpec` as
>   single-file corpus (library-side single point), binary location/subprocess invocation abstracted
>   to `tests/common/` with yx_runner migrated accordingly (prohibition three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. The trigger source uses the rfc027
>   test-pinned `SumUpTo(3, r)` form (return position/call site/binding position)—the `Sorted(3)`
>   annotation example in 02 would be evaluated directly as Disproven in the current code, not
>   Unproven silent; the Sorted(3) in 07 pseudocode is illustrative only. Red-state empirical data
>   is at the head of the test file.
> - The two red criteria are mounted as `#[ignore]` perpetual per the 2.4.5 red skeleton precedent:
>   default suite all green, `-- --ignored` reproduces red state; **remove attribute to turn green
>   when P3 fix lands**.
> - The reason for the comparison normalization's deliberate deviation from the (code, file, line)
>   table in 07 (dropping file/line, comparing only error code sets) is documented in the 2.4.1 test
>   comments.
>
> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **are removed from P2**: the `Program` / `Obligations` types they reference only exist in P4, and
> P2 cannot compile. The former is P4's 4.1.4 (avoid double registration), the latter's `#[ignore]`
> red skeleton merges into 4.3.1.
>
> **2.3.3 Implementation (2026-10-07)**: C4 mandatory coverage list item-by-item check (07 §143), 10
> semantic corpus categories all covered and within the corpus-parity.jsonl differential baseline
> (26 representative files 26/26 hit, grep empirical evidence)—ref/borrow/move full 05-ownership
> directory (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep, etc.), closure
> capture closures + closure_arg_inference + spawn_capture, currying curry_value_fix +
> curry_tail_expr, spawn 04-concurrency ×13, iterator for spawn_for and 16 other files, sum type
> sum_type_* + pattern_or_guard + match_call_scrutinee, existential coercion enforcement point
> interface_dynamic_dispatch (Vec(Animal) existential heterogeneous packaging triggers
> existential_coercions) + interface_rebind_dispatch, `?`/Try propagation question_propagation +
> option_try + try_methods + user_try_type, method overload method_overload (+
> method_overload_ambiguous_err at the diagnostic differential level), refinement constraints Drop
> sequence refined_* ×5 + 05-ownership driven ReleasePlan (span key contract has another perpetual
> 2.4.4 red criterion). Performance baseline `benches/pipeline.rs` merged (07 §145 second of three
> smoke tests): `corpus_compile_frontend` first measurement of full corpus compilation 17.6s,
> `cli_cold_start_hello` process cold start 169.7ms; interpreter throughput is already covered by
> benches/lib.rs hotpath group (interp_fib_recursive_27 / interp_loop_steady_10m), no duplicate
> construction. Regression gate check-perf-regression.sh is under P9 unified list (2.5).

## P3 Fixing Correctness Vulnerabilities (Minimum Solution)

> **Error type labeling for completed rows** (retrospective labeling, history not altered): 3.1 =
> B ｜ 3.2 = B (defensive empty arm, criterion green-turn carried by 3.1) ｜ 3.3 = A ｜ 3.4.1 /
> 3.4.2 = A (new W codes) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 / 3.4.7 = D
> (user-visible surface, already user-adjudicated) ｜ 3.4.8 ①② = C (main) + A (incidental three
> places) ｜ 3.5 / 3.6 / 3.7 / 3.8 = D (all user-adjudicated).

> **P3 Diagnosis (2026-10-12, requested by user)**: An accounting triage in response to the
> perception that "P3 entries keep growing."
>
> **I. First clarify the scale (the perception is amplified by two accounting defects)**
>
> 1. **Numbering conflict**: `3.5.x` is shared by two task families—(a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 three-level: lexer and parser branches / AST `is_pub` / checker
>    side / dead code exemption convergence / peripheral echo / std and diagnostic text / language
>    reference convergence, all checked); (b) 2026-10-10 registered **3.5 higher-order type
>    parameter call path completion** (3 three-level). The two share `3.5.x` ⇒ cross-reference
>    ambiguity (**2026-10-12 per user adjudication, renumbered: the 029g family is changed to
>    `029g.x`**), and the 029g row is miscounted as P3. **Pending adjudication**: renumber the 029g
>    family to `029g.x` (or move it to its proper phase), with all other in-document references
>    updated.
> 2. **Missed check**: `3.7`'s 4 three-levels (3.7.1–3.7.4) are all completed (d922a551 / 38bd4b9e)
>    but unchecked ⇒ this item is corrected here.
>
> **II. P3 actual accounting (12 second-level rows)**
>
> | Row                                                                  | Status                                                                                                                                                                             | Disposition suggestion                                                                                                    |
> | -------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                      | Completed                                                                                                                                                                          | —                                                                                                                         |
> | 3.4 silent drop points addendum (8 three-level)                      | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (precondition 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (precondition 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4/3.4.5 unlocked with 4.3.1                                                                |
> | 3.5 higher-order type parameter call path completion (3 three-level) | Not started (declaration side live, call side broken)                                                                                                                              | Keep P3; **does not block refactor mainline**, but is "accepted feature unimplemented", higher priority than pure cleanup |
> | 3.6 function type value call path (1 three-level)                    | Not started (semantics has been pruned (i))                                                                                                                                        | **Keep**: correctness defect, criterion clear                                                                             |
> | 3.7 init segment call name qualification (4 three-level)             | **All completed**                                                                                                                                                                  | Check and close this round                                                                                                |
> | 3.8 native shadowing (1 three-level)                                 | Not started                                                                                                                                                                        | **Stuck on decision** (priority of name resolution first), not a scheduling issue                                         |
> | 3.9 / 3.10 / 3.11                                                    | Completed (2026-10-12)                                                                                                                                                             | —                                                                                                                         |
> | 3.12 runtime mode inter-executor config propagation (1 three-level)  | Not started (D class, user adjudication B: separate team)                                                                                                                          | **Suggest moving out of P3 → P8 (execution layer)**, pending adjudication                                                 |
>
> **IV. 4.3.1's leverage (mainline marker)**: `4.3.1 obligations.rs + assert_drained()` is also the
> precondition for **3.4.4**, **3.4.5**, and **4.5.1** (cross-layer contract PlanId-ization) ⇒
> taking it once advances the mainline and clears two P3 stuck items. 3.5 (HKT), 3.6 (function type
> value call) and 4.6.1/4.9.1 have **no unsatisfied preconditions**, all can be done anytime; 3.8 is
> stuck on semantic decision (user-defined priority vs explicit diagnostic).
>
> **III. Mechanism suggestions (prevent recurrence)**: Newly registered second/third-level rows
> **must carry two fields**—"Owning phase" (which P it truly belongs to) and "Whether blocks
> mainline"; `registered pending scheduling` is no longer the default state, unmarked is treated as
> unschedulable. This way expansion is visible and sortable, not indiscriminately accumulated.
> **Pending adjudication**: whether to write this into this document's registration convention and
> `src/dev/coding-rules.md`.

> **P3 Implementation (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + sole getter
> (recorded as `pub(crate)` at the time; **2026-10-07 verified actual implementation is
> `pub(super)`**—the `#434` adjudication cited in code comments has no doc-side registration, see
> 3.4.3; full private would require rewriting 11 construction literal sites, not taken during the
> hemorrhage phase; P4 Obligations migration will close it); execution mechanism abstracted to
> `src/frontend/proof_execution.rs` (prohibition three single point—cannot be placed in typecheck/,
> that would create L3→L4 reverse dependency); 3.1.2 per 02 evidence chain step 5 supplemented with
> **four entry points** (WBS row omitted compile_project `:99`, per 3.1.1 acceptance "criterion
> green-turn" must be supplemented and registered); 3.2.1 empty arm changed to bookkeeping
> (proof_calls throws up) + diagnostic (the established `into_result` conversion path); 3.3.1 SOLVER
> slot Option-ified, Z3 missing falls back to `SMTResult::Unknown` conservatively (E2031 family
> diagnostic replaces process crash). Two vulnerability criteria removed `#[ignore]` to normal
> green, corpus fixture `proof-obligation-honored` added; check-fanout inventory migrated
> accordingly (wiring point 19=inventory). Empirical: criteria 2.4.1/2.4.3 green-turned, full suite
> lib 2494 / integration 272 / corpus dual-layer all green.

> **3.4 Audit (2026-10-07, post-P3 implementation review)**: For the type of defect "failure
> produces no signal" (definition in 02 §41-52), without modifying code we re-surveyed and located 8
> places, all registered as 3.4.x in the table above. Method and evidence:
>
> - **Consumer-side survey** (16 obligation fields grepped all repo read sites):
>   `variant_ctor_calls` **zero consumers**—the only read at `checker.rs:1468` is "self-injected
>   into `TypeCheckResult`", IR side uses form detection `ir_gen.rs:7374`; `rfc011b.rs:659/688` only
>   asserts production side, exactly the "only test production side, naturally immune to
>   consumer-side defects" pattern criticized in 02 §186 → 3.4.4. `method_overload_ir_names`
>   result-level field likewise has zero external consumers, IR uses parallel `overload_resolutions`
>   → 3.4.5.
> - **Stage coverage review**: 02 §91 table row 1 (four entry points "no proof_execution") has been
>   fixed by 3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 are unchanged (multi-file `run` still
>   doesn't output W1003, doesn't run dead code family) → 3.4.6; LSP non-project-internal path (02
>   §77 "manual lex→parse") likewise doesn't run proof → 3.4.7.
> - **Same-form empty arm scan** (all-repo `Unproven` consumers are only four): checker three
>   refinement branches (covered by 3.1), the ownership arm fixed by 3.2.1 (**source unreachable**),
>   and `environment.rs:422` (**reachable, zero diagnostic, zero bookkeeping, and zero repo doc
>   registration**) → 3.4.1, the only "all-unknown" item in this audit.
> - **Solver acquisition side**: `default_solver()` three consumer forms differ (`predicate.rs:32`
>   process singleton / `checker.rs:1266` injected per `check_module` / `ownership.rs:621` newly
>   created per back-edge determination)—3.3.1 only eliminated the panic; the **silent surface** of
>   the other two goes to 3.4.2, **singleton-ization and cache hit** goes to P4's 4.4.2 (avoid
>   double registration).
> - **Already-attributed items not counted in 3.4**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys unconsumed, Drop silently lost) still P2 red-state perpetual, attribution
>   4.5.1/D20 unchanged; 4.4.1 (layer order declaration opposite to reality, `equivalence` layer not
>   in pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) already on P4 roster.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 preconditions, can be landed in place
> anytime (recommend doing 3.4.1 first—the only "all-repo-zero-registration" live vulnerability,
> smallest change surface); 3.4.4–3.4.8 execute with P4, their criteria depend on the obligation
> ledger (4.3.1) or unified Driver (4.2.x). **3.4 does not create new criterion system**, all reuse
> existing C2/C3 and P9's 9.2 gate.

> **Batch 3.13–3.16 implementation record (2026-10-13 registered → 2026-10-15 closed)**: User
> 2026-10-14 adjudicated merging four relevance minor items (#451 method binding panic / #452 named
> actual argument into resolution / #453 dead inference entry / #454 native export named actual
> argument—the last one is a boundary finding while verifying 3.14, same-day attribution experiment
> verified **not introduced by #452**, user adjudication ① merged into this batch) into one team for
> processing. Closure status: all four items completed, adjudication registered as RFC-039 **D60**
> (six binding rules + shared criterion single-point-ization).
>
> **I. Two contract corrections (lesson: contracts themselves can be wrong)**
>
> 1. **Acceptance fixture self-contradiction**: #452 task contract required both "mixed arguments
>    output 20" and "position and name at same position report error", but the pinned fixture
>    `Temp.pick(self, a)` has only one value parameter—the same argument combination is both "mixed"
>    and "conflicting". The first implementer honestly reported the blocker and set the task failed,
>    overall revert (not trading silence for green). Disposition: task reassign + contract
>    revision—given **six binding rules** and **8 value-level matrices** (fixture changed to two
>    value parameters `pick(self, n, a)`), attempt 2 completed by second implementer. The full text
>    of the six rules has been verbatim included in D60.
> 2. **inScope omits mechanically required surface**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **omitted the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 registration only ran `code-tables --fix`,
>    **did not re-run** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth red ("source
>    registered/emitted but doc missing 1: E1110", source 154 / doc 153, run **38518624964**—the
>    only CI red line in this batch). Fix = mechanical regeneration (8 files +83/−70, only
>    `docs/src/reference/`), reconciliation 154/154, CI 38520209026 green.
>
> **II. Review → Fix → Re-review two closed loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = meta-guard `named_args.is_empty()`
>    did not narrow with resolution ⇒ calls with named arguments in both multi-candidate zero-hit
>    and single-candidate hit paths silently pass through the whole chain) → fix round added
>    external code **E1111 (argument_order_invalid)** (code table + locales + RFC-013 + generator
>    four-piece set in same commit; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared parsing entry + native arm settled by name) → independent
>    verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations mechanically
>    reversibly replayed by `scratch.py trial` all caught, restoration byte-for-byte consistent) →
>    round-1 review needs_revision (F1 = typecheck native arm **inlined copy** of a binding
>    criterion, missing rule ④ ⇒ method and native same-form calls have **adjudication
>    divergence**—method reports E1111, native silently exit=0; ir_gen has a third pre-existing
>    duplicate with old code E1015) → convergence round collapsed criterion to single implementation
>    `bind_args_to_params` (`arg_binding.rs:35`, four call sites `expressions.rs:2727` / `:5416`,
>    `ir_gen.rs:7776` / `:8279`) → round-2 review caught a **regression** introduced by the
>    convergence round (IR convergence block lost native optional position ⇒ programs that
>    previously succeeded by omitting optional position are misreported E1010) → fix round (optional
>    position criterion and parameter name co-emitted from `signature.rs:236` as sole exit) →
>    round-3 review PASS (independent reproduction red→green; mutations under pinned FAILED
>    empirically prove criterion can fail). **Lesson**: review must cover "unmodified branches of
>    similar paths" (this batch's two needs_revision both resulted from probes all falling on the
>    hit side); the fix round itself can also introduce regressions (optional position), caught by
>    the next round of swap-person review.
>
> **III. Process events (registered truthfully, not pointing to individuals)**
>
> 1. **External full-tree discard (2026-10-14 around 17:14)**: the work tree was externally restored
>    without sound—the implementer's in-progress fix (typecheck/ir_gen/code table/locales/RFC-013
>    etc.) all disappeared, **at the same moment two review documents edited by the user themselves
>    also disappeared from git status** ⇒ determined to be a repository-level discard (most likely
>    user-side manual cleanup/IDE "discard all changes"). Three pieces of evidence excluded group
>    members: empty stash, no new commits, no reset in reflog. Implementer replayed in-progress work
>    from context and pushed; discipline changed to "**save patch to scratch first, then sample red
>    state**".
> 2. **Verification member context exhausted, swap person**: the independent verifier verifying 3.16
>    ran out of room midway (no closing message), per load rules swapped person—the successor
>    continued using its already-ready artifacts (green-state results, corpus equivalence script,
>    mutation plan), verification finally PASS. A positive precedent for partial artifact handover.
> 3. **Review round 5 rotation and historical round rollback**: when rotating the review per load
>    cap rule ⑤, `remove_member` **also rolled back** the already-adjudicated terminal
>    review/verification rounds, the scheduler immediately re-ran (t3/t8 were re-taken by new
>    members, duplicate work blocking downstream). Disposition: team lead took over and canceled the
>    rolled-back historical rounds (t3/t8/t13), added AGENTS.md rule 12 per this (handle rolled-back
>    rounds immediately after member swap). This batch's closure record registers this event per
>    that rule.
> 4. **scratch governance (~6.3 GB)**: each task in early batch phase had an external tree, 6
>    directories scattered outside the repo (~780 MB, cleaned) + two members' external verification
>    trees (~2.2 GB / 3.3 GB, self-deleted after use). Output governance tool
>    `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md rule 9
>    (batch tree sole entry), rule 10 (verification environment six rules: mechanically reversible
>    mutation / line number from referenced revision / CI-only gates must run etc.), rule 11 (writer
>    load cap), rule 12 (handle rolled-back rounds after swap), and two top meta-rules (no private
>    content / verify before asserting).
> 5. **Contract defect (not pointing to members)**: #454 fix round's contract inScope only circled 2
>    files, actual mechanically required surface 8 files (rule 6 is to prevent exactly this—contract
>    template should pre-set mechanically required surface list); also the task's terminal field was
>    once occupied by probe payload, formal acceptance/command evidence was supplemented by team
>    lead per spec.
>
> **IV. Remaining items (not fixed in this batch, registered for record)**: round-3 review three
> low-severity findings—① method path's `build_arg_slots` (`ir_gen.rs:521`) is still a **second
> copy** of the criterion (pre-existing, registered as part of closure per last round's scope, not
> converged)—**2026-10-16 pre-release audit added three points**: (a) it has **two** call sites:
> `ir_gen.rs:7888` (method binding path, uses the parameter name of the bound function
> `binding.function`) and `:7975` (overload resolution path, uses the leaf segment of the resolved
> mixed name), the original registration only wrote "around `ir_gen.rs:521`"; (b) it has **already
> diverged from the shared implementation `bind_args_to_params`**: missing rule ⑤ (duplicate name →
> E1015), rule ⑥ (missing required parameter → E1010) and the "optional position" concept, and
> hard-codes the receiver position as `1 + i`; the two call sites `continue` uniformly on empty
> slots (comment says "diagnostic goes to typecheck (it guarantees no empty slots if resolution
> passes)"), this is a **dangling invariant written in comments with no assertion safeguard**; (c)
> **RFC-039 D60's "all call sites converge on it" enumeration missed these two places**, the
> declaration is inconsistent with the fact, and the cited line numbers were not re-fetched on the
> referenced revision, D60 text has been corrected along with this entry; ② batch verification tree
> (worktree) internal `cargo fmt --all` error is a worktree workspace artifact, the gate checklist
> should note fmt goes to main repo; ③ repair class contract template should pre-set mechanically
> required surface into inScope (see III.5).
>
> **V. Final gate (closure revision, local two implementation commits + one doc commit pushed
> together—the only push in this batch)**: fmt --check 0 / clippy -D warnings 0 / `cargo test --lib`
> **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** / `--test yx_runner`
> 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 / corpus_probe **342
> rows line-by-line consistent with baseline** (only regenerated row: `named_arg_duplicate_err.yx`
> E1015@14 → E1111@16, expected code and line number synchronized, manually reviewed) / IR snapshot
> zero drift.
>
> **VI. Run links (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (same run's MSRV/Test was canceled by subsequent
> push per concurrency—cancel ≠ fail, covered by descendant run); governance tool push → CI
> 38546373435 / 38561405117; WBS registration (3.16 row + table header 56→57 / 147→148) →
> CI 38528560364. The closure push's own CI / Deploy Docs run was registered into the task record
> after push.

> **3.4.8 ③④ Implementation (2026-10-11, 8ecf56f93)**: ③ `CHECK_STAGES` tail added `Linking` (**pure
> merge**, the whole block skips `is_bin_role` entry check—E3020 criterion still belongs to
> `RoleClassification` to prevent double reporting; another hard reason: the run path uses merged IR
> to determine "has main", under collection semantics "entry itself errors → no IR" would be
> misjudged as missing main and hard-aborted) → `Monomorphization` (check form);
> `ir_generation_check` retains unit IR (**in-memory only**, for pure merge and mono consumption, no
> external emission/no disk write/no snapshot produced—without retention the aggregation request is
> empty, mono falsely greens with `NoObligations`); mono failure goes through per-file collection
> channel in Check (diagnostic attributed to entry file + `failed_stages`, not set to failure),
> MultiFile keeps `fail_with`; `into_outcome` changed to "take collection first, then judge
> failure". ④ empirically **zero production change**: whole-program BFS already covers cross-unit
> mutual recursion (pathological → E3005, well-formed three-layer chain executes correct values),
> delivered four e2e groups + one mono unit test as living criteria.
>
> **Verification chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-for-byte cross-comparison with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-form matrix
> (single/multi/check three forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increased not decreased).
>
> **F1 (found in verification phase, D59 A/B class, fixed in same round)**: under rootless
> multi-file check without project root, the aggregation request name (`lib.f`) and unit IR
> definition key (`f`) are from different sources → mono falls into "native no IR definition" silent
> `continue`, check reports 0 error/exit 0. Fix: aggregation side adds reverse-lookup by each unit's
> **actual definition key** (exact enumeration + literal equality + ambiguity abandon, not suffix
> heuristic; project form not registered, existing path unchanged). **Erratum (same day)**: this
> reverse-lookup was **deleted** in the subsequent regression fix; root cause resolved by
> `module_key` one-duty-two-use decoupling (added `script_mode`) from a naming perspective—IR
> definition key = IR call name = aggregation request name = `{unit key}.{name}`. **Regression
> record**: after ③ implementation CI's `Check tutorial/guide code examples compile` red (258 blocks
> / 10 failures: 6 same-file same-name rebindings + 4 collisions with `<std/list>`, rootless check
> doesn't do qualification triggering `link_module_irs`'s collision check); fix = ir_gen split
> `script_mode` (qualified name ⊥ Script/Bin, E3023 gate changed to look at it) +
> `ir_generation_check` per unit always pass unit key + `.with_script_mode(rootless)` + Script
> top-level same-name rebinding "latter takes effect" (consistent with run path's existing
> behavior); verification = doc gate 258/0, three-binary comparison (HEAD/③ tree/fix tree) three
> types of collision fixtures, check form E3005 no regression, E3023 not opened.
>
> **F2/F3 (accepted)**: mono diagnostic attributed to entry file item, span can cross units
> (consistent with run path's `path=entry`); Check form holds one ModuleIR per unit (in-memory only,
> released at check end).
>
> **Existing issues left for follow-up (not regressions in this round)**: ① E3005's help text still
> reads "internal error; please report this issue", inconsistent with resource protection semantics
> (precedent: `e3xxx.rs:47` once moved similar "user-written error" out of E3005)—registered pending
> scheduling; ② multi-file E3005's span rendering label offset (reproduced with HEAD binary,
> pre-existing); ③ request collection side has **type parameter position dependency** (type
> parameter not at value parameter position generic calls don't emit instantiation request, mono
> records `NoObligations`, convergent mutual recursion is this form)—same surface as 3.5 (HKT call
> path), registered to that row.

## External Track RFC-029g: Removing `pub` and Auto-Binding (#399)

> **P3.5 Implementation (2026-10-08)**: all seven second-level tasks completed, the whole batch is
> one commit (independently revertible).
>
> - **Empirical mismatch with plan (registered truthfully)**: 029g.1.2/029g.1.3 acceptance
>   originally wrote "`pub f = ...` reports **parse error**"—actually after deleting `KwPub`, `pub`
>   falls back to a normal identifier, the old form lands at semantic layer reporting **E1001
>   Unknown variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specifically probing
>   removed syntax is an anti-pattern", **no** new migration-specific diagnostic is added; corpus
>   fixture `pub_declaration_err.yx` pins E1001, doc (`syntax.md`) honestly writes this consequence.
> - **Design convergence (beyond RFC literal, needs review)**: `project_refs` and `cross_file_refs`
>   two fields have completely overlapping semantics after pub removal (same "intra-package
>   reference union", prohibition one forbids parallel representation) → merged into single
>   `project_refs`, `set_exempt_pub`/`set_cross_file_refs` two switches deleted. Behaviorally
>   Internal role's non-pub items change from "intra-file reachability" to "reference pool judgment"
>   (loosened, direction of under-reporting); Lib tightens from "pub absolute exemption" to
>   reference pool judgment; Script top-level bindings not referenced start reporting
>   W1001/W1002/W1004/W1005.
> - **Dead table cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports write-back all deleted (production-side table
>   injection, repo-wide zero consumers).
> - **Test migration** (test count not decreased): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub is recognized" assertion reversed to "pub is no
>   longer a modifier"; `passes/tests/dead_code.rs` 7 role semantic cases rewritten (assertion
>   reversed 2); `tests/integration/cli_e2e.rs` 8 role cases rewritten (`lib`/`internal` exemption
>   assertions reversed to report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` doesn't participate in IR); corpus differential
>   baseline only adds 1 (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerated and manually reviewed.
> - **Empirical**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-Layer Contract PlanId-ization (D20)

> **4.5 Implementation Record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, closure
> (registration / WBS check / gate / push) completed by t7. Precondition = 4.3.1 obligation ledger
> (red state keeps 173 files / 533 keys on file).
>
> **I. Carrier adjudication (A: `PlanId` walks with AST node)**: added `PlanId`
> (`src/util/plan_id.rs`, `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX`
> represents synthesized nodes outside the parser), issued by **sole allocator**
> `ParserState::alloc_plan_id`; carrier two places—`Stmt.plan_id` (statement identity) and
> `Expr::Call.plan_id` (call site identity), `Expr::call` is the sole call constructor in the repo.
> Rejected is the "two sides each derive the same identity by span" position-coincidence form (D20's
> original defect).
>
> **II. Two-level verification and review (members independent)**: each functional commit has an
> independent verifier and a reviewer, **verification/review do not reuse implementer's evidence**:
> 4.5.1 = t1 implementation / t2 independent verification (external tree + 5 mutations) / t3 review;
> 4.5.2 = t4 implementation / t5 independent verification (external tree + 5 mutations) / t6 review.
> Review time point and reviewed revision both fall on immutable history (`git show` verifiable).
> **provider renaming caused t5/t6 to swap members for continuation**: original verification/review
> members couldn't continue due to provider renaming, per orchestration convention swapped and
> continued, **record the reason** (cross-round context retention priority, swap only when
> infeasible to compress); the continued members' conclusions are consistent with original rounds
> (PASS), no reuse of implementer evidence.
>
> **III. Two defects self-discovered and fixed during implementation (t1, all within inScope)**: ①
> consumer exemption criterion was once hooked on `UnitState.ir.is_some()`—Check form's Linking/Mono
> would take `unit.ir` away, truly unconsumed in check form **silently passed** (mutation empirical:
> D41 red but CLI check exit 0); changed to read the **ledger's own completion marker**, and added
> regression cases. ② Drop sequence drifted with process hash seed (partial order `sort_by` +
> `HashSet` iteration): snapshot criterion produced 106/120/114 row drift in three consecutive runs;
> changed to **total order sorting** (same name takes max depth + name ascending) and zero drift in
> two consecutive runs.
>
> **IV. Criteria and evidence numbers**: after 4.5.1 `test_release_plan_spans_consumed` removed
> `#[ignore]` perpetual (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated in the
> same commit (programmatic review: 1083 new Drop rows + 86 jump index shift, zero deletion, no
> reordering), corpus 342 differential empty; 4.5.2 added `test_overload_resolutions_consumed`
> (difference set empty + evidence surface floor `examined ≥ 1` / `consumed ≥ 2`; empirical
> examined=1, consumed items 2, no resolution skip 210), resolution form 9 items pinned,
> `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero drift**. Mutation-driven: consumer
> key offset by one ⇒ criterion red + real program `error [W1090] ... produced key not consumed` +
> corpus gate diff; restored and re-greened.
>
> **V. Semantic correction classification basis (D59)**: `Type.method(positional_arg, name = value)`
> before the change **silently gave wrong answer** (empirical compile clean, run output `1` instead
> of `20`) ⇒ **C class** (old behavior not successful), fix takes "loud": after change same program
> compile-time `error [W1090]`; "named argument participates in resolution" terminal semantics is
> **D class**, registered §3.14 and issue #452 left for specialized team to take on. Also: `Drop` in
> interpreter is no-op (`backends/interpreter/executor/ops/control.rs` wildcard arm only
> `advance()`)—key migration doesn't change runtime behavior.
>
> **VI. F-1 / F-2 disposition (t5 independent verification finding)**: **F-1 (medium)** unbound
> consumption point (`ir_gen.rs:7766-7790`) is unreachable in current corpus and test surface, key
> correctness has no red state to falsify ⇒ per "known boundary" registered into
> [02](02-stage-contract.md) §2 revision note fifth paragraph (reservation reason + no new task);
> **F-2 (low)** pointer drift ⇒ `types.rs`'s `OverloadResolutions` doc comment changed to precise
> pointers (production `inference/expressions.rs:4550`, consumer `ir_gen.rs:7697` / `:7772`), single
> small commit, not mixed into doc commit.
>
> **VII. Closure (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumer point line numbers (`:360` read / `:3103` use / `:1095` and `:1097` settlement markers);
> 07 criteria table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG recorded one line of behavior change; `check-obligations.py` and `check_tracking.py`
> green, CI and Deploy Docs green. **Accounting deviation found at closure**: t3 / t6 report's
> pointers `ir_gen.rs:357` / `:3102` / `:1096` differ from code fact by one to three lines (actually
> `:360` / `:3103` / `:1095`+`:1097`, due to 4.5.2's insertion)—registered per code fact, this note
> also records the deviation.

## P4 4.10.3 Standalone run and MultiFile Orchestration Unification (4.2.5 Adjudication A's run half)

> **Bing Batch Record (2026-10-15 registered → 2026-10-15 closed)**: user 2026-10-15 adjudicated
> Bing—this round only does 4.10.3 (same-batch candidate 3.4.6 per B1 ([02](02-stage-contract.md)
> §Runtime behavior table "dead code family warnings don't enter compile path") deferred to after
> 4.9, precondition re-hooked and annotated); implementation form = **complete single team** (user
> instructed no DAG, no team, single-session implementation + self-verification). Pre-batch debt
> landed first: #455 dependency announcement fix (rustls 0.23.46 + serde_yaml_ng, `cargo audit`
> recovered from exit 1 to exit 0), WBS registration debt (#456 registered as 3.17 + incomplete rows
> added "owning phase / whether blocks mainline" fields).
>
> **I. Red first, then green**: 3 driver-level red criteria
> (`src/driver/tests/run_rootless.rs`—rootless merged IR must not carry entry_function (red state
> `Some("main.main")`), rootless top-level executable statement must not report E3023 (red state
> produces no module), Bin control entry_function maintained (green state perpetual)) + 3 CLI e2e
> pins (standalone relative import run (red state E5001), Script top-level statement execution and
> #413 hint two green state pins).
>
> **II. Implementation side**: CLI `run` unified through `compile_project` (the project/standalone
> fork in `run_file_with_diagnostics` deleted); Linking's `entry_function` gated by `is_bin_role`
> (#413 T4—Script doesn't auto-call main); #413 hint keying changed from
> `module.source_files.is_empty()` to call-site project probe (after unification ModuleIR always
> carries source_files, original marker ineffective); `script_main_never_called` adapted to
> qualified name form (function `{key}.main`, global slot empirical also qualified name—IR
> evidence-based, not from comment inference).
>
> **III. Existing gaps exposed by unification (fixed along the way, A/B/C class—only fix failing
> programs to succeed, zero impact on originally successful programs)**:
>
> 1. **Explicit type argument pre-specialization name** (`mk(Int)(5)`): ir_gen per D6.3 directly
>    emits specialized name (mono will strip unspecialized original), `resolve_call_target` three
>    arms (rename / aliases / dot prefix) don't cover `mk(int64)` form ⇒ call site dangling E6006.
>    Fix = this single point parser adds fourth arm (base name qualification, type parameter string
>    preserved as-is). **Bin project same-form reproduction (pre-existing defect, not introduced by
>    this batch)**.
> 2. **Script top-level `mut` assignment statement repeated slot assignment**: `extract_global_defs`
>    and `register_global_slots` count `counter = counter + 1` (Assign with same form as
>    declaration) per statement ⇒ same variable assigned 3 slots, initialization writes the
>    later-assigned slot but read uses the first-assigned slot (runtime Add type mismatch). Fix =
>    two places merge by name (same criterion same source).
> 3. **Same-module constrained generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually is `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidate + table verification (exact hit → generic's
>    unit key qualified hit → fallback to bare name current state; not suffix heuristic, same form
>    as `resolve_generic_name`).
>
> **IV. Registration gap (not fixed in this batch)**: constrained dispatch in **type and generic
> definition in different modules** still E6006 (rootless dual-file probe empirical; Bin same
> form)—root cause = MonoType user type name doesn't carry defining module, correct fix belongs to
> P6 type identity convergence; registered WBS **3.18** (owning P3, precondition pending P6, not
> blocking mainline), `qualify_deferred_method` annotation same surface.
>
> **V. Test adaptation (honest registration)**: `tests/integration/multifile.rs` fixture added
> manifest—what it tests is RFC-029 project orchestration (Bin auto-entry) not Script semantics;
> `cli_e2e` one pin (top-level diagnostic variable name visibility) and `script_main_hint` fixture
> (7 cases) synchronized per qualified name form—production/consumer side only eats merged IR,
> fixture directly drives same form, no pipeline product stand-in.
>
> **VI. Final gate (closure revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new pins) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> differential baseline regenerated **12 rows**—row-by-row structural check: exit / stdout /
> diagnostic code all same item by item, only stack frame function names and variable names
> qualified per unit key (manually reviewed); after regeneration `check-corpus-parity.py` exit=0.
>
> **VII. Process events (registered truthfully)**: this batch once during pre-commit hook running
> concurrently edited workspace docs, was eaten by the hook's stash rollback and that commit failed
> (configuration comment warned about this concurrent form)—discipline = don't move workspace during
> hook running; #455's commit thus split into two attempts (second all green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closure)

> **4.6 Batch Record (2026-10-16 registered → 2026-10-16 closed)**: user 2026-10-16 instructed
> advancing 4.6 → 4.9 → 4.4 → P5 in order, do 4.6 first; implementation form = **complete single
> team** (continuing last batch's user instruction: no DAG, no team, single-session implementation +
> self-verification). Task definition (09 three-level table): project-internal file's semantic data
> (SemanticDB) goes through orchestrator's same pipeline, same source as diagnostic; cross-file
> reference carries `resolves_to`. Acceptance: project-internal cross-file navigation hits
> definition; semantic data and diagnostic come from the same compilation.
>
> **I. Current state empirical (verify before asserting)**: didOpen/didChange triggers **two
> passes** of compilation—diagnostic goes through unified Driver pipeline (4.2.3/4.2.6 already
> unified), semantic data goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registration table ⇒
> project-internal `use` parsing fails, cross-file reference events not produced,
> navigation/rename/reference find silently falls through for imported symbols—D55 registered silent
> failure). Orchestration arm's checker unified name `"<module>"` (SemanticDB file identity lost),
> `type_result` (containing semantic_db) discarded when diagnostic non-empty and never externally
> emitted; `checker.rs` drain overwrites `resolves_to`'s DefId file identity to current file;
> `Export`/`ModuleInfo` have no definition location surface.
>
> **II. Red first, then green**: LSP e2e pin (`src/lsp/tests/cross_file.rs`)—temp dual-file project
> after didOpen main.yx send GotoDefinition to `add_one` call site, **before fix returns None (red,
> empirically the silent failure in previous section)**, after fix hits lib.yx line 0 definition
> (green). Mid-process probe: compiler side chain (driver probe dump) already fully connected but
> e2e still empty—root cause is `resolve_reference` matches exactly by reference **start point**
> (D55 precise-only current scope), test cursor falls in middle of identifier; this tolerance
> belongs to existing behavior surface, this batch doesn't touch (see VII).
>
> **III. Implementation side**: (1) `Export.definition_span` (extract_module_info harvested from AST
> identifier node, same source and same value as typecheck definition event's `def_span`; native std
> always None) + `ModuleInfo.definition_file` (Registry arm filled per DiscoveredFile, including
> vendor; std always None); (2) checker reference drain via `import_def_locations` (use line binding
> span → definition file + definition span) redirects cross-file import items' `resolves_to` out of
> this file; std import not registered, maintained falling to use line; (3) checker module identity
> changed from unified `"<module>"` to unit file path (empirically `env.module_name` all 4 consumer
> sites zero diagnostic consumption); (4) `DriverOutcome.semantic_db` channel—CollectAll form (Lsp
> target file **regardless of diagnostic success** + SingleFile fallback) merged, Lsp arm tail
> injects non-target file top-level export definition surface from registry (`external_definitions`,
> unopened files can also navigate; type/submodule exports not injected—typecheck side type
> registration doesn't produce definition events, both sides keep consistent gap); (5)
> `orchestrator::analyze_source_in_project` (`check_source_in_project` trans call takes diagnostic
> half); (6) LSP `analysis.rs` single compilation entry replaces manual pipeline
> (`update_semantic_db` deleted), World key unified normalized file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
> differences normalized to same key, cross-file DefId therefore stable hit); 5 handlers +
> semanticTokens dual request connected to `db_key`.
>
> **IV. Along-the-way fix (D59分流 B class—old behavior silent no-action, originally successful
> programs zero impact externally)**: ① project-internal file cross-file navigation/rename/reference
> find changed from silently falling through to hitting real definition; ② when lexer errored, old
> manual pipeline deleted semantic info (highlighting disappeared) → residual AST highlighting
> preserved (4.2.6 editor philosophy extension); ③ when target file has type error, semantic data no
> longer discarded along with `type_result` (typing mid-state highlighting/navigation not lost).
>
> **V. Pins**: 6 driver-level (`src/driver/tests/lsp_semantic_db.rs`—target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface round-trip,
> std import fallback use line, error file preserved, single-file fallback path key, single-file
> error preserved) + 1 LSP e2e (red first then green) + 4 URI normalization
> (`src/lsp/tests/uri.rs`—drive letter conversion, space encoding, idempotent normalization,
> non-file scheme pass-through).
>
> **VI. Mutation verification (scratch.py trial, mechanically reversible)**: batch tree wbs-46 @
> closure revision 39546020, two mutations both made hit pins turn red, after restoration blob
> consistent with HEAD + git status CLEAN—m1 removed `import_def_locations` registration
> (statements.rs) ⇒ cross-file driver pin + LSP e2e pin red, other 7 green; m2 removed
> `external_definitions` injection (arms.rs) ⇒ same two pins red (round-trip parsing lost anchor),
> other 7 green. Batch tree gc'd (freed 4.8 GB, `status` no batch tree).
>
> **VII. Final gate (closure revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new pins) / `--test integration` **287 passed** / corpus
> differential **byte-for-byte zero drift** / IR snapshot zero drift / check-obligations PASS
> (13·13·0·8, `semantic_db` non-obligation-reason consumer anchor synchronized with server.rs:482
> and 02 §Obligation field table) / check-docs-truth 0 / docs-examples 258 0 failures / docs-orphan
> 0 / markdownlint repo-wide only the user's unsubmitted 032 two MD056s (user's own edit, not
> touched) / pnpm docs:build 0. **Process event (registered truthfully)**: en/09a-wbs-history.md:54
> one MD037 was introduced by auto-translate bot re-translating last batch's 09a (zh original has no
> space), this batch fixed in passing—markdownlint red not from this batch's change surface but same
> file as 09a, registered together. **Left observation (registered pending discussion, not
> touched)**: `resolve_reference` only matches by reference start point exactly—when cursor falls in
> middle of identifier, navigation/hover/rename silently no-action (D55 precise-only current state,
> whole file form isomorphic), if tolerance matching is needed it belongs to independent behavior
> change, report to user for adjudication.
>
> **Same-day continuation batch (4.6.2, implemented after user adjudication)**: after delivering the
> closure report, user adjudicated the "left observation ①" as "OK, then change it"—query key
> tolerance implemented per plan A: definition/hover/references/rename four handlers' query key
> changed from cursor position (via UTF-16 conversion) to **identifier's own start point** (already
> located by `find_identifier_at_position`, same coordinate system as semantic events, no
> conversion); `resolve_reference`'s start point exact match semantics unchanged (D55 precise-only
> unchanged). references/rename's definition point fallback determination (`start <= col < end`)
> holds for identifier start point isomorphically. Red first then green: definition middle cursor
> pin red first (None) then green; four handlers middle cursor each 1 pin + cross-file e2e
> start/middle dual position assertion, `--lib` 2748 (+4) all green, clippy -D warnings 0.
> Registered WBS 4.6.2 (three-level count 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix)—Round 1 Closure (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 Batch Record (2026-10-16 registered → 2026-10-16 closed)**: user 2026-10-16 adjudicated
> merging 4.9.1 / 4.9.2 / 4.9.3 three related three-level tasks **into one team one round** (reason:
> they all belong to the "pool source and identity" surface, merged implementation avoids repeated
> rewriting of intermediate state after `collect_project_refs` deletion; 4.9.4 / 4.9.5 belong to
> consumer refinement and test pool separation, depend on 4.6.1's `use` path resolution, forcing
> same batch would expand the gate surface). #450 ownership **pending adjudication**—evidence shows
> it's "overload resolution's name not passed back to reference count" class, still reproduces under
> round 1 implemented form, registered per independent WBS entry for disposition (team lead
> recommends A: register as independent entry in round 2; **pending user adjudication**). Freeze rev
> = `3863a9ff0` (parent commit `62f6e1a59` is pre-fix baseline); verification PASS attempt
> `aab92fe0` (i.e. t2); review pass attempt `96452696` (i.e. t3). **Closure registration commit =
> `38d30b846` (docs(docs): 4.9 reference pool round 1 closure registration)**—this was actually
> written and pushed by closure member integration-refpool; its closing probe once mis-marked the
> closure task as terminal failed and overwrote output, team lead took over new attempt and changed
> back to completed per verified delivery fact, hereby correcting attribution and label. Evidence on
> file: `target/yx-scratch/yx49/EVIDENCE.md` (containing 8 mutation patches and
> warn_compare_v8.json).
>
> **I. Merge adjudication**
>
> 1. **The adjudication itself**: three related three-level tasks merged into round 1. **4.9.4
>    (consumer refinement) / 4.9.5 (test pool separation) next round**—the former depends on 4.6.1's
>    delivered `use` path resolution (already landed at `39546020`, can start); the latter is same
>    batch as RFC-029f table incorporation, registered at end of RFC-029f pending.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 are preconditions for each other (4.9.2 changes pool type
>    → 4.9.1 assembly side needs rewrite → 4.9.3 has denominator-judged role table), splitting into
>    rounds must write one intermediate state and change twice; (b) all three belong to the "pool
>    source and identity" surface, 4.9.4 / 4.9.5 belong to another surface, splitting into batches
>    is by "cohesion" not "independently revertible".
> 3. **Merge result**: single commit `3863a9ff0` (one line `:recycle: refactor(driver)`), single
>    push, single batch gate. **Hash explanation**: pre-push `git pull --rebase` merged
>    auto-translate bot's commit (`72f364169`); rebase lands after the other party's commit, this
>    batch's two (implementation `3863a9ff0`, its pre-fix baseline兼 other party's doc commit
>    `62f6e1a59`) **not rewritten, maintain original hash**—the first version mistakenly wrote this
>    as "`3863a9ff0`→`3863a9ff0`" identity mapping (looks like "coincidentally same after rewrite",
>    actually "not rewritten"), corrected at closure 2026-10-17; the other party's code commit
>    `33f14e8ab`→`b122b7ad1`. **External references all use post-push hash as authoritative**
>    (rewritten old hash unreachable in new clone, identity determined by patch blob comparison).
>
> **II. Five criteria red/green empirical scope**
>
> | #   | Criterion                                              | Red state (pre-fix / P1 directory scan reentry patch)                                                                                                      | Green state (round 1 landed)                                                                                                                                                                                                                                                                                                                                                                  |
> | --- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | ①   | Pool root = package target (#334)                      | Directory recursive scan ⇒ tests/ files' identifiers also enter pool, exempting production code's unused definitions                                       | `collect_package_refs` (`helpers.rs:481`) per `ExplicitSurfaces` root calls `discover_with_used`; all-root degenerates to fallback `collect_compilation_refs` (`helpers.rs:454`)                                                                                                                                                                                                              |
> | ②   | Pool granularity = `(module, name)`                    | Same-name local variable (`let foo = …`) accidentally exempts same-name top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` cross-contaminate | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) is sole assembly entry; `arms.rs:603-609`'s `other_module_refs` unions "all except this unit" module items then sends to `DeadCodeAnalyzer`; module's own name (top-level definition and local binding, `use` imported name excepted) no longer exempts other modules' same-name definitions |
> | ③   | `FileRole::Script` doesn't use pool                    | Script affected by other files' references in project                                                                                                      | `arms.rs:611-623`'s `match us.role` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`), Script/Test goes to `_ => {}` default arm (per file-internal definition-use graph determination)                                                                                                                                                                |
> | ④   | Fixture A: lib.yx line 4 definition must report W1001  | Pre-fix: reports `W1003@1` (actually "unused import"), not W1001                                                                                           | Red state empirical (red 1 fixture red1) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                                   |
> | ⑤   | Fixture B: main.yx line 4 definition must report W1001 | Pre-fix: reports `W1003@1`                                                                                                                                 | Same as above                                                                                                                                                                                                                                                                                                                                                                                 |
>
> **Criterion ②'s "inferred red"** (implementer self-stated + verifier closed): red2 fixture's
> `let foo = …` is in helper.yx; old pool under bare-name union treats `foo` as a whole "used by
> helper", lib.yx's `foo` exempt. **Empirical**: red2 fixture vs pre-fix binary → no W1001 (old
> scope exempt); red2 vs post-fix binary → W1001@1.
>
> **Criterion ③** (Script doesn't use pool)'s red state fixture **first version failed**: first
> version fixture placed same-name local variable in same file triggering E2010 (language doesn't
> allow same-module shadowing top-level definition), changed to another module (helper.yx) and was
> inferred red state. This point t3 review and t2 verification both independently reproduced, **not
> a contract defect**—same-module shadowing is the language's hard constraint, fixture must be
> cross-module.
>
> **III. +8 and +16 two count sets correspondence** (**with t3 review evidence-dead as
> authoritative**)
>
> `target/yx-scratch/yx49/EVIDENCE.md` section C writes `+16/−0`, but missed "**new fixture
> records**" this ingredient—v8 instrument (`warn_compare_v8.json`, 331 files symmetric diff,
> `only_before=0`) scope **includes** new fixtures; t1 implementer's self-built "+8" is
> **warn-before / warn-after (each 325) aggregated per fixture** (each member file checks once).
> Review maps item by item:
>
> - **+8** (`8` new warnings, all `check <non-entry file>`, `check <entry file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (unified attribution)**: these 8 fixtures all **didn't declare package target** ⇒
>     4.9.1 root degenerates to checked file's own compilation set, consumer main.yx not in it; this
>     is design-intended change (before, these files' check surface was already zero warnings, not
>     exemption path). Repo root `yaoxiang.toml` is empty file ⇒ same degeneration path empirically
>     holds.
> - **+16 residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files in t1's `warn-before.jsonl` have `exit=None` (capture time point
>     **doesn't exist**—new fixtures added in this batch). So +8's "+ 0 − 0" diff doesn't have them,
>     +16's v8 symmetric diff has them.
>   - `v8`'s `stray.yx` only has `W1001@2(probe)` one: reason is pre-fix binary also reports
>     `probe@stray.yx:4` (this fixture runs `probe` once per file), **cancels out** leaving only
>     lib's 1—three datasets (+8 / +16 / dual binary ABBA) are mutually consistent.
> - **8 new fixtures** vs **+16 residual 8**: same batch of files (4.9.1 / 4.9.2 newly added
>   `ref-pool-compile-set/` and `ref-pool-module-identity/`), instrument treats their each check as
>   independent record—same file may trigger different diagnostic entries for multiple
>   `role/context`.
>
> **Conclusion**: **+16 = +8 (aggregated per fixture) ∪ 8 new fixtures**—both counts are correct,
> **different scopes**: t1 reports "8 new" is fixture layer scope, t2 reports "+16" is file layer
> scope (symmetric diff includes new fixtures). t3 review ① +8⊆+16 evidence-dead; **review ② writes
> "don't copy EVIDENCE.md section C verbatim"**—section C missed "new fixture records" ingredient,
> this entry rewritten per t3 review `pass` conclusion.
>
> **IV. Mutation experiments (mechanically reversible, all `scratch.py trial`)**
>
> 8 mutation patches (`target/yx-scratch/yx49/*.patch`, p1 directory scan, p2b use imported name
> removal, p3b script joins pool, p3 flat consumption, p4 std joins pool, p7final self-built 4
> criteria, p8 std closed, p9 vendor joins pool + p_stack2 / p_stack3 stacking surfaces). Each trial
> output includes: HEAD blob → mutated blob → restored blob consistent with HEAD = yes +
> `git status` CLEAN. Local `core.autocrlf=true` requires `GIT_CONFIG_*` injecting `false`
> (`trial_env.ps1`), otherwise patch new lines convert to CRLF causing apply/restore double
> failure—**only affects patch mechanical property, doesn't change tested behavior**.
>
> **V. Process events / environment lessons (must-read for next round handoff)**
>
> 1. **core.autocrlf injection**: local `core.autocrlf=true` must inject `false` via `GIT_CONFIG_*`
>    to guarantee patch mechanical reversibility; see `target/yx-scratch/yx49/trial_env.ps1`. **Next
>    round don't reuse** any trial script without the injection.
> 2. **Same tree trial serial**: `trial` and `cargo build` **on the same tree must be strictly
>    serial**. `trial` internal `git apply` and `cargo build` both touch worktree, parallel will
>    have "patch incomplete when seen by build" or "build and apply deadlock" two race conditions.
> 3. **trial rebuild binary needs confirmation synchronized**: after `trial` completes it rebuilds
>    the in-tree binary (cargo's fingerprint includes source path, **switching tree = full
>    rebuild**), before subsequent measurement must confirm `bin/yaoxiang-rs` synchronized with
>    HEAD. **This round once used residual mutation binary to get fake data**, already rebuilt and
>    re-measured to correct.
> 4. **Second tree yx49-b's creation reason**: AB comparison needs pre-fix binary (62f6e1a59), no
>    ready-made in batch; `scratch.py init yx49-b` opened a second tree (eating another full build).
>    **Tree already released** (see VI), deletion left to batch closure `gc` disposition.
> 5. **cli_e2e two Internal use case fixtures supplemented to legal package (added `[run].main`,
>    assertions untouched) reason**: pre-fix `cli_e2e`'s two `Internal` role pin fixtures **didn't
>    declare `[run].main`** ⇒ after 4.9.1 implementation `compile_project` goes through
>    `ExplicitSurfaces` root call path, can't find `main` ⇒ whole batch 2 fixtures behavior shifts
>    from "Script fallback" to "entry not found"—this is 4.9.1's introduced fallback behavior, not
>    defect. After adding `[run].main` field the two fixtures go through normal package path,
>    **assertions** (exit code / diagnostic code / warning set) verbatim untouched.
> 6. **Second round tree deletion safe way**: after release use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`), it will check "no mount" then delete.
>    This batch already released before closure, next step `gc` naturally cleans up.
>
> **VI. Batch tree state**
>
> Before closure: yx49 already released (4.8 GB freed), yx49-b already released. Two trees no mount,
> state can be re-checked in `scratch.py status`. This task output ends with `scratch.py gc`
> complete status.
>
> **VII. RFC-029f "Known Defects" Table Flip**
>
> Previous version (2026-10-09) described in present tense "three design defects of reference pool"
> (including already-deleted code positions like `collect_project_refs` / `orchestrator.rs:603-648`
> / `orchestrator.rs:393-396`). This batch closure synchronously rewritten to "**Known defects:
> landed (2026-10-16, WBS 4.9 round 1)**": ① current pool root = `collect_package_refs`
> (`helpers.rs:481`), per `ExplicitSurfaces` parsing, no directory scan; ② current pool type
> `HashMap<String, HashSet<String>>` (key = module key), assembly sole entry `collect_ref_pool`
> (`helpers.rs:429-449`), consumer `arms.rs:603-609`; ③ current Script doesn't join pool,
> `arms.rs:611-623`'s `match us.role` default arm. **All file:line refetched on referenced revision
> `3863a9ff0`**. `collect_project_refs` and `orchestrator.rs` already moved to
> `src/driver/{state,arms,helpers}.rs` with P4 4.2.9 split (`c59fb751`), old positions in table no
> longer hit—this is "declaration inconsistent with fact" fix (t3 review note ①).
>
> **VIII. Final gate (closure revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 rows** (two new fixtures
> behavior run, 338a339,340; other 342 rows byte-for-byte unchanged) / IR snapshot 204 zero drift /
> check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process event**: this batch
> one cargo build due to `core.autocrlf` converting script line endings to CRLF, under team scratch
> path `yaoxiang-rs` startup reported `BadImage`, empirically encoding inconsistent with git
> index—passed after rebuild (parallel to 4.6 batch "external full-tree discard" lesson, different
> cause).
>
> **IX. Left (handoff to next round)**
>
> 1. **#450 ownership adjudication**: evidence shows it's "overload resolution's name not passed
>    back to reference count" class, **still reproduces under round 1 implemented form**—dispense
>    per user adjudication Bing flow. **#450 probe**: dual binary same set `{W1002@2, W1005@4}`
>    (empirical, rootless form), no direct relation with round 1 refactor, but #450's fix may need
>    to wait for 4.9.4 (consumer refinement) together—because root cause is "overload resolution's
>    name passed to reference count" needs use item's precise consumer surface, 4.9.4's "real used
>    after import" is the precise version of same criterion.
> 2. **dead_code.rs:683 pointer nit**: comment writes "two pools' assembly scope see driver side
>    `collect_compilation_refs`"—actually `collect_compilation_refs` is fallback path, main path is
>    `collect_package_refs` (`helpers.rs:481`). Comment inconsistent with code main path scope
>    (**not in this batch's write scope**), leave for 4.9.4 / 4.9.5 implementation to correct
>    together.
> 3. **4.9.4 / 4.9.5 preconditions unlocked**: 4.6.1 (`use` path resolution) already delivered by
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs to first decide
>    RFC-029f table incorporation (already laid by this batch's "VII." flip).
> 4. **`yaoxiang check` multi-file `run` never reports dead code family warnings** (WBS 3.4.6,
>    2026-10-15 user adjudication Bing deferred to after 4.9)—this round 1 didn't involve; next
>    round 4.9.4 / 4.9.5 implementation same batch processing.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix)—Round 2 Closure (4.9.4 / 4.9.5 / 4.9.6)

> **4.9 Round 2 Batch Record (2026-10-16 implementation → 2026-10-17 closure)**: covers 4.9.4
> (consumer criterion = real used after import), 4.9.5 (test pool separation), 4.9.6 (method
> reachability scope, issue #450, user 2026-10-16 adjudication A merged into round 2). Closure fix
> (verification-side remedy M1/M2/M4 recompile redo + F1 fix round + PA-2 delete dead branch) and
> registration gap issue #464 (WBS 4.9.7) settled together.
>
> **I. Adjudication chain and commits (post-push true value)**
>
> | Step | Content                                                                                                                                           | Post-push commit | Pre-push local value | Determination method                                    |
> | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- | ------------------------------------------------------- |
> | 1    | fix(util): check explicit path not intercepted by default exclusion table + --exclude always empty (issue #463, P0-0.4)                           | 366bc5a69        | ef959ca55            | Patch blob same                                         |
> | 2    | Stage A = 4.9.4 + 4.9.5 (RefPool per import attribution + Test doesn't contribute consumer surface)                                               | e6cf1aeaa        | 84a6f6978            | Patch blob same                                         |
> | 3    | Stage B = 4.9.6 (#450 per-candidate method reachability; including generated warning-codes.md)                                                    | ddcbb12d0        | cd629b5fe            | Patch blob same                                         |
> | 4    | docs(meta): AGENTS rule 10⑦ (commit period stash window convention)                                                                               | ec94fa9d8        | 9a4ed5f3b            | Patch blob same                                         |
> | 5    | Fix round = F1 (overload sibling overload restore report W1005) + F2 (verification-side M1/M2/M4 recompile redo) + F4 (snapshot generation noted) | b8380e6ab        | 3d9ef6eab            | Patch blob same (36962f318e42…, git range-diff no diff) |
> | 6    | Closure fix = PA-2 (delete strip_unit_prefix dead branch + 029f/CHANGELOG three false statement corrections, register #464)                       | b940300ac        | e13984465            | Patch blob same (patch-id 9bda3cfe4c…)                  |
>
> **Hash explanation**: each push's pre-push git pull --rebase will rewrite local hash;
> "Determination method" column above is all "patch blob same"
> (`git diff <rev>~1 <rev> | git hash-object --stdin` both sides consistent + `git range-diff` no
> diff). This article doesn't cite any pre-push hash as authoritative source.
>
> **II. Three criteria red/green empirical scope** (verifier self-built reproduction, doesn't take
> implementer's criterion)
>
> | Criterion                  | Red state (before binary 366bc5a69 empirical)                                                                                                                                                | Green state (ec94fa9d8 and after)                                                                                       |
> | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
> | 4.9.4 import≠consume       | Declaration surface project check a.yx only 1 W1001 (main's use b.{foo} + foo() rescues a.yx same-name foo; guard only imported not used also rescued)                                       | Same fixture 2 honestly reported (collision name not rescued + unused import not counted as consume); ref_pool 8 passed |
> | 4.9.5 test pool separation | src/util.yx's debug_print rescued by tests/ reference (only 1)                                                                                                                               | Same fixture 2 (test reference doesn't overflow); ref_pool's test_ref_pool_excludes_test_role_references passed         |
> | 4.9.6 method reachability  | Self-built fixture (2 groups disambiguation overload call + 1 not called) 3 W1005 false reports; method_call_args.yx 3 / method_overload.yx 1 (overload covered by full name only reports 1) | Same fixture exactly reports uncalled orphan; method_unused 3 pins passed; corpus call_args 3→0 / overload 1→0          |
>
> **Fix round (F1) further tightened**: not hit resolution's sibling overloads restore reporting
> W1005 (b8380e6ab—same fixture call ->Float reports ->String sibling, call ->String reports ->Float
> sibling, bidirectional line number pin). **Regression boundary (registered #464)**: cross-unit
> (registry resolution) method call doesn't produce resolution items, falls back to name-level
> exemption, cross-unit same-name same-order not reported before—registered 4.9.7.
>
> **III. Process events and lessons (four hard requirements correspondence)**
>
> 1. **PA-1 changed from blocker to existing capability gap (medium)**: round 2 review once
>    registered "cross-unit same-name same-order not reported" as blocker (PA-1); after two fact
>    reconciliations (implementer kept on file + verifier on b8380e6ab/b940300ac dual binary
>    re-measured same fixture set, diagnostic set byte-for-byte consistent) confirmed it's
>    **capability gap existing before round 2** not this round's regression—mechanism = cross-unit
>    method call doesn't produce resolution items, falls back to name-level exemption; disposition =
>    change to medium, register issue #464, WBS 4.9.7 schedule next round.
> 2. **Reviewer publicly corrected before data and withdrew regression determination**: reviewer's
>    initially cited before data ("−42 contains called method") inconsistent with their original
>    output (W1005 count=0)—verifier reconciled and confirmed before capture is 0, reviewer's
>    regression determination **publicly withdrawn** per this; −42's correct scope = 26 files'
>    called method false report elimination (Stage B), traceable to behavior-run exit=0 files
>    (method truly called executed).
> 3. **t13 terminated because contract premise doesn't hold (not implementation defect)**: this
>    task's acceptance premise wrote "cross-unit same-name same-order will report W1005 regression
>    fix", but empirical (t14 dual binary) confirmed this behavior is already-registered gap (#464)
>    not regression—contract premise doesn't exist, task terminated; its probe value taken by t16
>    read-only determination (confirming `strip_unit_prefix` is dead branch).
> 4. **Doc commit's real criterion is "actually build once"**: round 2 closure's doc commit
>    (`042c07860`) made Deploy Docs red on VitePress's Vue template compilation—the angle bracket
>    placeholders like `<rev>` / `<before-exe>` / `<after-exe>` in records didn't go into backticks,
>    were parsed as elements (reporting `Element is missing end tag`, pointing to the `<rev>` line);
>    markdownlint can't catch this type, and pre-commit's markdownlint hook is judged as
>    `no files to check` skipped in commits containing only docs (empirical). Disposition:
>    placeholders uniformly go into backticks, run `npm run docs:build` locally before push (fix
>    commit `0c837e1b4` with local build exit=0 evidence); if using `--no-verify` to commit,
>    commit-msg check needs to be manually run.
> 5. **Contract description not synchronized process omission**: t14's task description text
>    residual PA-1 old hypothesis (writes "b.yx reports W1005" expectation), verifier actively
>    pointed out in report "per contract field judgment, description text not counted" and asked
>    team lead to confirm—disposition = take task contract field as adjudication basis, description
>    text left as process lesson (**task description must be synchronized with latest adjudication
>    chain when reassigning/dispatching**).
>
> **IV. Mutation experiments (mechanically reversible, all scratch.py trial)**
>
> Round 2 verifier total 9 patches (under target/yx-scratch/yx49/, m1b/m2b/m4b are F2 post
> recompilable versions—initial M1/M2/M4 commented out if condition line leaving dangling block
> body, judged as "compile failure mis-recorded as criterion red" by t8 review F2 and voided; redone
> version per patch cargo check exit=0 + trial red). M3 (collision attribution reentry) initial
> version was real mutation, empirical ref_pool 5 criteria turned red, retained original judgment.
> M1b anchor point refetched on b8380e6ab (post-F1 fix probe section refactored, old anchor
> ineffective). All trials: HEAD blob → mutated blob → restored blob consistent with HEAD + git
> status CLEAN. Environment injection uses trial_env.ps1 (core.autocrlf=false via GIT_CONFIG_*).
>
> **V. Warning surface full-volume AB comparison (scope first)**
>
> Instrument = warn_surface_r2.py (self-built parser, (file, code, line, column) 4-tuple symmetric
> diff, project form channel cwd=file parent dir, full path key); command =
> `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>` (337 files × 2
> binaries). **Scope**: this round's before capture already contains all fixtures ⇒ matched set =
> all 337, doesn't have round 1's +8 (matched increment) / +16 (full set) dual scope issue. Result:
> only_before = 42 W1005 (26 files, all are called method false report elimination = Stage B's −42;
> per-file check they're all on behavior-run exit=0 baseline, method truly called executed—**no one
> true dead code rescued**) / only_after = 3 W1001 (ref-pool-import-use's a.yx:4 check surface ×2 +
> ref-pool-test-pool's ../src/util.yx:2 = Stage A criterion surface). **Snapshot generation (F4)**:
> Stage A time point's after capture (Stage B pre 57 residuals) and final version (Stage B post
> 15+2=17) are **two generations**, must note when citing; also 1 exception
> (interface_member_conflict.yx's Dog.name@18) is field access form, not in Stage B's W1005 removal
> set, consistent before and after.
>
> **VI. Final gate (closure revision b940300ac)**
>
> lib 2761 passed / integration 288 passed / IR snapshot zero drift (snapshot 7 passed) / corpus
> baseline zero drift (byte count three-way comparable: baseline = b8380e6ab = b940300ac) / clippy
> -D warnings 0 / docs-truth exit=0 / check_tracking exit=0 / generator already run no product
> change. **Gate check arrangement (rule 13)**: this batch's push CI / Deploy Docs / Auto Translate
> run conclusions are registered by scheduled check (run id + conclusion + continuous push coverage
> relationship: descendant run covers previous), not waiting within session.
>
> **VII. Batch tree state**
>
> yx49 and yx49-b both released (gc uniformly deleted at closure); no new directory added outside
> repo in this batch.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix)—Round 3 Closure (4.9.7)

> **4.9 Round 3 Batch Record (2026-10-17 implementation → 2026-10-17 closure)**: covers 4.9.7
> (cross-unit method call's resolution registration and receiver attribution, issue `#464`—WBS 4.9's
> last three-level task; after this round closure 4.9 whole second-level task kanban checked). Chain
> = production side t19 → independent verification t20 PASS → review t21 needs_revision (F1:
> injection surface false report) → fix round t24 → independent verification t25 PASS → review t26
> PASS (incremental, verdict=pass). All commit numbers are post-push true value (determination
> method = patch blob same; `git diff <rev>~1 <rev> | git hash-object --stdin` both sides
> consistent).
>
> **I. Adjudication chain and commits (post-push true value)**
>
> | Step | Content                                                                                                                                                                                         | Post-push commit | Determination method   |
> | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ---------------------- |
> | 1    | Production side: cross-unit method call produces D20 resolution items (`feat(typecheck)`, t19)                                                                                                  | `b8b3cc3f6`      | Patch blob same        |
> | 2    | Driver side attribution: method reachability per definition unit gives marker, resolution-recorded name group stops pool's name-level exemption (t20 verification PASS / t21 review catches F1) | `d5c00235a`      | Patch blob same        |
> | 3    | Injection surface fix round F1: candidate injection per "name ← providing module" **per-pair** filter, eliminate E1107 false report (t24, including F2 statement and F3 form single point)      | `d5dd1a37b`      | Patch blob same        |
> | 4    | Pin test repositioning: `src/frontend/core/typecheck/tests/environment.rs`'s F3 pin moved to declaration surface `src/driver/tests/overload_injection.rs`                                       | `c2f887843`      | Patch blob same        |
> | 5    | dev top                                                                                                                                                                                         | `e9675c9dd`      | merge commit, see "II" |
>
> **II. Hash mapping and dev top's two merges (source unconfirmed, don't write as member action)**:
> round 3's four commits (table 1–4 above) all pushed to origin/dev; `git worktree list`'s main
> mount is at `e9675c9dd`. dev top has two **non-this-team-command-surface-produced** merge commits
> (`5da05b885` / `e9675c9dd`, reflog shows as `pull --tags` + push form), source unconfirmed—per
> rule 10④ (citation must be resolvable by anyone) and rule 13 (gate to dedicated check) same
> closure scope, here only register fact and reflog evidence, no member attribution; also see "V."
> lesson ② (push attribution convention must be verifiable).
>
> **III. D20 contract extension actual form (key form and consumer point)**: production side packs
> "candidate / mixed IR name / declaration-period parameter name" three parallel tables **in same
> order** into `ModuleInfo.method_overloads` (`src/frontend/module/mod.rs:157`) carried across units
> with unit, registry per module key ascending deterministically merged
> (`src/frontend/module/registry.rs:150`); typecheck arm per this unit's import face per-pair
> injection (`src/driver/arms.rs:354` → `visible_method_overloads`, `src/driver/arms.rs:1521-1579`:
> name ← providing module per pair, same name conflict full name skip, own key position not
> externally merged), form single point `append_overload_group`
> (`src/frontend/core/typecheck/environment.rs:399`: three tables same-order append + equal length
> debug assertion); consumer side attributes D20 resolution product per **definition unit**
> (`MethodMarks`, `src/driver/helpers.rs:569-573`; `typed_owners` `:591`; `resolved_method_marks`
> `:621`), and makes resolution-recorded name group stop pool's name-level exemption
> (`src/frontend/core/typecheck/passes/dead_code.rs:252-267`, `:825-841`). Key form = item name
> `{Type}.{method}#k` (`{Type}` is definition-side bare name; `#k` declaration order).
>
> **IV. Key scope and false report defense empirical (fix round F1 and five residual boundaries)**:
> F1's red state is pre-fix `d5c00235a`'s injection surface "name set × module set" combination
> merges another module-defined same-name type candidate into same key position ⇒ `use a.{Temp}` +
> `use b.{Other}` (a also defines Other, b also defines Temp, methods same-name same-order) fixture
> `check` reports 2×E1107, exits 1 (previously successful program compile fails); after fix
> `check`/`run` both exit 0, 4 W1005 honestly report two sides' **uncalled** same-name definitions
> (reverse under-report surface also checked). Merged fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`; driver-level pin
> `src/driver/tests/overload_injection.rs` 3 + `src/driver/tests/method_unused.rs` round 2's 5 #464
> criteria and 4 #450 old criteria all green; main acceptance fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` diagnostic set consistent item by item
> before and after fix (not regressed). Five residual boundaries and per-item empirical (separately
> `check` definition file not reported / name layer ambiguity E5006 full name skip / boundary (c)
> rewritten per t20 empirical / inline alias and `ns.T` not injected into candidate surface / whole
> import `use M` and std candidate surface not produce items) full text see
> [029f](../../rfc/accepted/029f-target-semantics.md) §Cross-unit method reachability attribution
> "Residual boundaries", not repeated here.
>
> **V. Process events and lessons (four types continuation)**:
>
> 1. **Contract wording too narrow becomes defect directly**: F1's root cause is contract only wrote
>    "only inject module surface that can be named by bare type name", didn't write "per-name-pair
>    attribution"—implementation per literal took "name set × module set" combination, review caught
>    E1107 false report. When describing behavior boundary in contract must write **combinatorics**
>    dead (per-name pair / full combination), can't leave for implementation to understand.
> 2. **Convention must be verifiable: "push by team lead" only written in dispatch message ⇒ when
>    third-party push/merge appears can't attribute**. This round dev top appeared two
>    source-unconfirmed merges (see "II"), finally via reflog back-trace and withdrew one wrong
>    attribution. Verifiable form = members don't run `git push`/`fetch`/`pull`, synchronization
>    uniformly `pull --rebase`, use reflog to check.
> 3. **Infrastructure unstable, save patch first then swap person**: this batch total 5
>    unrecoverable faults (3×TRANSPORT, 1×INVALID_REQUEST (thinking protocol), 1×background
>    sub-agent no-message exit); disposition convention = already-produced evidence and patches
>    first fall to in-repo path (`target/yx-scratch/<batch>/`) then swap person to continue, same
>    session consecutive two failures don't bet on third.
> 4. **Doc commit's real criterion is "local really build once"**: round 2 closure's doc commit
>    (`042c07860`) made Deploy Docs red on VitePress's Vue template compilation (angle bracket
>    placeholders like `<rev>` not in backticks parsed as elements, reporting
>    `Element is missing end tag`); markdownlint can't catch this type, and pre-commit's
>    markdownlint hook is judged as `no files to check` skipped in commits containing only docs
>    (empirical twice). Fix commit `0c837e1b4` with local build exit=0 evidence. This round the
>    lesson's red→green run: Deploy Docs **39011946793** (red) → **39014696984** (green, see "VI").
>
> **VI. Round 2 gate conclusion supplementary registration (rule 13, scheduled check backfill)**:
> round 2 closure commit `b940300ac`'s push—CI **39011946778** success / Auto Translate
> **39011946796** success / Deploy Docs **39011946793** **failure** (VitePress build red, i.e.
> "V.4"'s angle bracket placeholders; fixed by `0c837e1b4`); fix commit `0c837e1b4` after push three
> green: CI **39014697000** / Auto Translate **39014697044** / Deploy Docs **39014696984**.
> Continuous push coverage: CI@`0c837e1b4`'s **39014356642** was canceled by subsequent run per
> concurrency (cancel ≠ fail, descendant run covers previous, consistent with round 2 §VI scope).
>
> **VII. Final gate (closure revision `e9675c9dd`, local number; CI conclusion to scheduled check,
> see "VIII")**: lib 2770 passed / integration 289 passed / yx_multifile_runner 1 passed / IR
> snapshot 7 passed zero drift / clippy `-D warnings` 0 / docs-truth 0 / generator idempotent
> (re-run `scripts/docs/gen-error-code-docs.py` then `git status` clean); corpus baseline 346 → 347
> → 348 rows (each +1 behavior record, other rows byte-for-byte unchanged; baseline
> `tests/baselines/corpus-parity.jsonl` at `e9675c9dd` total 348 rows).
>
> **VIII. Gate check arrangement and conclusion (rule 13)**: this batch's push CI / Auto Translate /
> Deploy Docs conclusions are registered by scheduled check (not waiting within session, also not
> writing "no wait" as "already green"). **Backfill (2026-10-17 scheduled check)**: `d5c00235a`'s CI
> `39023417670` = **failure**—Docs Truth reported "error code doc lagging behind code
> table/locales", i.e. that one missed generated product same commit; fixed by subsequent
> `d5dd1a37b` (regenerate product): `5da05b885` CI `39027788246` / Deploy Docs `39027788268` / Auto
> Translate `39027788340` all success, covers previous; `e9675c9dd` CI `39028827871` / Deploy Docs
> `39028827896` success; `d0cfc4e62` CI `39033963240` / Deploy Docs `39033963278` / Auto Translate
> `39033963261` success. Three's runs each ran to terminal state, this round no concurrency cancel.
> Lesson: **"local generator idempotent" doesn't equal "generated product already merged in same
> commit"**—should check CI conclusion per commit before closure, not only look at latest one.
>
> **IX. Batch tree state (closure gc, status output)**: before closure `scratch.py status`—t25
> (holder=-) and yx49b (holder=-, previously held by deactivated member verify-r3 stale, team lead
> `release --force` released) two trees both no holder; `gc` deleted two, freed about 12.7 GB; after
> gc `status` = "no batch tree", `git worktree list` only main mount. Also:
> `target/yx-scratch/yx49r3/` and `yx49r4/` are not batch trees (no meta.json), are members'
> evidence/probe directories, not "residual batch tree". No new directory added outside repo in this
> batch.
