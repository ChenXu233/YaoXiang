# WBS Historical Notes (Implementation Record · Audit · Decision Trail)

> This document is a **historical note** for [09-execution-wbs.md](./09-execution-wbs.md) (the
> construction task table): implementation records, audit findings, decision trails, and corrections
> for each phase. **Division of labor**: 09 describes only the **plan** (kanban, three-level task
> table, dependencies, risks, acceptance); this document records only **history** (what was done,
> what was found, how decisions were made), appended chronologically without rewriting history.

## P2 Establishing the Equivalence Criterion Baseline

> **P2 Implementation (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green.
> **D38 Prophecy Fulfilled**—the green process empirically discovered and fixed three real defects
> in ir_gen (see 06 §G6 for details): method locals table truncated to parameter count (local_count
> underestimated, runtime relies on Frame::set_slot for fault tolerance), unannotated parameters
> dropped by filter_map (three places, params metadata misaligned with frame layout), match result
> slot depends on frame's implicit Void initialization (now explicitly pre-initialized). The
> realistic semantics of jump target is the flattened instruction index (not the label keying in the
> 07 text), CFG is built at instruction level—registered in the verify.rs file header. **Commit
> Correspondence**: ir_gen fix = `a9ea199b`; verify main body = `d5b18390` (⚠ Parallel flow
> COMMIT_EDITMSG collision caused its message to be mislabeled as "docs(rfc): RFC-027a…", but the
> content is actually verify.rs + six test files, buried in the history middle and cannot be
> amended, hereby registered).
>
> **P2 Narrowing Part Implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> hemostasis channel pre-conditions unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; the judgment contract and single-file corpus share the same
>   `TestFileSpec` (library-side single point), binary positioning/subprocess launching abstracted
>   to `tests/common/` and yx_runner migrated synchronously (prohibition three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. The trigger source uses the
>   `SumUpTo(3, r)` form nailed down by the rfc027 test (return position/call point/binding
>   position)—the `Sorted(3)` annotation example in 02 would be substituted and evaluated in the
>   current code to directly Disprove, not showing Unproven silence, the Sorted(3) in 07 pseudocode
>   is only illustrative. Red state empirical data is in the test file header.
> - The two red criteria are hung with `#[ignore]` resident per the 2.4.5 red skeleton precedent:
>   the default suite is fully green, `-- --ignored` reproduces the red state; **when P3 fix is
>   implemented, removing the attribute turns it green**.
> - The reason for the intentional deviation from the 07 (code, file, line) table in comparison
>   normalization (removing file/line, comparing only error code sets) is written in the 2.4.1 test
>   comment.
>
> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> are **moved out of P2**: the `Program` / `Obligations` types they reference only exist in P4, and
> P2 cannot compile. The former is P4's 4.1.4 (no double registration), and the latter's `#[ignore]`
> red skeleton is merged into 4.3.1.
>
> **2.3.3 Implementation (2026-10-07)**: C4 mandatory coverage list item-by-item check (07 §143), 10
> types of semantic corpus layers fully covered and all within the corpus-parity.jsonl diff baseline
> (26 representative files 26/26 hits, grep empirical proof)—ref/borrow/move 05-ownership full
> directory (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep, etc.), closure
> capture closures + closure_arg_inference + spawn_capture, currying curry_value_fix +
> curry_tail_expr, spawn 04-concurrency ×13, iterator for spawn_for etc. 16 files, sum type
> sum_type_* + pattern_or_guard + match_call_scrutinee, existential forced point
> interface_dynamic_dispatch (Vec(Animal) heterogeneous existential type wrapping triggers
> existential_coercions) + interface_rebind_dispatch, `?`/Try propagation question_propagation +
> option_try + try_methods + user_try_type, method overload method_overload (+
> method_overload_ambiguous_err in diagnostic diff surface), refinement constraint Drop sequence
> refined_* ×5 + 05-ownership driven ReleasePlan (span key contract has another 2.4.4 red criterion
> resident). Performance baseline `benches/pipeline.rs` checked in (07 §145 three smoke tests of
> which two): `corpus_compile_frontend` full corpus compilation first measurement 17.6s,
> `cli_cold_start_hello` process cold start 169.7ms; interpreter throughput is already covered by
> the benches/lib.rs hotpath group (interp_fib_recursive_27 / interp_loop_steady_10m), no duplicate
> construction. The regression gate check-perf-regression.sh is attributed to P9 unified list (2.5).

## P3 Fix Correctness Vulnerabilities (Minimum Solution)

> **Error Type Annotations for Already-Implemented Rows** (retrospective annotation, does not change
> history): 3.1 = B ｜ 3.2 = B (defensive empty arm, criterion turn green borne by 3.1) ｜ 3.3 =
> A ｜ 3.4.1 / 3.4.2 = A (new W code) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 /
> 3.4.7 = D (user-visible surface, already adjudicated by user) ｜ 3.4.8 ①② = C (main) + A
> (incidental three places) ｜ 3.5 / 3.6 / 3.7 / 3.8 = D (all adjudicated by user).
>
> **P3 Diagnosis (2026-10-12, user requested)**: Conduct an account diagnosis in response to the
> perception of "P3 items continuing to grow".
>
> **I. First clarify the scale (the perception is amplified by two account defects)**
>
> 1. **Duplicate numbering**: `3.5.x` is shared by two task families—(a) **029g `pub` visibility
>    convergence** on 2026-10-08 (7 three-level: lexical and parser branch / AST `is_pub` / checker
>    side / dead code exemption convergence / peripheral echo / std and diagnostic text / language
>    reference convergence, all checked); (b) **3.5 Higher-order type parameter call path
>    completion** registered on 2026-10-10 (3 three-level). The two share `3.5.x` ⇒ cross-reference
>    ambiguity (**2026-10-12 renumbered per user adjudication: the 029g family changed to
>    `029g.x`**), and caused 029g's rows to be miscounted into P3. **Pending adjudication**:
>    Renumber the 029g family to `029g.x` (or move into its belonging stage), and synchronize other
>    references in the document.
> 2. **Missed checks**: The 4 three-level (3.7.1–3.7.4) of `3.7` are all completed (d922a551 /
>    38bd4b9e) but not checked ⇒ this item is corrected this time.
>
> **II. P3 Actual Account (12 second-level rows)**
>
> | Row                                                                  | Status                                                                                                                                                                                | Disposition Suggestion                                                                                                           |
> | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                      | Completed                                                                                                                                                                             | —                                                                                                                                |
> | 3.4 Silent drop point additions (8 three-level)                      | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (precondition 4.2.1 completed on 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (precondition 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4/3.4.5 unblock with 4.3.1                                                                        |
> | 3.5 Higher-order type parameter call path completion (3 three-level) | Not started (declaration side alive, call side broken)                                                                                                                                | Keep P3; **does not block refactor main line**, but is "accepted feature unimplemented", higher priority than pure cleanup items |
> | 3.6 Function type value call path (1 three-level)                    | Not started (semantics cut (i))                                                                                                                                                       | **Keep**: correctness defect, clear criteria                                                                                     |
> | 3.7 init segment call name qualification (4 three-level)             | **All completed**                                                                                                                                                                     | Check and close this time                                                                                                        |
> | 3.8 native shadowing (1 three-level)                                 | Not started                                                                                                                                                                           | **Stuck at decision** (determine name resolution priority first), not a scheduling issue                                         |
> | 3.9 / 3.10 / 3.11                                                    | Completed (2026-10-12)                                                                                                                                                                | —                                                                                                                                |
> | 3.12 Runtime mode executor configuration propagation (1 three-level) | Not started (Class D, user adjudication B: separate team)                                                                                                                             | **Suggest moving out of P3 → P8 (execution layer)**, pending adjudication                                                        |
>
> **IV. Leverage of 4.3.1 (main line marker)**: `4.3.1 obligations.rs + assert_drained()` is also
> the precondition for **3.4.4**, **3.4.5** and **4.5.1** (cross-layer contract PlanId-ization) ⇒
> taking it once advances the main line and clears two P3 stranded items. 3.5 (HKT), 3.6 (function
> type value call) and 4.6.1/4.9.1 have **no unsatisfied preconditions**, all are doable anytime;
> 3.8 is stuck at semantic decision (user-defined priority vs. explicit diagnostics).
>
> **III. Mechanism Suggestion (prevent recurrence)**: Newly registered second/three-level rows
> **must carry two fields**—"belonging stage" (which P it really belongs to) and "whether it blocks
> the main line"; `registered pending scheduling` is no longer the default state, and unmarked is
> considered unschedulable. In this way, bloat is visible and sortable, rather than undifferentiated
> accumulation. **Pending adjudication**: Whether to write this article into the registration
> convention of this document and `src/dev/coding-rules.md`.
>
> **P3 Implementation (2026-10-06)**: 3.1.1 `proof_calls` narrows visibility + unique getter
> (recorded as `pub(crate)` at the time; **2026-10-07 verified that the actual implementation is
> `pub(super)`**—the `#434` decision referenced in the code comment has no record on the docs side,
> see 3.4.3; full privacy requires rewriting 11 construction literals, not taken during the
> hemostasis phase; will be closed when P4 Obligations migrate); execution mechanism abstracted to
> `src/frontend/proof_execution.rs` (prohibition three single point—cannot be placed in typecheck/,
> that would create L3→L4 reverse dependency); 3.1.2 supplements **four entries** per 02 evidence
> chain step 5 (WBS row number missed compile_project `:99`, must be supplemented and registered per
> 3.1.1 acceptance "criterion turn green"); 3.2.1 empty arm changed to bookkeeping (proof_calls
> throws up) + diagnostics (`into_result` established conversion path); 3.3.1 SOLVER slot
> Option-ized, Z3 missing conservatively downgraded per `SMTResult::Unknown` (E2031 family
> diagnostics replace process crash). The two vulnerability criteria remove `#[ignore]` and turn
> normal green, supplement corpus fixture `proof-obligation-honored`; check-fanout inventory
> migrated accordingly (wiring point 19=inventory). Empirical test: criteria 2.4.1/2.4.3 turn green,
> full suite lib 2494 / integration 272 / corpus dual-layer fully green.
>
> **3.4 Audit (2026-10-07, post-P3 implementation review)**: For defects of the type "failure
> produces no signal" (definition see 02 §41-52), without changing the code, 8 more locations were
> identified, all registered as 3.4.x in the above table. Methods and evidence:
>
> - **Consumer-side inventory** (16 obligation fields grep all-warehouse read points one by one):
>   `variant_ctor_calls` **zero consumers**—the only read `checker.rs:1468` is "self-pour into
>   `TypeCheckResult`", IR side takes form detection `ir_gen.rs:7374`; `rfc011b.rs:659/688` only
>   asserts the production end, which is exactly the "only test production end, naturally immune to
>   consumer end defects" form criticized in 02 §186 → 3.4.4. The result-level field
>   `method_overload_ir_names` is also zero external consumers, IR uses parallel
>   `overload_resolutions` → 3.4.5.
> - **Stage coverage review**: 02 §91 table row 1 (four entries "no proof_execution") has been fixed
>   by 3.1.2, current state is 5 places share one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 remain unchanged (multi-file `run` still
>   does not output W1003, still does not run dead code family) → 3.4.6; LSP non-in-project path (02
>   §77 "manual lex→parse") also does not run proof → 3.4.7.
> - **Same-shape empty arm scan** (`Unproven` all-warehouse consumers only four places): checker
>   three refinement branches (covered by 3.1), the ownership arm fixed by 3.2.1 (**source code
>   unreachable**), and `environment.rs:422` (**reachable, zero diagnostics, zero bookkeeping, and
>   zero registration in the all-warehouse docs**) → 3.4.1, the only "all-unknown" living
>   vulnerability in this audit.
> - **Solver acquisition side**: `default_solver()` three consumer forms are different
>   (predicate.rs:32 process singleton / checker.rs:1266 per `check_module` injection /
>   ownership.rs:621 per back-edge judgment new)——3.3.1 only removed the panic one; the **silent
>   side** of the other two belongs to 3.4.2, **singleton-ization and cache hit** belongs to P4's
>   4.4.2 (no double registration).
> - **Not counted in 3.4's existing attribution**: 2.4.4 (`ReleasePlan` span key mismatch, 168 files
>   / 373 keys not consumed, Drop silently lost) is still P2 red state resident, attribution
>   4.5.1/D20 unchanged; 4.4.1 (layer order declaration opposite to actual, `equivalence` layer not
>   in pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger downgrade) already on P4 books.
>
> **Execution Convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 preconditions, can be implemented
> on-site at any time (suggest doing 3.4.1 first—the only "all-warehouse zero registration" living
> vulnerability, with the smallest change surface); 3.4.4–3.4.8 are executed with P4, and their
> criteria depend on the obligation ledger (4.3.1) or the unified Driver (4.2.x). **3.4 does not
> establish a new criterion system**, uniformly reusing the existing C2/C3 and P9's 9.2 gate.
>
> **Batch 3.13–3.16 Implementation Record (2026-10-13 registered → 2026-10-15 closed)**: User
> 2026-10-14 adjudicated merging four relevance small items (#451 method binding panic / #452 named
> actual argument into resolution / #453 dead inference entry / #454 native export named actual
> argument—the last item is a boundary discovery when verifying 3.14, on the same day through
> attribution experiment verified **not introduced by #452**, user adjudication ① merged into this
> batch) into one team for processing. Closure status: all four items completed, adjudication
> registered as RFC-039 **D60** (six binding rules + shared criterion single-point).
>
> **I. Two contract corrections (lesson: the contract itself can also be wrong)**
>
> 1. **Acceptance fixture self-contradictory**: #452 task contract simultaneously requires "mixed
>    actual arguments output 20" and "position and name fight for the same position report error",
>    but the pinned fixture `Temp.pick(self, a)` has only one value formal parameter—the same actual
>    argument combination is both "mixed" and "conflict". The first implementer honestly reported
>    the blocker and set the task to failed, overall revert (not taking silence for green light).
>    Disposition: Task reassignment + contract revision—giving **six binding rules** and **8
>    value-level matrix** (fixture changed to two value formal parameters `pick(self, n, a)`) then
>    attempt 2 completed by the second implementer. The full text of the six rules has been verbatim
>    incorporated into D60.
> 2. **inScope misses mechanically necessary surface**: The #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 was registered, only `code-tables --fix` was
>    run, **did not re-run** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth red ("source code
>    registered/emitted but documentation missing 1: E1110", source code 154 / documentation 153,
>    run **38518624964**—the only CI red line in this batch). Fix = mechanical regeneration (8 files
>    +83/−70, only touching `docs/src/reference/`), reconciliation 154/154, CI 38520209026 green.
>
> **II. Review → Fix → Re-review two closed loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed with resolution ⇒ calls containing named actual arguments are silently silent in
>    both multi-candidate zero-hit and single-candidate hit paths) → fix round added outward code
>    **E1111 (argument_order_invalid)** (code table + locales + RFC-013 + generator four-piece set
>    same submission; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared parsing entry + native arm settles by name) → independent
>    verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations through
>    `scratch.py trial` mechanically reversible replay all caught, restoration byte-by-byte
>    consistent) → round-1 review needs_revision (F1 = typecheck native arm **inlined copy** of a
>    binding criterion, missing rule ④ ⇒ method and native same-form call **adjudication
>    divergence**—method reports E1111, native silent exit=0; ir_gen has another third stock copy
>    with old code E1015) → convergence round unified the criterion to a single implementation
>    `bind_args_to_params` (`arg_binding.rs:35`, four call points `expressions.rs:2727` / `:5416`,
>    `ir_gen.rs:7776` / `:8279`) → round-2 review caught the **regression** introduced by the
>    convergence round (IR convergence block lost native omittable position ⇒ originally successful
>    programs with omitted optional positions were misreported as E1010) → fix round (optional
>    position criterion and formal parameter name co-output from `signature.rs:236` unique exit) →
>    round-3 review PASS (independently reproduced red→green; under mutation the in-warehouse pin
>    board FAILED empirical proof criterion can fail). **Lesson**: Review must cover "unmodified
>    branches of similar paths" (in this batch, both needs_revisions were due to probes all falling
>    on the hit surface); the fix round itself can also introduce regression (omittable position),
>    caught by the next round of reviewer change.
>
> **III. Process Events (Honestly recorded, not pointing to individuals)**
>
> 1. **External whole-tree discard (2026-10-14 around 17:14)**: The working tree was externally
>    restored in silence—the implementer's fix-in-progress (typecheck/ir_gen/code
>    table/locales/RFC-013 etc.) all disappeared, **at the same moment the two review documents
>    edited by the user themselves also disappeared from git status** ⇒ judged as repository-level
>    discard (most likely user's manual cleanup/IDE "discard all changes"). Three evidence rules out
>    group members: stash empty, no new commits, reflog no reset. The implementer replayed the
>    work-in-progress per context and pushed; discipline changed to "**first save patch to scratch,
>    then collect red state**".
> 2. **Verifier ran out of room, change of person**: The independent verifier midway through
>    verifying 3.16 ran out of room (no closing message), changed person per load rules—the
>    successor continued using the already ready products (green state results, corpus equivalence
>    script, mutation plan), verification finally PASS. Positive precedent of partial product
>    handover.
> 3. **Review round 5 rotation and historical round withdrawal**: When rotating review per load
>    limit item ⑤, `remove_member` withdrew the **already adjudicated terminal review/verification
>    rounds** back to the shared pool, and the scheduler immediately re-ran (t3/t8 were re-taken by
>    new members, duplicate work and blocked downstream). Disposition: Team leader took over and
>    cancelled the withdrawn historical rounds (t3/t8/t13), added AGENTS.md item 12 accordingly
>    (dispose withdrawn rounds immediately after person change). The closure record of this batch is
>    registered per this item.
> 4. **scratch governance (about 6.3 GB)**: In the early stage of the batch, each task had an
>    external tree, 6 directories scattered outside the repository (about 780 MB, cleaned) + two
>    members' external verification trees (about 2.2 GB / 3.3 GB, self-deleted after use). Produced
>    governance tool `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and
>    AGENTS.md item 9 (batch tree unique entry), item 10 (verification environment six rules:
>    mechanically reversible mutation / line number taken from referenced revision / CI exclusive
>    gate must run etc.), item 11 (writer load limit), item 12 (dispose withdrawn rounds after
>    person change) and top meta rules two (no private content / verify before asserting).
> 5. **Contract defect (not pointing to members)**: The #454 fix round's contract inScope only
>    circled 2 files, actual mechanically necessary surface 8 (item 6 is to prevent exactly
>    this—contract template should preset mechanically necessary surface list); in addition, the
>    task's final state field was once occupied by probe load, the formal acceptance/command
>    evidence was supplemented by the team leader per the convention.
>
> **IV. Remaining Items (Not fixed in this batch, registered for future reference)**: round-3 review
> three low-severity findings——① Method path's `build_arg_slots` (`ir_gen.rs:521`) is still the
> **second copy** of the criterion (stock, per the previous round's caliber registered with closure,
> not converged)——**2026-10-16 pre-release audit supplementary note three points**: (a) It has
> **two** call points: `ir_gen.rs:7888` (method binding path, using the bound function
> `binding.function`'s formal parameter name) and `:7975` (overload resolution path, using the
> resolved mangled name leaf segment), the original registration only wrote "`ir_gen.rs:521`
> vicinity"; (b) It **has diverged from the shared implementation `bind_args_to_params`**: missing
> rule ⑤ (duplicate name → E1015), rule ⑥ (missing required formal parameter → E1010) and the
> "omittable position" concept, and hard-codes the receiver position as `1 + i`; the two call points
> `continue` uniformly for empty slots (comment writes "diagnostics attributed to typecheck (it
> guarantees no empty slots if resolution passes)"), which is a **hanging invariant written in a
> comment, with no assertion backing**; (c) **RFC-039 D60's "all call points converge to it"
> enumeration missed these two places**, which is a mismatch between declaration and fact, and the
> referenced line numbers were not re-taken on the referenced revision, and the D60 text has been
> corrected accordingly with this item; ② The batch verification tree (worktree) `cargo fmt --all`
> error is a worktree workspace artifact, the gate list should note fmt goes to the main repository;
> ③ Repair-type contract templates should preset mechanically necessary surface into inScope (see
> III.5).
>
> **V. Final Gate (Closure revision, local two implementation commits + one document commit unified
> push—the only push in this batch)**: fmt --check 0 / clippy -D warnings 0 / `cargo test --lib`
> **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** / `--test yx_runner`
> 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 / corpus_probe **342
> lines consistent line-by-line with baseline** (the only regenerated line:
> `named_arg_duplicate_err.yx` E1015@14 → E1111@16, expected code and line number synchronized,
> manually reviewed) / IR snapshot zero drift.
>
> **VI. Run Links (Push-side Evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (MSRV/Test in the same run were cancelled by
> subsequent pushes per concurrency—cancellation ≠ failure, covered by descendant run); governance
> tool push → CI 38546373435 / 38561405117; WBS registration (3.16 row + table head 56→57 / 147→148)
> → CI 38528560364. The CI / Deploy Docs run of the closure push itself is supplemented to the task
> record after the push.
>
> **3.4.8 ③④ Implementation (2026-10-11, 8ecf56f93)**: ③ `CHECK_STAGES` tail added `Linking` (**pure
> merge**, the whole block skips `is_bin_role` entry check—E3020 criterion still belongs to
> `RoleClassification` to prevent double reporting; another hard reason: the run path judges "has
> main" by merged IR, under collection semantics "entry itself errored → no IR" would be misjudged
> as missing main and hard-aborted) → `Monomorphization` (check form); `ir_generation_check` retains
> unit IR (**in-memory only**, for pure merge and mono consumption, not externally sent/not written
> to disk/not generating snapshot—if not retained the aggregation request is empty, mono falsely
> greens with `NoObligations`); mono failure in Check goes through per-file collection channel
> (diagnostics attributed to entry file + `failed_stages`, not setting failure), MultiFile keeps
> `fail_with`; `into_outcome` changed to "first take collection then judge failure". ④ Through
> empirical **zero production change**: full program BFS already covers cross-unit mutual recursion
> (pathological → E3005, well-formed three-layer chain executes correct value), delivered four
> groups of e2e + one mono single test as living criterion.
>
> **Verification Chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross comparison with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 examples + dual binary comparison) all pass; t5 cross-form matrix
> (single/multi/check three forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increase, not decrease).
>
> **F1 (Discovered during verification, D59 A/B class, fixed in same round)**: For standalone
> multi-file check without project root, the aggregation request name (`lib.f`) and unit IR
> definition key (`f`) are from different sources → mono falls into "native no IR definition" silent
> `continue`, check reports 0 error/exit 0. Fix: The aggregation side added a reverse lookup jump
> per each unit's **real definition key** (precise enumeration + verbatim equality + ambiguity
> abandonment, not suffix heuristic; project form not registered, existing path unchanged). **Errata
> (same day)**: This reverse lookup jump has been **deleted** in a subsequent regression fix; the
> root cause is solved from the naming caliber by decoupling `module_key`'s one-duty-two-uses (added
> `script_mode`)—IR definition key = IR call name = aggregation request name = `{unit_key}.{name}`.
> **Regression Record**: After ③ implementation, CI's `Check tutorial/guide code examples compile`
> red (258 blocks / 10 failures: 6 same-file same-name rebinding + 4 collided with `<std/list>`,
> rootless check not restricted triggering `link_module_irs` conflict check); fix = ir_gen split
> `script_mode` (qualified name ⊥ Script/Bin, E3023 gate changed to look at it) +
> `ir_generation_check` per unit always pass unit key + `.with_script_mode(rootless)` + Script
> top-level same-name rebinding "latter takes effect" (consistent with run path existing behavior);
> verification = documentation gate 258/0, three binary comparisons (HEAD/③ tree/fix tree) three
> types of collision fixtures, check form E3005 no regression, E3023 not opened.
>
> **F2/F3 (Accepted)**: mono diagnostics attributed to entry file entries, span can cross units
> (same caliber as run path `path=entry`); Check form holds one ModuleIR per unit (in-memory only,
> released at the end of check).
>
> **Issues Left to Subsequent (Not This Round's Regression)**: ① E3005's help text still writes
> "internal error; please report this issue", which is inconsistent with resource protection
> semantics (precedent: `e3xxx.rs:47` once moved similar "user writing error" out of
> E3005)—registered pending scheduling; ② multi-file E3005's span rendering label misalignment
> (reproduced with HEAD binary, pre-existing); ③ The request collection side has **type parameter
> position dependency** (generic calls where type parameters are not in value parameter positions do
> not send instantiation requests, mono records `NoObligations`, convergent mutual recursion is this
> form)—same surface as 3.5 (HKT call path), registered to that row.

## External Track RFC-029g: Delete `pub` and Auto-binding (#399

> **P3.5 Implementation (2026-10-08)**: All seven second-level tasks completed, the whole batch in
> one commit (independently revertible).
>
> - **Empirical test inconsistent with plan (honestly recorded)**: 029g.1.2/029g.1.3's acceptance
>   originally wrote "`pub f = ...` reports **parse error**"—after actually deleting `KwPub`, `pub`
>   falls back to a regular identifier, the old writing falls to the semantic layer and reports
>   **E1001 Unknown variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specifically
>   probing removed syntax is an anti-pattern", **no new** migration-specific diagnostic is added;
>   corpus fixture `pub_declaration_err.yx` pins E1001, documentation (`syntax.md`) honestly writes
>   the consequence.
> - **Design Convergence (Beyond RFC Letter, Requires Review)**: `project_refs` and
>   `cross_file_refs` two fields have completely overlapping semantics after pub deletion (the same
>   "package-internal reference union", prohibition one prohibits parallel representation) → merged
>   into a single `project_refs`, `set_exempt_pub`/`set_cross_file_refs` two switches deleted.
>   Behaviorally, Internal role's non-pub items change from "file-internal reachability" to
>   "reference pool judgment" (relaxed, in the direction of rather under-reporting); Lib is
>   tightened from "pub absolute exemption" to reference pool judgment; Script unreferenced
>   top-level bindings start reporting W1001/W1002/W1004/W1005.
> - **Dead Table Cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports writeback all deleted (production-side fill,
>   all-warehouse zero consumption).
> - **Test Migration** (test count not reduced): Added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub is recognized" assertion reversed to "pub is no
>   longer a modifier"; `passes/tests/dead_code.rs` 7 role semantic test cases rewritten (2
>   assertion reversals); `tests/integration/cli_e2e.rs` 8 role test cases rewritten
>   (`lib`/`internal` exemption assertions reversed to report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` does not participate in IR); corpus diff baseline
>   only added 1 (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerated and manually reviewed.
> - **Empirical Test**: lib 2556 / integration 272 / corpus 322 entries baseline fully green;
>   `cargo fmt` / `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` fully green.

## P4 4.5 Cross-Layer Contract PlanId-ization (D20)

> **4.5 Implementation Record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, closure
> (registration / WBS check / gate / push) completed by t7. Precondition = 4.3.1 obligation ledger
> (red state filed 173 files / 533 keys).
>
> **I. Carrier Adjudication (A: `PlanId` follows AST node)**: Added `PlanId` (`src/util/plan_id.rs`,
> `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` for synthetic nodes
> outside parser), issued by **sole allocator** `ParserState::alloc_plan_id`; carrier two
> places——`Stmt.plan_id` (statement identity) and `Expr::Call.plan_id` (call point identity),
> `Expr::call` is the all-warehouse sole call constructor. The rejected form is "two sides each
> derive the same identity by span" position coincidence (D20's original defect).
>
> **II. Two-Level Verification and Review (Members Independent)**: Each functional commit has an
> independent verifier and a reviewer, **verification/review does not reuse implementer evidence**:
> 4.5.1 = t1 implementation / t2 independent verification (external tree + 5 mutations) / t3 review;
> 4.5.2 = t4 implementation / t5 independent verification (external tree + 5 mutations) / t6 review.
> The review time point and the reviewed revision both fall on immutable history (`git show`
> verifiable). **Provider renaming caused t5 / t6 member change and reassignment**: the original
> verification/review members could not continue due to provider renaming, per the orchestration
> convention changed person and reassigned and **the reason is written in the record** (cross-round
> context retention priority, change only when compression is unavailable); after reassignment, the
> conclusion is consistent with the original round (PASS), implementer evidence not reused.
>
> **III. Two Defects Found and Fixed During Implementation Self-Check (t1, both in inScope)**: ①
> Consumer exemption criterion was hung on `UnitState.ir.is_some()`——Linking/Mono of check form
> would take `unit.ir` away, real unconsumed in check form **silently passes** (empirical test: D41
> red but CLI check exit 0); changed to read **the completion marker of the ledger itself**, and
> added regression test cases. ② Drop sequence drifts with process hash seed (partial order
> `sort_by` + `HashSet` iteration): snapshot criterion ran 106/120/114 line count drift in three
> consecutive runs; changed to **total order sorting** (same name takes max depth + name ascending)
> then ran twice zero drift.
>
> **IV. Criteria and Evidence Numbers**: After 4.5.1, `test_release_plan_spans_consumed`
> de-`#[ignore]` resident (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated in
> the same commit (programmatic review: added 1083 lines Drop + 86 lines jump index translation,
> zero deletion, no rearrangement), corpus 342 entries diff empty; 4.5.2 added
> `test_overload_resolutions_consumed` (difference set empty + evidence surface floor `examined ≥ 1`
> / `consumed ≥ 2`; empirical examined=1, consumer entries 2, no resolution skip 210), resolution
> form 9 items pin board, `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero drift**.
> Mutation-driven: consumer key offset by one bit ⇒ criterion red + real program
> `error [W1090] ... produced key not consumed` + corpus gate difference; restored to green.
>
> **V. Classification Basis for Semantic Correction (D59)**:
> `Type.method(positional actual, name = value)` **silently gives wrong answer** before the change
> (empirical test compile clean, run output `1` instead of `20`) ⇒ **Class C** (old behavior is not
> successful), the fix takes "resounding": the same program after the change reports `error [W1090]`
> at compile time; the final semantics of "named actual arguments participate in resolution" is
> **Class D**, registered in §3.14 and left issue #452 for a dedicated team to undertake. In
> addition: `Drop` is a no-op in the interpreter (`backends/interpreter/executor/ops/control.rs`
> wildcard arm only `advance()`)—key migration does not change runtime behavior.
>
> **VI. F-1 / F-2 Disposition (t5 Independent Verification Discovery)**: **F-1 (medium)** Unbound
> consumption point (`ir_gen.rs:7766-7790`) is unreachable in the current corpus and test surface,
> no red state can falsify the key control correctness ⇒ per "known boundary" registered in
> [02](02-stage-contract.md) §2 revision note fifth paragraph (retention reason + no new task);
> **F-2 (low)** Pointer drift ⇒ `types.rs`'s `OverloadResolutions` documentation comment changed to
> precise pointers (produce `inference/expressions.rs:4550`, consume `ir_gen.rs:7697` / `:7772`), in
> a single small commit, not mixed into document commit.
>
> **VII. Closure (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settlement marker);
> 07 criteria table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG recorded one line behavior change; `check-obligations.py` and `check_tracking.py` green,
> CI and Deploy Docs green. **Bookkeeping Deviation Discovered in Closure**: t3 / t6 report's
> pointers `ir_gen.rs:357` / `:3102` / `:1096` differ from the code fact by one to three lines
> (actually `:360` / `:3103` / `:1095`+`:1097`, due to 4.5.2's insertion)——registered per the code
> fact, this note also keeps a record.

## P4 4.10.3 Standalone run and MultiFile Orchestration Unification (4.2.5 Adjudication A's run Half)

> **Batch C Record (2026-10-15 registered → 2026-10-15 closed)**: User 2026-10-15 adjudicated
> C——this round only does 4.10.3 (same batch candidate 3.4.6 deferred per B1
> ([02](02-stage-contract.md) §Runtime Behavior Table "dead code family warnings do not enter
> compile path") to after 4.9, the precondition has been rehung and annotated); implementation form
> = **full single team** (user instruction does not open DAG, does not build a team, single session
> implementation + self-verification). Pre-batch debts landed first: #455 dependency announcement
> fix (rustls 0.23.46 + serde_yaml_ng, `cargo audit` from exit 1 restored to exit 0), WBS
> registration debt (#456 registered as 3.17 + uncompleted rows supplemented with "belonging stage /
> whether blocks main line" fields).
>
> **I. Red Before Green**: driver-level red criteria 3 (`src/driver/tests/run_rootless.rs`—rootless
> merged IR must not carry entry_function (red state `Some("main.main")`), rootless top-level
> executable statement must not report E3023 (red state does not produce module), Bin control
> entry_function retained (green state resident)) + CLI e2e 3 pins (standalone relative import run
> (red state E5001), Script top-level statement execution and #413 hint two green state pins).
>
> **II. Implementation Side**: CLI `run` uniformly goes through `compile_project` (the
> project/standalone bifurcation of `run_file_with_diagnostics` deleted); Linking's `entry_function`
> gated by `is_bin_role` (#413 T4——Script does not automatically call main); #413 hint keying
> changed from `module.source_files.is_empty()` to call point project probe (after unification
> ModuleIR always carries source_files, original marker fails); `script_main_never_called` adapted
> to qualified name form (function `{key}.main`, global slot empirically the same qualified
> name—based on IR evidence, not inferred from comments).
>
> **III. Existing Gaps Exposed by Unification (Same Batch Incidental Fix, A/B/C Class——only fixing
> failed programs to succeed, zero impact on originally successful programs)**:
>
> 1. **Explicit Type Actual Argument Pre-Specialization Name** (`mk(Int)(5)`): ir_gen directly emits
>    specialized name per D6.3 (mono will strip unspecialized original), the three arms of
>    `resolve_call_target` (rename / aliases / dot prefix) all do not cover the `mk(int64)` form ⇒
>    call point dangling E6006. Fix = adding a fourth arm to this single point resolver (base name
>    qualification, type parameter string preserved as is). **Same-Form Reproduction in Bin
>    Project** (pre-existing defect, not introduced by this batch).
> 2. **Script Top-Level `mut` Assignment Statement Repeated Slot Allocation**: `extract_global_defs`
>    and `register_global_slots` count `counter = counter + 1` (Assign same shape as declaration)
>    statement by statement ⇒ the same variable allocates 3 slots, the initialization writes the
>    later-allocated slot but the read uses the first-allocated slot (runtime Add type mismatch).
>    Fix = merging by name at both places (same criterion same source).
> 3. **Same-Module Constrained Generic Operator Dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually is `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidate + table verification (precise hit → generic's
>    unit key qualification hit → fall back to bare name current state; not suffix heuristic, same
>    form as `resolve_generic_name`).
>
> **IV. Registration Gap (Not Fixed in This Batch)**: Constraint dispatch still E6006 when **type
> and generic definition are in different modules** (rootless two-file probe empirical; Bin same
> shape)——root cause = MonoType user type name does not carry definition module, correct fix belongs
> to P6 type identity convergence; registered WBS **3.18** (belonging P3, precondition pending P6,
> does not block main line), `qualify_deferred_method` annotation same surface.
>
> **V. Test Adaptation (Honestly Recorded)**: `tests/integration/multifile.rs` fixture supplemented
> with manifest——what it tests is RFC-029 project orchestration (Bin auto entry) not Script
> semantics; one pin board of `cli_e2e` (top-level diagnostic variable name visibility) and
> `script_main_hint` fixture (7 examples) synchronized per qualified name form——production consumer
> side only takes merged IR, fixture directly drives the same form, no need to use pipeline product
> substitute.
>
> **VI. Final Gate (Closure revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new pins) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> diff baseline regenerated **12 lines**——line-by-line structured check: exit / stdout / diagnostic
> code all identical item by item, only stack frame function name and variable name qualified per
> unit key (manual review passed); after regeneration `check-corpus-parity.py` exit=0.
>
> **VII. Process Events (Honestly Recorded)**: In this batch once during the pre-commit hook running
> period concurrently edited the workspace document, was eaten by the hook's stash rollback and that
> commit failed (concurrent form pre-announced in the configuration comment)——discipline = do not
> touch the workspace during hook running; the #455 commit was therefore split into two attempts
> (the second time all green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closure)

> **4.6 Batch Record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 instructed to
> advance in the order 4.6 → 4.9 → 4.4 → P5, first do 4.6; implementation form = **full single
> team** (following the previous batch's user instruction: no DAG, no team, single session
> implementation + self-verification). Task definition (09 three-level table): Project-internal file
> semantic data (SemanticDB) goes through orchestrator's same pipeline, same source as diagnostics;
> cross-file references carry `resolves_to`. Acceptance: Project-internal cross-file jump hits
> definition; semantic data and diagnostics come from the same compilation.
>
> **I. Current State Empirical Verification (Verify Before Asserting)**: didOpen/didChange triggers
> **two** compilations——diagnostics go through the unified Driver pipeline (4.2.3/4.2.6 already
> unified), semantic data goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registry ⇒ project-internal
> `use` resolution fails, cross-file reference events do not occur, jump/rename/reference lookup
> silently falls through for imported symbols——the silent failure registered in D55). The
> orchestration arm's checker uniform name `"<module>"` (SemanticDB file identity lost),
> `type_result` (including semantic_db) is discarded when diagnostics are non-empty and never
> externally sent; `checker.rs`'s row clearing always covers `resolves_to`'s DefId file identity
> with the current file; both `Export`/`ModuleInfo` have no definition location surface.
>
> **II. Red Before Green**: LSP e2e pin (`src/lsp/tests/cross_file.rs`)——temp two-file project after
> didOpen main.yx sends GotoDefinition for `add_one` call point, **before fix returns None (red,
> empirically the silent failure in the previous section)**, after fix hits lib.yx line 0 definition
> (green). During the process a probe positioning: the compiler-side link (driver probe dump) is
> already fully connected but e2e is still empty——root cause is `resolve_reference` matches
> **exactly by reference start point** (D55 precise-only current state), test cursor falls in the
> middle of the identifier; this tolerance is an existing behavior surface, this batch does not
> touch (see VII).
>
> **III. Implementation Side**: (1) `Export.definition_span` (extract_module_info self-reaps from
> AST identifier node, same source and same value as typecheck definition event's `def_span`; native
> std always None) + `ModuleInfo.definition_file` (Registry arm fills per DiscoveredFile, including
> vendor; std always None); (2) checker reference row clearing through `import_def_locations` (use
> line binding span → definition file + definition span) redirects cross-file imported item's
> `resolves_to` out of this file; std import has no registration, maintains falling to use line; (3)
> checker module identity changed from unified `"<module>"` to unit file path (empirical
> `env.module_name` all 4 consumer points zero diagnostic consumption); (4)
> `DriverOutcome.semantic_db` channel——CollectAll form (Lsp target file **regardless of diagnostic
> success or failure** + SingleFile fallback) converges, Lsp arm tail injects non-target file
> top-level export definition surface from registry (`external_definitions`, files not opened can
> also jump; type/submodule exports not injected——typecheck side type registration does not produce
> definition events, both sides maintain consistent gap); (5)
> `orchestrator::analyze_source_in_project` (transferred from `check_source_in_project` taking
> diagnostic half); (6) LSP `analysis.rs` single compilation entry replaces manual pipeline
> (`update_semantic_db` deleted), World key uniformly normalizes file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
> differences normalized to the same key, cross-file DefId thus stable hits); 5 handlers +
> semanticTokens dual request take `db_key`.
>
> **IV. Incidental Fix (D59 Branching Class B——old behavior silently no action, originally
> successful programs have zero impact on outward performance)**: ① Project-internal file cross-file
> jump/rename/reference lookup changes from silently falling through to hitting real definition; ②
> When there is a lexical error, the old manual pipeline deletes semantic information (staining
> disappears) → incomplete AST staining retained (extension of 4.2.6 editor philosophy); ③ When the
> target file has type errors, semantic data is no longer discarded together with `type_result`
> (typing intermediate state staining/jump does not disappear).
>
> **V. Pin Board**: driver-level 6 (`src/driver/tests/lsp_semantic_db.rs`——target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface round trip,
> std import fallback to use line, error file retention, single file fallback path key, single file
> error retention) + LSP e2e 1 (red before green) + uri normalization 4
> (`src/lsp/tests/uri.rs`——drive letter conversion, space encoding, idempotent normalization,
> non-file scheme passthrough).
>
> **VI. Mutation Verification (scratch.py trial, mechanically reversible)**: batch tree wbs-46 @
> closure revision 39546020, two mutations both make hit pin turn red, after restoration blob
> consistent with HEAD + git status CLEAN——m1 removing `import_def_locations` registration
> (statements.rs) ⇒ cross-file driver pin + LSP e2e pin red, remaining 7 green; m2 removing
> `external_definitions` injection (arms.rs) ⇒ same two pins red (round-trip parsing loses anchor),
> remaining 7 green. Batch tree gc'd (released 4.8 GB, `status` no batch tree).
>
> **VII. Final Gate (Closure revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new pins) / `--test integration` **287 passed** / corpus
> diff **byte-by-byte zero drift** / IR snapshot zero drift / check-obligations PASS (13·13·0·8,
> `semantic_db` non-obligation reason consumer anchor synchronized server.rs:482 with 02 §Obligation
> Field Table) / check-docs-truth 0 / docs-examples 258 entries 0 failures / docs-orphan 0 /
> markdownlint all-warehouse only the user's unsubmitted 032 two MD056 (user's own editing, not
> touched) / pnpm docs:build 0. **Process Events (Honestly Recorded)**: en/09a-wbs-history.md:54 one
> MD037 was introduced by the auto-translate bot retranslating the previous batch's 09a (zh original
> text has no spaces), this batch incidentally fixed back——markdownlint red is not this batch's
> change surface but is in the same file as 09a, registered together. **Remaining Observations
> (Registered Pending Discussion, Not Touched)**: `resolve_reference` only matches exactly by
> reference start point——when the cursor falls in the middle of the identifier, jump/hover/rename
> silently does not act (D55 precise-only current state, whole-file form isomorphic), if tolerance
> matching is needed it is an independent behavior change, reported to user for adjudication.
>
> **Same-Day Continued Batch (4.6.2, Implemented After User Adjudication)**: After the closing
> report was delivered, the user adjudicated on "Remaining Observation ①" with "okay, then change
> it"——the query key tolerance was implemented per plan A: the query keys of
> definition/hover/references/rename four handlers changed from cursor position (after UTF-16
> conversion) to use the **identifier's own start point** already located by
> `find_identifier_at_position` (same coordinate system as semantic events, no conversion needed);
> the start point exact match semantics of `resolve_reference` unchanged (D55 precise-only
> unchanged). The definition point fallback judgment of references/rename (start <= col < end) is
> isomorphic for identifier start point. Red before green: definition middle cursor pin first red
> (None) then green; four handlers middle cursor 1 pin each + cross-file e2e start point/middle
> double position assertion, `--lib` 2748 (+4) all green, clippy -D warnings 0. Registered WBS 4.6.2
> (three-level count 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction) — Round 1 Closure (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 Batch Record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 adjudicated
> merging 4.9.1 / 4.9.2 / 4.9.3 three related three-level tasks **into one team one round** (reason:
> they all belong to the "pool source and identity" surface, merging to land can avoid repeated
> rewriting of intermediate state after `collect_project_refs` deletion; 4.9.4 / 4.9.5 belong to
> consumer refinement and test pool separation, depending on 4.6.1's `use` path resolution, forcibly
> batching will expand the gate surface). #450 attribution **pending adjudication**——evidence shows
> it is "overload resolution after name not passed back to reference count" class, still reproduces
> in the round 1 already landed form, per independent WBS item registered for disposition (team
> leader recommends A: register as independent item when merging into round 2; **pending user
> adjudication**). Frozen rev = `3863a9ff0` (parent commit `62f6e1a59` is the pre-fix baseline);
> verification PASS attempt `aab92fe0` (i.e. t2); review pass attempt `96452696` (i.e. t3).
> **Closure registration commit = `38d30b846` (docs(docs): 4.9 reference pool round 1 closure
> registration)**——this commit was actually written and pushed by the closing member
> integration-refpool; its closing probe once mislabeled the closing task as terminal failed and
> overwrote the output, the team leader took over a new attempt and changed back to completed per
> the verified delivery facts, hereby correcting the attribution and label. Evidence filed:
> `target/yx-scratch/yx49/EVIDENCE.md` (containing 8 mutation patches and warn_compare_v8.json).
>
> **I. Merge Adjudication**
>
> 1. **The adjudication itself**: three related three-level tasks merged into round 1. **4.9.4
>    (consumer refinement) / 4.9.5 (test pool separation) next round**——the former depends on the
>    `use` path resolution delivered by 4.6.1 (already landed at `39546020`, can start); the latter
>    needs to be merged with the RFC-029f table integration, registered at the end of RFC-029f for
>    modification.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 are mutually preconditioned (4.9.2 changes pool type →
>    4.9.1 assembly side needs to be rewritten → 4.9.3 has the role table for denominator judgment),
>    splitting rounds inevitably writes an intermediate state then changes twice; (b) all three
>    belong to the "pool source and identity" surface, 4.9.4 / 4.9.5 belong to another surface, the
>    split is by "cohesion" rather than "independently revertible" cut.
> 3. **Merge result**: single commit `3863a9ff0` (one line `:recycle: refactor(driver)`), single
>    push, single batch gate. **Hash explanation**: Before push `git pull --rebase` merged the
>    auto-translate bot commit (`72f364169`); the rebase landed after the other party's commit, this
>    batch's two (implementation `3863a9ff0`, its pre-fix baseline and other party's document commit
>    `62f6e1a59`) **were not rewritten, kept the original hash**——the first version mistakenly wrote
>    here as "`3863a9ff0`→`3863a9ff0`" identity mapping (looks like "rewritten happens to be the
>    same value", actually "not rewritten"), corrected on 2026-10-17 at closure; the other party's
>    code commit `33f14e8ab`→`b122b7ad1`. **External references uniformly use the post-push hash as
>    the standard** (the rewritten old hash is unreachable in a new clone, identity is determined by
>    patch blob comparison).
>
> **II. Red/Green Empirical Caliber of Five Criteria**
>
> | #   | Criterion                                                | Red State (Pre-Fix / P1 Directory Scan Rollback Patch)                                                                                                     | Green State (Round 1 Landed)                                                                                                                                                                                                                                                                                                                                                                     |
> | --- | -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
> | ①   | Pool's root = package target (#334)                      | Directory recursive scan ⇒ files under tests/ also enter the pool, exempting unused definitions in production code                                         | `collect_package_refs` (`helpers.rs:481`) per `ExplicitSurfaces` root calls `discover_with_used`; fallback `collect_compilation_refs` (`helpers.rs:454`) when all roots degenerate                                                                                                                                                                                                               |
> | ②   | Pool granularity = `(module, name)`                      | Same-name local variable (`let foo = …`) accidentally exempts same-name top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` cross-contaminate | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) assembly unique entry; `arms.rs:603-609`'s `other_module_refs` puts "all modules except this unit" into union then sends to `DeadCodeAnalyzer`; module's own name (top-level definition and local binding, `use` imported name exception) no longer exempts other modules' same-name definition |
> | ③   | `FileRole::Script` does not use the pool                 | Script is affected by references from other files in the project                                                                                           | `arms.rs:611-623`'s `match us.role` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`), Script/Test go to `_ => {}` default arm (per file-internal definition-use graph judgment)                                                                                                                                                                          |
> | ④   | Fixture A: lib.yx line 4 definition should report W1001  | Pre-fix: report `W1003@1` (actually "unused import"), not W1001                                                                                            | Red state empirical test (red 1 fixture red1) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                                 |
> | ⑤   | Fixture B: main.yx line 4 definition should report W1001 | Pre-fix: report `W1003@1`                                                                                                                                  | Same as above                                                                                                                                                                                                                                                                                                                                                                                    |
>
> **The "Deductive Red" of Criterion ②** (implementer self-stated + verifier closed): The
> `let foo = …` in the red2 fixture is in helper.yx; the old pool under bare name union treated
> `foo` as a whole as "used by helper", exempting lib.yx's `foo`. **Empirical test**: red2 fixture
> vs. pre-fix binary → no W1001 (old caliber exempt); red2 vs. post-fix binary → W1001@1.
>
> **The First Version Failure of the Red State Fixture of Criterion ③** (Script does not use the
> pool): The first version of the fixture placed a same-name local variable in the same file
> triggering E2010 (the language does not allow the same module to shadow top-level definitions),
> changed to another module (helper.yx) to be a deductive red state. This point t3 review and t2
> verification both independently reproduced, **not a contract defect**——same-module shadowing is
> the language's hard constraint, fixture must be cross-module.
>
> **III. Correspondence Between +8 and +16 Two Counts (Authoritatively Confirmed by t3 Review)**
>
> `target/yx-scratch/yx49/EVIDENCE.md`'s section C writes `+16/−0`, but missed the **"new fixture
> record"** component——v8 instrument (`warn_compare_v8.json`, 331 files symmetric diff,
> `only_before=0`) caliber **includes** new fixtures; t1 implementer's self-built "+8" is
> **warn-before / warn-after (each 325 entries) aggregated by fixture** (each member file checked
> once). Review mapped one by one:
>
> - **+8** (8 new warnings, all `check <non-entry file>`, `check <entry file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (unified attribution)**: These 8 fixtures **did not declare package target** ⇒ 4.9.1
>     root degenerates to the checked file's own compilation set, consumer main.yx is not in it;
>     this is an in-design change (before this these files' check surface had zero warnings, not an
>     exemption path). The warehouse root `yaoxiang.toml` is an empty file ⇒ the same degeneration
>     path is empirically established.
> - **+16 residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files in t1's `warn-before.jsonl` have `exit=None` (the capture time point
>     **does not exist**——these are new fixtures added by this batch). Therefore +8's "+ 0 − 0" diff
>     does not include them, +16's v8 symmetric diff includes them.
>   - v8's `stray.yx` only has `W1001@2(probe)` left: the reason is the pre-fix binary also reports
>     `probe@stray.yx:4` (this fixture runs `probe` once for each file), **cancels out** leaving
>     only lib's 1——three data sources (+8 / +16 / dual binary ABBA) are mutually consistent.
> - **New fixtures 8** vs **+16 residual 8**: The same batch of files (4.9.1 / 4.9.2 newly added
>   `ref-pool-compile-set/` and `ref-pool-module-identity/`), the instrument treats each of their
>   checks as an independent record——the same file may trigger different diagnostic entries for
>   multiple `role / context`.
>
> **Conclusion**: **+16 = +8 (aggregated by fixture) ∪ new fixtures 8**——both counts are correct,
> **different calibers**: t1 reported "added 8" is the fixture-layer caliber, t2 reported "+16" is
> the file-layer caliber (symmetric diff includes new fixtures). t3 review ① +8⊆+16 confirmed;
> **review ② wrote "do not copy EVIDENCE.md section C verbatim"**——section C missed the "new fixture
> record" component, this item was rewritten per t3 review `pass` conclusion.
>
> **IV. Mutation Experiments (Mechanically Reversible, All `scratch.py trial`)**
>
> 8 mutation patches (`target/yx-scratch/yx49/*.patch`, p1 directory scan, p2b use imported name
> removal, p3b script connecting pool, p3 flat consumption, p4 std connecting pool, p7final
> self-built 4 criteria, p8 std closing, p9 vendor connecting pool + p_stack2 / p_stack3 overlay
> surface). Each trial output contains: HEAD blob → post-mutation blob → post-restoration blob
> consistent with HEAD = yes + `git status` CLEAN. This machine's `core.autocrlf=true` needs to be
> injected with `false` via `GIT_CONFIG_*` (`trial_env.ps1`), otherwise patch new lines convert to
> CRLF causing apply/restoration double failure——**only affects patch mechanics, does not change the
> tested behavior**.
>
> **V. Process Events / Environment Lessons (Must Read for Handoff to Next Round)**
>
> 1. **core.autocrlf injection**: This machine's `core.autocrlf=true` must be injected with `false`
>    via `GIT_CONFIG_*` to ensure patch mechanical reversibility; see
>    `target/yx-scratch/yx49/trial_env.ps1`. **Do not reuse** any trial script without injection in
>    the next round.
> 2. **Same-tree trial serial**: On the same tree, `trial` and `cargo build` **must be strictly
>    serial**. `trial`'s internal `git apply` and `cargo build` both touch the worktree, parallelism
>    can cause two types of races: "patch not completed but seen by build" or "build and apply
>    deadlock".
> 3. **Trial rebuilding binary needs synchronization confirmation**: After `trial` completes, it
>    rebuilds the tree's binary (cargo's fingerprint contains source path, **changing tree = full
>    rebuild**), before subsequent measurement must confirm `bin/yaoxiang-rs` is synchronized with
>    HEAD. **This round once used residual mutation binary to measure false data**, rebuilt and
>    re-measured to correct.
> 4. **Reason for the second tree yx49-b**: The cross-comparison needs the pre-fix binary
>    (62f6e1a59), no ready-made in the batch; `scratch.py init yx49-b` opened the second tree
>    (another full build). **Tree released** (see VI), deletion leaves batch closure `gc` for
>    disposition.
> 5. **Reason for cli_e2e two Internal test case fixtures supplemented as legal package (supplement
>    `[run].main`, assertion unchanged)**: Pre-fix in `cli_e2e` two `Internal` role pin fixtures
>    **did not declare `[run].main`** ⇒ after 4.9.1 lands, `compile_project` goes through the
>    `ExplicitSurfaces` root call path, cannot find `main` ⇒ the whole batch of 2 fixtures' behavior
>    drifts from "fall back per Script" to "cannot find entry"——this is the fallback behavior
>    introduced by 4.9.1, not a defect. After supplementing the `[run].main` field, the two fixtures
>    go through the normal package path, **assertion** (exit code / diagnostic code / warning set)
>    unchanged verbatim.
> 6. **Safe way to delete the second round tree**: After release use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`), it will check "no mount" before
>    deleting. Released before this batch's closure, next step `gc` naturally cleans up.
>
> **VI. Batch Tree State**
>
> Before closure: yx49 released (4.8 GB released), yx49-b released. Both trees have no mounts,
> status can be reviewed in `scratch.py status`. This task's output is appended with `scratch.py gc`
> full status.
>
> **VII. RFC-029f "Known Defects" Table Flip**
>
> The previous version (2026-10-09) described in the present tense "three design defects of the
> reference pool" (including `collect_project_refs` / `orchestrator.rs:603-648` /
> `orchestrator.rs:393-396` and other already-deleted code locations). This batch's closure
> synchronously rewrites to "**Known Defects: Landed (2026-10-16, WBS 4.9 Round 1)**": ① The current
> pool's root = `collect_package_refs` (`helpers.rs:481`), parsed per `ExplicitSurfaces`, does not
> scan directories; ② The current pool type `HashMap<String, HashSet<String>>` (key = module key),
> assembly unique entry `collect_ref_pool` (`helpers.rs:429-449`), consumer side `arms.rs:603-609`;
> ③ Current Script does not connect to the pool, `arms.rs:611-623`'s `match us.role` default arm.
> **All file:line are re-taken on the referenced revision `3863a9ff0`**. `collect_project_refs` and
> `orchestrator.rs` have been split per P4 4.2.9 (`c59fb751`) into
> `src/driver/{state,arms,helpers}.rs`, the old locations in the table no longer hit——this is the
> "mismatch between declaration and fact" fix (t3 review remark ①).
>
> **VIII. Final Gate (Closure revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 lines** (two new fixture
> behavior runs, 338a339,340; another 342 lines byte-by-byte unchanged) / IR snapshot 204 zero drift
> / check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process Event**: This
> batch once had cargo build fail because `core.autocrlf` converted script line endings to CRLF,
> `yaoxiang-rs` in team scratch path reported `BadImage` at startup, empirically it was encoding
> inconsistent with git index——passed after rebuild (parallel to 4.6 batch "external whole-tree
> discard" lesson, different reason).
>
> **IX. Remaining (Handoff to Next Round)**
>
> 1. **#450 Attribution Adjudication**: Evidence shows it is "overload resolution after name not
>    passed back to reference count" class, **still reproduces in the round 1 already landed
>    form**——per user adjudication C flow dispatch. **#450 Probe**: Dual binary same set
>    `{W1002@2, W1005@4}` (empirical, rootless form), not directly related to round 1 refactoring,
>    but the fix for #450 may need to wait for 4.9.4 (consumer refinement) to be done
>    together——because the root cause is "overload resolution after name passing to reference count"
>    needs the precise consumption surface of use items, 4.9.4's "really used after being imported"
>    is the precise version of the same kind of criterion.
> 2. **dead_code.rs:683 pointer nit**: The comment writes "the assembly caliber of the two pools is
>    in driver side collect_compilation_refs"——actually `collect_compilation_refs` is the fallback
>    path, the main path is `collect_package_refs` (`helpers.rs:481`). The comment is inconsistent
>    with the code main path caliber (**not in this batch's writing scope**), left to be corrected
>    when 4.9.4 / 4.9.5 land.
> 3. **4.9.4 / 4.9.5 Preconditions Unlocked**: 4.6.1 (`use` path resolution) already delivered at
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs to first determine
>    the RFC-029f table integration (already laid by this batch's "VII." flip).
> 4. **`yaoxiang check` multi-file `run` never reports dead code family warnings** (WBS 3.4.6,
>    2026-10-15 user adjudication C deferred to after 4.9)——this batch's round 1 not involved; next
>    round 4.9.4 / 4.9.5 land in the same batch processing.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction) — Round 2 Closure (4.9.4 / 4.9.5 / 4.9.6)

> **4.9 Round 2 Batch Record (2026-10-16 implemented → 2026-10-17 closed)**: Covers 4.9.4 (consumer
> criterion = really used after being imported), 4.9.5 (test pool separation), 4.9.6 (method
> reachability caliber, issue #450, user 2026-10-16 adjudication A merged into round 2). Closure fix
> (verification-side remedy M1/M2/M4 recompile redo + F1 fix round + PA-2 dead branch deletion) and
> registration gap issue #464 (WBS 4.9.7) are settled together.
>
> **I. Adjudication Chain and Commits (Uniformly Post-Push Truth)**
>
> | Step | Content                                                                                                                                              | Post-Push Commit | Pre-Push Local Value | Determination Method                                          |
> | ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- | ------------------------------------------------------------- |
> | 1    | fix(util): check explicit path not blocked by default exclude table + --exclude always empty (issue #463, P0-0.4)                                    | 366bc5a69        | ef959ca55            | Patch blob same                                               |
> | 2    | Phase A = 4.9.4 + 4.9.5 (RefPool attributed per import + Test does not contribute consumer surface)                                                  | e6cf1aeaa        | 84a6f6978            | Patch blob same                                               |
> | 3    | Phase B = 4.9.6 (#450 per-candidate method reachability; including generated warning-codes.md)                                                       | ddcbb12d0        | cd629b5fe            | Patch blob same                                               |
> | 4    | docs(meta): AGENTS item 10 ⑦ (stash window convention during commit)                                                                                 | ec94fa9d8        | 9a4ed5f3b            | Patch blob same                                               |
> | 5    | Fix round = F1 (overload sibling overload restored reporting W1005) + F2 (verification-side M1/M2/M4 recompile redo) + F4 (snapshot generation note) | b8380e6ab        | 3d9ef6eab            | Patch blob same (36962f318e42…, git range-diff no difference) |
> | 6    | Closure fix = PA-2 (delete strip_unit_prefix dead branch + 029f/CHANGELOG three misstatements correction, register #464)                             | b940300ac        | e13984465            | Patch blob same (patch-id 9bda3cfe4c…)                        |
>
> **Hash Explanation**: The git pull --rebase before each push will rewrite the local hash; the
> "Determination Method" column in the above table is all "patch blob same"
> (`git diff <rev>~1 <rev> | git hash-object --stdin` consistent on both sides + `git range-diff` no
> difference). This article does not reference any pre-push hash as authoritative.
>
> **II. Red/Green Empirical Caliber of Three Criteria** (Verifier Self-Built Reproduction, Does Not
> Adopt Implementer's Criteria)
>
> | Criterion                  | Red State (before binary 366bc5a69 empirical)                                                                                                                                              | Green State (ec94fa9d8 and after)                                                                                   |
> | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
> | 4.9.4 import≠consumption   | Project check a.yx on declaration side only 1 W1001 (main's use b.{foo} + foo() saves a.yx same-name foo; guards only imported and not used are also saved)                                | Same fixture 2 as reported (collision name not saved + unused import not counted as consumption); ref_pool 8 passed |
> | 4.9.5 test pool separation | src/util.yx's debug_print saved by tests/ reference (only 1)                                                                                                                               | Same fixture 2 (test references do not spill); ref_pool's test_ref_pool_excludes_test_role_references passed        |
> | 4.9.6 method reachability  | Self-built fixture (2 disambiguation overload calls + 1 not called) 3 W1005 false positives; method_call_args.yx 3 / method_overload.yx 1 (overload per full name override only reports 1) | Same fixture exactly reports orphan not called; method_unused 3 pins passed; corpus call_args 3→0 / overload 1→0    |
>
> **Fix Round (F1) Further Tightening**: Sibling overload not hit by resolution restored reporting
> W1005 (b8380e6ab——the same fixture when calling ->Float reports ->String sibling, when calling
> ->String reports ->Float sibling, bidirectional line number pins). **Fallout Boundary (Register
> #464)**: Cross-unit (registry resolution) method calls do not produce resolution entries, fall
> back to name-level exemption, cross-unit same-name same-order previously not reported——register
> 4.9.7.
>
> **III. Process Events and Lessons (Four Hard Requirements Correspondence)**
>
> 1. **PA-1 Changed from Blocker to Pre-existing Capability Gap (medium)**: Round 2 review once
>    registered "cross-unit same-name same-order not reported" as a blocker (PA-1); subsequently two
>    fact reconciliations (implementer filed + verifier re-tested same fixture set on
>    b8380e6ab/b940300ac dual binary, diagnostic set byte-by-byte consistent) confirmed it is a
>    **capability gap that existed before round 2** rather than this round's regression——mechanism =
>    cross-unit method calls do not produce resolution entries, fall back to name-level exemption;
>    disposition = change to medium, register issue #464, WBS 4.9.7 schedule next round.
> 2. **Reviewer Publicly Corrects before Data and Withdraws Regression Determination**: The
>    reviewer's initially quoted before data ("−42 entries include called methods") was inconsistent
>    with its original output (W1005 count=0)——verifier checked and confirmed before capture was 0
>    entries, the reviewer's regression determination was **publicly withdrawn** based on this; the
>    correct caliber of −42 = 26 files' called methods false positive elimination (Phase B),
>    traceable item by item to behavior-run exit=0 files (method really called and executed).
> 3. **t13 Terminated Because Contract Premise Does Not Hold (Not Implementation Defect)**: This
>    task's acceptance premise wrote "cross-unit same-name same-order reports W1005 regression fix",
>    but empirical (t14 dual binary) confirmed this behavior is a registered gap (#464) not
>    regression——contract premise does not exist, task terminated; its probe value is taken over by
>    t16 read-only determination (confirming `strip_unit_prefix` is a dead branch).
> 4. **The Real Criterion of Document Commit is "Really Build Once"**: Round 2 closure's document
>    commit (`042c07860`) made Deploy Docs red in VitePress's Vue template compilation——the angle
>    bracket placeholders like `<rev>` / `<before-exe>` / `<after-exe>` etc. in the record did not
>    enter backticks, were parsed as elements (reporting `Element is missing end tag`, pointing to
>    the line of `<rev>`); markdownlint cannot catch this type, and pre-commit's markdownlint hook
>    is judged as `no files to check` skipped in commits containing only docs (empirical).
>    Disposition: placeholders uniformly enter backticks, locally run `npm run docs:build` once
>    before push (fix commit `0c837e1b4` with local build exit=0 as evidence); if committing with
>    `--no-verify`, commit-msg check needs to be manually supplemented.
> 5. **Contract Description Not Synchronized Process Omission**: t14's task description text has
>    PA-1's old hypothesis left (writes "b.yx reports W1005" expectation), verifier proactively
>    pointed out in the report "judging per the contract field, description text does not count" and
>    asked the team leader to confirm——disposition = use the task contract field as the basis for
>    adjudication, description text left as process lesson (**task description must be synchronized
>    with the latest adjudication chain when reassigning/dispatching**).
>
> **IV. Mutation Experiments (Mechanically Reversible, All scratch.py trial)**
>
> Round 2 verifier total 9 patches (under target/yx-scratch/yx49/ m1b/m2b/m4b is the compilable redo
> version after F2——the initial version of M1/M2/M4 commented out the if condition line leaving a
> dangling block body, judged as "compilation failure mis-recorded as criterion red" by t8 review F2
> and voided; the redo version after patch-by-patch cargo check exit=0 + trial red). M3 (collision
> name attribution rollback) initial version is a true mutation, empirically ref_pool 5 criteria
> turn red, original kept. M1b anchor re-taken on b8380e6ab (after F1 fix probe segment refactoring,
> old anchor failed). All trial: HEAD blob → post-mutation blob → post-restoration blob consistent
> with HEAD + git status CLEAN. Environment injection follows trial_env.ps1 (core.autocrlf=false via
> GIT_CONFIG_*).
>
> **V. Warning Surface Full Volume Cross-Comparison (Caliber First Clarified)**
>
> Instrument = warn_surface_r2.py (self-built parser, (file, code, line, column) four-tuple
> symmetric diff, project mode channel cwd=file parent directory, full path key); command =
> `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>` (337 files × 2
> binaries). **Caliber**: This round's before capture already includes all fixtures ⇒ matched set =
> universal set 337, no round 1's +8 (matched increment) / +16 (universal set) dual caliber problem.
> Results: only_before = 42 W1005 (26 files, all called methods' false positive elimination = Phase
> B's −42; per file check all in behavior-run exit=0 baseline, method really called and
> executed——**no one true dead code is saved**); only_after = 3 W1001 (ref-pool-import-use a.yx:4
> check surface ×2 + ref-pool-test-pool ../src/util.yx:2 = Phase A criterion surface). **Snapshot
> Generation (F4)**: Phase A time point's after capture (57 residual before Phase B) and the final
> version (15+2=17 after Phase B) are **two generations**, must be noted when referencing; in
> addition, 1 exception case (interface_member_conflict.yx's Dog.name@18) is a field access form,
> not in Phase B's W1005 removal set, consistent before and after.
>
> **VI. Final Gate (Closure revision b940300ac)**
>
> lib 2761 passed / integration 288 passed / IR snapshot zero drift (snapshot 7 passed) / corpus
> baseline zero drift (byte count three-way comparable: baseline = b8380e6ab = b940300ac) / clippy
> -D warnings 0 / docs-truth exit=0 / check_tracking exit=0 / generator already run no product
> change. **Gate Check Arrangement (Item 13)**: This batch's pushed CI / Deploy Docs / Auto
> Translate run conclusions are registered by the scheduled check link (run id + conclusion +
> connected push coverage relationship: descendant run covers the previous one), not waiting within
> the session.
>
> **VII. Batch Tree State**
>
> yx49 and yx49-b both released (gc unified deletion at closure); no batch-added directories outside
> the warehouse.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction) — Round 3 Closure (4.9.7)

> **4.9 Round 3 Batch Record (2026-10-17 implemented → 2026-10-17 closed)**: Covers 4.9.7
> (cross-unit method call resolution registration and receiver attribution, issue `#464`——WBS 4.9's
> last three-level task; after this round's closure, the entire 4.9 second-level task kanban is
> checked). Chain = production side t19 → independent verification t20 PASS → review t21
> needs_revision (F1: injection surface false positive) → fix round t24 → independent verification
> t25 PASS → review t26 PASS (incremental, verdict=pass). All commit numbers are post-push truth
> (determination method = patch blob same; `git diff <rev>~1 <rev> | git hash-object --stdin`
> consistent on both sides).
>
> **I. Adjudication Chain and Commits (Post-Push Truth)**
>
> | Step | Content                                                                                                                                                                                                  | Post-Push Commit | Determination Method   |
> | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ---------------------- |
> | 1    | Production side: cross-unit method calls produce D20 resolution entries (`feat(typecheck)`, t19)                                                                                                         | `b8b3cc3f6`      | Patch blob same        |
> | 2    | Driver-side attribution: method reachability per definition unit gives marker, name group with resolution record stops using pool's name-level exemption (t20 verification PASS / t21 review catches F1) | `d5c00235a`      | Patch blob same        |
> | 3    | Injection surface fix round F1: candidate injection per "name ← providing module" **per-pair** filtering, eliminating E1107 false positive (t24, including F2 expression and F3 pin single point)        | `d5dd1a37b`      | Patch blob same        |
> | 4    | Pin test repositioning: F3 pin in `src/frontend/core/typecheck/tests/environment.rs` moved to declaration side `src/driver/tests/overload_injection.rs`                                                  | `c2f887843`      | Patch blob same        |
> | 5    | dev top                                                                                                                                                                                                  | `e9675c9dd`      | merge commit, see "II" |
>
> **II. Hash Mapping and Two Merges at dev Top (Source Not Confirmed, Member Behavior Not Allowed to
> Be Written)**: Round 3's four commits (above table 1–4) have all been pushed to origin/dev;
> `git worktree list`'s main mount is at `e9675c9dd`. Two **non-this-team
> command-surface-generated** merge commits exist at dev top (`5da05b885` / `e9675c9dd`, reflog
> shows as `pull --tags` + push form), source not confirmed——per item 10 ④ (reference must be
> resolvable by anyone) and item 13 (gate handed to special check) of the same closure caliber, here
> only register the fact and reflog evidence, do not do member attribution; also see "V." lesson ②
> (push attribution convention must be verifiable).
>
> **III. Actual Form of D20 Contract Extension (Key Form and Consumption Point)**: The production
> side packs the three parallel tables "candidate / mangled IR name / declaration period formal
> parameter name" **in the same order** into `ModuleInfo.method_overloads`
> (`src/frontend/module/mod.rs:157`) for unit cross-border, the registry deterministically merges
> per module key ascending (`src/frontend/module/registry.rs:150`); typecheck arm injects per this
> unit's import face per pair (`src/driver/arms.rs:354` → `visible_method_overloads`,
> `src/driver/arms.rs:1521-1579`: name ← providing module per pair, same-name conflict whole name
> skipped, own key position not externally merged), pin single point `append_overload_group`
> (`src/frontend/core/typecheck/environment.rs:399`: three tables same order append + equal length
> debug assert); consumer side attributes D20 resolution products per **definition unit**
> (`MethodMarks`, `src/driver/helpers.rs:569-573`; `typed_owners` `:591`; `resolved_method_marks`
> `:621`), and lets name group with resolution record stop using pool's name-level exemption
> (`src/frontend/core/typecheck/passes/dead_code.rs:252-267`, `:825-841`). Key form = entry name
> `{Type}.{method}#k` (`{Type}` is the definition side bare name; `#k` declaration order).
>
> **IV. Key Scope and False Positive Defense Empirical Test (Fix Round F1 and Five Residual
> Boundaries)**: The red state of F1 is the pre-fix `d5c00235a`'s injection surface "name set ×
> module set" combination puts the same-name type candidate defined by another module into the same
> key position ⇒ `use a.{Temp}` + `use b.{Other}` (a also defines Other, b also defines Temp, method
> same name same order) fixture `check` reports 2×E1107, exits 1 (originally successful program
> compilation fails); after fix `check`/`run` both exit 0, 4 W1005 honestly report the two sides'
> **uncalled** same-name definitions (reverse false negative surface also checked). Library fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`; driver-level pin
> `src/driver/tests/overload_injection.rs` 3 + `src/driver/tests/method_unused.rs` round 2's 5 #464
> criteria and 4 #450 old criteria all green; main acceptance fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` diagnostic set item-by-item consistent
> before and after fix (not regressed). Five residual boundaries and item-by-item empirical test
> (single `check` definition file not reported / name-level ambiguity E5006 whole name skipped /
> boundary (c) rewritten per t20 empirical / inline alias and `ns.T` not injected into candidate
> surface / whole import `use M` and std candidate surface do not produce entries) full text see
> [029f](../../rfc/accepted/029f-target-semantics.md) §Cross-Unit Method Reachability Attribution
> "Residual Boundary", not repeated here.
>
> **V. Process Events and Lessons (Four Categories Continuation)**:
>
> 1. **Contract wording written too narrowly directly becomes a defect**: One of the root causes of
>    F1 is that the contract only writes "only inject module face that can be named by bare type
>    name", does not write "attributed by name per pair"——the implementation per the letter took the
>    "name set × module set" combination, review only caught the E1107 false positive. When
>    describing behavior boundaries in the contract, **combinatorics** must be written dead (per
>    name per pair / full combination), cannot be left to the implementation to understand.
> 2. **Convention must be verifiable: "push by team leader" only written in the dispatch message ⇒
>    when third-party push/merge occurs, cannot be attributed**. This round's dev top has two
>    source-unconfirmed merges (see "II"), ultimately relying on reflog to reverse-deduce and
>    withdraw one incorrect attribution. Verifiable form = members do not run
>    `git push`/`fetch`/`pull`, synchronization uniformly `pull --rebase`, use reflog to check.
> 3. **Infrastructure unstable, first save patch then change person**: This batch total 5
>    unrecoverable failures (3×TRANSPORT, 1×INVALID_REQUEST (thinking protocol), 1×background
>    sub-agent no message exit); disposition convention = produced evidence and patch first to the
>    warehouse internal path (`target/yx-scratch/<batch>/`) then change person to continue, same
>    session two consecutive failures do not bet on the third.
> 4. **The real criterion of document commit is "local really build once"**: Round 2 closure's
>    document commit (`042c07860`) made Deploy Docs red in VitePress's Vue template compilation
>    (angle bracket placeholders `<rev>` etc. not entered backticks were parsed as elements,
>    reporting `Element is missing end tag`); markdownlint cannot catch this type, and pre-commit's
>    markdownlint hook is judged as `no files to check` skipped in commits containing only docs
>    (empirical twice). Fix commit `0c837e1b4` with local build exit=0 as evidence. This round the
>    lesson's red→green run: Deploy Docs **39011946793** (red) → **39014696984** (green, see "VI"
>    for details).
>
> **VI. Round 2 Gate Conclusion Supplementary Registration (Item 13, Scheduled Check Link
> Backfill)**: Round 2 closure commit `b940300ac`'s push—— CI **39011946778** success / Auto
> Translate **39011946796** success / Deploy Docs **39011946793** **failure** (VitePress build red,
> i.e. "V.4"'s angle bracket placeholder; fixed by `0c837e1b4`); after fix commit `0c837e1b4` push
> three green: CI **39014697000** / Auto Translate **39014697044** / Deploy Docs **39014696984**.
> Connected push coverage relationship: CI@`0c837e1b4`'s **39014356642** was cancelled by subsequent
> run per concurrency (cancellation ≠ failure, descendant run covers the previous one, consistent
> with round 2 §VI's caliber).
>
> **VII. Final Gate (Closure revision `e9675c9dd`, local number; CI conclusion handed to scheduled
> check, see "VIII")**: lib 2770 passed / integration 289 passed / yx_multifile_runner 1 passed / IR
> snapshot 7 passed zero drift / clippy `-D warnings` 0 / docs-truth 0 / generator idempotent
> (re-run `scripts/docs/gen-error-code-docs.py` then `git status` clean); corpus baseline 346 → 347
> → 348 lines (each +1 behavior record, remaining lines byte-by-byte unchanged; baseline
> `tests/baselines/corpus-parity.jsonl` at `e9675c9dd` total 348 lines).
>
> **VIII. Gate Check Arrangement and Conclusion (Item 13)**: This batch's pushed CI / Auto Translate
> / Deploy Docs conclusions are registered by the scheduled check link (not waiting within the
> session, nor writing "not waited" as "already green"). **Backfill (2026-10-17 Scheduled Check)**:
> `d5c00235a`'s CI `39023417670` = **failure**——Docs Truth reports "error code documentation behind
> code table/locales", i.e. this commit missed the generated product same submission; fixed by its
> subsequent `d5dd1a37b` (regenerate generated product): `5da05b885` CI `39027788246` / Deploy Docs
> `39027788268` / Auto Translate `39027788340` all success, covering the previous one; `e9675c9dd`
> CI `39028827871` / Deploy Docs `39028827896` success; `d0cfc4e62` CI `39033963240` / Deploy Docs
> `39033963278` / Auto Translate `39033963261` success. The three commits' runs each ran to terminal
> state, this round has no concurrency cancellation. Lesson: **"local generator idempotent" does not
> equal "generated product already submitted to library"**——before closure should check CI
> conclusion commit by commit, not just look at the latest one.
>
> **IX. Batch Tree State (Closure gc, status output)**: Before closure `scratch.py status`——t25
> (holder=-) and yx49b (holder=-, previously held by the deactivated member verify-r3, team leader
> `release --force` released) both trees have no holders; `gc` deleted the two, released about 12.7
> GB; after gc `status` = "no batch tree", `git worktree list` only has main mount. In addition:
> `target/yx-scratch/yx49r3/` and `yx49r4/` are not batch trees (no meta.json), are directories
> where members put evidence/probes, not "residual batch trees". No batch-added directories outside
> the warehouse.

## P4 4.4 Proof Layer and wasm Wrap-up — Closure (4.4.1 / 4.4.2 / 4.4.3)

> **I. Batch Composition and Rounds (Batch name `yx44`)**: Three three-level tasks 4.4.1 / 4.4.2 /
> 4.4.3 and wasm attribute reachability determination in the same batch; batch verification tree
> `target/yx-scratch/yx44/tree` (first full build). Roles and rounds: implementation 4.4.2 = t1 (12
> files; local `1fc135437` → post-push `24028ffe8`) ｜ implementation 4.4.1's layer order rewrite =
> t2 (only document/comments, zero behavior change, based on `b5e68b3c9`) ｜ implementation 4.4.3 =
> t3 (11 files, including new code W1082; local `2d5e610cd` → post-push `52d2276ca`) ｜ wasm
> reachability determination = t4 (only determination, zero cleanup, determination revision
> `b5e68b3c9`) ｜ 4.4.1 closure and one document commit = t8 (7 files 173+/43-; local `29ebd2145` →
> post-push `d07993157`) ｜ independent verification = t5 (frozen `29ebd2145`) ｜ round 1 review =
> t6 (revision `1b593e993`) ｜ supplement = t9 (3 files 9+/10-; local `1ebddaf95` → post-push
> `1b593e993`) ｜ fix round = t10 (3 files 23+/23-; `657c3d5e3`) ｜ round 2 review = t11
> (`657c3d5e3`) ｜ closure = t7 (check / actual record / batch tree gc / document real build / one
> document commit). **Failure and Redo**: t6 ended in failure with `needs_revision` (the review
> round per the contract must fail and give findings, not an accident); t7 first round was
> interrupted at the batch tree gc stage, completed products left in the main repository working
> tree uncommitted, taken over by the second round "supplement gap + review"——not redoing the
> completed part (this round new: 4.4 check and evidence annotation, this actual record, three path
> rewrites pointing to the deleted batch directory, document real build and closure commit). No
> cancelled rounds.
>
> **II. Per-Task Output and Measurement Numbers (Original Commands and Output See Evidence Summary
> and Filing; Here Only Record Conclusions)**:
>
> - **4.4.1**: `layers/README.md` (including `layers/mod.rs` top comment) layer order table
>   rewritten to **actual call order** (predicate → termination → ownership, call point anchored to
>   `checker.rs::check_module_impl`), `equivalence` described per real identity (no production call
>   point); same-source declaration 7 places closure (`01-routing` / `02-stage-contract` / RFC-039 /
>   `06` / typecheck README / layers two files); mechanical reconciliation script in batch tree
>   green baseline **11/11 PASS** (5 line anchors + `check_predicate` three sites + no short
>   circuit + equivalence zero production call), mutation (c) rollback old declaration →
>   reconciliation 2 FAIL i.e. red.
> - **4.4.2**: Solver acquisition side unified to `proof/smt/backend.rs`'s process-level shared
>   singleton + unique closure port `with_shared_solver(consumer, f)`; three consumer points
>   (predicate / termination / ownership) all changed to go through the shared port, **in the
>   compilation production chain `default_solver()` call points = 0** (the only backend selection
>   point sank into private `new_backend()`). The `wasm/src/lib.rs`'s `test_smt` self-check call
>   point is **pre-existing** (earlier than this batch, from `b5e68b3c9`) non-test call point,
>   belongs to "test and independent scenario" surface, does not constitute D58 prohibition
>   violation. Independent verification self-constructed readings (one compilation): predicate acq=2
>   hit=1 miss=1 ｜ termination acq=1 hit=2 miss=2 ｜ ownership acq=2 hit=1 miss=1, three-point
>   hit/miss sum (4,4) = shared instance accumulated `Some((4,4))`; mutation (a) singleton
>   degeneration → predicate hit=0 i.e. red. **Change Vocabulary Disclosure**: This commit includes
>   `z3_backend.rs` 15 lines——`cache_stats` migrated from inherent method into `Solver` trait block
>   (returns `Some(instance-internal count)`) + 1 line comment changed pointer, no semantic
>   coupling. D58 prohibition has been mechanized: same-thread reentry → panic containing D58, probe
>   `recv_timeout(120s)` does not hang.
> - **4.4.3**: Silent downgrade when `body_checker` is missing converged to
>   `TypeChecker::ownership_inputs` (sole criterion point) + new code **W1082** (not overlapping
>   with existing W code facts: W1080 constraint downgrade / W1081 solver missing / W1090 opposite
>   direction); pin test 2/2 green; healthy path corpus 348:348 zero diff; **downgrade branch
>   production surface unreachable exhaustive proof holds** (CLI four entries + eval/repl + format +
>   LSP + wasm playground + all `ProgramKind` converges into `check_module_impl`:
>   `init_body_checker` unconditionally before ownership block, function body 0
>   return/break/continue, `body_checker` no `.take()`) ⇒ W1082 has no user-visible surface.
>   Mutation (b) delete emission → pin test i.e. red. **W1082 belongs to outward contract table
>   change (Class E), user-acknowledged**; `locales/zh.json` / `locales/en.json` and code in the
>   same commit, remaining languages supplemented by i18n bot after push (run see "IX").
> - **wasm attribute reachability determination (t4)**: `orchestrator.rs` current tree **10 entries
>   = 4 positive arms + 6 negative arms** (caliber = same-line `#[cfg(...)]` attribute text, both
>   positive and negative counted; `cfg!` runtime macro 0 hits); 4 wasm positive arms are
>   unreachable in playground runtime but **compile-time load-bearing** (call point not gated,
>   deleting arm alone ⇒ wasm compilation fails), 6 `not(wasm32)` negative arms native three-stage
>   table load-bearing ⇒ D35's envisioned "delete unreachable" has **no applicable target** in this
>   file, the real cleanup form = the whole project-mode subtree gated on wasm32 (registered P10 S7,
>   three unverified preserved as is).
>
> **III. Independent Verification (t5, frozen `29ebd2145` = post-push `d07993157`'s content,
> `git range-diff` three commits all `=`**): Gate **8/8 all green**——`cargo fmt --check` 0 ｜
> `clippy --all --all-features --all-targets -- -D warnings` 0 ｜ `cargo test` **3128 passed / 0
> failed** (7 test targets total; baseline 3126 + this batch 2) ｜ corpus diff **348:348**
> line-by-line zero difference (sort diff 0 lines) ｜ IR snapshot `UPDATE_SNAPSHOTS=1` re-run then
> `git status` zero drift ｜ `check-obligations.py` 21 = 13 + 8, containment 0 gap ｜
> `check-docs-truth.py` source code 156 / documentation 156, CLI 21 commands consistent ｜ error
> code generator re-run before and after tree all clean. Build warnings only 4 bot language W1082
> missing entries (known state) + Z3 download noise, zero code warnings. **Contract verify Caliber
> Correction (Must Two JSONL Paths)**: Corpus gate's call caliber =
> `python scripts/ci/check-corpus-parity.py tests/baselines/corpus-parity.jsonl <this generated>.jsonl`——**both
> JSONL paths must be given**; missing the second parameter is comparing baseline to baseline
> (always zero difference, criterion idling). This batch executed per the two-path caliber and filed
> (t5's `corpus-r44.jsonl` product).
>
> **IV. Mutation Evidence (3/3, Mechanically Reversible)**: All through
> `python scripts/dev/scratch.py trial` (`git apply` → run command → `git apply -R` → per-file blob
> compared with HEAD → require `git status` clean):
>
> - (a) Singleton degeneration → shared reading predicate hit=0 miss=2, test FAILED exit 101;
>   `backend.rs` HEAD=`3e2b2d150b24` → mutation `6d00bad4d4aa` → restoration = HEAD.
> - (b) Delete W1082 emission → pin test FAILED exit 101; `checker.rs` HEAD=`70c04a41e020` →
>   mutation `db0a623c9b9a` → restoration = HEAD.
> - (c) Rollback old layer order declaration → reconciliation 2 FAIL exit 1; `layers/README.md`
>   HEAD=`8b1ae875db32` → mutation `e88f9af6c42d` → restoration = HEAD. After three trials'
>   restoration, `git status` all CLEAN. Trial (c) first run failed because absolute path was parsed
>   by WSL failed exit 127 (not criterion red), changed to relative path re-run as standard.
>
> **V. Review Rounds and Conclusions**: Round 1 (t6, revision `1b593e993`) **verdict =
> needs_revision**——3 findings all document line number anchor inaccuracy (F-01 medium: `02` §"wasm
> current state" 4 groups of expired line numbers; F-02 low: `02` and `09`'s 4.4.3 delivery anchor
> `checker.rs:1283-1293`; F-03 low: `06`'s control statement), four substantive deliveries all pass.
> → Fix round (t10, `657c3d5e3`, 3 files 23+/23-, with two mechanical criteria) → Round 2 (t11, same
> revision) **verdict = pass, new finding = 0**. **Final Check Basis = Round 2's pass**. **T10
> Contract's Caliber Adjudication**: The fix surface is based on **the task record's 5 F-01…F-05**
> (F-04/F-05 are mechanical criteria), the dispatch brief is not the criterion.
>
> **VI. User Adjudication (2026-10-18) and Rule Solidification**:
>
> 1. **D58 Port Name Rewritten In Place**: `with_shared_solver(f)` →
>    `with_shared_solver(consumer: SolverConsumer, f)`, in-place correction of the body, **no
>    revision note written** (t8 completed in one commit of 4.4.1).
> 2. **wasm Determination Registered as P10 S7** (Adjudication B): With "belonging stage: P10;
>    whether blocks main line: No" + three unverified **preserved as is**; `06`'s F2 "needs
>    decision" line changed to point to this registration; t4's produced "### F2 Determination"
>    section not touched.
> 3. **Crash Item Registered as P3 3.19** (Adjudication A): Symptom / cause point to
>    `proof/smt/translate.rs`'s sort inference / D59 **Class A/B** (crash) / acceptance line (this
>    shape no longer aborts——conservative downgrade or diagnostic, corpus and IR snapshot zero
>    drift); **qualification = pre-existing** (t5 two revision comparison, see "VII" ⑤).
> 4. **Rule Solidification One** "Correction Written Directly to the Body, No Old-New Comparison
>    Left": Land `coding-rules.md` (new clause), `07-equivalence-oracle.md` §Semantic Correction
>    Branching Criterion Item 3, RFC-039 **D59 line** Item 3 (same submission registration = WBS row
>    labeled error type + in-place correction of affected body + PR required block "Error Type" one
>    line).
> 5. **Rule Solidification Two** "Contract `inScope` Uniformly Writes Path Prefix Resolvable by Path
>    Matcher": Land `AGENTS.md` Team Orchestration Convention Item 6.
> 6. **New Code W1082 Acknowledged** (Outward Contract Table Change, Class E).
> 7. **Register P10 S8** "Document Correction Form Convergence (In-Place Rewrite, Delete Revision
>    Note)": Including 7 audit checklist scope and acceptance line.
> 8. `09` Top Count Table: Second-level **63** (60 + 3.19 / S7 / S8), Three-level **156** (153 + 3),
>    with 2026-10-18 annotation.
>
> **VII. Process Facts That Must Be Honestly Recorded**: ① 4.4.2 commit used
> `SKIP=cargo-fix,cargo-clippy-fix` to skip two `--fix` hooks——shared working tree had teammates'
> work in progress, `--fix` would incidentally modify other people's files; **equivalent gate
> already manually fully run in the final tree** (fmt --check / clippy `-D warnings` both 0). ②
> `pre-commit`'s stash window indeed occurred (`Stashing unstaged files…` /
> `Restored changes from…`), after commit `git status` check shows teammate's work in progress
> intact. ③ **4.4.3 inScope Lesson**: The prose limitation
> `docs/src/ (only error code generated products)` was rejected by `changedPaths` validation ⇒
> `inScope` uniformly writes matchable path prefix (already solidified as a rule, see "VI" 5). This
> task **actually 11 files**, of which 7 because the declaration writing method cannot be parsed by
> the path matcher and were not listed in `changedPaths`, already listed one by one in the task
> result (`codes/w1xxx.rs`, RFC-013 code table, `reference/error-code/`'s index and each E segment
> page, etc.). ④ **W1082's locale Window Closed**: `locales/zh.json` / `locales/en.json` and code in
> the same commit, remaining languages supplemented by i18n bot after push (run see "IX"). ⑤ **Z3
> `Sort mismatch` Crash Determination (t5 Criterion ⑦)**: **Pre-existing**——the same probe, the same
> input on `b5e68b3c9` and frozen revision `29ebd2145` stdout **byte-by-byte consistent**
> (`Error: Sort mismatch at argument #1 for function (declare-fun not (Bool) Bool) supplied sort is Int`),
> both exit 1; static attribution consistent (4.4.2 0 line change to `translate.rs`, only
> `cache_stats` migration to `z3_backend.rs`). **Downgrade branch production surface unreachable
> exhaustive proof holds** (W1082 has no user-visible surface). Probe three lines of source code:
> `NonNegative: (x: Int) -> Type = { x >= 0 }` / `AtMost: (n: Int, x: Int) -> Type = { x <= n }` /
> `f: (x: NonNegative(x)) -> AtMost(x, x) = { return x }`. ⑥ **T5 Frozen Surface and T9's Coverage
> Difference**: t9 (three places of comment/document supplement, `1b593e993`) commit later than t5's
> frozen revision `29ebd2145`, is zero behavior change, its acceptance is covered by t9's own
> mechanical check + t6 review (t6 **re-ran** t5's key measurements on `1b593e993`, not retold). ⑦
> `docs/src/en/**` not manually modified, regenerated by the auto-translate workflow after push
> (this batch the bot landed two commits: `92d490695` i18n locales supplement W1082 remaining
> languages, `63b6cdf37` auto-translate documentation). ⑧ **T9 Out-of-Bounds Observation Three
> Adjudications**: ㈠ t9 incidentally corrected `02` same sentence's `wasm/src/lib.rs` line number
> and gating point line number (73 / 139·153·170 → 152 / 151·170·187)——**Accepted** (same caliber
> same number necessary surface, commit message already listed); ㈡ RFC-039 **D35 line** still
> writes "wasm 27 files' cfg branches"——**Kept unchanged** (the resolution line is the historical
> adjudication record, its premise correction has been carried by P10 S7 registration, not rewriting
> history); ㈢ `docs/src/en/**`'s old port name translation aligned by auto-translate. ⑨ Two paths
> in `09` / `09a` originally pointing to the deleted batch directory (`…/yx44-evidence/EVIDENCE.md`,
> `…/probe-sort-mismatch.yx`) in the closure round changed to point to
> `target/yx-scratch/close-44/evidence-summary.md`, probe three lines of source code inlined in
> place.
>
> **VIII. T8 Acceptance ④ Caliber Adjudication**: The acceptance original text requires
> `git diff | grep '修订注记\|原为\|现改为'` to be empty. Empirical **default context caliber
> unreachable**——D58 line's adjacent line D59 / D61 resolution lines themselves contain
> "修订注记 / 原为" wordings (not this task's change). Adjudication: accept by the **newly added
> line / deleted line / `-U0` increment line** three calibers all empty, and write clearly
> **subsequent same-kind acceptance only grep newly added/deleted line**——the criterion should fall
> on this change's line, not the whole diff's context.
>
> **IX. CI Gate Conclusion (AGENTS Item 13 ①, Scheduled Check 2026-10-18 17:26 Backfill; Detail
> `target/yx-scratch/close-44/gate-runs.md`)**: First push (3 commits, tip = `d07993157`): CI
> **39110908163** success ｜ Deploy Docs **39110908155** success ｜ Auto Translate Docs
> **39110908193** success ｜ Auto Translate I18n **39110908144** success; this tip covers
> `24028ffe8` (4.4.2) / `52d2276ca` (4.4.3) / `d07993157` (4.4.1) all three commits' content. Second
> push (1 commit, tip = `1b593e993`): CI **39112293391** success ｜ Deploy Docs **39112293062**
> success ｜ Auto Translate Docs **39112292903** success; covers all four commits including t9
> supplement. **No concurrency cancellation**, latter covers former. Unrelated: Nightly
> **39081392694** failure's headSha = `290f9ebc`, not this batch's commit surface. Closure commit
> (this document commit) pushed conclusion backfilled by the second round scheduled check.
>
> **X. Two `pull --rebase` (Automatic Commit Opposite Party During Push)**: The opposite parties are
> auto-translate and i18n commits respectively; three delivery + supplement commit's hashes
> rewritten (local `1fc135437` → `24028ffe8`, `2d5e610cd` → `52d2276ca`, `29ebd2145` → `d07993157`,
> `1ebddaf95` → `1b593e993`), `git range-diff` checked **each commit content unchanged** (three
> commits all `=`). Members did not execute `git push` / `fetch` / `pull`, synchronization and push
> uniformly by team leader. **Evidence Reminder**: After this, reference commit numbers uniformly
> use **post-push hash** (the right value in the above list); `09`'s check row per convention does
> not write commit number.
>
> **XI. Uncovered Surface (Cannot Write "Not Tested" as "Passed")**: ① doc-tests 0/0 not carrying
> criterion; ② **wasm32 target actual compilation not done** (this surface is covered by CI's
> `_build-wasm.yml`, this batch's wasm determination is static argument); ③ Mutation only covers the
> 3 places named by the contract, other gates not done reverse break-bad verification; ④ Corpus diff
> not single-run on `b5e68b3c9` (only run on frozen revision); ⑤ i18n bot supplemented remaining
> language W1082 text behavior not verified (only checked `zh.json` / `en.json`); ⑥ Round 1 review
> and fix round reported **same-kind but not in contract findings pre-existing drift** not fixed
> (list see "XII"), attribution pending user adjudication.
>
> **XII. `02` All-Article Pre-existing Drift Observation List (Reported to User, Pending
> Registration After Adjudication)**: Caliber = all are **pre-existing drift** (not introduced by
> this batch), **does not constitute this batch's finding**; the list was discovered by the fix
> round, round 2 review independently sampled on `657c3d5e3` and gave mapping, recorded item by item
> (full list filed `target/yx-scratch/close-44/s9-observations.md`):
>
> 1. `02` §(a) Entry Table 3a-3d / 5 / 5b: `02:72` quotes `orchestrator.rs:273-399` → current
>    `:155`; `02:73` `:450` → `:250`; `02:74` `:1374` → `:985`; `02:78` `util/diagnostic/mod.rs:565`
>    → `:681`; `lsp/handlers/diagnostics.rs:146`'s `run_diagnostics` no longer there; **Content
>    level**: `run_diagnostics` and "otherwise manual lex→parse" have been replaced by 4.6.1 (delete
>    manual sequence).
> 2. `02:1016`: `compile_project :99-237` → current `:107-134` (28 lines Program constructor).
> 3. `02:360-361`: "downgrade to warning when missing" → current tree `_build-wasm.yml:50-51` is
>    `::error` + `exit 1` (#435); `build.rs:65-72` has been changed to `panic!` hard failure;
>    `_build-wasm.yml:35-59` interval needs re-take.
> 4. `02:127` mechanism pointer: mechanism actually in `src/driver/arms.rs::typecheck_orchestrated`
>    (`:288-289` self-described); conclusion still holds (multi-file arm drops warnings), inaccurate
>    is the "result is the only downstream use" sentence (`type_result` also goes through
>    `arms.rs:1389` → `helpers.rs:259/:636` feeding `overload_resolutions`); suggestion = pointer
>    change `typecheck_orchestrated`, "only downstream use" change to "warnings field has no read
>    point".
> 5. `02:128`: `check_project:385-395` / `250-252` needs re-take.
> 6. `02:334` (§S5 (d) Investigation Section): The old code snippet quoted has been replaced by
>    4.4.3's `ownership_inputs` criterion point.
> 7. `02` Change List Table Remaining Rows: `predicate.rs:34-36`, `checker.rs:1303-1314` / `:1244`,
>    `backend.rs:67-72`, `wasm/src/lib.rs:30-36` / `:42-51`, `util/diagnostic/mod.rs:621-661`.
>    **This Closure Does Not Preset S9's WBS Item** (user not adjudicated); after adjudication then
>    per registration form land `09`.
>
> **XIII. Batch Tree Recovery (gc and status Original Output; The Tree Path in the Output Is
> Recorded Per Repository Relative Path, The Original Is Absolute Path)**:
> `python scripts/dev/scratch.py gc` → `[scratch] 已删除 yx44-evidence（释放约 0.0 MB）` /
> `[scratch] gc 完成，共释放约 0.0 MB`; `python scripts/dev/scratch.py status` →
> `yx44-evidence            holder=-` next line `target/yx-scratch/yx44/tree`——the path recorded by
> the registry **does not exist**, this name is the registry residue (`gc`'s `_remove` deletes
> `meta["path"]`'s parent directory i.e. `target/yx-scratch/yx44`, the residual `meta.json` still
> falls in the scan surface; its content batch=`yx44`, `base_rev=29ebd2145`, `lock=null`, containing
> t5 / t6 / t11's holding history); The closure round renamed this registry file to
> `meta.retired.json` to make it exit the scan (**does not delete any evidence file, does not create
> any new tree**) → `python scripts/dev/scratch.py status` = `[scratch] 没有批次树`; then run `gc` =
> `[scratch] 没有批次树，无需回收`; `git worktree list` only has main mount
> `E:/git/YaoXiang 657c3d5e3 [dev]`. The batch original evidence directory
> `target/yx-scratch/yx44-evidence/` (no `meta.json`, **not a batch tree**) is retained as filing,
> the closure summary is separately filed at `target/yx-scratch/close-44/`. Acceptance: `git status`
> clean + no batch-added directories outside the warehouse (`E:/git` and `E:/` top level check) + no
> residual batch tree.
>
> **XIV. Document Real Build and Commit Hygiene**: `npm run docs:build` locally **exit 0** (AGENTS
> Item 10 ⑨——markdownlint cannot catch VitePress's Vue compilation error, and commits containing
> only docs will be judged as `no files to check` skipping markdownlint); this batch's check /
> annotation / registration / rule modification accumulated into **one** document commit (Item 8),
> landed in batch closure; members did not execute `git push` / `fetch` / `pull`. **To-Push List**
> (commit by commit + coverage explanation, handed to team leader): `657c3d5e3` (round 1 findings'
> document anchor point fix, 3 files) + this closure document commit (check / actual record / three
> path rewrites / registration and rule modification).
