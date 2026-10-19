# WBS Historical Notes (Implementation Record · Audit · Decision Trail)

> This document is a **historical note** for [09-execution-wbs.md](./09-execution-wbs.md) (work task
> table): implementation records, audit findings, decision trails, and corrections from each phase.
> **Division of labor**: 09 describes only the **plan** (kanban, three-level task table,
> dependencies, risks, acceptance); this document records only **history** (what was done, what was
> found, how decisions were made), appended chronologically without rewriting history.

## P2 Establishing Equivalence Criterion Baseline

> **P2 implementation (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green
> run. **D38 prediction fulfilled** — during the green run, three real defects in ir_gen were found
> and fixed (see 06 §G6 for details): method locals table truncated to parameter count (local_count
> underestimated, runtime relies on Frame::set_slot for fault tolerance), un-annotated parameters
> dropped by filter_map (three places, params metadata misaligned with frame layout), match result
> slot relies on implicit Void initialization of the frame (now explicitly pre-initialized). The
> actual semantic of jump target is the flattened instruction index (not the label-keyed form in
> 07's text); CFG is built at instruction level — recorded in verify.rs file header. **Commit
> cross-reference**: ir_gen fix = `a9ea199b`; verify main body = `d5b18390` (⚠ parallel flow
> COMMIT_EDITMSG collision caused its message to be mislabeled as "docs(rfc): RFC-027a…", content is
> actually verify.rs + six test files, buried in history middle and cannot be amended, hereby
> recorded).
>
> **P2 narrowed portion implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> stop-bleeding channel precondition unlocked**.
>
> - 2.3.2: 5 fixtures in `tests/yaoxiang-multifile/` (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; decision contract uses the same `TestFileSpec` as single-file
>   corpus (library-side single point), binary location / subprocess spawning abstracted to
>   `tests/common/` with yx_runner migrated in sync (Rule 3).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. Trigger source uses the form
>   `SumUpTo(3, r)` nailed by rfc027 tests (return position / call site / binding position) — 02's
>   `Sorted(3)` annotation example in current code would be substituted for evaluation and directly
>   Disproved, not showing Unproven silent; 07's pseudocode Sorted(3) is only illustrative. Red
>   state measured data in test file header.
> - Two red criteria attached `#[ignore]` permanently per the 2.4.5 red skeleton precedent: default
>   suite all green, `-- --ignored` reproduces red state; **removed when P3 fix lands to turn
>   green**.
> - The reason for deliberate deviation from 07's (code, file, line) table in comparison
>   normalization (removing file/line, comparing only error code sets) is in 2.4.1 test comments.
>
> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **moved out of P2**: the `Program` / `Obligations` types they reference only exist in P4, P2
> cannot compile. The former is P4's 4.1.4 (avoid double registration), the latter's `#[ignore]` red
> skeleton merged into 4.3.1.
>
> **2.3.3 implementation (2026-10-07)**: C4 mandatory coverage list item-by-item check (07 §143), 10
> semantic corpus categories fully covered and all within corpus-parity.jsonl diff baseline (26
> representative files 26/26 hit, grep verified) — ref/borrow/move 05-ownership full directory
> (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep etc.), closure capture
> closures + closure_arg_inference + spawn_capture, currying curry_value_fix + curry_tail_expr,
> spawn 04-concurrency ×13, iterator for spawn_for etc. 16 files, sum type sum_type_* +
> pattern_or_guard + match_call_scrutinee, existential enforcement point interface_dynamic_dispatch
> (Vec(Animal) existential type heterogeneous packaging triggers existential_coercions) +
> interface_rebind_dispatch, `?`/Try propagation question_propagation + option_try + try_methods +
> user_try_type, method overload method_overload (+ method_overload_ambiguous_err on diagnostic diff
> side), refinement constraint Drop sequence refined_* ×5 + 05-ownership-driven ReleasePlan (span
> key contract has another 2.4.4 red criterion permanent), and so on. Performance baseline
> `benches/pipeline.rs` checked in (07 §145 three smoke tests of two): `corpus_compile_frontend`
> full corpus compile first measurement 17.6s, `cli_cold_start_hello` cold process startup 169.7ms;
> interpretation throughput is covered by benches/lib.rs hotpath group (interp_fib_recursive_27 /
> interp_loop_steady_10m), no duplicate construction. Regression gate check-perf-regression.sh
> consolidated to P9 unified list (2.5).

## P3 Fix Correctness Vulnerabilities (Minimum Plan)

> **Error type labels for completed rows** (retrospective labels, no history rewriting): 3.1 = B |
> 3.2 = B (defensive empty arm, criterion turning green borne by 3.1) | 3.3 = A | 3.4.1 / 3.4.2 = A
> (new W codes) | 3.4.3 = equivalent refactor (manual review) | 3.4.6 / 3.4.7 = D (user-visible
> surface, already user-adjudicated) | 3.4.8 ①② = C (main) + A (three ancillary) | 3.5 / 3.6 / 3.7 /
> 3.8 = D (all user-adjudicated).
>
> **P3 diagnosis (2026-10-12, requested by user)**: A differential diagnosis for the perception of
> "P3 items continuing to grow".
>
> **I. First clarify scale (perception amplified by two accounting defects)**
>
> 1. **Duplicate numbering**: `3.5.x` is shared by two task families — (a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 three-level items: lexical and parser branches / AST `is_pub` /
>    checker side / dead code exemption convergence / peripheral echo / std and diagnostic text /
>    language reference convergence, all checked); (b) 2026-10-10 registered **3.5 Higher-Kinded
>    Type parameter call path completion** (3 three-level items). Both share `3.5.x` ⇒
>    cross-reference ambiguity (**2026-10-12 renumbered per user ruling: 029g family changed to
>    `029g.x`**), and caused 029g's rows to be miscounted into P3. **Pending ruling**: rename the
>    029g family to `029g.x` (or move to its owning phase), synchronize other references in the
>    document.
> 2. **Missing checks**: All 4 three-level items of `3.7` (3.7.1–3.7.4) are completed (d922a551 /
>    38bd4b9e) but not checked ⇒ this item corrected in this round.
>
> **II. P3 actual accounting (12 secondary rows)**
>
> | Row                                                                   | Status                                                                                                                                                                                  | Disposition                                                                                                                     |
> | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                       | Completed                                                                                                                                                                               | —                                                                                                                               |
> | 3.4 Silent drop point addendum (8 three-level)                        | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (precondition 4.2.1 completed 2026-10-09 with be60e01f); 3.4.4 and 3.4.5 **still blocked** (precondition 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4/3.4.5 unlocks with 4.3.1                                                                       |
> | 3.5 Higher-Kinded Type parameter call path completion (3 three-level) | Not started (declaration side active, call side broken)                                                                                                                                 | Keep in P3; **does not block main refactor line**, but is "accepted feature not implemented", priority higher than pure cleanup |
> | 3.6 Function type value call path (1 three-level)                     | Not started (semantics already cut (i))                                                                                                                                                 | **Keep**: correctness defect, criterion clear                                                                                   |
> | 3.7 init segment call name qualification (4 three-level)              | **All completed**                                                                                                                                                                       | Check and close in this round                                                                                                   |
> | 3.8 native shadowing (1 three-level)                                  | Not started                                                                                                                                                                             | **Stuck on decision** (define name resolution priority first), not a scheduling issue                                           |
> | 3.9 / 3.10 / 3.11                                                     | Completed (2026-10-12)                                                                                                                                                                  | —                                                                                                                               |
> | 3.12 Runtime pattern executor config propagation (1 three-level)      | Not started (D class, user ruling B: separate team)                                                                                                                                     | **Recommend moving out of P3 → P8 (execution layer)**, pending ruling                                                           |
>
> **IV. Leverage of 4.3.1 (main line marker)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the precondition for **3.4.4**, **3.4.5** and **4.5.1** (cross-layer contract
> PlanId-ization) ⇒ taking it once advances the main line and clears two P3 stranded items. 3.5
> (HKT), 3.6 (function type value call) and 4.6.1/4.9.1 all have **no unsatisfied preconditions**,
> available anytime; 3.8 is stuck on semantic decision (user-defined priority vs explicit
> diagnostic).
>
> **III. Mechanism recommendations (prevent recurrence)**: Newly registered secondary/tertiary rows
> **must carry two fields** — "owning phase" (which P it really belongs to) and "whether it blocks
> the main line"; `pending scheduling` is no longer the default state, unmarked is treated as
> unschedulable. This way, bloat is visible, sortable, rather than undifferentiated pile-up.
> **Pending ruling**: whether to write this into the registration convention of this document and
> `src/dev/coding-rules.md`.
>
> **P3 implementation (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + sole getter (at the
> time recorded as `pub(crate)`; **2026-10-07 verified actual implementation is `pub(super)`** — the
> `#434` ruling cited by code comments has no record on docs side, see 3.4.3; all-private requires
> rewriting 11 construction literals, not taken at stop-bleeding phase; consolidated during P4
> Obligations migration); execution mechanism abstracted to `src/frontend/proof_execution.rs` (Rule
> 3 single point — cannot go in typecheck/, that would create L3→L4 reverse dependency); 3.1.2
> supplemented four entries per 02 evidence chain step 5 (WBS row number missed compile_project
> `:99`, must be added and registered as required by 3.1.1 acceptance "criterion turning green");
> 3.2.1 empty arm changed to accounting (proof_calls throw) + diagnostic (`into_result` established
> conversion path); 3.3.1 SOLVER slot Option-ized, Z3 missing conservatively downgraded per
> `SMTResult::Unknown` (E2031 family diagnostic replaces process crash). Two vulnerability criteria
> removed `#[ignore]` and turned normally green, supplementary corpus fixture
> `proof-obligation-honored`; check-fanout inventory migrated accordingly (wiring point
> 19=inventory). Measured: criteria 2.4.1/2.4.3 turn green, full suite lib 2494 / integration 272 /
> corpus both layers all green.
>
> **3.4 audit (2026-10-07, P3 post-implementation review)**: For the defect class "failure produces
> no signal" (definition in 02 §41-52), recounted on unchanged code, located 8 more places, all
> registered as 3.4.x in the above table. Methods and evidence:
>
> - **Consumer-side enumeration** (16 obligation fields grepped repository-wide for read points):
>   `variant_ctor_calls` **zero consumers** — the only read at `checker.rs:1468` is "self-injected
>   into `TypeCheckResult`", IR side uses form detection at `ir_gen.rs:7374`; `rfc011b.rs:659/688`
>   only asserts production side, exactly the "only tests production side, naturally immune to
>   consumer-side defects" form criticized in 02 §186 → 3.4.4. `method_overload_ir_names`
>   result-level field similarly zero external consumers, IR uses parallel `overload_resolutions` →
>   3.4.5.
> - **Phase coverage review**: 02 §91 table row 1 (four entries "no proof_execution") has been fixed
>   by 3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 unchanged (multi-file `run` still doesn't
>   output W1003, doesn't run dead code family) → 3.4.6; LSP non-project-internal path (02 §77
>   "manual lex→parse") similarly doesn't run proof → 3.4.7.
> - **Same-shape empty arm scan** (`Unproven` repository-wide consumers only 4 places): checker
>   three refinement branches (covered by 3.1), 3.2.1 fixed ownership arm (**source unreachable**),
>   and `environment.rs:422` (**reachable, zero diagnostics, zero accounting, and zero registration
>   in repository docs**) → 3.4.1, the only "all unknown" item in this audit.
> - **Solver acquisition surface**: `default_solver()` three consumer forms differ
>   (`predicate.rs:32` process singleton / `checker.rs:1266` inject per `check_module` /
>   `ownership.rs:621` new per back-edge determination) — 3.3.1 only eliminated one panic; the
>   **silent surface** of the other two belongs to 3.4.2, **singleton-ization and cache hit**
>   belongs to P4's 4.4.2 (avoid double registration).
> - **Not counted in 3.4 existing ownerships**: 2.4.4 (`ReleasePlan` span key mismatch, 168 files /
>   373 keys unconsumed, Drop silently lost) still P2 red state permanent, ownership 4.5.1/D20
>   unchanged; 4.4.1 (layer order declaration opposite to actual, `equivalence` layer not in
>   pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger downgrade) already in P4.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 preconditions, can be landed in place
> anytime (recommend doing 3.4.1 first — the only "repository-wide zero registration" live
> vulnerability, smallest change surface); 3.4.4–3.4.8 follow P4 execution, their criteria depend on
> obligation ledger (4.3.1) or unified Driver (4.2.x). **3.4 does not establish a new criterion
> system**, uniformly reuses existing C2/C3 and P9's 9.2 gates.
>
> **Batch 3.13–3.16 record (2026-10-13 registered → 2026-10-15 closed)**: User 2026-10-14 ruling to
> merge four relevance small items (#451 method binding panic / #452 named actual arg to resolution
> / #453 dead inference entry / #454 native export named arg — last item is a boundary discovery
> when verifying 3.14, same day attribution experiment verified **not introduced by #452**, user
> ruling ① merged into this batch) into one team. Closure status: all four completed, ruling
> registered as RFC-039 **D60** (six binding rules + shared criterion single point).
>
> **I. Two contract corrections (lesson: the contract itself can be wrong)**
>
> 1. **Acceptance fixture self-contradictory**: #452 task contract simultaneously requires "mixed
>    actual args output 20" and "position and name conflict on same position error", but nailed
>    fixture `Temp.pick(self, a)` has only one value parameter — same arg combination is both
>    "mixed" and "conflicting". First implementer reported blocker as-is and set task failed,
>    overall revert (no silent for green). Disposition: task re-assigned + contract revision — give
>    **six binding rules** and **8 value-level matrix** (fixture changed to two value parameters
>    `pick(self, n, a)`) then attempt 2 by second implementer completed. Six rules full text
>    word-for-word in D60.
> 2. **inScope missed mechanical-required surface**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 registration, only ran `code-tables --fix`,
>    **did not re-run** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth red ("source
>    registered/emitted but document missing 1: E1110", source 154 / document 153, run
>    **38518624964** — this batch's only CI red line). Fix = mechanical regeneration (8 files
>    +83/−70, only touched `docs/src/reference/`), reconciled 154/154, CI 38520209026 green.
>
> **II. Review → fix → re-review two closed loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed with resolution ⇒ calls containing named args in multi-candidate
>    zero-hit/single-candidate hit paths entire chain silent) → fix round added external code
>    **E1111 (argument_order_invalid)** (code table + locales + RFC-013 + generator four-piece same
>    commit; codes 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared parsing entry + native arm settle by name) → independent
>    verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations via
>    `scratch.py trial` mechanically reversibly replay all caught, restoration byte-for-byte
>    identical) → round-1 review needs_revision (F1 = typecheck native arm **inline copied** one
>    binding criterion, missing rule ④ ⇒ method and native same-form call **ruling divergence** —
>    method reports E1111, native silently exits=0; ir_gen has another third existing copy reporting
>    old code E1015) → convergence round consolidated criterion to single implementation
>    `bind_args_to_params` (`arg_binding.rs:35`, four call points `expressions.rs:2727` / `:5416`,
>    `ir_gen.rs:7776` / `:8279`) → round-2 review caught **regression** introduced by convergence
>    round (IR convergence block lost native omittable position ⇒ previously successful program with
>    omittable optional position was misreported as E1010) → fix round (optional position criterion
>    and parameter name co-emit from sole exit at `signature.rs:236`) → round-3 review PASS
>    (independent reproduction red→green; under mutation nailed FAILED verified criterion can fail).
>    **Lesson**: review must cover "unfixed branches of similar paths" (this batch's two
>    needs_revision are probes all falling on hit surface); fix round itself can also introduce
>    regression (optional position), caught by next round's swap reviewer.
>
> **III. Process events (registered as-is, not pointing at individuals)**
>
> 1. **External whole-tree discard (2026-10-14 around 17:14)**: Working tree silently restored by
>    external means — implementer's fix in progress (typecheck/ir_gen/code table/locales/RFC-013
>    etc.) all disappeared, **at the same moment the two review documents edited by user himself
>    also disappeared from git status** ⇒ judged as repository-level discard (most likely user-side
>    manual cleanup / IDE "discard all changes"). Three evidence excluding group members: stash
>    empty, no new commits, reflog no reset. Implementer replayed in-progress work by context and
>    pushed; discipline changed to "**first save patch to scratch, then take red state**".
> 2. **Verifier context exhausted, swap**: The independent verifier of 3.16 ran out of room mid-way
>    (no closing message), swapped per load rule — successor continued using ready products (green
>    state results, corpus equivalence script, mutation plan), verification eventually PASS.
>    Positive precedent for partial product handoff.
> 3. **Review round 5 swap and historical round rollback**: When swapping review per load cap rule
>    ⑤, `remove_member` **also rolled back** already adjudicated terminal review/verification rounds
>    to shared pool, scheduler immediately re-ran (t3/t8 re-taken by new members, duplicated work
>    and blocked downstream). Disposition: team lead took over and canceled rolled-back historical
>    rounds (t3/t8/t13), based on which added AGENTS.md rule 12 (handle rolled-back rounds
>    immediately after swap). This batch's closure record registers this event per that rule.
> 4. **scratch governance (about 6.3 GB)**: Early batch one external tree per task, 6 directories
>    scattered outside repository (about 780 MB, cleaned) + two members' external verification trees
>    (about 2.2 GB / 3.3 GB, self-deleted after use). Produced governance tool
>    `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md rule 9
>    (batch tree sole entry), rule 10 (verification environment six rules: mechanically reversible
>    mutation / line number taken from referenced revision / CI-only gate must run etc.), rule 11
>    (writer load cap), rule 12 (handle rolled-back rounds after swap) and top meta-rules two (no
>    private content / verify before assert).
> 5. **Contract defects (not pointing at members)**: #454 fix round's contract inScope only circled
>    2 files, actually mechanical-required surface is 8 (rule 6 exactly guards against this —
>    contract template should preset mechanical-required surface list); another, this task's
>    terminal fields were once occupied by probe payload, formal acceptance/command evidence was
>    supplemented by team lead per convention.
>
> **IV. Leftover items (this batch not fixed, registered for reference)**: round-3 review three
> low-severity findings — ① Method path's `build_arg_slots` (`ir_gen.rs:521`) is still the
> criterion's **second** implementation (existing, per last round convention registered at closure,
> not consolidated) — **2026-10-16 pre-release audit added three points**: (a) it has **two** call
> points: `ir_gen.rs:7888` (method binding path, uses bound function `binding.function`'s parameter
> names) and `:7975` (overload resolution path, uses resolved mixed name leaf segment), original
> registration only wrote "`ir_gen.rs:521` area"; (b) it has **diverged from shared implementation
> `bind_args_to_params`**: missing rule ⑤ (duplicate name → E1015), rule ⑥ (missing required
> parameter → E1010) and "optional position" concept, and hard-codes receiver position to `1 + i`;
> both call points `continue` uniformly for empty slots (comment says "diagnostic belongs to
> typecheck (it guarantees no empty slots after resolution passes)"), is a **dangling invariant
> written in comment with no assertion bottom**; (c) **RFC-039 D60's "all call points converge to
> it" enumeration missed these two**, is declaration inconsistent with fact, and cited line numbers
> were not re-taken on referenced revision, D60 text corrected with this item; ② Batch verification
> tree (worktree) `cargo fmt --all` error is worktree workspace artifact, gate list should note fmt
> goes to main repo; ③ repair-type contract template should preset mechanical-required surface into
> inScope (see III.5).
>
> **V. Final gates (closure revision, local two implementation commits + one doc commit unified push
> — this batch's only push)**: fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` **2730
> passed / 0 failed / 2 ignored** / `--test integration` **284 passed** / `--test yx_runner` 2
> passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 / corpus_probe **342
> rows line-by-line consistent with baseline** (only regenerated row: `named_arg_duplicate_err.yx`
> E1015@14 → E1111@16, expected code and line number synced, manual reviewed) / IR snapshot zero
> drift.
>
> **VI. run links (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (same run's MSRV/Test canceled by subsequent push
> per concurrency — cancel≠failure, covered by successor run); governance tool push → CI 38546373435
> / 38561405117; WBS registration (3.16 row + table header 56→57 / 147→148) → CI 38528560364.
> Closure push's own CI / Deploy Docs run registered in task record after push.
>
> **3.4.8 ③④ implementation (2026-10-11, 8ecf56f93)**: ③ Added `Linking` at end of `CHECK_STAGES`
> (**pure merge**, entire block skips `is_bin_role` entry validation — E3020 criterion still belongs
> to `RoleClassification` to prevent double reporting; also hard reason: run path determines "has
> main" by merged IR, under collection semantics "entry itself errors → no IR" would be misjudged as
> missing main and hard-aborted) → `Monomorphization` (check form); `ir_generation_check` retains
> unit IR (**in memory only**, for pure merge and mono consumption, no external emit / no disk write
> / no snapshot — without retention, aggregation request is empty, mono takes `NoObligations` fake
> green); mono failure in Check takes per-file collection channel (diagnostic belongs to entry
> file + `failed_stages`, not set to failure), MultiFile keeps `fail_with`; `into_outcome` changed
> to "take collection first then judge failure". ④ After empirical verification **zero production
> change**: full program BFS already covered cross-unit mutual recursion (pathological → E3005,
> well-formed three-layer chain executes correct value), delivered four e2e groups + one mono unit
> test as live criterion.
>
> **Verification chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross-comparison with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-form matrix
> (single/multi/check three forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increased).
>
> **F1 (discovered in verification period, D59 A/B class, same round fix)**: Under standalone
> multi-file check without project root, aggregation request name (`lib.f`) and unit IR definition
> key (`f`) are different sources → mono lands "native no IR definition" silent `continue`, check
> reports 0 error/exit 0. Fix: aggregation side adds new reverse-check jump by each unit's **actual
> definition key** (precise enumeration + literal equality + ambiguity abandon, not suffix
> heuristic; project form not registered, existing path unchanged). **Correction (same day)**: This
> reverse-check jump was **deleted** in subsequent regression fix; root cause resolved from naming
> caliber by `module_key` one-position-two-use decoupling (added `script_mode`) — IR definition key
> = IR call name = aggregation request name = `{unit key}.{name}`. **Regression record**: After ③
> implementation CI's `Check tutorial/guide code examples compile` red (258 blocks / 10 failures: 6
> same-file same-name rebinding + 4 collide with `<std/list>`, rootless check doesn't do
> qualification triggers `link_module_irs`'s conflict check); fix = ir_gen split `script_mode`
> (qualified name ⊥ Script/Bin, E3023 gate changed to look at it) + `ir_generation_check` per-unit
> constantly pass unit key + `.with_script_mode(rootless)` + Script top-level same-name rebinding
> "latter takes effect" (consistent with run path existing behavior); verification = doc gate 258/0,
> three-binary comparison (HEAD/③tree/fix tree) three collision fixtures, check form E3005 no
> regression, E3023 not opened.
>
> **F2/F3 (accepted)**: mono diagnostic belongs to entry file item, span can cross units (same
> caliber as run path `path=entry`); Check form holds one ModuleIR per unit (in memory only,
> released at end of check).
>
> **Pre-existing issues left for follow-up (not this round's regression)**: ① E3005's help text
> still writes "internal error; please report this issue", inconsistent with resource protection
> semantics (precedent: `e3xxx.rs:47` once moved similar "user write error" out of E3005) —
> registered pending; ② multi-file E3005's span render label misalignment (reproduced with HEAD
> binary, pre-existing); ③ Request collection surface has **type parameter position dependency**
> (generic calls where type parameter is not at value parameter position don't emit instantiation
> request, mono records `NoObligations`, convergent mutual recursion is this form) — same surface as
> 3.5 (HKT call path), registered to that row.

## External Track RFC-029g: Remove `pub` and Auto-binding (#399)

> **P3.5 implementation (2026-10-08)**: All seven secondary tasks completed, entire batch one commit
> (independently revertible).
>
> - **Measured inconsistent with plan (registered as-is)**: 029g.1.2/029g.1.3's acceptance
>   originally wrote "`pub f = ...` reports **parse error**" — actually after deleting `KwPub`,
>   `pub` falls back to ordinary identifier, old writing falls to semantic layer reporting **E1001
>   Unknown variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specially detecting
>   removed syntax is anti-pattern", **do not add** migration-specific diagnostic; corpus fixture
>   `pub_declaration_err.yx` nails E1001, document (`syntax.md`) writes this consequence as-is.
> - **Design convergence (beyond RFC literal, requires review)**: `project_refs` and
>   `cross_file_refs` two fields after pub deletion are semantically completely overlapping (same
>   "intra-package reference union", Rule 1 prohibits parallel representation) → merged to single
>   `project_refs`, `set_exempt_pub`/`set_cross_file_refs` two switches deleted. Behaviorally
>   Internal role's non-pub items change from "intra-file reachability" to "reference pool judgment"
>   (loosened, missing report direction); Lib tightened from "pub absolute exemption" to reference
>   pool judgment; Script unreferenced top-level bindings start reporting W1001/W1002/W1004/W1005.
> - **Dead table cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports writeback all deleted (production-side table
>   fill, repository-wide zero consumption).
> - **Test migration** (test count not decreased): Added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub recognized" assertion reversed to "pub no longer
>   modifier"; `passes/tests/dead_code.rs` 7 role semantic cases rewritten (assertion reversed 2);
>   `tests/integration/cli_e2e.rs` 8 role cases rewritten (`lib`/`internal` exemption assertion
>   reversed to report assertion).
> - **Baseline**: IR snapshot **zero-diff** (`pub` not in IR); corpus diff baseline only added 1
>   (`pub_declaration_err.yx` → E1001), command `cargo run --quiet --example corpus_probe`
>   regenerated then manual review.
> - **Measured**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-Layer Contract PlanId-ization (D20)

> **4.5 implementation record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, closure
> (registration / WBS check / gate / push) by t7. Precondition = 4.3.1 obligation ledger (red state
> archive 173 files / 533 keys).
>
> **I. Carrier ruling (A: `PlanId` goes with AST node)**: Added `PlanId` (`src/util/plan_id.rs`,
> `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` for synthetic nodes
> outside parser), issued by **sole allocator** `ParserState::alloc_plan_id`; carrier two places —
> `Stmt.plan_id` (statement identity) and `Expr::Call.plan_id` (call site identity), `Expr::call` is
> the repository-wide sole call constructor. Voted down was the "two sides independently derive same
> identity by span" position-coincidence form (D20's original defect).
>
> **II. Two-level verification and review (members independent)**: Each functional commit paired
> with one independent verifier and one reviewer, **verification/review don't reuse implementer's
> evidence**: 4.5.1 = t1 implement / t2 independent verify (external tree + 5 mutations) / t3
> review; 4.5.2 = t4 implement / t5 independent verify (external tree + 5 mutations) / t6 review.
> Review time point and reviewed revision both fall on immutable history (`git show` verifiable).
> **Provider rename caused t5 / t6 member swap continuation**: Original verify/review members
> couldn't continue due to provider rename, per orchestration convention swap and **state reason in
> record** (cross-round context retention priority, only swap when compression unavailable); after
> continuation, conclusion consistent with original round (PASS), implementer's evidence not reused.
>
> **III. Two defects found and fixed during self-check at implementation (t1, both within
> inScope)**: ① Consumption exemption criterion was once on `UnitState.ir.is_some()` — `check`
> form's Linking/Mono takes `unit.ir` away, actual unconsumed in check form **silently passes**
> (mutation test: D41 red but CLI check exit 0); changed to read **the ledger's own completion
> mark**, added regression case. ② Drop sequence drifts with process hash seed (partial order
> `sort_by` + `HashSet` iteration): snapshot criterion three consecutive runs gave 106/120/114 row
> drift; changed to **total order sort** (same name takes max depth + name ascending) then two
> consecutive runs zero drift.
>
> **IV. Criteria and evidence numbers**: After 4.5.1, `test_release_plan_spans_consumed` removed
> `#[ignore]` permanent (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated same
> commit (programmatic review: added 1083 lines Drop + 86 lines jump index translation, zero delete,
> no reorder), corpus 342 diff empty; 4.5.2 added `test_overload_resolutions_consumed` (difference
> set empty + evidence surface floor `examined ≥ 1` / `consumed ≥ 2`; measured examined=1, consumed
> items 2, no resolution skip 210), resolution form 9 nailed, `cargo test --lib` 2712 passed / 0
> failed, IR snapshot **zero drift**. Mutation drive: consume key shifted one position ⇒ criterion
> red + real program `error [W1090] ... output key not consumed` + corpus gate difference; restored
> then re-green.
>
> **V. Classification basis for semantic correction (D59)**:
> `Type.method(position arg, name = value)` **silently gave wrong answer** before change (measured
> compile clean, run output `1` not `20`) ⇒ **Class C** (old behavior not successful), fix takes
> "loud": after change, same program compile-time `error [W1090]`; "named arg participates in
> resolution" terminal semantics is **Class D**, registered §3.14 and left issue #452 for dedicated
> team. Another: `Drop` in interpreter is no-op (`backends/interpreter/executor/ops/control.rs`
> wildcard arm only `advance()`) — key migration doesn't change runtime behavior.
>
> **VI. F-1 / F-2 disposition (t5 independent verification found)**: **F-1 (medium)** No binding
> consumption point (`ir_gen.rs:7766-7790`) is unreachable in current corpus and test surface, key
> correctness has no red state falsifiable ⇒ per "known boundary" registered into
> [02](02-stage-contract.md) §2 revision note fifth paragraph (preservation reason + no new task);
> **F-2 (low)** Pointer drift ⇒ `types.rs`'s `OverloadResolutions` doc comment changed to precise
> pointer (production `inference/expressions.rs:4550`, consumption `ir_gen.rs:7697` / `:7772`),
> separate one small commit, not mixed into doc commit.
>
> **VII. Closure (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settle mark); 07
> criteria table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG one line behavior change; `check-obligations.py` and `check_tracking.py` green, CI and
> Deploy Docs green. **Accounting deviation found at closure**: t3 / t6 report's pointers
> `ir_gen.rs:357` / `:3102` / `:1096` differ from code fact by one to three lines (actually `:360` /
> `:3103` / `:1095`+`:1097`, due to 4.5.2 insertion) — register with code fact, this note leaves
> trace.

## P4 4.10.3 Standalone run and MultiFile Orchestration Unification (4.2.5 Ruling A's run half)

> **Third batch record (2026-10-15 registered → 2026-10-15 closed)**: User 2026-10-15 ruling C —
> this round only does 4.10.3 (same batch candidate 3.4.6 per B1 ([02](02-stage-contract.md)
> §Runtime Behavior Table "dead code family warnings don't enter compile path") deferred until after
> 4.9, precondition changed hook and noted); implementation form = **complete single team** (user
> instructed not to open DAG, not build team, single-session implement + self-verify). Pre-batch
> debt landed first: #455 dependency announcement fix (rustls 0.23.46 + serde_yaml_ng, `cargo audit`
> from exit 1 back to exit 0), WBS registration debt (#456 registered as 3.17 + incomplete rows
> supplement "owning phase / whether blocks main line" fields).
>
> **I. Red then green**: driver-level red criteria 3 (`src/driver/tests/run_rootless.rs` — rootless
> merged IR must not carry entry_function (red state `Some("main.main")`), rootless top-level
> executable statements must not report E3023 (red state doesn't produce module), Bin control
> entry_function maintained (green state permanent)) + CLI e2e 3 nails (standalone relative import
> run (red state E5001), Script top-level statement execution and #413 hint two green state nails).
>
> **II. Implementation surface**: CLI `run` uniformly goes through `compile_project`
> (`run_file_with_diagnostics`'s project/ standalone fork deleted); Linking's `entry_function` gated
> by `is_bin_role` (#413 T4 — Script doesn't auto-call main); #413 hint keying changed from
> `module.source_files.is_empty()` to call site project probe (after unification ModuleIR always
> carries source_files, original mark invalid); `script_main_never_called` adapted to qualified name
> form (function `{key}.main`, global slot measured also qualified name — based on IR evidence, not
> comment inference).
>
> **III. Existing gaps exposed by unification (fixed in same batch, A/B/C class — only fix failed
> program to succeed, originally successful program zero impact)**:
>
> 1. **Explicit type arg pre-specialized name** (`mk(Int)(5)`): ir_gen directly emits specialized
>    name per D6.3 (mono will strip un-specialized original), `resolve_call_target`'s three arms
>    (rename / aliases / dot prefix) don't cover `mk(int64)` form ⇒ call site dangling E6006. Fix =
>    that single point parser adds fourth arm (base name qualification, type parameter string
>    preserved as-is). **Bin project same-form reproduction (pre-existing defect, not introduced by
>    this batch)**.
> 2. **Script top-level `mut` assignment statement repeated slot allocation**: `extract_global_defs`
>    and `register_global_slots` count `counter = counter + 1` (same-form Assign as declaration) per
>    statement ⇒ same variable allocated 3 slots, initialization writes to later-allocated slot
>    while read uses earlier-allocated slot (runtime Add type mismatch). Fix = two places merge by
>    name (same criterion same source).
> 3. **Same-module constraint generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually is `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidate + table lookup validation (precise hit →
>    generic owning unit key qualified hit → fall back bare name current state; not suffix
>    heuristic, with `resolve_generic_name` same form).
>
> **IV. Registration gap (this batch not fixed)**: Constraint dispatch in **type and generic
> definition in different modules** still E6006 (rootless dual-file probe verified; Bin same form) —
> root cause = MonoType user type name doesn't carry definition module, correct fix belongs to P6
> type identity convergence; registered WBS **3.18** (owns P3, precondition pending P6, doesn't
> block main line), `qualify_deferred_method` notes same surface.
>
> **V. Test adaptation (honest registration)**: `tests/integration/multifile.rs` fixtures add
> manifest — what it tests is RFC-029 project orchestration (Bin auto-entry) not Script semantics;
> `cli_e2e` one nail (top-level diagnostic variable name visibility) and `script_main_hint` fixtures
> (7 cases) sync per qualified name form — production and consumption sides only eat merged IR,
> fixtures directly drive same form, no need for pipeline product stand-in.
>
> **VI. Final gates (closure revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new nails) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> diff baseline regeneration **12 rows** — line-by-line structured check: exit / stdout / diagnostic
> codes all line-by-line identical, only stack frame function name and variable name qualified per
> unit key (manual review passed); after regeneration `check-corpus-parity.py` exit=0.
>
> **VII. Process events (registered as-is)**: This batch one time during pre-commit hook running
> concurrent editing workspace doc, was eaten by hook's stash rollback and that commit failed
> (concurrent form warned in config comment) — discipline = don't move workspace during hook
> running; #455's commit therefore split into two attempts (second time all green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closure)

> **4.6 batch record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 instructed to
> advance in order 4.6 → 4.9 → 4.4 → P5, first do 4.6; implementation form = **complete single
> team** (following previous batch user instruction: not open DAG, not build team, single-session
> implement + self-verify). Task definition (09 three-level table): project's intra-file semantic
> data (SemanticDB) goes through orchestrator same pipeline, same source as diagnostic; cross-file
> references carry `resolves_to`. Acceptance: intra-project cross-file jump hits definition;
> semantic data and diagnostic come from same compile.
>
> **I. Current state empirical (verify before assert)**: didOpen/didChange triggers **two** compiles
> — diagnostic goes through unified Driver pipeline (4.2.3/4.2.6 unified), semantic data goes
> through `src/lsp/server.rs`'s manual lex→parse→ `TypeChecker::new(uri).check_module_collect_all`
> (no module registry ⇒ intra-project `use` resolution fails, cross-file reference event not
> generated, jump/rename/reference lookup silently fails for imported symbols — D55 registered
> silent failure). Orchestration arm's checker uniformly names `"<module>"` (SemanticDB file
> identity lost), `type_result` (containing semantic_db) is dropped when diagnostic non-empty and
> never externally emitted; `checker.rs` clearing consistently covers `resolves_to`'s DefId file
> identity as current file; `Export`/`ModuleInfo` both have no definition position surface.
>
> **II. Red then green**: LSP e2e nail (`src/lsp/tests/cross_file.rs`) — temp dual-file project
> didOpen main.yx then send GotoDefinition to `add_one` call site, **before fix returns None (red,
> empirically silent failure above)**, after fix hits lib.yx line 0 definition (green). Process one
> probe location: compiler-side link (driver probe dump) already through-through but e2e still empty
> — root cause is `resolve_reference` matches precisely by reference **start** (D55 precise-only
> current caliber), test cursor falls in identifier middle; this tolerance is existing behavior
> surface, this batch doesn't touch (see VII).
>
> **III. Implementation surface**: (1) `Export.definition_span` (extract_module_info self-harvests
> from AST identifier node, same source same value as typecheck definition event's `def_span`;
> native std always None) + `ModuleInfo. definition_file` (Registry arm fills per DiscoveredFile,
> includes vendor; std always None); (2) checker reference clearing via `import_def_locations` (use
> line binding span → definition file + definition span) redirects cross-file imported items'
> `resolves_to` out of this file; std import no registration, keeps falling to use line; (3) checker
> module identity changed from unified `"<module>"` to unit file path (empirically `env.module_name`
> all 4 consumption points zero diagnostic consumption); (4) `DriverOutcome.semantic_db` channel —
> CollectAll form (Lsp target file **regardless of diagnostic success or failure** + SingleFile
> fallback) conflux, Lsp arm tail injects non-target file top-level export definition surface from
> registry (`external_definitions`, unopened files can also jump; type/sub-module exports not
> injected — typecheck side type registration doesn't produce definition event, both sides keep
> consistent absence); (5) `orchestrator::analyze_source_in_ project` (`check_source_in_project`
> transfer takes diagnostic half); (6) LSP `analysis.rs` single compile entry replaces manual
> pipeline (`update_semantic_db` deleted), World key uniformly normalizes file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalize, drive letter case and %XX encoding differences
> normalize to same key, cross-file DefId therefore stable hit); 5 handlers + semanticTokens dual
> requests connect to `db_key`.
>
> **IV. Fixed in passing (D59 shunt B class — old behavior silently does nothing, originally
> successful program zero external impact)**: ① Intra-project file cross-file jump/rename/reference
> lookup from silently failing to hitting real definition; ② When lexical error, old manual pipeline
> deleted semantic information (staining disappeared) → residual AST staining preserved (4.2.6
> editor philosophy extension); ③ Target file with type error, semantic data no longer dropped with
> `type_result` (typing mid-state staining/jump doesn't disappear).
>
> **V. Nails**: driver-level 6 (`src/driver/tests/lsp_semantic_db.rs` — target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface round trip,
> std import fallback use line, error file preservation, single-file fallback path key, single-file
> error preservation) + LSP e2e 1 (red then green) + uri normalize 4 (`src/lsp/tests/uri.rs` — drive
> letter conversion, space encoding, idempotent normalize, non-file scheme passthrough).
>
> **VI. Mutation verification (scratch.py trial, mechanically reversible)**: Batch tree wbs-46 @
> closure revision 39546020, both mutations made hit nails turn red, after restoration blob
> identical to HEAD + git status CLEAN — m1 remove `import_def_locations` registration
> (statements.rs) ⇒ cross-file driver nail + LSP e2e nail red, other 7 green; m2 remove
> `external_definitions` injection (arms.rs) ⇒ same two nails red (round trip resolution lost
> anchor), other 7 green. Batch tree gc'd (freed 4.8 GB, `status` no batch tree).
>
> **VII. Final gates (closure revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new nails) / `--test integration` **287 passed** / corpus
> diff **byte-for-byte zero drift** / IR snapshot zero drift / check-obligations PASS (13·13·0·8,
> `semantic_db` non-obligation reason consumer anchor synced server.rs:482 and 02 §Obligation Field
> Table) / check-docs-truth 0 / docs-examples 258 0 failures / docs-orphan 0 / markdownlint
> repository-wide only user-uncommitted 032 two MD056 (user's own edit, not touched) / pnpm
> docs:build 0. **Process events (registered as-is)**: en/09a-wbs-history.md:54 one MD037 was
> introduced by auto-translate bot re-translating last batch's 09a (zh original no space), this
> batch fix back — markdownlint red not this batch's change surface but same file as 09a, register
> together. **Left-over observation (registered for discussion, not touched)**: `resolve_reference`
> only precisely matches by reference start — when cursor falls in identifier middle,
> jump/hover/rename silently does nothing (D55 precise-only current state, whole-file form same
> structure), if tolerance matching needed is independent behavior change, report to user ruling.
>
> **Same day continuation batch (4.6.2, landed after user ruling)**: After delivery of closing
> report, user ruled on "left observation ①" "OK, then change it" — reference lookup tolerance
> implemented per plan A: definition/hover/references/rename four handlers' lookup key changed from
> cursor position (after UTF-16 conversion) to **identifier's own start** already located by
> `find_identifier_at_position` (same coordinate system as semantic event, no conversion needed);
> `resolve_reference`'s start precise match semantics unchanged (D55 precise-only unchanged).
> references/rename's definition point fallback judgment (start <= col < end) holds isomorphic for
> identifier start. Red then green: definition middle cursor nail first red (None) then green; four
> handlers middle cursor each 1 nail + cross-file e2e start/middle dual position assertion, `--lib`
> 2748 (+4) all green, clippy -D warnings 0. Registered WBS 4.6.2 (three-level count 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction) — Round 1 Closure (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 batch record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 ruling to merge
> three related three-level tasks 4.9.1 / 4.9.2 / 4.9.3 **into one team one round** (reason: they
> belong to "pool's source and identity" side, merged landing can avoid `collect_project_refs`
> deletion's intermediate state repeated rewrite; 4.9.4 / 4.9.5 belong to consumption refinement and
> test pool separation, depend on 4.6.1's `use` path resolution, forcing same batch would expand
> gate surface). #450 ownership **pending ruling** — evidence shows belongs to "overload resolution
> name not passed back to reference count" class, still reproduces under round 1 landed form,
> register as independent WBS item for disposition (team lead recommends A: merge into round 2
> register as independent item; **pending user ruling**). Frozen rev = `3863a9ff0` (parent commit
> `62f6e1a59` is pre-fix baseline); verification PASS attempt `aab92fe0` (i.e., t2); review pass
> attempt `96452696` (i.e., t3). **Closure registration commit = `38d30b846` (docs(docs): 4.9
> reference pool round 1 closure registration)** — this commit actually written and pushed by
> closure member integration-refpool; its closing probe once mis-marked closure task as terminal
> failed and overwrote output, team lead took over new attempt and changed back to completed per
> verified delivery fact, hereby correct ownership and label. Evidence archived:
> `target/yx-scratch/yx49/EVIDENCE.md` (containing 8 mutation patches and warn_compare_v8.json).
>
> **I. Merge ruling**
>
> 1. **The ruling itself**: Three related three-level tasks merged into round 1. **4.9.4
>    (consumption refinement) / 4.9.5 (test pool separation) next round** — former depends on
>    4.6.1's delivered `use` path resolution (already landed at `39546020`, can start); latter
>    merged with RFC-029f table inclusion same batch, registered at RFC-029f end section pending
>    change.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 are mutual preconditions (4.9.2 changes pool type → 4.9.1
>    assembly surface must rewrite → 4.9.3 only has denominator judgment role table), splitting
>    rounds inevitably writes one intermediate state then changes twice; (b) all three belong to
>    "pool's source and identity" side, 4.9.4 / 4.9.5 belong to another side, batching is per
>    "cohesion" not "independently revertible" split.
> 3. **Merge result**: Single commit `3863a9ff0` (one line `:recycle: refactor(driver)`), single
>    push, single batch gate. **Hash explanation**: Before push `git pull --rebase` merged
>    auto-translate bot commit (`72f364169`); rebase landing point after other party's commit, this
>    batch's two (implementation `3863a9ff0`, its pre-fix baseline also serving as other party's doc
>    commit `62f6e1a59`) **not rewritten, maintained original hash** — initial version mis-wrote as
>    "`3863a9ff0`→`3863a9ff0`" identity mapping (looks like "rewrite happens to be same value",
>    actually "not rewritten"), corrected 2026-10-17 at closure; other party's code commit
>    `33f14e8ab`→`b122b7ad1`. **External references uniformly use post-push hash** (rewritten old
>    hash unreachable in new clone, identity determined by patch blob comparison).
>
> **II. Red/green measured caliber for five criteria**
>
> | #   | Criterion                                              | Red state (pre-fix / P1 directory scan regression patch)                                                                                             | Green state (round 1 landed)                                                                                                                                                                                                                                                                                                                                                                |
> | --- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | ①   | Pool root = package target (#334)                      | Directory recursive scan ⇒ tests/ files' identifiers also enter pool, exempt production code unused definition                                       | `collect_package_refs` (`helpers.rs:481`) per `ExplicitSurfaces` root calls `discover_with_used`; when all roots degenerate fallback `collect_compilation_refs` (`helpers.rs:454`)                                                                                                                                                                                                          |
> | ②   | Pool granularity = `(module, name)`                    | Same-name local variable (`let foo = …`) accidentally exempts same-name top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` cross-taste | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) assembly sole entry; `arms.rs:603-609`'s `other_module_refs` unions "all modules except this unit" entries then sends to `DeadCodeAnalyzer`; module's own name (top-level definition and local binding, `use` import name exception) no longer exempts other modules' same-name definition |
> | ③   | `FileRole::Script` doesn't use pool                    | Script affected by other files' references within project                                                                                            | `arms.rs:611-623`'s `match us.role` only for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`) assembly, Script/Test go to `_ => {}` default arm (per file's definition-use graph judgment)                                                                                                                                                                             |
> | ④   | Fixture A: lib.yx line 4 definition must report W1001  | Pre-fix: reports `W1003@1` (actually "unused import"), not W1001                                                                                     | Red state measured (red 1 fixture red1) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                                  |
> | ⑤   | Fixture B: main.yx line 4 definition must report W1001 | Pre-fix: reports `W1003@1`                                                                                                                           | Same as above                                                                                                                                                                                                                                                                                                                                                                               |
>
> **Criterion ②'s "deduced red"** (implementer self-stated + verifier closed): red2 fixture's
> `let foo = …` in helper.yx; old pool under bare name union took `foo` as a whole "used by helper",
> lib.yx's `foo` exempt. **Measured**: red2 fixture vs pre-fix binary → no W1001 (old caliber
> exempt); red2 vs post-fix binary → W1001@1.
>
> **Criterion ③** (Script doesn't use pool) red state fixture **first version failed**: First
> version fixture in same file puts same-name local variable triggers E2010 (this language doesn't
> allow same-module shadowing top-level definition), moved to another module (helper.yx) to deduce
> red state. This point t3 review and t2 verification both independently reproduced, **not contract
> defect** — same-module shadowing is this language's hard constraint, fixture must be cross-module.
>
> **III. Correspondence between +8 and +16 two counts** (**with t3 review as definitive basis**)
>
> `target/yx-scratch/yx49/EVIDENCE.md` section C writes `+16/−0`, but missed "**new fixture
> record**" component — v8 instrument (`warn_compare_v8.json`, 331 files symmetric diff,
> `only_before=0`) caliber **includes** new fixtures; t1 implementer's self-built "+8" is
> **warn-before / warn-after (each 325 items) aggregated per fixture** (each member file checks
> once). Review item-by-item mapping:
>
> - **+8** (`8` new warnings, all `check <non-entry file>`, `check <entry file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (unified attribution)**: These 8 fixtures all **didn't declare package target** ⇒
>     4.9.1 root degenerates to checked file's own compile set, consumer main.yx not in it; this is
>     design-intended change (before, these files' check surface was already zero warnings, not
>     exemption path). Repository root `yaoxiang.toml` is empty file ⇒ same degenerate path
>     empirically holds.
> - **+16 residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files in t1's `warn-before.jsonl` have `exit=None` (capture time point
>     **doesn't exist** — newly added by this batch fixtures). So +8's "+ 0 − 0" diff doesn't have
>     them, +16's v8 symmetric diff does.
>   - `v8`'s `stray.yx` only has `W1001@2(probe)` left: reason is pre-fix binary also reports
>     `probe@stray.yx:4` (this fixture checks `probe` once per file), **cancel out** leaving only
>     lib's 1 — three data (+8 / +16 / dual binary ABBA) mutually consistent.
> - **New fixtures 8** vs **+16 residual 8**: Same batch of files (4.9.1 / 4.9.2 newly added
>   `ref-pool-compile-set/` and `ref-pool-module-identity/`), instrument treats their each check as
>   independent record — same file may trigger different diagnostic items for multiple
>   `role / context`.
>
> **Conclusion**: **+16 = +8 (aggregated per fixture) ∪ new fixtures 8** — both counts are correct,
> **caliber different**: t1 reports "added 8 items" is fixture-layer caliber, t2 reports "+16 items"
> is file-layer caliber (symmetric diff includes new fixtures). t3 review ① +8⊆+16 verified;
> **review ② writes "don't copy EVIDENCE.md section C"** — section C missed "new fixture record"
> component, this item rewritten per t3 review `pass` conclusion.
>
> **IV. Mutation experiments (mechanically reversible, all `scratch.py trial`)**
>
> 8 mutation patches (`target/yx-scratch/yx49/*.patch`, p1 directory scan, p2b use import name
> removal, p3b script joins pool, p3 flat consumption, p4 std joins pool, p7final self-built 4
> criteria, p8 std close, p9 vendor joins pool + p_stack2 / p_stack3 overlay surface). Each trial
> output contains: HEAD blob → mutated blob → restored blob identical to HEAD = yes + `git status`
> CLEAN. This machine `core.autocrlf=true` needs to inject `false` via `GIT_CONFIG_*`
> (`trial_env.ps1`), otherwise patch added lines convert to CRLF causing apply/restore double
> failure — **only affects patch mechanicality, doesn't change tested behavior**.
>
> **V. Process events / environment lessons (must read for next round handoff)**
>
> 1. **core.autocrlf injection**: This machine `core.autocrlf=true` must inject `false` via
>    `GIT_CONFIG_*` to ensure patch mechanical reversibility; see
>    `target/yx-scratch/yx49/trial_env.ps1`. **Next round don't reuse** any trial script without
>    injection.
> 2. **Same tree trial serial**: On same tree `trial` and `cargo build` **must be strictly serial**.
>    `trial` internal `git apply` and `cargo build` both touch worktree, parallel will see "patch
>    incomplete seen by build" or "build and apply deadlock" two kinds of races.
> 3. **Trial rebuild binary needs sync confirmation**: After `trial` completes will rebuild tree's
>    binary (cargo's fingerprint contains source path, **changing tree = full rebuild**), before
>    subsequent measurement must confirm `bin/yaoxiang-rs` synced with HEAD. **This round once used
>    residual mutation binary to get fake data**, already rebuilt and remeasured to correct.
> 4. **Reason for second tree yx49-b**: Comparison needs pre-fix binary (62f6e1a59), no ready-made
>    in batch; `scratch.py init yx49-b` opened second tree (ate another full build). **Tree
>    released** (see VI), deletion left to batch closure `gc` disposition.
> 5. **Reason for cli_e2e two Internal cases fixture supplemented to legal package (supplement
>    `[run].main`, assertion not moved)**: Pre-fix `cli_e2e` two `Internal` role nail fixtures
>    **didn't declare `[run].main`** ⇒ after 4.9.1 lands `compile_project` goes through
>    `ExplicitSurfaces` root call path, can't find `main` ⇒ entire batch 2 fixtures behavior from
>    "per Script fallback" drifted to "can't find entry" — this is 4.9.1-introduced fallback
>    behavior not defect. After supplementing `[run].main` field two fixtures go through normal
>    package path, **assertion** (exit code / diagnostic code / warning set) word-for-word not
>    moved.
> 6. **Safe way to delete second-round tree**: After release use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`), will verify "no mount" then delete.
>    Already released before this batch closure, next `gc` naturally cleans.
>
> **VI. Batch tree state**
>
> Before closure: yx49 released (4.8 GB freed), yx49-b released. Two trees no mount, status can be
> verified in `scratch.py status`. This task output ends with `scratch.py gc` complete status.
>
> **VII. RFC-029f "Known Defects" table flip**
>
> Previous version (2026-10-09) described in present tense "three reference pool design defects"
> (including `collect_project_refs` / `orchestrator.rs:603-648` / `orchestrator.rs:393-396` etc.
> deleted code positions). This batch closure synchronously rewritten as "**known defect: landed
> (2026-10-16, WBS 4.9 round 1)**": ① Current pool's root = `collect_package_refs`
> (`helpers.rs:481`), per `ExplicitSurfaces` resolution, no directory scan; ② Current pool type
> `HashMap<String, HashSet<String>>` (key = module key), assembly sole entry `collect_ref_pool`
> (`helpers.rs:429-449`), consumer `arms.rs:603-609`; ③ Current Script doesn't join pool,
> `arms.rs:611-623`'s `match us.role` default arm. **All file:line re-taken on referenced revision
> `3863a9ff0`**. `collect_project_refs` and `orchestrator.rs` moved with P4 4.2.9 split (`c59fb751`)
> into `src/driver/{state,arms,helpers}.rs`, old positions in table no longer hit — this is
> "declaration inconsistent with fact" fix (t3 review note ①).
>
> **VIII. Final gates (closure revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 rows** (two new fixtures
> behavior run, 338a339,340; other 342 rows byte-for-byte unchanged) / IR snapshot 204 zero drift /
> check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process events**: This batch
> one cargo build due to `core.autocrlf` converting script line endings to CRLF, team scratch path
> `yaoxiang-rs` launch reported `BadImage`, empirically encoding inconsistent with git index —
> rebuilt then passed (parallel to 4.6 batch "external whole-tree discard" lesson, different cause).
>
> **IX. Leftover (handoff to next round)**
>
> 1. **#450 ownership ruling**: Evidence shows belongs to "overload resolution name not passed back
>    to reference count" class, **still reproduces under round 1 landed form** — per user ruling C
>    flow dispatch. **#450 probe**: Dual binary same set `{W1002@2, W1005@4}` (measured, rootless
>    form), no direct relationship with round 1 change, but #450's fix may need to wait for 4.9.4
>    (consumption refinement) together — because root cause is "overload resolution name passing
>    reference count" needs use item's precise consumption surface, 4.9.4's "really used after
>    import" is same-class criterion's precise version.
> 2. **dead_code.rs:683 pointer nit**: Comment writes "two pools' assembly caliber see driver side
>    collect_compilation_refs" — actually `collect_compilation_refs` is fallback path, main path is
>    `collect_package_refs` (`helpers.rs:481`). Comment inconsistent with code main path caliber
>    (**not in this batch's writing scope**), left for 4.9.4 / 4.9.5 landing to fix together.
> 3. **4.9.4 / 4.9.5 preconditions unlocked**: 4.6.1 (`use` path resolution) already delivered by
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs to first set
>    RFC-029f table inclusion (laid by this batch "VII." flip).
> 4. **`yaoxiang check` multi-file `run` never reports dead code family warnings** (WBS 3.4.6,
>    2026-10-15 user ruling C deferred to after 4.9) — this batch round 1 not involved; next round
>    4.9.4 / 4.9.5 landing handled same batch.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction) — Round 2 Closure (4.9.4 / 4.9.5 / 4.9.6)

> **4.9 round 2 batch record (2026-10-16 implemented → 2026-10-17 closed)**: Covers 4.9.4
> (consumption criterion = really used after import), 4.9.5 (test pool separation), 4.9.6 (method
> reachability caliber, issue #450, user 2026-10-16 ruling A merged into round 2). Closure fix
> (verifier-side remediation M1/M2/M4 compilable redo + F1 fix round + PA-2 delete dead branch) and
> registration gap issue #464 (WBS 4.9.7) settled together.
>
> **I. Adjudication chain and commits (all post-push truth)**
>
> | Step | Content                                                                                                                                        | Post-push commit | Pre-push local value | Judgment method                                              |
> | ---- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- | ------------------------------------------------------------ |
> | 1    | fix(util): check explicit path not blocked by default exclude table + --exclude always empty (issue #463, P0-0.4)                              | 366bc5a69        | ef959ca55            | Patch blob identical                                         |
> | 2    | Phase A = 4.9.4 + 4.9.5 (RefPool attributed per import + Test doesn't contribute consumption surface)                                          | e6cf1aeaa        | 84a6f6978            | Patch blob identical                                         |
> | 3    | Phase B = 4.9.6 (#450 per-candidate method reachability; includes generation warning-codes.md)                                                 | ddcbb12d0        | cd629b5fe            | Patch blob identical                                         |
> | 4    | docs(meta): AGENTS rule 10⑦ (commit period stash window convention)                                                                            | ec94fa9d8        | 9a4ed5f3b            | Patch blob identical                                         |
> | 5    | Fix round = F1 (overload sibling overload recover report W1005) + F2 (verifier side M1/M2/M4 compilable redo) + F4 (snapshot generation noted) | b8380e6ab        | 3d9ef6eab            | Patch blob identical (36962f318e42…, git range-diff no diff) |
> | 6    | Closure fix = PA-2 (delete strip_unit_prefix dead branch + 029f/CHANGELOG three inaccurate description corrections, register #464)             | b940300ac        | e13984465            | Patch blob identical (patch-id 9bda3cfe4c…)                  |
>
> **Hash explanation**: Each push pre `git pull --rebase` will rewrite local hash; above "Judgment
> method" column is all "patch blob identical" (`git diff <rev>~1 <rev> | git hash-object --stdin`
> both sides consistent + `git range-diff` no diff). This article doesn't cite any pre-push hash as
> authoritative source.
>
> **II. Red/green measured caliber for three criteria** (verifier self-built reproduction, not
> taking implementer's criteria)
>
> | Criterion                  | Red state (before binary 366bc5a69 measured)                                                                                                                                        | Green state (ec94fa9d8 and after)                                                                                           |
> | -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
> | 4.9.4 import≠consumption   | Declaration side project check a.yx only 1 W1001 (main's use b.{foo} + foo() revives a.yx same-name foo; guard only imported not used also revived)                                 | Same fixture 2 reported as-is (same-name collision not revived + unused import not counting consumption); ref_pool 8 passed |
> | 4.9.5 test pool separation | src/util.yx's debug_print revived by tests/ reference (only 1)                                                                                                                      | Same fixture 2 (test reference doesn't spill); ref_pool's test_ref_pool_excludes_test_role_references passed                |
> | 4.9.6 method reachability  | Self-built fixture (2 disambiguation overload calls + 1 uncalled) 3 W1005 false reports; method_call_args.yx 3 / method_overload.yx 1 (overload covers by full name only reports 1) | Same fixture reports uncalled orphan as-is; method_unused 3 nails passed; corpus call_args 3→0 / overload 1→0               |
>
> **Fix round (F1) tighten again**: Unhit resolution's sibling overload recover report W1005
> (b8380e6ab — same fixture calls ->Float reports ->String sibling, calls ->String reports ->Float
> sibling, bidirectional line nails). **Fallback boundary (register #464)**: Cross-unit (registry
> resolution) method calls don't produce resolution entries, fall back name-level exemption,
> cross-unit same-name same-order not previously reported — register 4.9.7.
>
> **III. Process events and lessons (four hard requirements correspond)**
>
> 1. **PA-1 changed from blocker to existing capability gap (medium)**: Round 2 review once
>    registered "cross-unit same-name same-order not reported" as blocker (PA-1); later two fact
>    reconciliations (implementer archive + verifier dual binary re-measurement on
>    b8380e6ab/b940300ac same fixture set, diagnostic set byte-for-byte identical) confirm it's
>    **existed before round 2 capability gap** not this round's regression — mechanism = cross-unit
>    method calls don't produce resolution entries, fall back name-level exemption; disposition =
>    re-judge medium, register issue #464, WBS 4.9.7 schedule next round.
> 2. **Reviewer publicly corrected before data and withdrew regression judgment**: Reviewer
>    initially cited before data ("−42 items contain called methods") didn't match their raw output
>    (W1005 count=0) — verifier verified before capture was 0 items, reviewer's regression judgment
>    **publicly withdrawn**; −42 correct caliber = 26 files' called methods false report elimination
>    (phase B), each item traceable to behavior-run exit=0 file (method really called executed).
> 3. **t13 terminated due to contract premise not holding (not implementation defect)**: This task's
>    acceptance premise was "cross-unit same-name same-order will report W1005 regression fix", but
>    measured (t14 dual binary) confirmed this behavior is already registered gap (#464) not
>    regression — contract premise doesn't exist, task terminated; its probe value taken over by t16
>    read-only judgment (confirm `strip_unit_prefix` is dead branch).
> 4. **Document commit's real criterion is "really build once"**: Round 2 closure's document commit
>    (`042c07860`) made Deploy Docs red at VitePress's Vue template compile — record's angle bracket
>    placeholders like `<rev>` / `<before-exe>` / `<after-exe>` didn't go into backticks, parsed as
>    elements (reported `Element is missing end tag`, pointing at `<rev>` line); markdownlint can't
>    catch this class, and pre-commit's markdownlint hook in doc-only commit judged
>    `no files to check` skip (measured). Disposition: placeholders all go into backticks, before
>    push locally run once `npm run docs:build` (fix commit `0c837e1b4` with local build exit=0 as
>    evidence); if use `--no-verify` commit, commit-msg check needs manual supplement.
> 5. **Process omission of contract description not synced**: t14's task description text residual
>    PA-1 old hypothesis (wrote "b.yx reports W1005" expectation), verifier in report actively
>    pointed out "judge per contract field, description text doesn't count" and asked team lead to
>    confirm — disposition = use task contract field as judgment basis, description text left as
>    process lesson (**redispatch/assignment task description must sync with latest adjudication
>    chain**).
>
> **IV. Mutation experiments (mechanically reversible, all scratch.py trial)**
>
> Round 2 verifier total 9 patches (target/yx-scratch/yx49/ under m1b/m2b/m4b are F2-post compilable
> redo version — first version M1/M2/M4 commented out if condition line leaving hanging block body,
> t8 review F2 judged "compile failure mis-recorded as criterion turn red" voided; redo version per
> patch cargo check exit=0 + trial turn red). M3 (same-name attribution regression) first version is
> real mutation, measured ref_pool 5 criteria turn red, keep original judgment. M1b anchor re-taken
> on b8380e6ab (F1 fix post probe segment refactor, old anchor invalid). All trial: HEAD blob →
> mutated blob → restored blob identical to HEAD + git status CLEAN. Environment injection continues
> trial_env.ps1 (core.autocrlf=false via GIT_CONFIG_*).
>
> **V. Warning surface full comparison (caliber first)**
>
> Instrument = warn_surface_r2.py (self-built parser, (file, code, line, column) four-tuple
> symmetric diff, project state channel cwd=file parent directory, full path key); command =
> `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>` (337 files × 2
> binaries). **Caliber**: This round before capture already includes all fixtures ⇒ matched set =
> full set 337, no round 1's +8 (matched increment) / +16 (full set) dual caliber problem. Result:
> only_before = 42 W1005 (26 files, all called methods' false report elimination = phase B's −42;
> each file verify all in behavior-run exit=0 baseline, method really called executed — **no real
> dead code revived**) / only_after = 3 W1001 (ref-pool-import-use a.yx:4 check surface ×2 +
> ref-pool-test-pool's ../src/util.yx:2 = phase A criterion surface). **Snapshot generation (F4)**:
> Phase A time point's after capture (phase B pre's 57 residual) and final (phase B post's 15+2=17)
> are **two generations**, must note when citing; another 1 item exception
> (interface_member_conflict.yx's Dog.name@18) is field access form, not in phase B's W1005 removal
> set, before-after consistent.
>
> **VI. Final gates (closure revision b940300ac)**
>
> lib 2761 passed / integration 288 passed / IR snapshot zero drift (snapshot 7 passed) / corpus
> baseline zero drift (byte count three-way comparable: baseline = b8380e6ab = b940300ac) / clippy
> -D warnings 0 / docs-truth exit=0 / check_tracking exit=0 / generator already run no product
> change. **Gate check arrangement (rule 13)**: This batch push's CI / Deploy Docs / Auto Translate
> run conclusions handed to scheduled check session for registration (run id + conclusion +
> consecutive push coverage: successor run covers previous), not wait in session.
>
> **VII. Batch tree state**
>
> yx49 and yx49-b both released (closure time gc unified delete); no newly added directories outside
> repository this batch.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction) — Round 3 Closure (4.9.7)

> **4.9 round 3 batch record (2026-10-17 implemented → 2026-10-17 closed)**: Covers 4.9.7
> (cross-unit method call resolution registration and receiver attribution, issue `#464` — WBS 4.9's
> last three-level task; this round closure 4.9 entire secondary task kanban checked). Chain =
> production side t19 → independent verification t20 PASS → review t21 needs_revision (F1: injection
> surface false report) → fix round t24 → independent verification t25 PASS → review t26 PASS
> (incremental, verdict=pass). All commit numbers are post-push truth (judgment method = patch blob
> identical; `git diff <rev>~1 <rev> | git hash-object --stdin` both sides consistent).
>
> **I. Adjudication chain and commits (post-push truth)**
>
> | Step | Content                                                                                                                                                                                                  | Post-push commit | Judgment method        |
> | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ---------------------- |
> | 1    | Production side: cross-unit method call produces D20 resolution entry (`feat(typecheck)`, t19)                                                                                                           | `b8b3cc3f6`      | Patch blob identical   |
> | 2    | Driver-side attribution: method reachability marked per definition unit, name group with resolution record deactivates pool's name-level exemption (t20 verify PASS / t21 review catch F1)               | `d5c00235a`      | Patch blob identical   |
> | 3    | Injection surface fix round F1: candidate injection per "name ← providing module" **pair-by-pair** filter, eliminate E1107 false report (t24, including F2 description and F3 landing form single point) | `d5dd1a37b`      | Patch blob identical   |
> | 4    | Test nail homing: F3 nail in `src/frontend/core/typecheck/tests/environment.rs` migrated to declaration side `src/driver/tests/overload_injection.rs`                                                    | `c2f887843`      | Patch blob identical   |
> | 5    | dev tip                                                                                                                                                                                                  | `e9675c9dd`      | merge commit, see "II" |
>
> **II. Hash mapping and two merge commits on dev tip (source unconfirmed, don't write as member
> action)**: Round 3's four commits (above table 1–4) all pushed to origin/dev;
> `git worktree list`'s main mount at `e9675c9dd`. dev tip has two **merge commits not generated by
> this team's command surface** (`5da05b885` / `e9675c9dd`, reflog shows `pull --tags` + push form),
> source unconfirmed — per rule 10④ (reference must be resolvable by anyone) and rule 13 (gate
> handed to special check) same closure caliber, here only register fact and reflog evidence, no
> member attribution; also see "V." lesson ② (push ownership convention must be verifiable).
>
> **III. Actual form of D20 contract extension (key form and consumption points)**: Production side
> packs "candidate / mixed IR name / declaration period parameter name" three parallel tables **in
> same order** into `ModuleInfo.method_overloads` (`src/frontend/module/mod.rs:157`) carried with
> unit crossing, registry per module key ascending deterministic merge
> (`src/frontend/module/registry.rs:150`); typecheck arm per this unit's import face pair-by-pair
> injection (`src/driver/arms.rs:354` → `visible_method_overloads`, `src/driver/arms.rs:1521-1579`:
> name ← providing module pair-by-pair, same-name conflict full name skip, own key position not
> externally merged), landing form single point `append_overload_group`
> (`src/frontend/core/typecheck/environment.rs:399`: three tables same order append + equal length
> debug assert); consumption side attributes D20 resolution product per **definition unit**
> (`MethodMarks`, `src/driver/helpers.rs:569-573`; `typed_owners` `:591`; `resolved_method_marks`
> `:621`), and makes name groups with resolution record deactivate pool's name-level exemption
> (`src/frontend/core/typecheck/passes/dead_code.rs:252-267`, `:825-841`). Key form = entry name
> `{Type}.{method}#k` (`{Type}` is definition-side bare name; `#k` declaration order).
>
> **IV. Key scope and false report defense measured (fix round F1 and five residual boundaries)**:
> F1's red state is pre-fix `d5c00235a`'s injection surface "name set × module set" combination
> merges another module's same-name type candidate into same key position ⇒ `use a.{Temp}` +
> `use b.{Other}` (a also defines Other, b also defines Temp, method same-name same-order) fixture
> `check` reports 2×E1107, exit 1 (pre-fix successful program compile failure); post-fix
> `check`/`run` both exit 0, 4 W1005 report two sides' **uncalled** same-name definition as-is
> (reverse under-report surface also verified). Archived fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`; driver-level nails
> `src/driver/tests/overload_injection.rs` 3 + `src/driver/tests/method_unused.rs` round 2's 5 #464
> criteria and 4 #450 old criteria all green; main acceptance fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` diagnostic set fix before and after
> line-by-line identical (not rolled back). Five residual boundaries and item-by-item measurement
> (single `check` definition file doesn't report / name layer ambiguity E5006 full name skip /
> boundary (c) per t20 measured rewrite / inline alias and `ns.T` doesn't inject candidate surface /
> whole import `use M` and std candidate surface doesn't produce entry) full text see
> [029f](../../rfc/accepted/029f-target-semantics.md) §Cross-unit Method Reachability Attribution
> "Residual Boundaries", not repeated here.
>
> **V. Process events and lessons (four continuations)**:
>
> 1. **Contract wording written narrow will directly become defect**: One of F1's root causes is
>    contract only wrote "only inject module surface that can be named by bare type name", didn't
>    write "attribute pair-by-pair per name" — implementation per literal took "name set × module
>    set" combination, review caught E1107 false report. When describing behavior boundary in
>    contract must write **combinatorics** to death (per name pair-by-pair / full combination),
>    can't leave to implementation self-understanding.
> 2. **Convention must be verifiable: "push by team lead" only written in dispatch message ⇒
>    third-party push/merge can't attribute when appears**. This round dev tip has two
>    source-unconfirmed merges (see "II"), finally rely on reflog reverse-infer and withdraw one
>    error attribution. Verifiable form = member doesn't run `git push`/`fetch`/`pull`, sync
>    uniformly `pull --rebase`, use reflog verify.
> 3. **Infrastructure unstable, first land patch then swap**: This batch total 5 unrecoverable
>    faults (3×TRANSPORT, 1×INVALID_REQUEST (thinking protocol), 1×background sub-agent no-message
>    exit); disposition convention = produced evidence and patches first land to in-repo path
>    (`target/yx-scratch/<batch>/`) then swap to continue, same session consecutive two failures
>    don't bet third.
> 4. **Document commit's real criterion is "locally really build once"**: Round 2 closure's document
>    commit (`042c07860`) made Deploy Docs red at VitePress's Vue template compile (angle bracket
>    placeholders `<rev>` etc. didn't go into backticks parsed as elements, reported
>    `Element is missing end tag`); markdownlint can't catch this class, and pre-commit's
>    markdownlint hook in doc-only commit judged `no files to check` skip (measured twice). Fix
>    commit `0c837e1b4` with local build exit=0 as evidence. This round's red→green run from this
>    lesson: Deploy Docs **39011946793** (red) → **39014696984** (green, see "VI").
>
> **VI. Round 2 gate conclusion supplementary registration (rule 13, scheduled check session
> backfill)**: Round 2 closure commit `b940300ac`'s push — CI **39011946778** success / Auto
> Translate **39011946796** success / Deploy Docs **39011946793** **failure** (VitePress build red,
> i.e., "V.4" angle bracket placeholders; already fixed by `0c837e1b4`); fix commit `0c837e1b4` push
> post three green: CI **39014697000** / Auto Translate **39014697044** / Deploy Docs
> **39014696984**. Consecutive push coverage: CI@`0c837e1b4`'s **39014356642** canceled by successor
> run per concurrency (cancel ≠ failure, successor run covers previous, consistent with round 2 §VI
> caliber).
>
> **VII. Final gates (closure revision `e9675c9dd`, local number; CI conclusion handed to scheduled
> check, see "VIII")**: lib 2770 passed / integration 289 passed / yx_multifile_runner 1 passed / IR
> snapshot 7 passed zero drift / clippy `-D warnings` 0 / docs-truth 0 / generator idempotent (rerun
> `scripts/docs/gen-error-code-docs.py` then `git status` clean); corpus baseline 346 → 347 → 348
> rows (each +1 behavior record, other rows byte-for-byte unchanged; baseline
> `tests/baselines/corpus-parity.jsonl` at `e9675c9dd` total 348 rows).
>
> **VIII. Gate check arrangement and conclusion (rule 13)**: This batch push post's CI / Auto
> Translate / Deploy Docs conclusions handed to scheduled check session for registration (not wait
> in session, also don't write "not waited" as "already green"). **Backfill (2026-10-17 scheduled
> check)**: `d5c00235a`'s CI `39023417670` = **failure** — Docs Truth reports "error code
> documentation behind code table/locales", i.e., this commit missed generation product same commit;
> fixed by subsequent `d5dd1a37b` (regenerate generation product): `5da05b885` CI `39027788246` /
> Deploy Docs `39027788268` / Auto Translate `39027788340` all success, covers previous; `e9675c9dd`
> CI `39028827871` / Deploy Docs `39028827896` success; `d0cfc4e62` CI `39033963240` / Deploy Docs
> `39033963278` / Auto Translate `39033963261` success. Three commits' runs each ran to terminal
> state, this round no concurrency cancellation. Lesson: **"local generator idempotent" doesn't
> equal "generation product already same commit archived"** — should verify CI conclusions
> item-by-item before closure, not just look at latest one.
>
> **IX. Batch tree state (closure gc, status output)**: Pre-closure `scratch.py status` — t25
> (holder=-) and yx49b (holder=-, previously stale held by deprecated member verify-r3, team lead
> `release --force` released) two trees no holder; `gc` deletes two, frees about 12.7 GB; post-gc
> `status` = "no batch tree", `git worktree list` only main mount. Another:
> `target/yx-scratch/yx49r3/` and `yx49r4/` are not batch trees (no meta.json), are members'
> evidence/probe directories, not "residual batch tree". No newly added directories outside
> repository this batch.

## P4 4.4 Proof Layer and wasm Wrap-up — Closure (4.4.1 / 4.4.2 / 4.4.3)

> **I. Batch composition and rounds (batch name `yx44`)**: Three three-level tasks 4.4.1 / 4.4.2 /
> 4.4.3 with wasm attribute reachability judgment same batch; batch verification tree
> `target/yx-scratch/yx44/tree` (first full build). Roles and rounds: implement 4.4.2 = t1 (12
> files; local `1fc135437` → post-push `24028ffe8`) | implement 4.4.1 layer order rewrite = t2 (only
> doc/comment, zero behavior change, based on `b5e68b3c9`) | implement 4.4.3 = t3 (11 files,
> including new code W1082; local `2d5e610cd` → post-push `52d2276ca`) | wasm reachability judgment
> = t4 (only judgment, zero cleanup, judgment revision `b5e68b3c9`) | 4.4.1 closure and one doc
> commit = t8 (7 files 173+/43-; local `29ebd2145` → post-push `d07993157`) | independent
> verification = t5 (frozen `29ebd2145`) | round 1 review = t6 (revision `1b593e993`) | patch = t9
> (3 files 9+/10-; local `1ebddaf95` → post-push `1b593e993`) | fix round = t10 (3 files 23+/23-;
> local `657c3d5e3` → post-push `e184d88b6`) | round 2 review = t11 (`657c3d5e3` = post-push
> `e184d88b6`, patch-id fully equal) | closure = t7 (check / record / batch tree gc / doc real build
> / one doc commit; local `c8412565c` → post-push `da724940b`). **Failure and redo**: t6 terminated
> failed with `needs_revision` (review round per contract must fail and give findings, not
> accident); t7 first round interrupted at batch tree gc stage, completed product left in main repo
> working tree not committed, second round took over "fill gap + review" — didn't redo completed
> parts (this round added: 4.4 check and evidence note, this record, three path changes pointing to
> deleted batch directory, doc real build and closure commit). No canceled rounds.
>
> **II. Per-task output and measurement numbers (raw commands and output in evidence summary and
> archive; here only record conclusion)**:
>
> - **4.4.1**: `layers/README.md` (including `layers/mod.rs` top comment)'s layer order table
>   rewritten to **actual call order** (predicate → termination → ownership, call point anchored at
>   `checker.rs::check_module_impl`), `equivalence` described per real identity (no production call
>   point); same source 7 places closure (`01-routing` / `02-stage-contract` / RFC-039 / `06` /
>   typecheck README / layers two files); mechanical reconciliation script on batch tree green
>   baseline **11/11 PASS** (criteria 11 = README side 4: actual call order declaration / two entry
>   anchors / old order declaration disappeared; code side 7: 5 line anchors + `check_module_impl`
>   no short-circuit + `equivalence` zero production call; script archive
>   `target/yx-scratch/yx44-evidence/t5-441-recon.sh`), mutation (c) revert old declaration →
>   reconciliation 2 FAIL i.e. red.
> - **4.4.2**: Solver acquisition surface unified to process-level shared singleton + sole closure
>   entry `with_shared_solver(consumer, f)` in `proof/smt/backend.rs`; three consumption points
>   (predicate / termination / ownership) all changed to shared entry, **in compile production chain
>   `default_solver()` call points = 0** (sole backend selection point lowered to private
>   `new_backend()`). `wasm/src/lib.rs`'s `test_smt` self-check call point is **pre-existing**
>   (earlier than this batch, from `b5e68b3c9`) non-test call point, is "test and independent
>   scenario" surface, doesn't constitute D58 prohibition violation. Independent verification
>   self-built reading (one compile): predicate acq=2 hit=1 miss=1 | termination acq=1 hit=2 miss=2
>   | ownership acq=2 hit=1 miss=1, three points hit/miss sum (4,4) = shared instance cumulative
>   `Some((4,4))`; mutation (a) singleton degenerate → predicate hit=0 i.e. red. **Modification word
>   disclosure**: This commit contains `z3_backend.rs` 15 lines — `cache_stats` moved from inherent
>   method into `Solver` trait block (return `Some(instance internal count)`) + 1 line comment
>   change pointer, no semantic coupling. D58 prohibition mechanized: same thread reentry → panic
>   contains D58, probe `recv_timeout(120s)` no hang.
> - **4.4.3**: `body_checker` missing silent downgrade converged to `TypeChecker::ownership_inputs`
>   (sole criterion point) + new code **W1082** (not overlapping with existing W codes in fact:
>   W1080 constraint downgrade / W1081 solver missing / W1090 opposite direction); nail test 2/2
>   green; healthy path corpus 348:348 zero diff; **downgrade branch production surface unreachable
>   exhaustion holds** (CLI four entries + eval/repl + format + LSP + wasm playground + all
>   `ProgramKind` conflux into `check_module_impl`: `init_body_checker` unconditionally before
>   ownership block, 0 return/break/continue in function, `body_checker` no `.take()`) ⇒ W1082 no
>   user-visible surface. Mutation (b) delete emit → nail test red. **W1082 is external contract
>   table change (Class E), user reconfirmed**; `locales/zh.json` / `locales/en.json` same commit as
>   code, other languages supplemented by i18n bot post-push (run see "IX").
> - **wasm attribute reachability judgment (t4)**: `orchestrator.rs` current tree **10 = 4 positive
>   arms + 6 negative arms** (caliber = `#[cfg(...)]` attribute text in same line, both positive and
>   negative counted; `cfg!` runtime macro 0 hit); 4 wasm positive arms in playground runtime
>   unreachable but **compile-time load-bearing** (call point not gated, single arm delete ⇒ wasm
>   compile failure), 6 `not(wasm32)` negative arms native three phase table load-bearing ⇒ D35
>   envisaged "delete unreachable" in this file **no applicable target**, real cleanup form = whole
>   project-mode subtree gated on wasm32 (register P10 S7, three unverified original kept).
>
> **III. Independent verification (t5, frozen `29ebd2145` = post-push `d07993157`'s content — two
> revisions per commit patch-id fully equal, whole tree diff only in `docs/src/en/**`,
> `docs/src/.i18n-cache.json` etc. bot product surface — `git range-diff` three all `=`)**: gates
> **8/8 all green** — `cargo fmt --check` 0 |
> `clippy --all --all-features --all-targets -- -D warnings` 0 | `cargo test` **3128 passed / 0
> failed** (7 test targets total; baseline 3126 + this batch 2) | corpus diff **348:348**
> line-by-line zero diff (sort diff 0 lines) | IR snapshot `UPDATE_SNAPSHOTS=1` rerun then
> `git status` zero drift | `check-obligations.py` 21 = 13 + 8, containment 0 gap |
> `check-docs-truth.py` source 156 / doc 156, CLI 21 commands consistent | error code generator
> rerun before-after tree both clean. Build warning only 4 bot language W1082 missing items (known
> state) + Z3 download noise, zero code warnings. **Contract verify caliber correction (must two
> JSONL paths)**: Corpus gate's call caliber =
> `python scripts/ci/check-corpus-parity.py tests/baselines/corpus-parity.jsonl <this generation>.jsonl`
> — **two JSONL paths must be given**; missing second parameter is comparing baseline to baseline
> (always zero diff, criterion idle). This batch executed per two-path caliber and archived (t5's
> `corpus-r44.jsonl` product).
>
> **IV. Mutation evidence (3/3, mechanically reversible)**: All via
> `python scripts/dev/scratch.py trial` (`git apply` → run command → `git apply -R` → per-file blob
> compared with HEAD → require `git status` clean):
>
> - (a) Singleton degenerate → shared reading predicate hit=0 miss=2, test FAILED exit 101;
>   `backend.rs` HEAD=`3e2b2d150b24` → mutation `6d00bad4d4aa` → restore = HEAD.
> - (b) Delete W1082 emit → nail test FAILED exit 101; `checker.rs` HEAD=`70c04a41e020` → mutation
>   `db0a623c9b9a` → restore = HEAD.
> - (c) Revert old layer order declaration → reconciliation 2 FAIL exit 1; `layers/README.md`
>   HEAD=`8b1ae875db32` → mutation `e88f9af6c42d` → restore = HEAD. Three trial restore post
>   `git status` all CLEAN. Trial (c) first run due to absolute path WSL resolution failure exit 127
>   (not criterion red), changed to relative path rerun as standard.
>
> **V. Review rounds and conclusion**: Round 1 (t6, revision `1b593e993`) **verdict =
> needs_revision** — 3 findings all doc line anchor inaccuracy (F-01 medium: `02` §"wasm current
> state" 4 group expired line numbers; F-02 low: `02` and `09`'s 4.4.3 delivery anchor
> `checker.rs:1283-1293`; F-03 low: `06`'s comparison statement), four substantial deliveries all
> pass. → Fix round (t10, local `657c3d5e3` = post-push `e184d88b6`, 3 files 23+/23-, with two
> mechanical criteria) → Round 2 (t11, same revision) **verdict = pass, new finding = 0**. **Final
> check basis = round 2's pass**. **Round 3 (closure tip `da724940b` on)**: Independent verification
> t12 **verdict = pass** (bound `da724940b`: four rule texts byte-for-byte, 3.19 / S7 / S8 fields
> and scope complete, count table and adjudication chain re-count, dead reference zero,
> `check-docs-truth` 0, `markdownlint` 0); review t13 **verdict = needs_revision**, 4 findings —
> F-01 medium (t10 / round 2 only gave rebase pre hash, §14 list inconsistent with origin/dev
> actual), F-02 low (`02:379`'s 09 4.2.8 anchor moved by this batch itself in `09:144`/`:182` each
> insert one line pushed to `:274`), F-03 low (`09a` writes this machine drive letter path), F-04
> low (three-level count row note and 157 ≠ table header 156) → fix round t14 (this one) close one
> by one. **t10 contract's caliber ruling**: Fix surface per **task record's 5 F-01…F-05**
> (F-04/F-05 are mechanical criteria), dispatch brief not criterion.
>
> **VI. User ruling (2026-10-18) and rule consolidation**:
>
> 1. **D58 entry name in-place rewrite**: `with_shared_solver(f)` →
>    `with_shared_solver(consumer: SolverConsumer, f)`, in-place correct text, **don't write
>    revision note** (t8 completed in 4.4.1 one commit).
> 2. **wasm judgment registered as P10 S7** (ruling B): with "owning phase: P10; whether blocks main
>    line: No" + three unverified **original kept**; `06`'s F2 "needs decision" line points to this
>    registration; t4 output's "### F2 Judgment" subsection not moved.
> 3. **Crash item registered as P3 3.19** (ruling A): symptom / cause points to
>    `proof/smt/translate.rs`'s sort inference / D59 **A/B class** (crash) / acceptance row (this
>    shape no longer abort — conservative downgrade or diagnostic, corpus and IR snapshot zero
>    drift); **characterization = pre-existing** (t5 two revision comparison, see "VII" ⑤).
> 4. **Rule consolidation one** "correction in-place write to text, no old-new comparison": land
>    `coding-rules.md` (new rule), `07-equivalence-oracle.md` §Semantic correction shunt criterion
>    rule 3, RFC-039 **D59 row** rule 3 (same commit registration = WBS row label error type +
>    in-place correct affected text + PR required block "error type" one line).
> 5. **Rule consolidation two** "contract `inScope` uniformly write path prefix resolvable by path
>    matcher": land `AGENTS.md` team orchestration convention rule 6.
> 6. **New code W1082 reconfirmed** (external contract table change, Class E).
> 7. **Register P10 S8** "document correction form convergence (in-place rewrite, delete revision
>    note)": includes 7 audit list scope and acceptance row.
> 8. `09` top count table: secondary **63** (60 + 3.19 / S7 / S8), three-level **157** (154 + 3),
>    with 2026-10-18 note; three-level item's account review see "XV".
>
> **VII. Process facts to be honestly recorded**: ① 4.4.2 commit used
> `SKIP=cargo-fix,cargo-clippy-fix` to skip two `--fix` hooks — shared working tree has teammates'
> in-progress products, `--fix` will casually rewrite others' files; **equivalent gate already fully
> ran manually in final tree** (fmt --check / clippy `-D warnings` both 0). ② `pre-commit`'s stash
> window indeed occurred (`Stashing unstaged files…` / `Restored changes from…`), post-commit
> `git status` check shows teammates' in-progress products intact. ③ **4.4.3's inScope lesson**:
> prose qualification `docs/src/(only error code generation product)` rejected by `changedPaths`
> check ⇒ `inScope` uniformly write matchable path prefix (consolidated as rule, see "VI" 5). This
> task **actually 11 files**, 7 of which because declaration writing can't be resolved by path
> matcher not listed in `changedPaths`, already item-by-item listed in task result
> (`codes/w1xxx.rs`, RFC-013 code table, `reference/error-code/`'s index and each E segment page
> etc.). ④ **W1082's locale window closed**: `locales/zh.json` / `locales/en.json` same commit as
> code, other languages supplemented by i18n bot post-push (run see "IX"). ⑤ **Z3 `Sort mismatch`
> crash judgment (t5 criterion ⑦)**: **pre-existing** — same probe, same input on `b5e68b3c9` and
> frozen revision `29ebd2145` stdout **byte-for-byte identical**
> (`Error: Sort mismatch at argument #1 for function (declare-fun not (Bool) Bool) supplied sort is Int`),
> both exit 1; static attribution consistent (4.4.2 0 line change to `translate.rs`, to
> `z3_backend.rs` only `cache_stats` migration). **Downgrade branch production surface unreachable
> exhaustion holds** (W1082 no user-visible surface). Probe three lines source:
> `NonNegative: (x: Int) -> Type = { x >= 0 }` / `AtMost: (n: Int, x: Int) -> Type = { x <= n }` /
> `f: (x: NonNegative(x)) -> AtMost(x, x) = { return x }`. ⑥ **t5 frozen surface and t9 coverage
> diff**: t9 (three place comment/doc patch, `1b593e993`) commit later than t5's frozen revision
> `29ebd2145`, is zero behavior change, its acceptance covered by t9's own mechanical check + t6
> review (t6 on `1b593e993` **reran** t5's key measurement, not retold). ⑦ `docs/src/en/**` not
> manually modified, regenerated by post-push auto-translate workflow (this batch the bot landed
> two: `92d490695` i18n locales supplement W1082 other languages, `63b6cdf37` auto-translate docs).
> ⑧ **t9 out-of-scope observation three rulings**: ㈠ t9 casually correct `02` same sentence's
> `wasm/src/lib.rs` line number and gate point line number (73 / 139·153·170 → 152 / 151·170·187) —
> **accept** (same caliber same number required surface, commit info already itemized); ㈡ RFC-039
> **D35 row** still writes "wasm 27 files' cfg branches" — **keep unchanged** (resolution row is
> historical ruling record, its premise correction carried by P10 S7 registration, don't rewrite
> history); ㈢ `docs/src/en/**`'s old entry name translation aligned by auto-translate. ⑨ `09` /
> `09a` originally pointing to deleted batch directory two paths (`…/yx44-evidence/EVIDENCE.md`,
> `…/probe-sort-mismatch.yx`) in closure round changed to point to
> `target/yx-scratch/close-44/evidence-summary.md`, probe three lines source in-place inline.
>
> **VIII. t8 acceptance ④'s caliber ruling**: Acceptance original text requires
> `git diff | grep '修订注记\|原为\|现改为'` empty. Measured **default context caliber unreachable**
> — D58 row adjacent row's D59 / D61 resolution row itself contains "修订注记 / 原为" wording (not
> this task's change). Ruling: per **add row / delete row / `-U0` increment row** three calibers all
> empty accept, and state **subsequent similar acceptance only grep add/delete rows** — criterion
> should fall on this change's rows, not whole diff's context.
>
> **IX. CI gate conclusion (AGENTS rule 13①, scheduled check 2026-10-18 17:26 backfill; detail
> `target/yx-scratch/close-44/gate-runs.md`)**: First push (3 commits, tip = `d07993157`): CI
> **39110908163** success | Deploy Docs **39110908155** success | Auto Translate Docs
> **39110908193** success | Auto Translate I18n **39110908144** success; this tip covers `24028ffe8`
> (4.4.2) / `52d2276ca` (4.4.3) / `d07993157` (4.4.1) three commits all content. Second push (1
> commit, tip = `1b593e993`): CI **39112293391** success | Deploy Docs **39112293062** success |
> Auto Translate Docs **39112292903** success; covers all four commits including t9 patch. **No
> concurrency cancellation**, latter covers former. Third push (2 commits, tip = `da724940b`): CI
> **39121538555** success | Deploy Docs **39121538562** success | Auto Translate Docs
> **39121538541** success — three runs' headSha all = `da724940b` and **all terminal state**
> (2026-10-18 third round scheduled check backfill); round 3 independent verification t12 at the
> time noted as "CI not terminal" observation closed here. Unrelated: Nightly **39081392694**
> failure's headSha = `290f9ebc`, not this batch's commit surface.
>
> **X. Three `pull --rebase` (auto-commit counterpart during push period)**: Counterparts are
> auto-translate and i18n commits; six hashes rewritten and item-by-item **patch-id fully equal** —
> local `1fc135437` → `24028ffe8`, `2d5e610cd` → `52d2276ca`, `29ebd2145` → `d07993157`, `1ebddaf95`
> → `1b593e993`, `657c3d5e3` → `e184d88b6`, `c8412565c` → `da724940b` (post third rebase origin/dev
> head = `da724940b`); `git range-diff` verified **each commit content unchanged** (three all `=`).
> Member didn't execute `git push` / `fetch` / `pull`, sync and push unified by team lead.
> **Evidence reminder**: thereafter reference commit number uniformly use **post-push hash** (above
> right value); `09`'s check row per convention doesn't write commit number.
>
> **XI. Not covered (must not write "not tested" as "passed")**: ① doc-tests 0/0 not bearing
> criterion; ② **wasm32 target real build not done** (this surface covered by CI's
> `_build-wasm.yml`, this batch's wasm judgment is static argument); ③ Mutation only covers 3 places
> named by contract, other gates no reverse-bad-thing verification; ④ Corpus diff not separately run
> on `b5e68b3c9` (only ran on frozen revision); ⑤ i18n bot supplemented other language W1082 text
> behavior not verified (only verified `zh.json` / `en.json`); ⑥ Round 1 review and fix round
> reported **same-class but not in contract findings pre-existing drift** not fixed (list see
> "XII"), ownership pending user ruling.
>
> **XII. `02` whole-doc pre-existing drift observation list (reported to user, pending registration
> after ruling)**: Caliber = all **pre-existing drift** (not introduced by this batch), **does not
> constitute this batch's finding**; list found by fix round, round 2 review on `657c3d5e3` (=
> post-push `e184d88b6`) independently spot-checked and given mapping, each item recorded verbatim
> (full list archive `target/yx-scratch/close-44/s9-observations.md`):
>
> 1. `02` §(a) entry table 3a-3d / 5 / 5b: `02:72` cites `orchestrator.rs:273-399` → current `:155`;
>    `02:73` `:450` → `:250`; `02:74` `:1374` → `:985`; `02:78` `util/diagnostic/mod.rs:565` →
>    `:681`; `lsp/handlers/diagnostics.rs:146`'s `run_diagnostics` no longer there; **content
>    level**: `run_diagnostics` and "otherwise manual lex→parse" already replaced by 4.6.1 (delete
>    manual sequence).
> 2. `02:1016`: `compile_project :99-237` → current `:107-134` (28-line Program constructor).
> 3. `02:360-361`: "when missing downgrade to warning" → current tree `_build-wasm.yml:50-51` is
>    `::error` + `exit 1` (#435); `build.rs:65-72` already changed to `panic!` hard fail;
>    `_build-wasm.yml:35-59` range needs re-take.
> 4. `02:127` mechanism pointer: mechanism actually in `src/driver/arms.rs::typecheck_orchestrated`
>    (`:288-289` self-stated); conclusion still holds (multi-file arm drops warnings), inaccurate is
>    "result sole downstream use" sentence (`type_result` also via `arms.rs:1389` →
>    `helpers.rs:259/:636` feeds `overload_resolutions`); suggested fix = pointer to
>    `typecheck_orchestrated`, "sole downstream use" change to "warnings field no read point".
> 5. `02:128`: `check_project:385-395` / `250-252` needs re-take.
> 6. `02:334` (§S5 (d) investigation paragraph): cited old code snippet already replaced by 4.4.3's
>    `ownership_inputs` criterion point.
> 7. `02` change list table other rows: `predicate.rs:34-36`, `checker.rs:1303-1314` / `:1244`,
>    `backend.rs:67-72`, `wasm/src/lib.rs:30-36` / `:42-51`, `util/diagnostic/mod.rs:621-661`.
>    **This closure doesn't preset S9's WBS item** (user not ruled); after getting ruling per
>    registration form land `09`.
>
> **XIII. Batch tree recycling (gc and status raw output; tree path in output per repo relative
> path)**: `python scripts/dev/scratch.py gc` →
> `[scratch] deleted yx44-evidence (freed about 0.0 MB)` /
> `[scratch] gc complete, total freed about 0.0 MB`; `python scripts/dev/scratch.py status` →
> `yx44-evidence            holder=-` next line `target/yx-scratch/yx44/tree` — registry's recorded
> path **doesn't exist**, this name is registry residual (`gc`'s `_remove` deletes `meta["path"]`'s
> parent directory i.e. `target/yx-scratch/yx44`, residual `meta.json` still falls in scan surface;
> its content batch=`yx44`, `base_rev=29ebd2145`, `lock=null`, contains t5 / t6 / t11's holding
> history); Closure round renamed this registry file to `meta.retired.json` to make it exit scan
> (**don't delete any evidence file, don't create any tree**) →
> `python scripts/dev/scratch.py status` = `[scratch] no batch tree`; rerun `gc` =
> `[scratch] no batch tree, no need to recycle`; `git worktree list` only main mount (`dev`). Batch
> original evidence directory `target/yx-scratch/yx44-evidence/` (no `meta.json`, **not batch
> tree**) kept as archive, closure summary separately land `target/yx-scratch/close-44/`. Acceptance
> description (parsable): `git status` clean; batch tree gc post
> `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only main mount; no
> newly added directories outside repository this batch.
>
> **XIV. Doc real build and commit hygiene**: `npm run docs:build` local **exit 0** (AGENTS rule 10⑨
> — markdownlint can't catch VitePress's Vue compile error, and doc-only commit will be judged
> `no files to check` skip markdownlint); this batch's check / note / registration / rule
> modification accumulated into **one** doc commit (rule 8), land in batch closure; member didn't
> execute `git push` / `fetch` / `pull`. **This batch's final two both pushed** (closure tip on
> `git rev-list --count origin/dev..HEAD` = 0, origin/dev head = `da724940b`): `e184d88b6` (round 1
> findings' doc anchor fix, 3 files 23+/23-) + `da724940b` (closure doc commit, 7 files 96+/10-;
> including check / record / three path change / registration and rule modification).
>
> **XV. Count table account review (round 3 fix F-04; method and result both verifiable)**: Judgment
> object = `09` top "positioning and scope" table's three-level count row. **Method** (two
> independent calibers, per revision use `git show <rev>:<file>` take byte recount): ① **Sum per
> secondary item** — take each secondary kanban row parenthetical "N three-level tasks" (row head
> `- [x]`/`- [ ]` bold title start, row contains "个三级任务", includes S7 row inline-written
> "1 个三级任务"): current tree 70 secondary items, **sum = 157**. ② **Historical self-consistency**
> — same caliber on `366bc5a69^` get 66, **sum = 151 = current period table header 151**
> (self-consistent); `366bc5a69` same day newly registered `4.9.6` and `0.4.1` two three-level (sum
> 151 → 153), but table header only writes 151 → 152 ⇒ **table header from this commit undercounts
> 1**; thereafter each revision all diff 1 (`92d490695` / `1b593e993` / `d07993157` / `e184d88b6`:
> sum 154 vs table header 153; `da724940b`: sum 157 vs table header 156). **Result**: Truth =
> **157**, so correct table header (`09:13` three-level 156 → 157); note 144+2+2+2+2+1+1+3 = 157
> originally correct, not changed; this batch's previous base = **154** (not 153). **Secondary count
> row not moved** — this row's caliber never aligned with kanban item count (`366bc5a69^` is 66 vs
> table header 59), this time no verifiable evidence overturning 53 + 10 = 63, don't change number
> to make it match. Supplementary correction (same round): §2's 4.4.1 parenthetical decomposition
> per reconciliation script actual item count change to "README side 4 + code side 7 = 11",
> consistent with **11/11** and script 11 `ck` calls.
