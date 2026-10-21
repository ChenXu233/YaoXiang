# WBS Historical Notes (Implementation Record · Audit · Decision Trail)

> This document is the **historical notes** for [09-execution-wbs.md](./09-execution-wbs.md) (the
> implementation task table): implementation records, audit findings, decision trails, and
> corrections for each phase. **Division of labor**: 09 only describes the **plan** (kanban,
> three-level task table, dependencies, risks, acceptance criteria); this document only records
> **history** (what was done, what was found, how it was decided), appended chronologically without
> rewriting history.

## P2 Establish Equivalence Criteria Baseline

> **P2 Implementation (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green.
> **D38 prophecy fulfilled**—the process of going green measured and fixed three real ir_gen defects
> (see 06 §G6 for details): method locals table truncated to parameter count (local_count
> underestimated, runtime tolerated by Frame::set_slot), unannotated parameters dropped by
> filter_map (three places, params metadata misaligned with frame layout), match result slot relies
> on frame's implicit Void initialization (now explicitly pre-initialized). The actual semantics of
> jump targets is flattened instruction index (not label-keyed as in 07 text), CFG is built at
> instruction level—recorded in the verify.rs file header. **Commit correspondence**: ir_gen fix =
> `a9ea199b`; verify main body = `d5b18390` (⚠ parallel stream COMMIT_EDITMSG collision caused its
> message to be incorrectly labeled as "docs(rfc): RFC-027a…", the content is actually verify.rs +
> six test files, already buried in history and cannot be amended, hereby recorded).
>
> **P2 narrow-scope implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> stop-the-bleed channel prerequisite unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; judgment contract uses the same `TestFileSpec` as single-file
>   corpus (single point on library side), binary location/subprocess invocation abstracted to
>   `tests/common/` and yx_runner migrated synchronously (prohibition three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. The trigger source uses the
>   `SumUpTo(3, r)` form nailed down by rfc027 tests (return position/call site/binding site)—the
>   `Sorted(3)` annotation example in 02 would be substituted for evaluation and directly Disproven
>   in current code, not showing Unproven silently, the `Sorted(3)` in 07 pseudocode is illustrative
>   only. Red-state measured data is in the test file header.
> - The two red criteria are marked `#[ignore]` permanently following the 2.4.5 red skeleton
>   precedent: default suite all green, `-- --ignored` reproduces red state; **remove attribute when
>   P3 fix lands to turn green**.
> - The rationale for comparison normalization's intentional deviation from the 07 (code, file,
>   line) table (excluding file/line, comparing only error code sets) is written in the 2.4.1 test
>   comments.
>
> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **moved out of P2**: the `Program` / `Obligations` types they reference don't exist until P4,
> cannot be compiled in P2. The former is P4's 4.1.4 (no double registration), the latter's
> `#[ignore]` red skeleton is merged into 4.3.1.
>
> **2.3.3 Implementation (2026-10-07)**: C4 mandatory coverage list verified item by item (07 §143),
> 10 semantic corpus layers all covered and within corpus-parity.jsonl difference baseline (26
> representative files 26/26 hit, grep verified)—ref/borrow/move 05-ownership full directory
> (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep, etc.), closures captures
> closures + closure_arg_inference + spawn_capture, currying curry_value_fix + curry_tail_expr,
> spawn 04-concurrency ×13, iterator for spawn_for and other 16 files, sum type sum_type__+
> pattern_or_guard + match_call_scrutinee, existential coercion point interface_dynamic_dispatch
> (Vec(Animal) existential heterogeneous wrapping triggers existential_coercions) +
> interface_rebind_dispatch, `?`/Try propagation question_propagation + option_try + try_methods +
> user_try_type, method overloading method_overload (+ method_overload_ambiguous_err in diagnostic
> diff), refined constraints Drop sequence refined__ ×5 + 05-ownership drives ReleasePlan (span key
> contract has 2.4.4 red criterion permanent), performance baseline `benches/pipeline.rs` merged (07
> §145 three smokes of two): `corpus_compile_frontend` full corpus first-compile 17.6s,
> `cli_cold_start_hello` process cold start 169.7ms; interpreter throughput covered by
> benches/lib.rs hotpath group (interp_fib_recursive_27 / interp_loop_steady_10m), not duplicating.
> Regression gate check-perf-regression.sh unified to P9 checklist (2.5).

## P3 Fix Correctness Bugs (Minimum Solution)

> **Error type annotation for landed rows** (retrospective annotation, not modifying history): 3.1 =
> B ｜ 3.2 = B (defensive empty arm, criterion turns green undertaken by 3.1) ｜ 3.3 = A ｜ 3.4.1 /
> 3.4.2 = A (new W code) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 / 3.4.7 = D
> (user-visible surface, already user-decided) ｜ 3.4.8 ①② = C (main) ＋ A (incidental three
> places) ｜ 3.5 / 3.6 / 3.7 / 3.8 = D (all user-decided).
>
> **P3 Diagnosis (2026-10-12, per user request)**: A diagnostic accounting check in response to the
> perception that "P3 entries keep growing".
>
> **I. First clarify the scale (perception amplified by two accounting defects)**
>
> 1. **Duplicate numbering**: `3.5.x` is shared by two task families—(a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 three-level entries: lexical and parser branches / AST `is_pub` /
>    checker side / dead code exemption convergence / peripheral echo / std and diagnostic text /
>    language reference convergence, all checked); (b) 2026-10-10 registered **3.5 higher-order type
>    parameter call path completion** (3 three-level entries). Both share `3.5.x` ⇒ cross-reference
>    ambiguity (**renumbered per user decision on 2026-10-12: 029g family changed to `029g.x`**),
>    and 029g's rows were mistakenly counted into P3. **Pending decision**: rename 029g's family to
>    `029g.x` (or move it to its phase), and synchronize other references in the document.
> 2. **Missing checkmarks**: `3.7`'s 4 three-level entries (3.7.1–3.7.4) are all completed (d922a551
>    / 38bd4b9e) but not checked ⇒ this entry corrected now.
>
> **II. P3 actual accounting (12 secondary rows)**
>
> | Row                                                                  | Status                                                                                                                                                                             | Disposition suggestion                                                                                                         |
> | -------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
> | 3.1 / 3.2 / 3.3                                                      | Completed                                                                                                                                                                          | —                                                                                                                              |
> | 3.4 Silent Drop Point Addendum (8 three-level)                       | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (prerequisite 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (prerequisite 4.3.1 not started) | 3.4.6 **doable now**; 3.4.4/3.4.5 unlocked with 4.3.1                                                                          |
> | 3.5 Higher-order type parameter call path completion (3 three-level) | Not started (declaration side alive, call side broken)                                                                                                                             | Keep in P3; **does not block refactor mainline**, but is "accepted feature not implemented", priority higher than pure cleanup |
> | 3.6 Function type value call path (1 three-level)                    | Not started (semantics trimmed (i))                                                                                                                                                | **Keep**: correctness bug, criteria clear                                                                                      |
> | 3.7 init segment call name qualification (4 three-level)             | **All completed**                                                                                                                                                                  | Check and close this time                                                                                                      |
> | 3.8 native shadowing (1 three-level)                                 | Not started                                                                                                                                                                        | **Stuck on decision** (first define name resolution priority), not a scheduling issue                                          |
> | 3.9 / 3.10 / 3.11                                                    | Completed (2026-10-12)                                                                                                                                                             | —                                                                                                                              |
> | 3.12 Runtime mode executor config propagation (1 three-level)        | Not started (D class, user decision ②: separate team)                                                                                                                              | **Suggest move out of P3 → P8 (execution layer)**, pending decision                                                            |
>
> **IV. 4.3.1's leverage (mainline milestone)**: `4.3.1 obligations.rs + assert_drained()` is also
> the prerequisite for **3.4.4**, **3.4.5** and **4.5.1** (cross-layer contract PlanId-ization) ⇒
> taking it once advances the mainline and clears two P3 stranded items. 3.5 (HKT), 3.6 (function
> type value call) and 4.6.1/4.9.1 all have **no unsatisfied prerequisites**, can be done anytime;
> 3.8 stuck on semantic decision (user-defined priority vs explicit diagnostic).
>
> **III. Mechanism suggestion (prevent recurrence)**: New secondary/three-level rows **must have two
> fields**—"belonging phase" (which P it truly belongs to) and "whether blocks mainline";
> `registered pending scheduling` is no longer the default state, unmarked entries are considered
> unschedulable. This way inflation is visible and sortable, rather than indiscriminately
> accumulated. **Pending decision**: whether to write this clause into this document's registration
> convention and `src/dev/coding-rules.md`.
>
> **P3 Implementation (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + unique getter
> (recorded as `pub(crate)` at the time; **verified on 2026-10-07 that the actual landing was
> `pub(super)`**—the code comment references the `#434` decision, not registered on the docs side,
> see 3.4.3; full private would require rewriting 11 construction literal sites, not taken in the
> stop-bleed phase; will be closed during P4 Obligations migration); execution mechanism extracted
> to `src/frontend/proof_execution.rs` (prohibition three single point—cannot be placed in
> typecheck/, that would create L3→L4 reverse dependency); 3.1.2 per 02 evidence chain step 5 filled
> **four entry points** (WBS row missed compile_project `:99`, must be filled and registered per
> 3.1.1 acceptance "criterion turns green"); 3.2.1 empty arm changed to bookkeeping (proof_calls
> re-throws) + diagnostics (`into_result` established conversion path); 3.3.1 SOLVER slot made
> Option, Z3 missing falls back conservatively as `SMTResult::Unknown` (E2031 family diagnostics
> replace process crash). Two bug criteria removed `#[ignore]` to turn normal green, added corpus
> fixture `proof-obligation-honored`; check-fanout inventory migrated accordingly (wiring point
> 19=inventory). Measured: criteria 2.4.1/2.4.3 turned green, full suite lib 2494 / integration 272
> / corpus dual-layer all green.
>
> **3.4 Audit (2026-10-07, post-P3 implementation review)**: For the class of defects "failure
> produces no signal" (definition in 02 §41-52), re-inventoried on unchanged code, located 8 more,
> all registered as 3.4.x above. Method and evidence:
>
> - **Consumption-side inventory** (16 obligation fields grep'd for all read points):
>   `variant_ctor_calls` **zero consumers**—the only read `checker.rs:1468` is "self-injected into
>   `TypeCheckResult`", IR side uses form detection `ir_gen.rs:7374`; `rfc011b.rs:659/688` only
>   asserts production side, exactly the form criticized in 02 §186 as "only test production side,
>   naturally immune to consumption-side defects" → 3.4.4. `method_overload_ir_names` result-level
>   field likewise zero external consumers, IR uses parallel `overload_resolutions` → 3.4.5.
> - **Stage coverage re-review**: 02 §91 table row 1 (four entry points "no proof_execution") fixed
>   by 3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 state unchanged (multi-file `run` still
>   doesn't output W1003, doesn't run dead code family) → 3.4.6; LSP non-project-internal path (02
>   §77 "manual lex→parse") also doesn't run proof → 3.4.7.
> - **Same-shape empty arm scan** (`Unproven` full warehouse consumers only four): checker three
>   refined branches (covered by 3.1), 3.2.1 fixed ownership arm (**source code unreachable**), and
>   `environment.rs:422` (**reachable, zero diagnostics, zero bookkeeping, and zero registration in
>   full warehouse docs**) → 3.4.1, the only "all-unknown" live bug in this audit.
> - **Solver acquisition side**: `default_solver()` three consumption forms differ
>   (`predicate.rs:32` process singleton / `checker.rs:1266` per `check_module` injection /
>   `ownership.rs:621` per back-edge judgment new)——3.3.1 only eliminated the panic one; the
>   **silent surface** of the other two is 3.4.2, **singleton-ization and cache hit** goes to P4's
>   4.4.2 (no double registration).
> - **Already-attributed items not counted into 3.4**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys not consumed, Drop silently lost) still in P2 red state permanent, attributed to
>   4.5.1/D20 unchanged; 4.4.1 (layer order declaration opposite to actual, `equivalence` layer not
>   in pipeline) / 4.4.3 (`unwrap_or_default()` empty bookkeeping downgrade) already in P4 registry.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 prerequisites, can land in place
> anytime (suggest doing 3.4.1 first—the only "full warehouse zero registration" live bug, smallest
> change surface); 3.4.4–3.4.8 follow P4 execution, their criteria depend on obligation ledger
> (4.3.1) or unified Driver (4.2.x). **3.4 does not establish a new criteria system**, reuses
> existing C2/C3 and P9's 9.2 gate uniformly.
>
> **Batch 3.13–3.16 Implementation Record (2026-10-13 registered → 2026-10-15 closed)**: User
> 2026-10-14 decision to merge four relevance sub-items (#451 method binding panic / #452 named
> actual arg participating in resolution / #453 dead inference entry / #454 native export named
> actual arg—the last item is a boundary discovered when verifying 3.14, on the same day attribution
> experiment confirmed **not introduced by #452**, user decision ① merged into this batch) into one
> team. Closure status: all four completed, decision registered as RFC-039 **D60** (six binding
> rules + shared criteria single point).
>
> **I. Two contract corrections (lesson: contracts themselves can be wrong)**
>
> 1. **Acceptance fixture self-contradictory**: #452 task contract simultaneously required "mixed
>    actual args output 20" and "position and naming compete for same position errors", but the
>    board fixture `Temp.pick(self, a)` only has one value parameter—the same actual arg combination
>    is both "mixed" and "conflict". The first implementer honestly reported blocker and set task to
>    failed, full revert (not trading silence for green light). Disposition: task reassigned +
>    contract revision—gave **six binding rules** and **8 value-level matrix** (fixture changed to
>    two value params `pick(self, n, a)`) then attempt 2 completed by second implementer. The full
>    text of the six rules has been transcribed verbatim into D60.
> 2. **inScope missed mechanically required surface**: #452 contract's inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 was registered, only `code-tables --fix` was
>    run, **didn't re-run** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth red ("source
>    registered/emitted but documentation missing 1: E1110", source 154 / documentation 153, run
>    **38518624964**—the only CI red line in this batch). Fix = mechanical regeneration (8 files
>    +83/−70, only touched `docs/src/reference/`), reconciliation 154/154, CI 38520209026 green.
>
> **II. Review → Fix → Re-review two closed loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed with resolution ⇒ calls with named actual args silently passed through two paths
>    of multi-candidate zero-hit/single-candidate hit) → fix round added external code **E1111
>    (argument_order_invalid)** (code table + locales + RFC-013 + generator four-piece set in same
>    commit; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared parsing entry + native arm settling by name) →
>    independent verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations
>    mechanically reversibly replayed via `scratch.py trial` all caught, restoration byte-by-byte
>    consistent) → round-1 review needs_revision (F1 = typecheck native arm **inline copied** a
>    binding criterion, missing rule ④ ⇒ method and native same-form calls **divergent
>    decisions**—method reports E1111, native silently exit=0; ir_gen has a third existing duplicate
>    reporting old code E1015) → convergence round consolidated criteria into single implementation
>    `bind_args_to_params` (`arg_binding.rs:35`, four call sites `expressions.rs:2727` / `:5416`,
>    `ir_gen.rs:7776` / `:8279`) → round-2 review caught a **regression** introduced by convergence
>    round (IR convergence block lost native omittable positions ⇒ previously successful programs
>    with optional positions omitted misreported E1010) → fix round (optional position criteria and
>    param names from same source `signature.rs:236` single exit) → round-3 review PASS
>    (independently reproduced red→green; under mutation warehouse board fixture FAILED empirically
>    criteria can fail). **Lesson**: review must cover "unfixed branches of similar paths" (this
>    batch's two needs_revision were both probes all hitting the success surface); fix round itself
>    can also introduce regression (omittable position), caught by next round's different-person
>    review.
>
> **III. Process events (recorded truthfully, not pointing to individuals)**
>
> 1. **External whole-tree discard (2026-10-14 around 17:14)**: Working tree was externally restored
>    silently—implementer's fix in-progress (typecheck/ir_gen/code table/locales/RFC-013, etc.) all
>    disappeared, **at the same moment the two review documents edited by the user themselves also
>    disappeared from git status** ⇒ judged as repository-level discard (most likely user-side
>    manual cleanup/IDE "discard all changes"). Three pieces of evidence exclude team members: empty
>    stash, no new commits, no reset in reflog. Implementer replayed in-progress work by context and
>    pushed; discipline changed to "**save patch to scratch first, then sample red state**".
> 2. **Verifier ran out of context, changed person**: Independent verifier validating 3.16 ran out
>    of room midway (no closing message), changed person per load rules—successor continued with
>    ready products (green state result, corpus equivalence script, mutation plan), verification
>    finally PASS. Positive precedent of partial product handover.
> 3. **Review round 5 rotation and historical round retraction**: When rotating review per load
>    limit ⑤, `remove_member` **retracted together** the already-decided terminal
>    review/verification rounds to the shared pool, scheduler immediately re-ran (t3/t8 picked up by
>    new members, duplicate work and blocked downstream). Disposition: captain took over and
>    cancelled retracted historical rounds (t3/t8/t13), added AGENTS.md rule 12 based on this
>    (handle retracted rounds immediately after personnel change). This batch's closure record
>    registers this event per that rule.
> 4. **scratch governance (about 6.3 GB)**: Early in the batch each task had an external tree, 6
>    directories scattered outside the warehouse (about 780 MB, cleaned) + two members' external
>    verification trees (about 2.2 GB / 3.3 GB, self-deleted after use). Produced governance tool
>    `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md rule 9
>    (batch tree single entry), rule 10 (verification environment six rules: mechanical reversible
>    mutation / line numbers taken from referenced revision / CI exclusive gates must run, etc.),
>    rule 11 (writer load cap), rule 12 (handle retracted rounds after personnel change) and two top
>    meta-rules (no private content / verify before assert).
> 5. **Contract defects (not pointing to members)**: #454 fix round's contract inScope only circled
>    2 files, actual mechanically required surface 8 (rule 6 is to prevent exactly this—contract
>    template should preset mechanically required surface list); also that task's terminal state
>    field was once occupied by probe payload, formal acceptance/command evidence was added by
>    captain per convention.
>
> **IV. Leftover items (not fixed in this batch, registered for future reference)**: round-3 review
> three low-severity findings—① Method path's `build_arg_slots` (`ir_gen.rs:521`) is still the
> criteria's **second** implementation (existing, registered per previous round's caliber with
> closure, not consolidated)—**2026-10-16 pre-release audit supplementary three points**: (a) it has
> **two** call sites: `ir_gen.rs:7888` (method binding path, uses bound function
> `binding.function`'s param names) and `:7975` (overload resolution path, uses resolved hybrid name
> leaf segment), original registration only wrote "`ir_gen.rs:521` area"; (b) it has already
> **diverged from shared implementation `bind_args_to_params`**: missing rule ⑤ (duplicate name →
> E1015), rule ⑥ (missing required param → E1010) and "optional position" concept, and hard-codes
> receiver position to `1 + i`; two call sites for empty slots `continue` unconditionally (comment
> writes "diagnostics go to typecheck (it guarantees resolution passes means no empty slots)"),
> which is a **dangling invariant written in comments with no assertion guard**; (c) RFC-039 D60's
> "all call sites consolidated into it" enumeration missed these two, declaration inconsistent with
> fact, and referenced line numbers were not re-taken on referenced revision, D60 text corrected
> accordingly; ② Batch verification tree (worktree) `cargo fmt --all` error is worktree workspace
> artifact, gate list should note fmt goes to main warehouse; ③ repair-type contract template should
> preset mechanically required surface into inScope (see III.5).
>
> **V. Final gate (closure revision, local two implementation commits + one doc commit unified
> push—the only push in this batch)**: fmt --check 0 / clippy -D warnings 0 / `cargo test --lib`
> **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** / `--test yx_runner`
> 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 / corpus_probe **342
> lines byte-by-byte consistent with baseline** (only regenerated line: `named_arg_duplicate_err.yx`
> E1015@14 → E1111@16, expected code and line synchronized, manually reviewed) / IR snapshot zero
> drift.
>
> **VI. Run links (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (same run's MSRV/Test cancelled by subsequent push
> per concurrency—cancellation≠failure, covered by later run); governance tool push → CI 38546373435
> / 38561405117; WBS registration (3.16 row + table header 56→57 / 147→148) → CI 38528560364.
> Closure push's own CI / Deploy Docs run was added to task record after push.
>
> **3.4.8 ③④ Implementation (2026-10-11, 8ecf56f93)**: ③ `CHECK_STAGES` tail added `Linking` (**pure
> merge**, entire block skips `is_bin_role` entry validation—E3020 criterion still owned by
> `RoleClassification` to prevent double reporting; also has hard reason: run path judges "has main"
> by merged IR, under collect semantics "entry itself errors → no IR" would be misjudged as missing
> main and hard abort) → `Monomorphization` (check form); `ir_generation_check` keeps unit IR (**in
> memory only**, for pure merge and mono consumption, not emitted/written to disk/no snapshot
> produced—not kept means aggregation request is empty, mono uses `NoObligations` false green); mono
> failure takes per-file collection channel in Check (diagnostic attributed to entry file +
> `failed_stages`, not set to failure), MultiFile keeps `fail_with`; `into_outcome` changed to "take
> collection first then judge failure". ④ Empirically **zero production changes**: full program BFS
> already covers cross-unit mutual recursion (pathological → E3005, well-formed three-layer chain
> executes correct value), delivered four e2e groups + one mono unit test as live criteria.
>
> **Verification chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross with HEAD binary), t4 (④, 10 fixture groups + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-form matrix
> (single/multi/check three forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increase, not decrease).
>
> **F1 (found during verification, D59 A/B class, same round fix)**: In standalone multi-file check
> without project root, aggregation request name (`lib.f`) and unit IR definition key (`f`) have
> different sources → mono falls into "native no IR definition" silent `continue`, check reports 0
> error/exit 0. Fix: aggregation side added reverse-lookup jump by each unit's **real definition
> key** (precise enumeration + exact match + ambiguity discard, not suffix heuristic; project form
> not registered, existing path unchanged). **Errata (same day)**: This reverse-lookup jump was
> **deleted** in subsequent regression fix; root cause instead solved by `module_key` dual-purpose
> decoupling (added `script_mode`) from naming dimension—IR definition key = IR call name =
> aggregation request name = `{unit_key}.{name}`. **Regression record**: After ③ landed, CI's
> `Check tutorial/guide code examples compile` red (258 blocks / 10 failures: 6 same-file same-name
> rebinding + 4 name collision with `<std/list>`, rootless check doesn't do qualification triggering
> `link_module_irs` collision check); fix = ir_gen split `script_mode` (qualified name ⊥ Script/Bin,
> E3023 gate looks at it instead) + `ir_generation_check` per unit always passes unit key +
> `.with_script_mode(rootless)` + Script top-level same-name rebinding "latter wins" (consistent
> with run path existing behavior); verification = doc gate 258/0, three binary comparison (HEAD/③
> tree/fix tree) three collision fixtures, check form E3005 no regression, E3023 not opened.
>
> **F2/F3 (accepted)**: mono diagnostic attributed to entry file entry, span can cross units
> (consistent with run path `path=entry` caliber); Check form holds one ModuleIR per unit (in memory
> only, released when check ends).
>
> **Issues left to subsequent tasks (not this round's regression)**: ① E3005's help text still
> writes "internal error; please report this issue", inconsistent with resource protection semantics
> (precedent: `e3xxx.rs:47` once moved same kind of "user write error" out of E3005)—registered
> pending scheduling; ② multi-file E3005's span rendering label misalignment (already reproduced
> with HEAD binary, pre-existing); ③ Request collection surface has **type parameter position
> dependency** (generic calls where type parameters are not at value parameter position don't emit
> instantiation requests, mono records `NoObligations`, convergent mutual recursion is this
> form)—same surface as 3.5 (HKT call path), registered to that row.

## External Track RFC-029g: Remove `pub` and Auto-binding (#399

> **P3.5 Implementation (2026-10-08)**: All seven secondary tasks completed, one commit for the
> whole batch (independently revertible).
>
> - **Measured inconsistent with plan (recorded truthfully)**: 029g.1.2/029g.1.3's acceptance
>   originally wrote "`pub f = ...` reports **parse error**"—after removing `KwPub` the actual `pub`
>   falls back to ordinary identifier, the old form lands on semantic layer reporting **E1001
>   Unknown variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specifically probing
>   removed syntax is an anti-pattern", **don't add** migration-specific diagnostic; corpus fixture
>   `pub_declaration_err.yx` pins E1001, documentation (`syntax.md`) honestly writes this
>   consequence.
> - **Design convergence (beyond RFC literal, requires review)**: `project_refs` and
>   `cross_file_refs` two fields have completely overlapping semantics after pub removal (same
>   "intra-package reference union", prohibition one prohibits parallel representation) → merged
>   into single `project_refs`, `set_exempt_pub`/`set_cross_file_refs` two switches deleted.
>   Behaviorally Internal role's non-pub items changed from "intra-file reachability" to "reference
>   pool judgment" (relaxed, false negative direction); Lib tightened from "pub absolute exemption"
>   to reference pool judgment; Script top-level unbound starts reporting W1001/W1002/W1004/W1005.
> - **Dead table cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs` exports writeback all deleted (production-side
>   injection, full warehouse zero consumption).
> - **Test migration** (test count not decreasing): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub is recognized" assertion reversed to "pub no longer
>   a modifier"; `passes/tests/dead_code.rs` 7 role semantic cases rewritten (2 assertion
>   reversals); `tests/integration/cli_e2e.rs` 8 role cases rewritten (`lib`/`internal` exemption
>   assertions reversed to report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` doesn't participate in IR); corpus difference
>   baseline added only 1 (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerated then manually reviewed.
> - **Measured**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-layer Contract PlanId-ization (D20)

> **4.5 Implementation Record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed as two functional commits, closure
> (registration / WBS check / gate / push) completed by t7. Prerequisite = 4.3.1 obligation ledger
> (red state archived 173 files / 533 keys).
>
> **I. Carrier decision (A: `PlanId` walks with AST node)**: Added `PlanId` (`src/util/plan_id.rs`,
> `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` for synthetic nodes
> outside parser), issued by **sole allocator** `ParserState::alloc_plan_id`; carriers at two
> places——`Stmt.plan_id` (statement identity) and `Expr::Call.plan_id` (call site identity),
> `Expr::call` is the warehouse's only call constructor. Rejected was the "both sides derive same
> identity from span" position coincidence form (D20's original defect).
>
> **II. Two-level verification and review (members independent)**: Each functional commit has an
> independent verifier and a reviewer, **verifier/reviewer don't reuse implementer evidence**: 4.5.1
> = t1 implementation / t2 independent verification (external tree + 5 mutations) / t3 review; 4.5.2
> = t4 implementation / t5 independent verification (external tree + 5 mutations) / t6 review.
> Review time and reviewed revision both fall in immutable history (`git show` verifiable).
> **Provider rename caused t5 / t6 to switch members and continue**: original verification/review
> members couldn't continue due to provider rename, switched members per orchestration convention
> and **wrote the reason in the record** (cross-round context retention priority, switch only when
> unusable); after continuation, conclusion consistent with original round (PASS), didn't reuse
> implementer evidence.
>
> **III. Two defects found and fixed during implementation self-check (t1, both in inScope)**: ①
> Consumption exemption criterion was once attached to `UnitState.ir.is_some()`——Linking/Mono in
> `check` form takes `unit.ir` away, real non-consumption **silently passes** in check form
> (mutation test: D41 red while CLI check exit 0); changed to read **the ledger's own completion
> flag**, added regression cases. ② Drop sequence drifted with process hash seed (partial order
> `sort_by` + `HashSet` iteration): snapshot criterion ran three times with 106/120/114 line drift;
> changed to **total order** (same name take max depth + name ascending), two consecutive runs zero
> drift.
>
> **IV. Criteria and evidence numbers**: After 4.5.1, `test_release_plan_spans_consumed` removed
> `#[ignore]` permanent (floor `examined ≥ 173` + no whitelist), 172 IR snapshots regenerated in
> same commit (programmatic review: added 1083 lines Drop + 86 lines jump index shift, zero
> deletions, no reordering), corpus 342 entries diff empty; 4.5.2 added
> `test_overload_resolutions_consumed` (difference set empty + evidence floor `examined ≥ 1` /
> `consumed ≥ 2`; measured examined=1, consumed entries 2, no resolution skipped 210), resolution
> form 9 items pinned, `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero drift**.
> Mutation driven: consumption key shifted by one position ⇒ criterion red + real program
> `error [W1090] ... produced key not consumed` + corpus gate difference; restored and re-greened.
>
> **V. Semantic correction classification basis (D59)**:
> `Type.method(positional args, name = value)` **silently gave wrong answer** before change
> (measured compile clean, runtime output `1` not `20`)⇒ **C class** (old behavior not successful),
> fix takes "loud": after change same program compile-time `error [W1090]`; "named actual arg
> participating in resolution" terminal semantics is **D class**, already registered §3.14 with
> issue #452 left for dedicated team. Also: `Drop` is no-op in interpreter
> (`backends/interpreter/executor/ops/control.rs` wildcard arm only `advance()`)—key migration
> doesn't change runtime behavior.
>
> **VI. F-1 / F-2 disposition (t5 independent verification found)**: **F-1 (medium)** unbound
> consumption point (`ir_gen.rs:7766-7790`) unreachable in current corpus and test surface, keying
> correctness has no red state falsifiable ⇒ registered as "known boundary" in
> [02](02-stage-contract.md) §2 revision note paragraph 5 (keep reason + don't open new task); **F-2
> (low)** pointer drift ⇒ `types.rs`'s `OverloadResolutions` doc comment changed to precise pointer
> (produce `inference/expressions.rs:4550`, consume `ir_gen.rs:7697` / `:7772`), separate small
> commit, not mixed into doc commit.
>
> **VII. Closure (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settle marks); 07
> criteria table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG recorded one row of behavior change; `check-obligations.py` and `check_tracking.py`
> green, CI and Deploy Docs green. **Accounting deviation found in closure**: pointers
> `ir_gen.rs:357` / `:3102` / `:1096` in t3 / t6 report differ by 1-3 lines from code fact (actually
> `:360` / `:3103` / `:1095`+`:1097`, due to 4.5.2 insertion)—registered per code fact, this note
> also keeps record.

## P4 4.10.3 standalone run and MultiFile Orchestration Unification (4.2.5 Decision A's run half)

> **Bing Batch Record (2026-10-15 registered → 2026-10-15 closed)**: User 2026-10-15 decision
> Bing—this round only does 4.10.3 (same batch candidate 3.4.6 deferred to after 4.9 per B1
> ([02](02-stage-contract.md) §runtime behavior table "dead code family warnings don't enter compile
> path"), prerequisite changed and noted); implementation form = **full single team** (user directed
> no DAG, no team, single-session implementation + self-verification). Pre-batch outstanding landed
> first: #455 dependency announcement fix (rustls 0.23.46 + serde_yaml_ng, `cargo audit` from exit 1
> recovered to exit 0), WBS registration debt (#456 registered as 3.17 + incomplete rows added
> "belonging phase / whether blocks mainline" fields).
>
> **I. Red first then green**: driver-level red criteria 3
> (`src/driver/tests/run_rootless.rs`——rootless merged IR must not have entry_function (red
> `Some("main.main")`), rootless top-level executable statement must not report E3023 (red doesn't
> produce module), Bin control entry_function keeps (green permanent)) + CLI e2e 3 pins (standalone
> relative import run (red E5001), Script top-level statement execution and #413 hint two green
> pins).
>
> **II. Implementation surface**: CLI `run` unified to `compile_project`
> (`run_file_with_diagnostics`'s project/standalone fork deleted); Linking's `entry_function` gated
> by `is_bin_role` (#413 T4——Script doesn't auto-call main); #413 hint keying changed from
> `module.source_files.is_empty()` to call site project probe (after unification ModuleIR always has
> source_files, original mark invalid); `script_main_never_called` adapted to qualified name form
> (function `{key}.main`, global slot also measured as qualified name—based on IR evidence, not
> inferred from comments).
>
> **III. Existing gaps exposed by unification (fixed in same batch, A/B/C class—only fix failed
> programs to success, originally successful programs zero impact)**:
>
> 1. **Explicit type actual arg pre-specialization name** (`mk(Int)(5)`): ir_gen per D6.3 directly
>    emits specialization name (mono strips unspecialized original), `resolve_call_target` three
>    arms (rename / aliases / dot prefix) don't cover `mk(int64)` form ⇒ call site dangling E6006.
>    Fix = this single point parser adds fourth arm (base name qualification, type parameter string
>    preserved as-is). **Bin project same-shape reproduction (pre-existing defect, not introduced by
>    this batch)**.
> 2. **Script top-level `mut` assignment statement duplicate slot allocation**:
>    `extract_global_defs` and `register_global_slots` count `counter = counter + 1` (same form as
>    declaration Assign) statement by statement ⇒ same variable allocated 3 slots, initialization
>    writes to later slot while read uses earlier slot (runtime Add type mismatch). Fix = merge by
>    name at both places (same criterion same source).
> 3. **Same module constraint generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidate + table lookup verification (precise hit →
>    generic owning unit key qualification hit → fall back to bare name current state; not suffix
>    heuristic, same form as `resolve_generic_name`).
>
> **IV. Registration gap (not fixed this batch)**: Constraint dispatch when **type and generic
> definition are in different modules** still E6006 (rootless dual-file probe verified; Bin same
> shape)—root cause = MonoType user type name doesn't carry defining module, correct fix belongs to
> P6 type identity convergence; registered WBS **3.18** (belongs to P3, prerequisite awaits P6,
> doesn't block mainline), `qualify_deferred_method` note same surface.
>
> **V. Test adaptation (honest registration)**: `tests/integration/multifile.rs` fixture added
> manifest—what it tests is RFC-029 project orchestration (Bin auto-entry) not Script semantics;
> `cli_e2e` one pin (top-level diagnostic variable name visibility) and `script_main_hint` fixture
> (7 cases) synchronized per qualified name form—production and consumption ends only consume merged
> IR, fixtures directly drive same form, no pipeline product substitute.
>
> **VI. Final gate (closure revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new pins) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> difference baseline regenerated **12 lines**——line-by-line structured check: exit / stdout /
> diagnostic code all identical per entry, only stack frame function names and variable names
> qualified by unit key (manually reviewed); after regeneration `check-corpus-parity.py` exit=0.
>
> **VII. Process events (recorded truthfully)**: This batch once had concurrent editing of workspace
> documents during pre-commit hook running, was eaten by hook's stash rollback and that commit
> failed (concurrent form pre-announced in config comment)—discipline = don't touch workspace during
> hook running; #455's commit thus split into two attempts (second full green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closure)

> **4.6 Batch Record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 directed to push
> forward in 4.6 → 4.9 → 4.4 → P5 order, do 4.6 first; implementation form = **full single team**
> (following previous batch's user direction: no DAG, no team, single-session implementation +
> self-verification). Task definition (09 three-level table): project-internal file's semantic data
> (SemanticDB) goes through orchestrator's same pipeline, same source as diagnostics; cross-file
> reference carries `resolves_to`. Acceptance: project-internal cross-file jump hits definition;
> semantic data and diagnostic come from same compilation.
>
> **I. Current state evidence (verify before assert)**: didOpen/didChange triggers **two rounds** of
> compilation—diagnostics go through unified Driver pipeline (4.2.3/4.2.6 unified), semantic data
> goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registry ⇒ project-internal
> `use` parse failure, cross-file reference events not produced, jump/rename/reference lookup
> silently fails for imported symbols—D55 registered silent failure). Orchestration arm's checker
> uniformly names `"<module>"` (SemanticDB file identity lost), `type_result` (containing
> semantic_db) discarded when diagnostics non-empty and never emitted; `checker.rs` drain covers
> `resolves_to`'s DefId file identity always to current file; `Export`/`ModuleInfo` both lack
> definition location surface.
>
> **II. Red first then green**: LSP e2e pin (`src/lsp/tests/cross_file.rs`)——temp dual-file project
> didOpen main.yx then send GotoDefinition to `add_one` call site, **before fix returns None (red,
> empirical silent failure above)**, after fix hits lib.yx line 0 definition (green). Probe
> positioning during process: compiler-side link (driver probe dump) already fully connected but e2e
> still empty—root cause is `resolve_reference` precisely matches by reference **start point** (D55
> precise-only current caliber), test cursor lands in middle of identifier; this tolerance is
> existing behavior surface, this batch doesn't touch (see VII).
>
> **III. Implementation surface**: (1) `Export.definition_span` (extract_module_info harvests from
> AST identifier node, same source and value as typecheck definition event's `def_span`; native std
> always None) + `ModuleInfo.definition_file` (Registry arm filled by DiscoveredFile, including
> vendor; std always None); (2) checker reference drain via `import_def_locations` (use line binding
> span → definition file + definition span) redirects cross-file import item's `resolves_to` out of
> this file; std import no registration, stays at use line; (3) checker module identity changed from
> unified `"<module>"` to unit file path (empirically `env.module_name`'s all 4 consumption points
> zero diagnostic consumption); (4) `DriverOutcome.semantic_db` channel——CollectAll form (Lsp target
> file **regardless of diagnostic success or failure** + SingleFile fallback) converges, Lsp arm
> tail injects non-target file top-level export definition surface from registry
> (`external_definitions`, even unopened files can jump; type/submodule exports not
> injected—typecheck side type registration doesn't produce definition event, both sides maintain
> consistent gap); (5) `orchestrator::analyze_source_in_project` (`check_source_in_project`
> redirected to take diagnostic half); (6) LSP `analysis.rs` single compilation entry replaces
> manual pipeline (`update_semantic_db` deleted), World key uniformly normalizes file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
> difference normalized to same key, cross-file DefId therefore stable hit); 5 handlers +
> semanticTokens double requests connected to `db_key`.
>
> **IV. Incidental fix (D59 routing B class——old behavior silently no action, originally successful
> programs zero external impact)**: ① Project-internal file cross-file jump/rename/reference lookup
> from silent failure to hitting real definition; ② Old manual pipeline deleted semantic info on
> lexical error (coloring disappeared) → residual AST coloring preserved (4.2.6 editor philosophy
> extension); ③ When target file has type errors, semantic data no longer discarded with
> `type_result` (typing intermediate state coloring/jump not disappear).
>
> **V. Pins**: driver-level 6 (`src/driver/tests/lsp_semantic_db.rs`——target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface round-trip,
> std import fallback to use line, error file preserve, single file fallback path key, single file
> error preserve) + LSP e2e 1 (red first then green) + uri normalization 4
> (`src/lsp/tests/uri.rs`——drive letter conversion, space encoding, idempotent normalization,
> non-file scheme passthrough).
>
> **VI. Mutation verification (scratch.py trial, mechanically reversible)**: batch tree wbs-46 @
> closure revision 39546020, two mutations both made hit pins turn red, after restoration blob
> consistent with HEAD + git status CLEAN——m1 removed `import_def_locations` registration
> (statements.rs) ⇒ cross-file driver pin + LSP e2e pin red, other 7 green; m2 removed
> `external_definitions` injection (arms.rs) ⇒ same two pins red (round-trip parse unanchored),
> other 7 green. Batch tree gc'd (released 4.8 GB, `status` no batch tree).
>
> **VII. Final gate (closure revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new pins) / `--test integration` **287 passed** / corpus
> difference **byte-by-byte zero drift** / IR snapshot zero drift / check-obligations PASS
> (13·13·0·8, `semantic_db` non-obligation reason consumer anchor synchronized with server.rs:482
> and 02 §obligation field table) / check-docs-truth 0 / docs-examples 258 entries 0 failures /
> docs-orphan 0 / markdownlint whole warehouse only user-uncommitted 032 two MD056 (user's own edit,
> not touched) / pnpm docs:build 0. **Process events (recorded truthfully)**:
> en/09a-wbs-history.md:54 one MD037 was introduced by auto-translate bot re-translating previous
> batch 09a (zh original has no space), this batch fixed in passing——markdownlint red not this
> batch's change surface but same file as 09a, registered together. **Remaining observation
> (registered for discussion, not touched)**: `resolve_reference` only precisely matches by
> reference start point——when cursor lands in middle of identifier jump/hover/rename silently no
> action (D55 precise-only current state, whole file form same), if tolerance matching is needed
> it's an independent behavior change, report to user for decision.
>
> **Same day continuation batch (4.6.2, implemented after user decision)**: After the closing report
> was delivered, user made decision on "remaining observation ①" "OK, then change it"——cursor key
> tolerance implemented per plan A: definition/hover/references/rename four handlers' query keys
> changed from cursor position (via UTF-16 conversion) to use `find_identifier_at_position` already
> located **identifier's own start point** (same coordinate system as semantic events, no conversion
> needed); `resolve_reference`'s start point precise matching semantics unchanged (D55 precise-only
> unchanged). references/rename's definition point fallback judgment (start <= col < end) is
> isomorphic for identifier start point. Red first then green: definition middle cursor pin first
> red (None) then green; four handlers middle cursor each 1 pin + cross-file e2e start/middle dual
> position assertion, `--lib` 2748 (+4) all green, clippy -D warnings 0. Registered WBS 4.6.2
> (three-level count 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 1 Closure (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 Batch Record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 decided to merge
> three related three-level tasks 4.9.1 / 4.9.2 / 4.9.3 **into one team one round** (reason: they
> all belong to "pool's source and identity" aspect, merged implementation avoids repeated rewrites
> of intermediate state after `collect_project_refs` deletion; 4.9.4 / 4.9.5 belong to consumption
> refinement and test pool separation respectively, depend on 4.6.1's `use` path resolution, forcing
> same batch would expand gate surface). #450 attribution **pending decision**—evidence shows it
> belongs to "overload resolution name not passed back to reference count" class, still reproduces
> under round 1 landed form, registered as independent WBS entry for disposition (captain recommends
> A: register as independent entry when merged into round 2; **pending user decision**). Freeze rev
> = `3863a9ff0` (parent commit `62f6e1a59` is pre-fix baseline); verification PASS attempt
> `aab92fe0` (i.e. t2); review pass attempt `96452696` (i.e. t3). **Closure registration commit =
> `38d30b846` (docs(docs): 4.9 reference pool round 1 closure registration)**——this commit was
> actually written and pushed by closure member integration-refpool; its closing probe once
> mistakenly marked the closure task as terminal failed and overwrote output, captain took over new
> attempt and changed back to completed per verified delivery facts, hereby correcting attribution
> and label. Evidence archive: evidence summary (8 mutation patch list, `warn_compare_v8.json`
> caliber and conclusion) inlined in this section "III. +8 and +16 two countings' correspondence"
> and "IV. mutation tests"; original files recycled with workspace, **not in repository, cannot be
> used as document reference source** (AGENTS meta-rule ①).
>
> **I. Merge decision**
>
> 1. **The decision itself**: three related three-level tasks merged into round 1. **4.9.4
>    (consumption refinement) / 4.9.5 (test pool separation) next round**——former depends on 4.6.1's
>    delivered `use` path resolution (already landed at `39546020`, can start); latter is same batch
>    as RFC-029f table incorporation, registered at RFC-029f end pending modification.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 are mutual prerequisites (4.9.2 changes pool type → 4.9.1
>    assembly surface needs rewrite → 4.9.3 has denominator-judging role table), splitting rounds
>    inevitably writes intermediate state then changes twice; (b) all three belong to "pool's source
>    and identity" aspect, 4.9.4 / 4.9.5 belong to another aspect, splitting is by "cohesion" rather
>    than "independently revertible" cut.
> 3. **Merge result**: single commit `3863a9ff0` (one line `:recycle: refactor(driver)`), single
>    push, single batch gate. **Hash explanation**: before push `git pull --rebase` merged
>    auto-translate bot commit (`72f364169`); rebase landed after the other party's commit, this
>    batch's two (implementation `3863a9ff0`, its pre-fix baseline兼 partner doc commit `62f6e1a59`)
>    **not rewritten, kept original hash**——initial version mistakenly wrote here as
>    "`3863a9ff0`→`3863a9ff0`" identity mapping (looks like "rewritten to same value", actually "not
>    rewritten"), corrected on 2026-10-17 closure; other party's code commit
>    `33f14e8ab`→`b122b7ad1`. **External references all use post-push hash as authoritative**
>    (rewritten old hash unreachable in new clone, identity determined by patch blob comparison).
>
> **II. Five criteria red/green measured caliber**
>
> | #   | Criterion                                              | Red state (before fix / P1 directory scan callback patch)                                                                                                  | Green state (round 1 landed)                                                                                                                                                                                                                                                                                                                                                     |
> | --- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | ①   | Pool's root = package target (#334)                    | Directory recursive scan ⇒ tests/ file identifiers also enter pool, exempting production code unused definitions                                           | `collect_package_refs` (`helpers.rs:481`) per `ExplicitSurfaces` root calls `discover_with_used`; full root degradation fallback `collect_compilation_refs` (`helpers.rs:454`)                                                                                                                                                                                                   |
> | ②   | Pool granularity = `(module, name)`                    | Same-name local variable (`let foo = …`) accidentally exempts same-name top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` cross-contaminate | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) sole assembly entry; `arms.rs:603-609`'s `other_module_refs` unions "except this unit" all module entries to `DeadCodeAnalyzer`; module's own names (top-level definition and local binding, `use` import name exception) no longer exempt other modules' same-name definitions |
> | ③   | `FileRole::Script` doesn't use pool                    | Script affected by other files' references in project                                                                                                      | `arms.rs:611-623`'s `match us.role` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`), Script/Test takes `_ => {}` default arm (per file's internal definition-usage graph)                                                                                                                                                               |
> | ④   | Fixture A: lib.yx line 4 definition must report W1001  | Before fix: reports `W1003@1` (actually "unused import"), not W1001                                                                                        | Red state measured (red 1 fixture red1) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                       |
> | ⑤   | Fixture B: main.yx line 4 definition must report W1001 | Before fix: reports `W1003@1`                                                                                                                              | Same as above                                                                                                                                                                                                                                                                                                                                                                    |
>
> **Criterion ②'s "deduced red"** (implementer self-stated + verifier closed): red2 fixture's
> `let foo = …` is in helper.yx; old pool in bare name union treated `foo` as a whole "used by
> helper", lib.yx's `foo` exempted. **Measured**: red2 fixture vs pre-fix binary → no W1001 (old
> caliber exempts); red2 vs post-fix binary → W1001@1.
>
> **Criterion ③** (Script doesn't use pool) red state fixture **first version failed**: first
> version fixture in same file put same-name local variable triggering E2010 (the language disallows
> same-module shadowing top-level definition), moved to another module (helper.yx) then deduced red
> state. This point was independently reproduced by t3 review and t2 verification, **not contract
> defect**——same-module shadowing is the language's hard constraint, fixture must cross modules.
>
> **III. +8 and +16 two countings' correspondence** (**t3 review proven as authoritative**)
>
> 4.9 round 1 evidence summary section C wrote `+16/−0`, but missed the **new fixture records**
> component——v8 instrument (`warn_compare_v8.json`, 331 files symmetric diff, `only_before=0`)
> caliber **includes** new fixtures; t1 implementer's self-built "+8" is **warn-before / warn-after
> (325 entries each) aggregated by fixture** (each member file checked once). Review mapped item by
> item:
>
> - **+8** (8 new warnings, all `check <non-entry file>`, `check <entry file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (unified attribution)**: these 8 fixtures all **didn't declare package target** ⇒
>     4.9.1 root degraded to checked file's own compilation set, consumer main.yx not in it; this is
>     by-design change (before these files' check surface was already zero warnings, not exemption
>     path). Warehouse root `yaoxiang.toml` is empty file ⇒ same degradation path empirically
>     established.
> - **+16 residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files in t1's `warn-before.jsonl` have `exit=None` (capture time point
>     **doesn't exist**——is this batch's new fixtures). So +8's "+ 0 − 0" diff doesn't have them,
>     +16's v8 symmetric diff does.
>   - `v8`'s `stray.yx` only has `W1001@2(probe)` left: reason is pre-fix binary also reports
>     `probe@stray.yx:4` (this fixture runs `probe` once per file), **cancels out** leaving only
>     lib's 1——three data sets (+8 / +16 / dual binary ABBA) mutually consistent.
> - **New fixture 8** vs **+16 residual 8**: same batch of files (4.9.1 / 4.9.2 newly added
>   `ref-pool-compile-set/` and `ref-pool-module-identity/`), instrument treats each of their checks
>   as independent record——same file may trigger different diagnostic entries for different
>   `role/context`.
>
> **Conclusion**: **+16 = +8 (aggregated by fixture) ∪ new fixture 8**——both countings are correct,
> **different calibers**: t1 reports "8 new" is fixture-level caliber, t2 reports "+16" is
> file-level caliber (symmetric diff includes new fixtures). t3 review ① +8⊆+16 proven; **review ②
> writes "don't copy EVIDENCE.md section C"**——section C missed "new fixture records" component,
> this item rewritten per t3 review `pass` conclusion.
>
> **IV. Mutation tests (mechanically reversible, all `scratch.py trial`)**
>
> 8 mutation patches (p1 directory scan, p2b use import name removal, p3b script receives pool, p3
> flat consumption, p4 std receives pool, p7final self-built 4 criteria, p8 std closed, p9 vendor
> receives pool + p_stack2 / p_stack3 overlay surface). Each trial output contains: HEAD blob →
> mutated blob → restored blob consistent with HEAD = yes + `git status` CLEAN. Local
> `core.autocrlf=true` needs `GIT_CONFIG_*` inject `false` (`trial_env.ps1`), otherwise patch new
> lines convert to CRLF causing apply/restore dual failure——**only affects patch mechanicality,
> doesn't change measured behavior**.
>
> **V. Process events / environment lessons (must read for handover)**
>
> 1. **core.autocrlf injection**: local `core.autocrlf=true` must be injected `false` via
>    `GIT_CONFIG_*` to ensure patch mechanical reversibility; that injection script was recycled
>    with workspace (criteria listed above, can be rewritten based on this). **Next round don't
>    reuse** any trial script without injection.
> 2. **Same tree trial serial**: `trial` and `cargo build` on the same tree **must be strictly
>    serial**. `trial` internally `git apply` and `cargo build` both touch worktree, parallel can
>    produce "patch incomplete when build sees it" or "build and apply deadlock" two kinds of races.
> 3. **trial rebuilding binary needs confirmation of sync**: after `trial` completes, will rebuild
>    tree-internal binary (cargo's fingerprint contains source path, **changing tree = full
>    rebuild**), before subsequent measurement must confirm `bin/yaoxiang-rs` is synced with HEAD.
>    **This round once used residual mutation binary to measure false data**, rebuilt and
>    re-measured to correct.
> 4. **Reason for second tree yx49-b's creation**: dual-拍 needs pre-fix binary (62f6e1a59), no
>    ready product in batch; `scratch.py init yx49-b` opened second tree (another full build).
>    **Tree released** (see VI), delete left to batch closure `gc` disposition.
> 5. **cli_e2e two Internal case fixtures补ed to legal package (added `[run].main`, assertion
>    untouched) reason**: pre-fix `cli_e2e`'s two `Internal` role pin fixtures **didn't declare
>    `[run].main`** ⇒ after 4.9.1 landed, `compile_project` takes `ExplicitSurfaces` root call path,
>    can't find `main` ⇒ whole batch 2 fixture behavior drifted from "fallback per Script" to "entry
>    not found"——this is 4.9.1's introduced fallback behavior, not defect. After adding `[run].main`
>    field, two fixtures take normal package path, **assertions** (exit code / diagnostic code /
>    warning set) **byte-by-byte unchanged**.
> 6. **Safe way to delete second round tree**: after release, use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`), will verify "no mount" before
>    deleting. This batch released before closure, next step `gc` naturally cleans.
>
> **VI. Batch tree state**
>
> Before closure: yx49 released (4.8 GB released), yx49-b released. Both trees no mount, status can
> recheck in `scratch.py status`. This task's output attached with `scratch.py gc` complete status.
>
> **VII. RFC-029f "Known Defects" table flip**
>
> Previous version (2026-10-09) described "three design defects of reference pool" (including
> `collect_project_refs` / `orchestrator.rs:603-648` / `orchestrator.rs:393-396` etc. deleted code
> locations) in present tense. This batch closure synchronously changed to "**known defect: landed
> (2026-10-16, WBS 4.9 round 1)**": ① Current pool's root = `collect_package_refs`
> (`helpers.rs:481`), per `ExplicitSurfaces` parse, no directory scan; ② Current pool type
> `HashMap<String, HashSet<String>>` (key = module key), sole assembly entry `collect_ref_pool`
> (`helpers.rs:429-449`), consumption end `arms.rs:603-609`; ③ Current Script doesn't receive pool,
> `arms.rs:611-623`'s `match us.role` default arm. **All file:line re-taken on referenced revision
> `3863a9ff0`**. `collect_project_refs` and `orchestrator.rs` already moved with P4 4.2.9 split
> (`c59fb751`) into `src/driver/{state,arms,helpers}.rs`, old positions in table no longer hit——this
> is the fix of "declaration inconsistent with fact" (t3 review note ①).
>
> **VIII. Final gate (closure revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 lines** (two new fixtures
> behavior run, 338a339,340; other 342 lines byte-by-byte unchanged) / IR snapshot 204 zero drift /
> check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process events**: this batch
> one cargo build due to `core.autocrlf` converting script line endings to CRLF, team scratch path
> `yaoxiang-rs` startup reported `BadImage`, measured as encoding inconsistent with git
> index——rebuilt and passed (parallel to 4.6 batch "external whole-tree discard" lesson, different
> reason).
>
> **IX. Leftover (handed over to next round)**
>
> 1. **#450 attribution decision**: evidence shows belongs to "overload resolution name not passed
>    back to reference count" class, **still reproduces under round 1 landed form**——dispatch per
>    user decision Bing process. **#450 probe**: dual binary same set `{W1002@2, W1005@4}`
>    (measured, rootless form), no direct relationship with round 1 changes, but #450's fix may need
>    to wait for 4.9.4 (consumption refinement) together——because root cause is "overload resolution
>    name passing reference count" needs use item's precise consumption surface, 4.9.4's "really
>    used after import" is the precise version of the same criterion.
> 2. **dead_code.rs:683 pointer nit**: comment writes "two pool assembly caliber see driver side
>    collect_compilation_refs"——actually `collect_compilation_refs` is fallback path, main path is
>    `collect_package_refs` (`helpers.rs:481`). Comment inconsistent with code main path caliber
>    (**not in this batch's scope**), left to 4.9.4 / 4.9.5 implementation time for correction.
> 3. **4.9.4 / 4.9.5 prerequisites unlocked**: 4.6.1 (`use` path resolution) already delivered by
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs RFC-029f table
>    incorporation first (already paved by this batch "VII." flip).
> 4. **`yaoxiang check` multi-file `run` never reports dead code family warnings** (WBS 3.4.6,
>    2026-10-15 user decision Bing deferred to after 4.9)——not involved in this batch round 1; next
>    round 4.9.4 / 4.9.5 implementation same batch processing.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 2 Closure (4.9.4 / 4.9.5 / 4.9.6)

> **4.9 Round 2 Batch Record (2026-10-16 implemented → 2026-10-17 closed)**: Covers 4.9.4
> (consumption criterion = really used after import), 4.9.5 (test pool separation), 4.9.6 (method
> reachability caliber, issue #450, user 2026-10-16 decision A merged into round 2). Closure fix
> (verification-side remedy M1/M2/M4 compilable redo + F1 fix round + PA-2 dead branch delete) and
> registration gap issue #464 (WBS 4.9.7) landed together.
>
> **I. Decision chain and commits (all post-push truth)**
>
> | Step | Content                                                                                                                                            | Post-push commit | Pre-push local | Judgment method                                               |
> | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------- | ------------------------------------------------------------- |
> | 1    | fix(util): check explicit path not blocked by default exclude list + --exclude always empty (issue #463, P0-0.4)                                   | 366bc5a69        | ef959ca55      | Patch blob same                                               |
> | 2    | Phase A = 4.9.4 + 4.9.5 (RefPool per import attribution + Test doesn't contribute consumption surface)                                             | e6cf1aeaa        | 84a6f6978      | Patch blob same                                               |
> | 3    | Phase B = 4.9.6 (#450 per-candidate method reachability; including artifact warning-codes.md)                                                      | ddcbb12d0        | cd629b5fe      | Patch blob same                                               |
> | 4    | docs(meta): AGENTS rule 10⑦ (commit period stash window convention)                                                                                | ec94fa9d8        | 9a4ed5f3b      | Patch blob same                                               |
> | 5    | Fix round = F1 (overload sibling overload restore report W1005) + F2 (verification-side M1/M2/M4 compilable redo) + F4 (snapshot generation noted) | b8380e6ab        | 3d9ef6eab      | Patch blob same (36962f318e42…, git range-diff no difference) |
> | 6    | Closure fix = PA-2 (delete strip_unit_prefix dead branch + 029f/CHANGELOG three false statements correct, register #464)                           | b940300ac        | e13984465      | Patch blob same (patch-id 9bda3cfe4c…)                        |
>
> **Hash explanation**: each push's pre-push git pull --rebase rewrites local hash; above "judgment
> method" column is all "patch blob same" (`git diff <rev>~1 <rev> | git hash-object --stdin` both
> sides consistent + `git range-diff` no difference). This article doesn't reference any pre-push
> hash as authoritative source.
>
> **II. Three criteria red/green measured caliber** (verifier self-built reproduction, not adopting
> implementer criterion)
>
> | Criterion                  | Red state (before binary 366bc5a69 measured)                                                                                                                                                 | Green state (ec94fa9d8 and after)                                                                                           |
> | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
> | 4.9.4 import≠consumption   | Declaration surface project check a.yx only 1 W1001 (main's use b.{foo} + foo() saves a.yx same-name foo; guard only imported not used also saved)                                           | Same fixture 2 reported truthfully (collision doesn't save + unused import doesn't count as consumption); ref_pool 8 passed |
> | 4.9.5 test pool separation | src/util.yx's debug_print saved by tests/ reference (only 1)                                                                                                                                 | Same fixture 2 (test reference doesn't spill); ref_pool's test_ref_pool_excludes_test_role_references passed                |
> | 4.9.6 method reachability  | Self-built fixture (2 disambiguation overload call groups + 1 uncalled) 3 W1005 false positives; method_call_args.yx 3 / method_overload.yx 1 (overload按 full name coverage only reports 1) | Same fixture reports precisely uncalled orphan; method_unused 3 pins passed; corpus call_args 3→0 / overload 1→0            |
>
> **Fix round (F1) tightens again**: unresolved resolution's sibling overload restores W1005 report
> (b8380e6ab——same fixture call ->Float reports ->String sibling, call ->String reports ->Float
> sibling, bidirectional line number pins). **Fallback boundary (register #464)**: cross-unit
> (registry parse) method call doesn't produce resolution entry, falls back to name-level exemption,
> cross-unit same-name same-order previously not reported——register 4.9.7.
>
> **III. Process events and lessons (four hard requirements correspond)**
>
> 1. **PA-1 changed from blocker to existing capability gap (medium)**: round 2 review once
>    registered "cross-unit same-name same-order not reported" as blocker (PA-1); after two fact
>    reconciliations (implementer archive + verifier dual binary re-measurement of same fixture set
>    on b8380e6ab/b940300ac, diagnostic set byte-by-byte consistent) confirmed it's **capability gap
>    existing before round 2** not this round's regression——mechanism = cross-unit method call
>    doesn't produce resolution entry, falls back to name-level exemption; disposition = change to
>    medium, register issue #464, WBS 4.9.7 schedule next round.
> 2. **Reviewer publicly corrected before data and withdrew regression judgment**: reviewer
>    originally cited before data ("−42 includes called methods") inconsistent with its original
>    output (W1005 count=0)——verifier confirmed before capture is 0 after checking, reviewer's
>    regression judgment **publicly withdrawn** based on this; −42's correct caliber = 26 files'
>    called method false positive eliminated (Phase B), traceable item by item to behavior-run
>    exit=0 files (method really called and executed).
> 3. **t13 terminated due to contract premise not holding (not implementation defect)**: that task's
>    acceptance premise was "cross-unit same-name same-order would report W1005 regression fix", but
>    measured (t14 dual binary) confirmed this behavior is registered gap (#464) not
>    regression——contract premise doesn't exist, task terminated; its probe value taken over by t16
>    read-only judgment (confirmed `strip_unit_prefix` is dead branch).
> 4. **Document commit's real criterion is "really build once"**: round 2 closure's doc commit
>    (`042c07860`) made Deploy Docs red in VitePress's Vue template compilation——angle bracket
>    placeholders like `<rev>` / `<before-exe>` / `<after-exe>` in records not in backticks, parsed
>    as elements (report `Element is missing end tag`, pointing to `<rev>` line); markdownlint can't
>    catch this kind, and pre-commit's markdownlint hook in commits only containing docs is judged
>    `no files to check` skipped (measured). Disposition: placeholders all in backticks, before push
>    locally run `npm run docs:build` once (fix commit `0c837e1b4` with local build exit=0
>    evidence); if using `--no-verify` commit, commit-msg check needs manual补 run.
> 5. **Contract description not synchronized process omission**: t14's task description
>    text残留 PA-1 old hypothesis (writes "b.yx reports W1005" expectation), verifier in report
>    actively pointed out "per contract field judgment, description text doesn't count" and asked
>    captain to confirm——disposition = use task contract field as judgment basis, description text
>    left as process lesson (**when re-dispatching/dispatching, task description must be
>    synchronized with latest decision chain**).
>
> **IV. Mutation tests (mechanically reversible, all scratch.py trial)**
>
> Round 2 verifier total 9 patches (m1b/m2b/m4b are F2 post-compilable redo version——initial version
> M1/M2/M4 commented out if condition line leaving hanging block, t8 review F2 judged "compile
> failure mis-recorded as criterion turning red" and voided; redo version per patch cargo check
> exit=0 + trial turning red). M3 (collision attribution callback) initial version is true mutation,
> measured ref_pool 5 criteria turn red, kept original judgment. M1b anchor re-taken on b8380e6ab
> (F1 fix后 probe section refactor, old anchor invalid). All trials: HEAD blob → mutated blob →
> restored blob consistent with HEAD + git status CLEAN. Environment injection follows trial_env.ps1
> (core.autocrlf=false via GIT_CONFIG_*).
>
> **V. Warning surface full dual-拍 (caliber first)**
>
> Instrument = warn_surface_r2.py (self-built parser, (file, code, line, col) quadruple symmetric
> diff, project mode channel cwd=file parent directory, full path key); command =
> `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>` (337 files × 2
> binaries). **Caliber**: this round's before capture already includes all fixtures ⇒ matched set =
> full set 337, doesn't exist round 1's +8 (matched increment) / +16 (full set) dual caliber issue.
> Result: only_before = 42 W1005 (26 files, all called method false positive elimination = Phase B's
> −42; per file check all in behavior-run exit=0 baseline, method really called and executed——**no
> one real dead code saved**) / only_after = 3 W1001 (ref-pool-import-use's a.yx:4 check surface
> ×2 + ref-pool-test-pool's ../src/util.yx:2 = Phase A criterion surface). **Snapshot generation
> (F4)**: Phase A time point's after capture (Phase B pre 57 residual) and final version (Phase B
> post 15+2=17) are **two generations**, must note when citing; also 1 case exception
> (interface_member_conflict.yx's Dog.name@18) is field access form, not in Phase B's W1005 removal
> set, consistent before and after.
>
> **VI. Final gate (closure revision b940300ac)**
>
> lib 2761 passed / integration 288 passed / IR snapshot zero drift (snapshot 7 passed) / corpus
> baseline zero drift (byte count三方 comparable: baseline = b8380e6ab = b940300ac) / clippy -D
> warnings 0 / docs-truth exit=0 / check_tracking exit=0 / generator already run no product change.
> **Gate check arrangement (rule 13)**: this batch push's CI / Deploy Docs / Auto Translate run
> conclusions registered by timed check环节 (run id + conclusion + push coverage relationship: later
> run covers previous), not waiting in session.
>
> **VII. Batch tree state**
>
> Both yx49 and yx49-b released (gc unified delete at closure); no this batch new directories
> outside warehouse.
>
> **VIII. Leftover observation handed over to next round (registered, not changed)**:
> `resolve_reference` only precisely matches by reference start point——when cursor lands in middle
> of identifier jump/hover/rename silently no action (D55 precise-only current state, whole file
> form same), if tolerance matching is needed it's an independent behavior change, report to user
> for decision.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 3 Closure (4.9.7)

> **4.9 Round 3 Batch Record (2026-10-17 implemented → 2026-10-17 closed)**: Covers 4.9.7
> (cross-unit method call resolution registration and receiver attribution, issue `#464`——WBS 4.9's
> last three-level task; after this round's closure 4.9's entire secondary task kanban checked).
> Chain = production side t19 → independent verification t20 PASS → review t21 needs_revision (F1:
> injection surface false positive) → fix round t24 → independent verification t25 PASS → review t26
> PASS (incremental, verdict=pass). All commit numbers are post-push truth (judgment method = patch
> blob same; `git diff <rev>~1 <rev> | git hash-object --stdin` both sides consistent).
>
> **I. Decision chain and commits (post-push truth)**
>
> | Step | Content                                                                                                                                                                                              | Post-push commit | Judgment method        |
> | ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ---------------------- |
> | 1    | Production side: cross-unit method call produces D20 resolution entry (`feat(typecheck)`, t19)                                                                                                       | `b8b3cc3f6`      | Patch blob same        |
> | 2    | Driver-side attribution: method reachability by definition unit gives mark, resolution-recorded name group disables pool's name-level exemption (t20 verification PASS / t21 review catches F1)      | `d5c00235a`      | Patch blob same        |
> | 3    | Injection surface fix round F1: candidate injection per "name ← providing module" **pairwise** filter, eliminate E1107 false positive (t24, including F2 statement and F3 landing form single point) | `d5dd1a37b`      | Patch blob same        |
> | 4    | Pin test relocation: `src/frontend/core/typecheck/tests/environment.rs`'s F3 pin moved to declaration surface `src/driver/tests/overload_injection.rs`                                               | `c2f887843`      | Patch blob same        |
> | 5    | dev tip                                                                                                                                                                                              | `e9675c9dd`      | merge commit, see "II" |
>
> **II. Hash mapping and two merges at dev tip (source not confirmed, don't write as member's
> action)**: Round 3's four commits (above table 1–4) all pushed to origin/dev;
> `git worktree list`'s main mount at `e9675c9dd`. dev tip has two **non-team command surface
> produced** merge commits (`5da05b885` / `e9675c9dd`, reflog shows as `pull --tags` + push form),
> source not confirmed——per rule 10④ (reference must be resolvable by anyone) and rule 13
> (gate交给 dedicated check) same closure caliber, here only register fact and reflog evidence, no
> member attribution; also see "V." lesson ② (push attribution convention must be verifiable).
>
> **III. D20 contract extension actual form (key form and consumption point)**: Production side
> packs "candidate / hybrid IR name / declaration-phase param name" three parallel tables **same
> order** into `ModuleInfo.method_overloads` (`src/frontend/module/mod.rs:157`) with unit crossing,
> registry merges deterministically by module key ascending (`src/frontend/module/registry.rs:150`);
> typecheck arm injects per this unit's import face pairwise (`src/driver/arms.rs:354` →
> `visible_method_overloads`, `src/driver/arms.rs:1521-1579`: name ← providing module pairwise,
> same-name conflict full name skip, own key position not externally merged), landing form single
> point `append_overload_group` (`src/frontend/core/typecheck/environment.rs:399`: three tables same
> order append + equal length debug assertion); consumption end attributes D20 resolution product by
> **definition unit** (`MethodMarks`, `src/driver/helpers.rs:569-573`; `typed_owners` `:591`;
> `resolved_method_marks` `:621`), and lets resolution-recorded name group disable pool's name-level
> exemption (`src/frontend/core/typecheck/passes/dead_code.rs:252-267`, `:825-841`). Key form =
> entry name `{Type}.{method}#k` (`{Type}` is definition-side bare name; `#k` declaration order).
>
> **IV. Key scope and false positive defense measured (fix round F1 and five residual boundaries)**:
> F1's red state is pre-fix `d5c00235a`'s injection surface "name set × module set" combination
> merging another module's same-name type candidate into same key position ⇒ `use a.{Temp}` +
> `use b.{Other}` (a also defines Other, b also defines Temp, method same name same order) fixture
> `check` reports 2×E1107, exit 1 (previously successful program compile failure); after fix
> `check`/`run` both exit 0, 4 W1005 truthfully report both sides' **uncalled** same-name
> definitions (reverse false negative surface also checked). Warehouse fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`; driver-level pin
> `src/driver/tests/overload_injection.rs` 3 + `src/driver/tests/method_unused.rs` round 2's 5 #464
> criteria and 4 #450 old criteria all green; main acceptance fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` diagnostic set before and after fix
> item-by-item consistent (not reverted). Five residual boundaries and item-by-item measurement
> (single `check` definition file not report / name layer ambiguity E5006 full name skip /
> boundary(c) per t20 measurement rewrite / inline alias and `ns.T` not inject candidate surface /
> whole import `use M` and std candidate surface not produce entry) full text see
> [029f](../../rfc/accepted/029f-target-semantics.md) §cross-unit method reachability attribution
> "residual boundary", not repeated here.
>
> **V. Process events and lessons (four types continuation)**:
>
> 1. **Contract wording written too narrow directly becomes defect**: F1's root cause partly is
>    contract only wrote "only inject module surface that can be named by bare type name", didn't
>    write "per name pair attribute"——implementation literally took "name set × module set"
>    combination, review only caught E1107 false positive. When contract describes behavior boundary
>    must write **combinatorics** dead (per name pair / full combination), can't leave to
>    implementer's understanding.
> 2. **Convention must be verifiable: "push by captain" only in dispatch message ⇒ third party
>    push/merge can't be attributed**. This round dev tip had two source-unconfirmed merges (see
>    "II"), finally relied on reflog reverse-推 and withdrew one wrong attribution. Verifiable form
>    = member doesn't run `git push`/`fetch`/`pull`, sync all `pull --rebase`, use reflog to check.
> 3. **Infrastructure unstable, land patch first then change person**: This batch total 5
>    unrecoverable faults (3×TRANSPORT, 1×INVALID_REQUEST (thinking protocol), 1×background
>    sub-agent no message exit); handling convention = already produced evidence and patches first
>    land to **workspace** batch directory (`target/`下, not in repository), but **conclusion and
>    numbers must be locally landed in this record**——workspace path cannot be used as document
>    reference source (AGENTS meta-rule ①). Then change person to continue, same session consecutive
>    two failures don't bet third.
> 4. **Document commit's real criterion is "really build locally once"**: Round 2 closure's doc
>    commit (`042c07860`) made Deploy Docs red in VitePress's Vue template compilation (angle
>    bracket placeholders `<rev>` etc. not in backticks parsed as elements, report
>    `Element is missing end tag`); markdownlint can't catch this kind, and pre-commit's
>    markdownlint hook in commits only containing docs is judged `no files to check` skipped
>    (measured twice). Fix commit `0c837e1b4` with local build exit=0 evidence. This round's lesson
>    red→green run: Deploy Docs **39011946793** (red) → **39014696984** (green, see "VI").
>
> **VI. Round 2 gate conclusion补 registration (rule 13, timed check环节 backfill)**: Round 2
> closure commit `b940300ac`'s push——CI **39011946778** success / Auto Translate **39011946796**
> success / Deploy Docs **39011946793** **failure** (VitePress build red, i.e. "V.4" angle bracket
> placeholders; already fixed by `0c837e1b4`); fix commit `0c837e1b4` push后 three green: CI
> **39014697000** / Auto Translate **39014697044** / Deploy Docs **39014696984**. 连 push coverage
> relationship: CI@`0c837e1b4`'s **39014356642** cancelled by subsequent run per concurrency
> (cancellation≠failure, later run covers previous, consistent with round 2 §VI caliber).
>
> **VII. Final gate (closure revision `e9675c9dd`, local numbers; CI conclusion交给 timed check, see
> "VIII")**: lib 2770 passed / integration 289 passed / yx_multifile_runner 1 passed / IR snapshot 7
> passed zero drift / clippy `-D warnings` 0 / docs-truth 0 / generator idempotent (re-run
> `scripts/docs/gen-error-code-docs.py`后 `git status` clean); corpus baseline 346 → 347 → 348 lines
> (each +1 behavior record, other lines byte-by-byte unchanged; baseline
> `tests/baselines/corpus-parity.jsonl` at `e9675c9dd` total 348 lines).
>
> **VIII. Gate check arrangement and conclusion (rule 13)**: This batch push后's CI / Auto Translate
> / Deploy Docs conclusion交给 timed check环节 registration (not waiting in session, also don't
> write "didn't wait" as "already green"). **Backfill (2026-10-17 timed check)**: `d5c00235a`'s CI
> `39023417670` = **failure**——Docs Truth reports "error code documentation lags behind code
> table/locales", i.e. this commit missed generator产物 same commit; fixed by its successor
> `d5dd1a37b` (regenerate产物): `5da05b885` CI `39027788246` / Deploy Docs `39027788268` / Auto
> Translate `39027788340` all success, covers previous; `e9675c9dd` CI `39028827871` / Deploy Docs
> `39028827896` success; `d0cfc4e62` CI `39033963240` / Deploy Docs `39033963278` / Auto Translate
> `39033963261` success. Three commits' runs each ran to terminal state, this round no concurrency
> cancellation. Lesson: **"local generator idempotent" doesn't equal "产物 already same commit in
> repository"**——before closure should check CI conclusion per commit, not just look at latest.
>
> **IX. Batch tree state (closure gc, status output)**: Before closure `scratch.py status`——t25
> (holder=-) and yx49b (holder=-, previously held by retired member verify-r3, captain
> `release --force` released) two trees both no holder; `gc` deletes two, releases about 12.7 GB;
> gc后 `status` = "no batch tree", `git worktree list` only main mount left. Also: `yx49r3` and
> `yx49r4` are not batch trees (no meta.json), are members' evidence/probe **workspace**
> directories, not "residual batch trees". No this batch new directories outside warehouse.

## P4 4.4 Proof Layer and wasm Wrap-up — Closure (4.4.1 / 4.4.2 / 4.4.3)

> **I. Batch composition and rounds (batch name `yx44`)**: Three three-level tasks 4.4.1 / 4.4.2 /
> 4.4.3 with wasm attribute reachability judgment same batch; batch verification tree built by
> `scripts/dev/scratch.py init` in workspace `target/` (first full build; deleted after closure gc).
> Roles and rounds: implementation 4.4.2 = t1 (12 files; local `1fc135437` → post-push
> `24028ffe8`) ｜ implementation 4.4.1 layer order rewrite = t2 (only doc/comment, zero behavior
> change, based on `b5e68b3c9`) ｜ implementation 4.4.3 = t3 (11 files, including new code W1082;
> local `2d5e610cd` → post-push `52d2276ca`) ｜ wasm reachability judgment = t4 (only judgment, zero
> cleanup, judgment revision `b5e68b3c9`) ｜ 4.4.1 closure and one doc commit = t8 (7 files
> 173+/43-; local `29ebd2145` → post-push `d07993157`) ｜ independent verification = t5 (freeze
> `29ebd2145`) ｜ round 1 review = t6 (revision `1b593e993`) ｜ 补 port = t9 (3 files 9+/10-; local
> `1ebddaf95` → post-push `1b593e993`) ｜ fix round = t10 (3 files 23+/23-; local `657c3d5e3` →
> post-push `e184d88b6`) ｜ round 2 review = t11 (`657c3d5e3` = post-push `e184d88b6`, patch-id
> fully equal) ｜ closure = t7 (check / record / batch tree gc / doc real build / one doc commit;
> local `c8412565c` → post-push `da724940b`). **Failure and redo**: t6 ended with `needs_revision`
> failure (review round per contract must fail and give findings, not accident); t7 first round
> interrupted at batch tree gc stage, completed product left in main warehouse work tree
> uncommitted, second round took over "补 gap + re-review"——didn't redo completed part (this round
> added: 4.4 check and evidence note, this record, three places pointing to deleted batch directory
> path rewrite, doc real build and closure commit). No cancelled rounds.
>
> **II. Per-task output and measurement numbers (original commands and output see evidence summary
> and archive; only conclusions recorded here)**:
>
> - **4.4.1**: `layers/README.md` (including `layers/mod.rs` top comment)'s layer order table
>   rewritten to **actual call order** (predicate → termination → ownership, call point anchored at
>   `checker.rs::check_module_impl`), `equivalence` described per actual identity (no production
>   call point); same-source declarations 7 closed (`01-routing` / `02-stage-contract` / RFC-039 /
>   `06` / typecheck README / layers two files); mechanical reconciliation script on batch tree
>   green baseline **11/11 PASS** (criterion 11 = README side 4: actual call order declaration / two
>   entry anchors / old order declaration disappear; code side 7: 5 line anchors +
>   `check_module_impl` no short circuit + `equivalence` zero production call; reconciliation script
>   is one-time script, recycled with workspace, criterion 11 item-by-item listed can be rewritten
>   based on this), mutation (c) revert old declaration → reconciliation 2 FAIL i.e. red.
> - **4.4.2**: Solver acquisition side unified to `proof/smt/backend.rs`'s process-level shared
>   singleton + sole closure port `with_shared_solver(consumer, f)`; three consumption points
>   (predicate / termination / ownership) all switched to shared port, **compilation production
>   chain `default_solver()` call point = 0** (sole backend selection point sunk to private
>   `new_backend()`). `wasm/src/lib.rs`'s `test_smt` self-check call point is **existing** (earlier
>   than this batch, since `b5e68b3c9`) non-test call point, is "test and independent scenario"
>   surface, doesn't constitute D58 prohibition violation. Independent verification self-built
>   reading (one compilation): predicate acq=2 hit=1 miss=1 ｜ termination acq=1 hit=2
>   miss=2 ｜ ownership acq=2 hit=1 miss=1, three points hit/miss sum (4,4) = shared instance
>   cumulative `Some((4,4))`; mutation (a) singleton degradation → predicate hit=0 i.e. red.
>   **Modification surface disclosure**: this commit contains `z3_backend.rs` 15
>   lines——`cache_stats` moved from inherent method to `Solver` trait block (returns
>   `Some(instance internal count)`) + 1 line comment change pointer, no semantic coupling. D58
>   prohibition已 mechanism-ized: same-thread re-entry → panic contains D58, probe
>   `recv_timeout(120s)` not hanging.
> - **4.4.3**: `body_checker` missing silent downgrade converged to `TypeChecker::ownership_inputs`
>   (sole criterion point) + new code **W1082** (not overlapping with existing W code facts: W1080
>   constraint downgrade / W1081 solver missing / W1090 opposite direction); pin test 2/2 green;
>   healthy path corpus 348:348 zero diff; **downgrade branch production surface unreachable
>   enumeration holds** (CLI four entries + eval/repl + format + LSP + wasm playground + all
>   `ProgramKind` converge to `check_module_impl`: `init_body_checker` unconditionally before
>   ownership block, function内 0 return/break/continue, `body_checker` no `.take()`) ⇒ W1082 no
>   user-visible surface. Mutation (b) delete emit → pin test i.e. red. **W1082 is external contract
>   table change (E class), user-ratified**; `locales/zh.json` / `locales/en.json` same commit with
>   code, other languages由 i18n bot push后补齐 (run see "IX").
> - **wasm attribute reachability judgment (t4)**: `orchestrator.rs` current tree **10 = 4 positive
>   arms + 6 negative arms** (caliber = same line `#[cfg(...)]` attribute text, positive and
>   negative both count; `cfg!` runtime macro 0 hit); 4 wasm positive arms unreachable in playground
>   runtime but **compile-time load-bearing** (call point not gated, separately delete arm ⇒ wasm
>   compile failure), 6 `not(wasm32)` negative arms native three-stage table load-bearing ⇒ D35
>   envisioned "delete unreachable" **has no applicable target** in this file, real cleanup form =
>   whole project-mode subtree gated on wasm32 (registered P10 S7, three unverified original
>   preserved).
>
> **III. Independent verification (t5, freeze `29ebd2145` = post-push `d07993157` content——two
> revisions per commit patch-id fully equal, whole tree difference only in `docs/src/en/**`,
> `docs/src/.i18n-cache.json` etc. bot product surface——`git range-diff` three commits all `=`)**
> Gate **8/8 all green**——`cargo fmt --check` 0 ｜
> `clippy --all --all-features --all-targets -- -D warnings` 0 ｜ `cargo test` **3128 passed / 0
> failed** (7 test targets total; baseline 3126 + this batch 2) ｜ corpus difference **348:348** per
> line zero difference (sort diff 0 lines) ｜ IR snapshot `UPDATE_SNAPSHOTS=1` re-run后 `git status`
> zero drift ｜ `check-obligations.py` 21 = 13 + 8, containment 0 gap ｜ `check-docs-truth.py`
> source 156 / doc 156, CLI 21 command consistent ｜ error code generator re-run前后 tree both
> clean. Build warning only 4 bot language W1082 missing entries (known state) + Z3 download noise,
> zero code warning. **Contract verify caliber correction (must two JSONL paths)**: Corpus gate call
> caliber =
> `python scripts/ci/check-corpus-parity.py tests/baselines/corpus-parity.jsonl <this generation>.jsonl`——**two
> JSONL paths both needed**; missing second parameter is comparing baseline with baseline (always
> zero diff, criterion idle). This batch executed and evidenced per two-path caliber (t5's
> `corpus-r44.jsonl` product).
>
> **IV. Mutation evidence (3/3, mechanically reversible)**: All via
> `python scripts/dev/scratch.py trial` (`git apply` → run command → `git apply -R` → per file blob
> vs HEAD → require `git status` clean):
>
> - (a) Singleton degradation → shared reading predicate hit=0 miss=2, test FAILED exit 101;
>   `backend.rs` HEAD=`3e2b2d150b24` → mutation `6d00bad4d4aa` → restore = HEAD.
> - (b) Delete W1082 emit → pin test FAILED exit 101; `checker.rs` HEAD=`70c04a41e020` → mutation
>   `db0a623c9b9a` → restore = HEAD.
> - (c) Revert old layer order declaration → reconciliation 2 FAIL exit 1; `layers/README.md`
>   HEAD=`8b1ae875db32` → mutation `e88f9af6c42d` → restore = HEAD. Three trials restored后
>   `git status` all CLEAN. Trial (c) first run failed because absolute path parsed by WSL failure
>   exit 127 (not criterion red), changed to relative path re-run as standard.
>
> **V. Review rounds and conclusions**: Round 1 (t6, revision `1b593e993`) **verdict =
> needs_revision**——3 findings all document line anchor inaccuracy (F-01 medium: `02` §"wasm status"
> 4 groups expired line numbers; F-02 low: `02` and `09`'s 4.4.3 delivery anchor
> `checker.rs:1283-1293`; F-03 low: `06`'s对照 statement), four substantial deliveries all pass. →
> Fix round (t10, local `657c3d5e3` = post-push `e184d88b6`, 3 files 23+/23-, with two mechanical
> criteria) → Round 2 (t11, same revision) **verdict = pass, new finding = 0**. **Final check basis
> = round 2's pass**. **Round 3 (closure tip `da724940b`上)**: Independent verification t12
> **verdict = pass** (bind `da724940b`: four rule texts byte-by-byte, 3.19 / S7 / S8 fields and
> scope ready, count table and decision chain recalc, dead reference zero, `check-docs-truth` 0,
> `markdownlint` 0); review t13 **verdict = needs_revision**, 4 findings——F-01 medium (t10 / round 2
> only gave rebase pre-hash, §XIV's list and origin/dev actual don't match), F-02 low (`02:379`'s 09
> 4.2.8 anchor被 this batch itself in `09:144`/`:182` each insert one line pushed to `:274`), F-03
> low (`09a` write local drive letter path), F-04 low (three-level count row note and 157 ≠ table
> header 156)→ fix round t14 (this commit) closed item by item. **t10 contract caliber decision**:
> Fix surface based on **task record's 5 F-01…F-05** as standard (F-04/F-05 are mechanical
> criteria), dispatch brief is not criterion.
>
> **VI. User decision (2026-10-18) and rule solidification**:
>
> 1. **D58 port name changed in place**: `with_shared_solver(f)` →
>    `with_shared_solver(consumer: SolverConsumer, f)`, in-place correct text, **no revision note**
>    (t8 completed in 4.4.1 one commit).
> 2. **wasm judgment registered as P10 S7** (decision B): with "belonging phase: P10; whether blocks
>    mainline: no" + three unverified **original preserved**; `06`'s F2 "needs decision" line change
>    to that registration; t4's "### F2 judgment" subsection not touched.
> 3. **Crash item registered as P3 3.19** (decision A): symptom / cause points to
>    `proof/smt/translate.rs`'s sort inference / D59 **A/B class** (crash) / acceptance row (this
>    form no longer abort——conservative downgrade or diagnostic, corpus and IR snapshot zero drift);
>    **characterization = pre-existing** (t5 two revision comparison, see "VII" ⑤).
> 4. **Rule solidification one** "correction written in place to text, no old-new comparison": fall
>    `coding-rules.md` (added clause), `07-equivalence-oracle.md` §semantic correction routing
>    criterion rule 3, RFC-039 **D59 row** rule 3 (same commit registration = WBS row annotate error
>    type + in-place correct affected text + PR required field "error type" one line).
> 5. **Rule solidification two** "contract `inScope` always write path prefix resolvable by path
>    matcher": fall `AGENTS.md` team orchestration convention rule 6.
> 6. **New code W1082 ratified** (external contract table change, E class).
> 7. **Register P10 S8** "document correction form convergence (in-place rewrite, delete revision
>    note)": include 7 audit list scope and acceptance row.
> 8. `09` top count table: secondary **63** (60 + 3.19 / S7 / S8), three-level **157** (154 + 3),
>    with 2026-10-18 note; three-level item accounting review see "XV".
>
> **VII. Process facts that must be truthfully recorded**: ① 4.4.2 commit used
> `SKIP=cargo-fix,cargo-clippy-fix` to skip two `--fix` hooks——shared work tree had teammate's
> in-progress product, `--fix` would modify other's files; **equivalent gate already fully run
> manually in final tree** (fmt --check / clippy `-D warnings` both 0). ② `pre-commit`'s stash
> window indeed happened (`Stashing unstaged files…` / `Restored changes from…`), commit后
> `git status` check shows teammate's in-progress product intact. ③ **4.4.3's inScope lesson**:
> prose qualification `docs/src/（only error code产物）` rejected by `changedPaths` check ⇒
> `inScope` always write matchable path prefix (already solidified as rule, see "VI" 5). This task
> **actually 11 files**, of which 7 not listed in `changedPaths` because declaration写法 cannot be
> resolved by path matcher, item-by-item listed in task result (`codes/w1xxx.rs`, RFC-013 code
> table, `reference/error-code/`'s index and each E segment page, etc.). ④ **W1082's locale window
> closed**: `locales/zh.json` / `locales/en.json` same commit with code, other languages由 i18n bot
> push后补齐 (run see "IX"). ⑤ **Z3 `Sort mismatch` crash judgment (t5 criterion ⑦)**:
> **pre-existing**——same probe, same input on `b5e68b3c9` and freeze revision `29ebd2145` stdout
> **byte-by-byte consistent**
> (`Error: Sort mismatch at argument #1 for function (declare-fun not (Bool) Bool) supplied sort is Int`),
> both exit 1; static attribution consistent (4.4.2 0 line change to `translate.rs`, only
> `cache_stats` migration to `z3_backend.rs`). **Downgrade branch production surface unreachable
> enumeration holds** (W1082 no user-visible surface). Probe three-line source:
> `NonNegative: (x: Int) -> Type = { x >= 0 }` / `AtMost: (n: Int, x: Int) -> Type = { x <= n }` /
> `f: (x: NonNegative(x)) -> AtMost(x, x) = { return x }`. ⑥ **t5 freeze surface and t9's coverage
> difference**: t9 (three place comment/doc补 port, `1b593e993`) committed later than t5's freeze
> revision `29ebd2145`, is zero behavior change, its acceptance由 t9's own mechanical check + t6
> review cover (t6 on `1b593e993` **re-ran** t5's key measurement, not retold). ⑦ `docs/src/en/**`
> not manually modified, regenerated by push后's auto-translate workflow (this batch bot landed two
> commits: `92d490695` i18n locales补 W1082 other languages, `63b6cdf37` auto-translate docs). ⑧
> **t9 out-of-scope observation three decisions**: ㈠ t9 in passing corrected `02` same sentence内
> `wasm/src/lib.rs` line count and gate point line number (73 / 139·153·170 → 152 /
> 151·170·187)——**accept** (same caliber same number required surface, commit
> message已 listed); ㈡ RFC-039 **D35 row** still writes "wasm 27 files' cfg branches"——**keep
> unchanged** (decision row is historical decision record, its premise correction already carried by
> P10 S7 registration, no rewrite history); ㈢ `docs/src/en/**`'s old port name
> translation由 auto-translate aligned. ⑨ `09` / `09a` originally pointing to deleted batch
> directory's two places path changed to **inline in place** in closure round (evidence summary
> merged into this record this section, probe three-line source inline). **2026-10-21 correction**:
> At that time the change pointer was still workspace path, equivalent to moving violation down one
> level; from this round document no longer references any workspace path (AGENTS meta-rule ① / rule
> 10④).
>
> **VIII. t8 acceptance ④ caliber decision**: Acceptance original required
> `git diff | grep '修订注记\|原为\|现改为'` empty. Measured **default context caliber
> unreachable**——D58 line adjacent line D59 / D61 decision rows themselves contain "修订注记 / 原为"
> text (not this task's change). Decision: accept per **new line / delete line / `-U0` increment
> line** three calibers all empty, and write **subsequent similar acceptance only grep new/delete
> line**——criterion should land on this change's line, not whole diff's context.
>
> **IX. CI gate conclusion (AGENTS rule 13①, timed check 2026-10-18 17:26 backfill; run id and
> conclusion inline)**: First push (3 commits, tip = `d07993157`): CI **39110908163**
> success ｜ Deploy Docs **39110908155** success ｜ Auto Translate Docs **39110908193**
> success ｜ Auto Translate I18n **39110908144** success; this tip covers `24028ffe8` (4.4.2) /
> `52d2276ca` (4.4.3) / `d07993157` (4.4.1) three commits全部 content. Second push (1 commit, tip =
> `1b593e993`): CI **39112293391** success ｜ Deploy Docs **39112293062** success ｜ Auto Translate
> Docs **39112292903** success; covers all four including t9补 port. **No concurrency
> cancellation**, latter covers former. Third push (2 commits, tip = `da724940b`): CI
> **39121538555** success ｜ Deploy Docs **39121538562** success ｜ Auto Translate Docs
> **39121538541** success——three runs' headSha all = `da724940b` and **all已 terminal state**
> (2026-10-18 third round timed check backfill); round 3 independent verification t12 then记 as "CI
> not terminal" observation closed. Unrelated: Nightly **39081392694** failure's headSha =
> `290f9ebc`, not in this batch's commit surface.
>
> **X. Three `pull --rebase` (during push auto-commits from other side)**: Other side respectively
> auto-translate and i18n commits; six hashes rewritten and per **patch-id fully equal**——local
> `1fc135437` → `24028ffe8`, `2d5e610cd` → `52d2276ca`, `29ebd2145` → `d07993157`, `1ebddaf95` →
> `1b593e993`, `657c3d5e3` → `e184d88b6`, `c8412565c` → `da724940b` (third rebase后 origin/dev head
> = `da724940b`); `git range-diff` verified **per commit content unchanged** (three all `=`). Member
> didn't run `git push` / `fetch` / `pull`, sync and push由 captain统一 do. **Forensics reminder**:
> thereafter reference commit number always use **post-push hash** (above right value); `09`'s check
> row per convention not write commit number.
>
> **XI. Uncovered surface (must not write "not tested" as "passed")**: ① doc-tests 0/0 not bearing
> criterion; ② **wasm32 target actual compile not done** (this surface由 CI's `_build-wasm.yml`
> cover, this batch's wasm judgment is static argument); ③ mutation only covers contract-named 3
> places, other gates not reverse-broken verified; ④ corpus difference not单独 run on `b5e68b3c9`
> (only on freeze revision); ⑤ i18n bot补齐's other languages W1082 text behavior not verified (only
> checked `zh.json` / `en.json`); ⑥ round 1 review and fix round reported **same-class but not in
> contract findings' existing drift** not fixed (list see "XII"), attribution pending user decision.
>
> **XII. `02` whole article existing drift observation list (user 2026-10-18 agreed to register;
> 2026-10-21 in repository as S10)**: Caliber = all **pre-existing drift** (not introduced by this
> batch), **not constitute this batch's finding**; list由 fix round found, round 2 review in
> `657c3d5e3` (= post-push `e184d88b6`)上 independently抽验 and gave mapping, item-by-item copied
> (list inline as below, original files recycled with workspace):
>
> 1. `02` §(a) entry table 3a-3d / 5 / 5b: `02:72` references `orchestrator.rs:273-399` → current
>    `:155`; `02:73` `:450` → `:250`; `02:74` `:1374` → `:985`; `02:78` `util/diagnostic/mod.rs:565`
>    → `:681`; `lsp/handlers/diagnostics.rs:146`'s `run_diagnostics` already not at that place;
>    **content level**: `run_diagnostics` and "otherwise manual lex→parse" already replaced by 4.6.1
>    (delete manual sequence).
> 2. `02:1016`: `compile_project :99-237` → current `:107-134` (28 lines Program constructor).
> 3. `02:360-361`: "missing downgrade to warning" → current tree `_build-wasm.yml:50-51` is
>    `::error` + `exit 1` (#435); `build.rs:65-72` already changed `panic!` hard fail;
>    `_build-wasm.yml:35-59` interval needs re-take.
> 4. `02:127` mechanism pointer: mechanism actually in `src/driver/arms.rs::typecheck_orchestrated`
>    (`:288-289` self-stated); conclusion still holds (multi-file arm drops warnings), inaccurate is
>    "result's sole downstream use" sentence (`type_result` also via `arms.rs:1389` →
>    `helpers.rs:259/:636` feeds `overload_resolutions`); suggestion = pointer change to
>    `typecheck_orchestrated`, "sole downstream use" change to "warnings field no read point".
> 5. `02:128`: `check_project:385-395` / `250-252` needs re-take.
> 6. `02:334` (§S5 (d) investigation段): referenced old code snippet already replaced by 4.4.3's
>    `ownership_inputs` criterion point.
> 7. `02` change list table other rows: `predicate.rs:34-36`, `checker.rs:1303-1314` / `:1244`,
>    `backend.rs:67-72`, `wasm/src/lib.rs:30-36` / `:42-51`, `util/diagnostic/mod.rs:621-661`.
>    **This closure doesn't pre-set S9's WBS entry** (user didn't decide); after getting decision,
>    follow registration form fall `09`. **Subsequent (2026-10-21 in repository)**: User 2026-10-18
>    agreed to register, this list已 as `S10` "02 text and fact alignment (line anchor + content
>    level expired)" fall `09` (secondary +1 / three-level +1 = S10.1)——**originally拟 S9
>    number已被 "pre-release audit abstraction closure" occupied** (S9.1 = issue #473 / S9.2 = issue
>    #474), per 09 "number unique in whole document" registration convention顺延 as S10.
>    **Registration timing deviated from original plan** (originally wrote "with P5's closure doc
>    commit一并 in repository"), per user 2026-10-21指示 this batch即 in repository.
>
> **XIII. Batch tree recycling (gc and status original output; output's tree path per warehouse
> relative path记)**: `python scripts/dev/scratch.py gc` →
> `[scratch] 已删除 yx44-evidence（释放约 0.0 MB）` / `[scratch] gc 完成，共释放约 0.0 MB`;
> `python scripts/dev/scratch.py status` → `yx44-evidence            holder=-` next line
> `<workspace>/yx44/tree`——registry recorded path **doesn't exist** anymore, that name is registry
> residual (`gc`'s `_remove` deletes `meta["path"]`'s parent directory i.e. workspace下's `yx44`,
> residual's `meta.json` still falls in scan surface; its content batch=`yx44`,
> `base_rev=29ebd2145`, `lock=null`, containing t5 / t6 / t11's hold history); closure round renamed
> that registry file to `meta.retired.json` to make it exit scan (**don't delete any evidence file,
> don't create any new tree**) → `python scripts/dev/scratch.py status` = `[scratch] 没有批次树`;
> re-run `gc` = `[scratch] 没有批次树，无需回收`; `git worktree list` only main mount left (`dev`).
> Batch original evidence directory (no `meta.json`, **not batch tree**)当时 preserved as archive;
> **2026-10-21起 workspace directory no longer as evidence source**——closure summary已 inlined in
> this section, original directory随 cleanup deleted (AGENTS meta-rule ①). Acceptance statement
> (resolvable): `git status` clean; batch tree gc后 `python scripts/dev/scratch.py status` = no
> batch tree; `git worktree list` only main mount left; no this batch new directories outside
> warehouse.
>
> **XIV. Document real build and commit hygiene**: `npm run docs:build` local **exit 0** (AGENTS
> rule 10⑨——markdownlint can't catch VitePress's Vue compile error, and commits only containing docs
> will be judged `no files to check` skip markdownlint); this batch's check / note / register / rule
> modification攒成 **one** doc commit (rule 8), lands at batch closure; member didn't run `git push`
> / `fetch` / `pull`. **This batch's final two both已 pushed** (closure tip上
> `git rev-list --count origin/dev..HEAD` = 0, origin/dev head = `da724940b`): `e184d88b6` (round 1
> findings' doc anchor fix, 3 files 23+/23-) + `da724940b` (closure doc commit, 7 files 96+/10-;
> including check / record / three place path rewrite / registration and rule modification).
>
> **XV. Count table accounting review (round 3 fix F-04; method and result both verifiable)**:
> Judgment object = `09` top "positioning and scope" table's three-level count row. **Method** (two
> independent calibers, per revision use `git show <rev>:<file>` take bytes recalc): ① **Sum per
> secondary entry**——take each secondary kanban row括号内's "N three-level tasks" (row首
> `- [x]`/`- [ ]` bold title start, row内 contains "个三级任务", including S7 row内嵌 written
> "1 个三级任务"): current tree 70 secondary entries, **sum = 157**. ② **Historical
> self-consistency**——same caliber on `366bc5a69^` get 66 entries, **sum = 151 = period table header
> 151** (self-consistent); `366bc5a69` same day new register `4.9.6` and `0.4.1` two three-level
> (sum 151 → 153), but table header only writes 151 → 152 ⇒ **table header since that
> commit少记 1**; thereafter each revision all差 1 (`92d490695` / `1b593e993` / `d07993157` /
> `e184d88b6`: sum 154 vs table header 153; `da724940b`: sum 157 vs table header 156). **Result**:
> truth = **157**, therefore correct table header (`09:13` three-level 156 → 157); note
> 144+2+2+2+2+1+1+3 = 157本来 correct, not changed; this batch's previous base = **154** (not 153).
> **Secondary count row not moved**——that row's caliber never aligned with kanban entry count
> (`366bc5a69^`上 is 66 entries vs table header 59), this time no verifiable evidence to overturn
> 53 + 10 = 63, not凑数 change. Incidental correction (same round): §二's 4.4.1 bracketed
> decomposition per reconciliation script actual item count changed to "README side 4 + code side 7
> = 11", consistent with **11/11** and script 11 `ck` calls.

## P3 Debt Round 1 Closure (3.5 / 3.6 / 3.4.6 / 0.5)

> **I. Batch composition and rounds**: This batch closes P3 side remaining debt——**3.5 higher-order
> type parameter call path** (3.5.1–3.5.3), **3.6 function type value call path** (3.6.1–3.6.4),
> **3.4.6 compile path consumes warnings** (D class, user 2026-10-19 decision "do"),
> and一并 register **0.5 test suite layering** (P0, issue #486 slice) and **3.20 dead code analysis
> existing false positives** (3.20.1 / 3.20.2). Roles and rounds:
>
> - Implementation: 3.5.1–3.5.3 = t1 ｜ 3.6.1 = t2 ｜ 3.4.6 = t6 ｜ 3.6.2 = t7 ｜ 3.6.3 = t8 (main +
>   closing two commits) ｜ 3.6.4 = t9 ｜ 0.5.1 = t10 (#486 slice)
> - Independent verification: round 1 = t3 (freeze revision `81d5e414e`, covers 3.5 / 3.6.1–3.6.3 /
>   3.4.6) ｜ 3.6.4 = t11 (round 2, freeze `8e32e8636`)
> - Review: round 1 = t4 (verdict = **needs_revision**, F1/F2/F3) ｜ fix round = t12 (one
>   commit) ｜ round 2 = t13 (verdict = **pass**)
> - Closure = t5 (check / register / this record / batch tree gc / doc real build / one doc commit)
>
> **Failure and redo**: t4 ended with needs_revision failure (review round per contract must fail
> and give findings, not accident) → t12真 fix → t13 review pass. t8 internal one attempt
> interruption (process crash), previous round evidence preserved in batch directory, continuation
> attempt补齐后 two commits in repository. No cancelled rounds.
>
> **II. Per-task output and measurement numbers (conclusion caliber; original commands and output
> see each batch evidence directory)**:
>
> - **3.5.1** (unifier matches constructor variable in head position): Criterion converged to
>   **single implementation** `is_ctor_reference` (`pub(super)` free function, warehouse 1
>   definition 3 consumption)——this is t12's t4 F1真 fix (originally `inference/statements.rs` had
>   another same-name method copy, constituting prohibition three significance's same-name two
>   implementations). Head rename no longer inline copy: `environment.rs` and `expressions.rs` each
>   first traverse TypeRef / variable table construct head_map, then统一 to shared implementation
>   `substitute_ctor_heads` (`types/substitute.rs`) execute——two places head_map construction
>   criterion unchanged, rename only acts on head by name, empty table returned as-is (t12's
>   F2真 fix). Equivalence argument: traversal doesn't change Generic head name.
> - **3.5.2** (explicit type actual arg accepts type constructor, E1010 disappears): Red E1010 /
>   E1002, green exit 0.
> - **3.5.3** (mono product replaces constructor variable): Symbolic type actual arg criterion
>   narrowed by container generic parameter name; constructor variable in product replaced with real
>   constructor (IR dump empirical: specialization function `Function probe(Box, int64)`).
> - **3.6.1** (local function type value called by name): Criterion single source =
>   `is_static_fn_name` (warehouse 1 definition 1 call), aligned with variable evaluation path's
>   name resolution priority (local → global), didn't add second resolution table; IR empirical:
>   module initialization segment changed from `LoadConst String("f")` + `CallNative` to
>   `LoadGlobal` + `CallDyn`. Regression surface: same-name "function binding + value binding"
>   unreachable (always compile-time E2010), non-function value called by name still E1065.
> - **3.6.2** (`use` alias imported function value called by name): Originally reported
>   `E6006 Native function not found: lib.f` (`use lib.{f}` and `use lib.{f as g}` two forms). Fix =
>   in `is_static_fn_name`内 **add one arm** (not copy), new sole implementation
>   `use_alias_value_slot` undertakes "use alias → qualified name → global slot" two-table joint
>   query, consumption point 2 (call form criterion + variable evaluation path).
> - **3.6.3** (nested function body references outer frame value): **Criterion changed to
>   judgment**——not "let it run correct", but this form per RFC-009 §2.3 + SPEC §12.3 should be
>   **rejected at compile time** (D59 C class: change silent wrong value to loud; per `09` §P3's D59
>   row, "silently give wrong answer" is C class——this batch closure brief记 as "A/B class", both
>   point to "old behavior not successful", landing method both "red first then green + compile-time
>   loud reject"). Original form = **no error, calculate wrong value (void)**: red stdout `void`,
>   EXIT=0 (silent wrong value archive); green compile-time E1001 `Unknown variable`, EXIT=1, with
>   existing "unknown variable" path **same code same text**. Criterion single source = capture
>   barrier (`enter_capture_barrier`), compile-time name (type parameter) not blocked; E1001 still
>   only emitted in typecheck, registered sole. **Assignment path由 E2010兜住** (t8's closing
>   commit: declaration / assignment judgment doesn't apply capture barrier)——otherwise this fix
>   itself would open a new "same-name assignment silent wrong value" channel (c1 form: only
>   barrier时 degrade to exit 0 print 10).
> - **3.6.4** (`use lib` whole import后 `lib.f(...)`): Takes FieldAccess callee's namespace branch,
>   originally reported E6006 `lib.f`. Fix = namespace branch解 out qualified name后 first judge
>   global slot layout → switch to indirect call; qualified name resolution and slot query reuse
>   `resolve_namespace_data_access` (same source as data access), didn't add resolution table.
> - **3.4.6** (compile path consumes warnings, D class): Change前后 external behavior = **multi-file
>   `run` previously恒 didn't output any W code** (red archive: multi-file [] vs single-file
>   [(W1001,3),(W1002,5),(W1003,1)]), now four paths (single-file run / multi-file run / single-file
>   check / multi-file check)'s W code set恒 is {W1003@1, W1001@3, W1002@5} (non-empty set). Corpus
>   baseline: 348 records中 **46 changed**, new W code **118** (W1001 34 / W1002 12 / W1003 13 /
>   W1004 42 / W1005 17), **118/118 item-by-item attributed to real existing unused definitions in
>   file (0 not located)**, pure increment (0 deleted, 0 exit / stdout / check changed). **D59 four
>   promises fulfilled**: ①red first then green (red archive + turn pin test) ②not change
>   "originally successful program"'s external behavior (pure increment, 0 exit / stdout / check
>   changed) ③same commit register (this record + `09`'s 3.4.6 row) ④baseline same commit regenerate
>   and manual review diff. **Superset approved**: W1004 / W1005 / W1006一并 enter `run` (user
>   2026-10-19 approved), reason = **one criterion, one channel**——only putting W1001/W1002/W1003
>   equals creating second filter decision. **External contract surface change**:
>   `orchestrator::compile_project` return value由 `Result<ModuleIR, _>` changed to
>   `Result<(ModuleIR, FileDiagnostics), _>` (warehouse内 consumer 2 + 2 test direct drive points
>   synchronized adaptation).
> - **0.5.1** (test resource profile tag + exclusive lock + fast/full minimum consumption surface):
>   Added `scripts/dev/testsuite.py` (`markers --check` and `cargo test --list` bidirectional
>   reconciliation, `plan` budget bin packing, `run fast|full` budget insufficient即 report gap not
>   silently skip, `manual` tier never implicitly execute) and `src/util/test_support.rs`
>   (cross-process file lock, lock taken in test body ⇒ any call path互斥); in-place `cost:` tag 3
>   places (2600 tier exclusive, peak from 5000 tier measured linear extrapolation, **not
>   measured**; 5000 tier exclusive, 15.6 GiB / 72.6 s measured; snapshot update manual, peak and
>   duration both extrapolated). Self-verification numbers: `markers --check` checked 3149
>   `#[test]`, 3 mechanical mutations each exit 1, `run fast` batch wave 2793 passed / 47.7 s +
>   exclusive wave 1 passed / 14.6 s, budget 1 GiB时 `run full` listed gap 7.11 GiB, two-process
>   lock contention measured `held→blocked→held(waited 2.0s)`. **Independent verification belongs to
>   #486 follow-up scheduling** (this batch only implementer self-verification). **0.5.2 pending
>   scheduling** (pre-commit / CI wiring + 2600 tier measured calibration + `test-specification`
>   detailed rules).
>
> **III. Independent verification (t3, freeze `81d5e414e`; red baselines `c599de7bb` / `1725d0215` /
> `ef2334de5` all HEAD ancestors, `git merge-base --is-ancestor` verified)** Probes all verifier
> self-built (batch tree由 `scripts/dev/scratch.py init` built in workspace下, probes and patches in
> its `probes/` / `patches/` subdirectories), not retold implementer conclusions; this batch tree
> together with its in-tree evidence已 deleted in this batch closure's gc (see "IX"), therefore this
> section将 measurement conclusions and numbers **as-is landed in record** for evidence. 3.5 red
> state four probes all report E1003+E1002 / E1010, green state exit 0 and value correct (stdout 1 /
> 7 / 1), also has IR value-level evidence (specialization function `Function probe(Box, int64)`);
> 3.6.1 green state output 6 / 10, comparing "value position call" green, "non-function value called
> by name" compile-time E1065; 3.6.2 red state two forms E6006, green state exit 0 (probe内 `assert`
> failure即 non-0, exit 0即 value correct); 3.6.3 red state stdout `void` / EXIT=0 (silent wrong
> value verifiable), green state compile-time E1001 / EXIT=1, while **expression position inline
> lambda capturing this function parameter** allowed (EXIT=0, output 6)——allowed surface not误伤.
> 3.4.6's C2 normalization由 verifier self-built fixture independent reproduction (four paths W
> set恒等, non-empty), baseline difference independent recount (changed=46, added=0, removed=0,
> expectation / run.exit / run.stdout / check change = 0, new W code head 118 consistent with
> implementer), attribution sample 10 all hit real definition. Gate on freeze tree all green: fmt /
> clippy (`-D warnings`) / `cargo test --all` (**lib 2790 passed + 2 ignored, integration 297
> passed**, 0 failed) / corpus difference 348:348 difference empty / IR snapshot 204 zero drift /
> obligation ledger / docs-truth. **t11 (round 2, freeze `8e32e8636`)** independently reproduce
> 3.6.4's red→green (red parent commit 16e2a5c88: E6006 `lib.f`, EXIT=1; green EXIT=0, stdout 27,
> probe内 4 assertions all pass), criterion single source (`resolve_namespace_data_access` 1
> definition 2 consumption, no third "qualified name → slot" joint query), no regression 8 forms all
> pass, verdict = **pass**.
>
> **IV. Review rounds and conclusions**: Round 1 (t4, judge `20abd3ebc`'s parent revision) verdict =
> **needs_revision**, three findings all real defects: F1 = `is_ctor_reference` same-name two copies
> (`inference/statements.rs` method copy), F2 = head rename in `environment.rs` and `expressions.rs`
> each inline one copy, F3 = visibility overstep (that private not private, that `pub(super)` not
> converged). → Fix round t12 (one commit, 4 files +96/−85) → Round 2 (t13, reviewed revision
> `20abd3ebc`) verdict = **pass**, new finding = 0: 8 self-built probes' value and exit code per
> item same as pre-fix, 4 IR dumps **byte-by-byte consistent** with round 1 green state, corpus
> 348/348 zero drift, IR snapshot 204 zero drift, `cargo test --all` EXIT=0 (lib 2794 passed + 2
> ignored, integration 297 passed, 0 failed), fmt / clippy / docs-truth all exit 0,
> `check-boundary`'s `pub(crate)` count 167 / frontend-core 26 (= 3.5 pre-baseline), `check-fanout`
> same-name definition zero hit. **Final check basis = round 2's pass**.
>
> **V. Mutation evidence (all via `python scripts/dev/scratch.py trial`, apply → run command → apply
> -R → per file blob compare + require `git status` clean)**:
>
> | #   | Target                                        | Post-mutation blob | Capture result                                                                                                                                               |
> | --- | --------------------------------------------- | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
> | M1  | 3.6.1 `is_static_fn_name`'s global lookup arm | `022986cbea65…`    | Two probes reproduce E6006, EXIT=1 (after restore blob = HEAD, CLEAN)                                                                                        |
> | M2  | 3.5.2 explicit type actual arg branch         | `abbcc5ecf116…`    | Probe check reports E1002, EXIT=1 (comparison's function form still green ⇒ attribution precise to 3.5.1)                                                    |
> | M3  | 3.4.6's `MULTI_FILE_STAGES` dead code arm     | `372f7b92755d…`    | run channel W code all灭 (C2 red); M3b attribution shows check channel criterion healthy, broken is exactly 3.4.6 newly opened compile external emit channel |
> | M4  | 3.6.3 capture barrier entry                   | `7a7204c60383…`    | Capture class turn pin test 4 failed, TEST_EXIT=101                                                                                                          |
> | M5  | 3.6.4 value slot → CallDyn block              | `25853376639b…`    | Probe reproduce E6006, EXIT=1 (blob chain `0d837e9d30de…` → `25853376639b…` → `0d837e9d30de…`)                                                               |
> | M8  | 3.5.1 turn pin test                           | —                  | After fix re-run still can fail (E1003, TEST_EXIT=101, mechanically reversible restore CLEAN) ⇒ criterion not lost failability                               |
>
> **VI. User decision and this batch registration**:
>
> 1. **3.4.6 = D class** (externally observable: multi-file `run` new W code), user 2026-10-19
>    decision "do"; **superset (W1004 / W1005 / W1006) same authorized**, reason = one criterion /
>    one channel. Original 2026-10-15 decision Bing "defer per decision B1" **prerequisite** (4.9
>    pool semantic fix)已 on 2026-10-17 closed.
> 2. **3.6.3 criterion changed to judgment**: from "let it run correct" to "nested function body
>    references outer frame value → compile-time E1001"; assignment path由 E2010兜住.
> 3. **Two "earlier than this task" silent forms registration not changed**: ①Same-block sibling
>    nested function reference——baseline即 E1001 (outer block local family), this batch not expand
>    scope; ②Nested function annotation references outer type parameter still prints `void` (t1
>    registration item). Both are **not** introduced by this batch, both original preserved.
> 4. **3.20 "dead code analysis existing false positives" register** (user 2026-10-19 agreed per
>    Bing disposition后 register): 3.20.1 = W1001 doesn't track reference inside closure
>    (`docs/src/reference/stdlib/test.md`'s `t_add` form: referenced inside closure yet reported
>    unused); 3.20.2 = W1003 judges variant destructuring required import as unused
>    (`docs/src/reference/stdlib/weak.md`'s `use std.option` form: delete即 E1002). Both forms also
>    reproduce on baseline before 3.4.6, **not introduced by 3.4.6**. **Rollback obligation (written
>    dead)**: This group after fix must restore `weak.md` / `test.md`'s examples rewritten to dodge
>    false positive **back to natural写法** and **regenerate baseline** (baseline will move again).
> 5. **0.5 test suite layering** (P0, issue #486): 0.5.1 completed; 0.5.2 (pre-commit / CI wiring +
>    2600 tier measured calibration + `test-specification` detailed rules) pending scheduling.
> 6. **#489 ureq 3 migration**: dependency surface tracking item, **not in WBS**;
>    `.cargo/audit.toml`'s `RUSTSEC-2026-0381` exemption **with hard removal condition**——ureq
>    migrate to >= 3後 delete that ignore, at that time if still red cannot extend period, must
>    first fix dependency. 0.26 line has no fix version (0.26.12 doesn't exist), ureq 2.x no update
>    version.
> 7. **3.20 number conflict disposition (this round new registration)**: 3.20 number
>    originally由 2026-10-18 registered "pre-release audit (v0.8.4)补 register gap" occupied
>    (3.20.1–3.20.4), and user 2026-10-19 decision register "dead code analysis existing false
>    positives" needs 3.20. Whole document number uniqueness rule doesn't allow two 3.20, so **move
>    pre-release audit group whole to 3.21** (three-level change 3.21.1–3.21.4, issue anchors
>    #468–#471 unchanged, entry content unchanged), 3.20 number given to "dead code analysis
>    existing false positives". Migration only changes number, doesn't change any registration's
>    content and attribution; historical commit information not rewritten (migration fact记 in this
>    record and `09`'s count table note).
>
> **VII. Process facts that must be truthfully recorded**:
>
> 1. **Two inScope written narrow each换来 one round rework**: ①3.6 series implementer wrote inScope
>    narrow to `ir_gen.rs` predicate layer, didn't cover `types/`下's shared replace implementation
>    → review round 1's F2 when落地才 found head rename in two files each inline one copy, must open
>    another round convergence; ②3.5's fix round in `types/` shared implementation and two call
>    sides之间 transport change时, again撞上 inScope and actual change surface don't match
>    (AGENTS.md rule 6: inScope must cover mechanically required surface, path prefix matchable by
>    matcher). Both rework以 "implementer inScope written narrow" as cause, **not implementation
>    defect**——but cost is two rework rounds and one review findings.
> 2. **External sync (2026-10-19 13:08 `pull --rebase --autostash origin dev`, 13:19
>    `pull --tags origin dev`) and V0.8.5 release commit**: Per user 2026-10-19 confirmed as
>    **normal release process, completed, doesn't affect this team**; member commits intact not
>    lost. Forensics (`git reflog --date=iso` / `git reflog show origin/dev --date=iso`): 13:08:43's
>    `pull --rebase --autostash` picked up is release chain's "rewrite CHANGELOG retention policy's
>    criterion and self-check", 13:19:34's `pull --tags` is fast-forward; V0.8.5's release commit
>    (`:twisted_rightwards_arrows: V0.8.5: …` and its `:bookmark:` companion commit, CHANGELOG
>    generation and retention policy two commits) all in `v0.8.5` tag reachable range.
> 3. **Three rebase caused local hash rewrite (reference old number will miss)**: `b3245aac9` →
>    `56ae469e1` (3.6.3 main), `415f0154b` → `ef2334de5` (3.6.2), `bdbe859e1` → `8e32e8636` (3.6.4).
>    Verifier proved three as **pure rebase equivalence** by "src/, tests/ tree diff empty" (content
>    unchanged, only parent chain changed), this record uniformly uses **currently reachable** hash.
> 4. **F2's unfixable part**: 3.5.1 that commit's commit message writes "sole implementation" is
>    history, **not rewrite**; correction can only以 "shared implementation convergence's that
>    commit + code comment per fact correct" bear——this record keeps this sentence, in case reader
>    mistakenly thinks at that time no one found this problem.
> 5. **Three observation items (verified, not blocking, but registered in case日后 misjudged as
>    untreated violation)**: **O1** Head rename converged to shared implementation后 coverage
>    surface more down-drill `MetaType` / `AssocType` / `Literal`, but three coverage probes **all
>    parse fail** (`Pipe` / `Ampersand` / old `Type[...]` syntax; `ast::Type::Union` no construction
>    point in parser)⇒ **by design stronger, currently unreachable**, not behavior change; **O2**
>    `check-fanout.py` after fix round new reports `add_type_param: 2 files`, introducer is 3.6.3's
>    closing commit (`inference/scope.rs`) **not** this batch's fix round, review judged as
>    **heuristic false positive** (delegation wrapper + sole implementation's division of labor,
>    same form as existing `add_param`); **O3** `09`'s 3.6 row text still writes "1 three-level
>    task" while actually has 3.6.1–3.6.4, 3.5 three rows missing completion note——由 this closure
>    round一并补 register (see `09`).
> 6. **`src/driver/arms.rs`'s "decision B1" comment not随 code same commit rewrite (leftover, this
>    batch not changed)**: 3.4.6 has changed `typecheck_orchestrated`'s FailFast branch to
>    **staging** `result.warnings` (originally directly discarded), but that function上方's doc
>    comment still writes "`result.warnings` discard——decision B1", opposite to 3.4.6後's code fact;
>    same source's another comment (`src/driver/tests/program.rs`'s stage table description)
>    **已随 3.4.6 that commit same commit rewrite** as "decision B1 'dead code family warnings don't
>    enter compile path' hereby revoked". This batch `src/**` out of scope, so **not changed** that
>    comment, register next team (belongs to AGENTS.md rule 6 "declaration carrier directly named by
>    decision" class debt). `02`'s runtime behavior table B1 row已 in-place correct to current state
>    (dead code family warnings now through same criterion enter MultiFile `run` path).
> 7. **Member didn't run `git push` / `fetch` / `pull`**: sync and push由 captain统一 do (in
>    `git reflog show origin/dev` each is "update by push", member side no push record).
>
> **VIII. Uncovered surface (copied as-is, must not write "not tested" as "passed")**:
>
> - **t3's 7**: ①wasm32 target actual compile not done; ②performance impact not measured (3.4.6 two
>   new stages, 3.6.x's indirect call overhead all no baseline number); ③**W1005 item-by-item
>   attribution not independent reproduction** (verifier's message form regex didn't cover its text
>   form, only checked new W code total 118 consistent); ④3.6.3 closing commit (declaration /
>   assignment doesn't apply barrier) not单独 mutation, c1 form not independent re-test; ⑤t8's 29
>   probe matrix not全量 re-run (only re-measured three representative forms + M4 turn pin); ⑥3.6.4
>   belongs to round 2 scope, round 1 not independent measurement; ⑦CI gate per convention not wait
>   in session,交 closure环节 unified check.
> - **t11's 4**: ①With generic actual arg's qualified name callee (`lib.g(Int)(3)`) and named actual
>   arg call function type value——warehouse内 no sample, this round not construct re-test;
>   ②`use lib` whole import下 without item import's `p.get_x()` (existing E6006 boundary, earlier
>   than this task) not re-test mutation state; ③In `8e32e8636`上 not repeat full gate (由 t9's gate
>   log and t3 in containing that commit's `81d5e414e`上's all-green cover);
>   ④CI交 closure环节 check.
> - **This closure round new registration**: This round's doc change **not re-run
>   `cargo test --all`** (change surface is `docs/**` three files, zero code surface); closure
>   round's gate is `npm run docs:build` + `check-docs-truth.py` + batch tree recycling +
>   `git status` check.
>
> **IX. Batch tree recycling (gc and status original output)**: `python scripts/dev/scratch.py gc` →
> `[scratch] 已删除 verify-p3-t3（释放约 13220.5 MB）` /
> `[scratch] 已删除 verify-p3-t4（释放约 14428.0 MB）` / `[scratch] gc 完成，共释放约 27648.5 MB`;
> `python scripts/dev/scratch.py status` → `[scratch] 没有批次树`; `git worktree list` →
> `E:/git/YaoXiang 20abd3ebc [dev]` (only main mount left). **Gc also deleted two batch
> tree内 verification evidence** (`verify-p3-t3/` contains independent verification's probes,
> patches, original logs and conclusion text; `verify-p3-t4/` contains review round 2's
> evidence)——they are **registered batch directories**,随 tree recycled; this record sections III,
> IV, V have put its conclusions, mutation table and gate numbers as-is into record, conclusion
> doesn't失效 due to file deletion. **Unaffected archive** (not registered batch directory, gc
> doesn't touch): each batch's evidence directory `wbs35` / `wbs35repair2` / `wbs346` / `wbs36` /
> `wbs362` / `wbs363b` / `wbs364` / `wbs486` (located in workspace `target/`下, **not in
> repository**; conclusion已 item-by-item into this record, workspace path not as reference source).
> Acceptance statement (resolvable): `git status` clean; `python scripts/dev/scratch.py status` = no
> batch tree; `git worktree list` only main mount left; no this batch new directories outside
> warehouse.
>
> **X. Doc real build and commit hygiene**: This batch's check / note / register / doc correction
> **攒成 one** doc commit (AGENTS.md rule 8), lands at batch closure; generator regenerated doc
> merge into its functional commit. Before commit locally run `npm run docs:build` (AGENTS.md rule
> 10⑨——markdownlint can't catch VitePress's Vue compile error, and commits only containing docs will
> be judged `no files to check` skip markdownlint) and `python scripts/ci/check-docs-truth.py`.
> **Commit number taken after push**: This record per historical record convention record each round
> actually occurred commit and hash (**note division of labor with `09` check row**——`09`'s check /
> completion register only write "task id + evidence (gate numbers, issue number, file:line)" not
> write commit number, because `pull --rebase` will rewrite local hash; this record is round's
> process historical record, keep hash and同时 give rebase前后 comparison, see "VII" 3). **CI /
> Deploy Docs conclusion交给 "unified check"环节**, not wait in session (AGENTS.md rule 13)——this
> batch's CI run id and conclusion由 captain在 unified check时 backfill.
>
> **XI. Count table accounting review (this round caliber)**: `09` top's secondary / three-level
> count table continues "**base + increment**" caliber (same as 2026-10-18's registration). This
> round increment = secondary +2 (0.5 / 3.20, of which 3.20 is post-relocation new occupation,
> pre-release audit group change 3.21 is net zero), three-level +7 (3.6.2 / 3.6.3 / 3.6.4 / 0.5.1 /
> 0.5.2 / 3.20.1 / 3.20.2). **Remaining pre-existing deviation (this round not correct, for
> subsequent accounting review reference)**: with "per secondary entry括号内 'N three-level tasks'
> sum" as caliber recount, this tree get 162, differs from table header by 7——difference comes from
> 2026-10-18 registration时 new three-level but not synchronously rewrite parent entry count note's
> 5 places (4.9 write 7 actually 8, 3.14 write 1 actually 2, 4.10 write 3 actually 5, 4.4 write 3
> actually 4, S2 write 1 actually 2) and 1 reverse deviation (7b write 6 actually 5), also 1.2 / 1.3
> / 9.1 / S7 four parent entries themselves don't have that note (not计入 caliber). This difference
> **not introduced by this batch**, this batch not change number for other's entries, only register
> here.
>
> **XII. Leftover to-do (this batch register, not fixed)**: ①`src/driver/arms.rs`'s "decision B1"
> comment opposite to 3.4.6後 code fact (see "VII" 6, `src/**` out of scope); ②3.20.1 / 3.20.2 two
> existing false positives (after fix must restore `weak.md` / `test.md`'s natural写法 and
> regenerate baseline); ③0.5.2 (pre-commit / CI wiring + 2600 tier measured calibration +
> `test-specification` detailed rules); ④#489 ureq 3 migration (dependency surface, exemption with
> hard removal condition); ⑤3.8 (native对 user function name shadowing) and 3.12 (runtime
> mode间 executor config propagation) pending decision, 3.18 prerequisite awaits P6——**all not in
> this team scope**.

## P3 3.17 Closure (Round 2: 3.17.1 manually-annotated FFI named actual arg + second binding criterion merge)

> **I. Scope and round relationship**: This section is **round 2**'s closure registration, scope =
> **3.17.1** (issue [#456](https://github.com/ChenXu233/YaoXiang/issues/456)), not including other
> secondary tasks. Round 1 (3.5 / 3.6 / 3.4.6)'s closure see previous section "P3 Debt Round 1
> Closure"; two rounds' relationship written here: **round 1's review round 1 (t4) that
> needs_revision (F1 / F2 / F3) has been covered by t12 fix round + t13 review round 2 (verdict =
> pass), not repeated in this section**. This section's own criterion chain = implementation **t15**
> (attempt 2 delivered; **previous attempt due to provider timeout, zero product**) → independent
> verification **t16** (verdict = pass) → adversarial review **t17** (verdict = pass, **no untreated
> findings**, register 4 observation items).
>
> **II. Item-by-item output and measurement numbers (3.17.1)**:
>
> - **Param name table same source**: `add_native_signature` changed to receive **signature string**
>   and return `Result`——`signature::parse_signature` produces type, sole exit
>   `signature::export_signature_param_info` same-injects "param name + required lower bound"; key
>   and signature table same (same name). Warehouse zero caller so signature can change; doc clearly
>   writes "callers that only get `MonoType` should parse out signature string on their own side and
>   then走 this API", and this function **must not** itself parse param names.
> - **Red first then green** (red state measured on parent commit `537b46b74`): manually-annotated
>   surface four forms + duplicate name + legal call total 6, **5 failed, diagnostics all `[]`**
>   (named actual arg silently discarded); green same file **12 all pass**——four forms E1014 / E1010
>   / E1111 / E1111, duplicate name E1015, legal named call and omitted optional position allowed.
> - **Three call surfaces same criterion same code**: manually-annotated FFI (typecheck stage) /
>   ordinary user function (IR stage) / stdlib native export (`assert.assert`) three surfaces **per
>   code equality assertion** (E1014 / E1010 / E1111; E1015 see manually-annotated surface and user
>   surface). Different observation stage only reflects "who discovered first"; host native doesn't
>   enter registry, IR stage inherently unresolvable (**earlier than this task's embedding surface
>   limitation**).
> - **Criterion single source**: `bind_args_to_params` **1 definition**
>   (`inference/arg_binding.rs:35`) + **6 calls** (typecheck 2 + ir_gen 4); second criterion
>   `build_arg_slots` **0 places** (deleted, `ir_gen.rs:516` only left comment explaining its
>   history); `ir_gen` only left pure transport `slot_order_arg_indices` (`ir_gen.rs:521`, 1
>   definition 2 calls, zero judgment). Line count: `ir_gen` **+58 / −92 (net −34)**; warehouse 4
>   files **+448 / −102** (of which 353 lines are new tests).
> - **No regression**: 8 CLI probes (u1–u4 / d1 / d2 / n1 / n3) per code consistent mutation前后;
>   `ir_gen` blob `0d837e9d…` ↔ `ffe1af8e…` check. Merge initial version one self-injury already
>   fixed: IR side重判 "missing param" reported legal overload `t.pick2(a = 2)` false E1010
>   (named_args unit test caught)⇒ missing param judgment goes to typecheck, IR only replay slot
>   binding; after fix 19 named_args + 12 new unit tests all green. Corpus 348-348 zero drift, IR
>   snapshot zero drift.
>
> **III. Independent verification (t16, freeze `c3082a160`; red baseline `537b46b74`, both经
> `git merge-base --is-ancestor` verified as HEAD ancestors; probes all self-built)**:
>
> - **Red→green independent reproduction** (manually-annotated FFI surface): self-built independent
>   Rust unit test `verify_host_ffi.rs` (4: unknown name E1014 / compete for slot E1111 /
>   named后 position E1111 / legal named call allowed), via `scratch.py trial` temporary挂 into
>   `tests/mod.rs` (apply → test → apply -R → CLEAN). Two states each use **own interface form**'s
>   probe variant (parent commit `add_native_signature(name, MonoType)` old signature / fix state
>   `(name, &str) -> Result` new signature——interface itself is fix surface): parent commit **3
>   failed / 1 passed**, 3 failed diagnostics **all `[]`** (`got []`), TEST_EXIT=101; fix state **4
>   passed / 0 failed**, TEST_EXIT=0. Red state另 in fix tree revoke fix's mutation state reproduce
>   (`M7v2` merge patch: revoke fix + revoke implementer test +挂 self-built old API probe)——3
>   failed `got []`, TEST_EXIT=101, after restore four files blob consistent with HEAD, `git status`
>   CLEAN, after revoke `ir_gen.rs` blob = `0d837e9d30de` (i.e. parent commit original blob). CLI
>   surface (user function + stdlib) two states per code consistent——as expected: these two surfaces
>   3.16.1已 loud, this commit's fix surface only in manually-annotated FFI.
> - **Four forms item by item** (CLI self-built probe, `c3082a160` measured): param name error E1014
>   `has no parameter named x (available: a, b)`, missing required position E1010, position and name
>   compete for slot E1111, named后 position E1111; additional: duplicate name E1015, stdlib unknown
>   name E1014, stdlib missing required position E1010 (help names cond), stdlib compete for slot
>   E1111, legal named call EXIT=0 output 3/3, omitted optional position EXIT=0.
> - **Criterion真 delete not rename**: `git show c3082a160:src/middle/core/ir_gen.rs`内
>   `build_arg_slots` **0 hit** (parent commit same file 3 hit, definition at :521);
>   `slot_order_arg_indices` (fix state :521) per line check =
>   `sort_by_key(|&arg_idx| positions[arg_idx])` pure sort, no position/name/optional position
>   judgment residual, doc comment states "no position judgment, no name judgment, no optional
>   position judgment"; deleted 71 line judgment block (parent commit :521-593, six rules: empty
>   table loud / position take slot / order invalid E1111 / unknown name E1014 / compete for slot
>   E1111 / enter slot) no longer exists; `bind_args_to_params` 1 definition 6 calls (typecheck 2:
>   `expressions.rs:2854` / `:5653`; ir_gen 4: :7828 / :7926 / :8021 / :8351), consistent with
>   implementer report; param name sole exit = `export_signature_param_info` (environment.rs
>   manually-annotated / typecheck::mod stdlib / ir_gen lookup), no second self-resolve.
> - **Self-injury regression pin** (`t.pick2(a = 2)`): `git show c3082a160 --name-only` **doesn't
>   contain `named_args.rs`** (4 files: environment.rs / tests/mod.rs / native_param_names.rs /
>   ir_gen.rs)⇒ pin test not changed; independently re-run that case 1 passed, named_args full set
>   **19 passed / 0 failed**.
> - **No regression gate** (`c3082a160` tree): `cargo test --all --quiet` (4×4 caliber) EXIT=0——lib
>   **2806** + 2 ignored, integration **297**, 52/44/9/2/1/9 all green 0 failed; `cargo fmt --check`
>   / `clippy -D warnings` both EXIT=0; corpus difference (dual parameter) 348/348 difference empty;
>   `check-snapshot-drift.sh` PASS (204 snapshot zero drift); `check-docs-truth.py` EXIT=0; 3.16.1
>   four form comparison (stdlib surface) E1014 / E1010 / E1111 / allowed.
> - **Uncovered surface (t16 five, copied as-is)**: ①`testsuite.py run full` (CI caliber resource
>   profile channel) not re-run——implementer已 per decision fall back (exclusive wave peak 8.11
>   GiB > budget 7.51 GiB, exit 2 is tool按 design reject); this round with 4×4 and 2×1 two
>   calibers' `cargo test --all` cover. ②Manually-annotated FFI surface (host-FFI) can only be
>   driven via Rust test (`add_native_signature` warehouse内 zero caller, no CLI surface)——"host
>   embedded process内 real FFI call" not tested. ③Variadic (`...`) named actual arg form not
>   construct sample (no warehouse内 criterion sample). ④`check-boundary.py` / `check-fanout.py` not
>   re-run (implementer已 pass; no cross with behavior criterion). ⑤CI (GitHub Actions) per
>   convention not wait,交 closure环节 check.
>
> **IV. Adversarial review (t17, reviewed revision `c3082a16051cb1bfb8b84bdc62ec6bd29d90ff2d2`,
> parent = `537b46b7488a60b42cdc35110e0a413c80a58169`; all content用 `git show <rev>:<file>` take,
> line numbers re-taken on referenced revision)**:
>
> - **① Criterion single source reverse check**: `slot_order_arg_indices` (ir_gen.rs:521-527)
>   **doesn't reorder by name** (sort key is shared criterion's given slot number, name only appears
>   in `arg_binding::bind_args_to_params`内), **missing param no兜底** (absent slot根本 doesn't
>   produce actual arg index), **doesn't distinguish optional position** (IR two places传 full
>   optional, other two places传 real value `i >= required_min`). Warehouse "per param name find
>   position / find member" grep per item checked: **slot calculation only `arg_binding.rs:76` one
>   place**, other 7 hits each belong to other concerns (generic type parameter's constraint
>   interface check / const generic actual arg replace / generic type variable binding / generic
>   type parameter replace / const generic parameter lookup / method definition lookup / struct
>   field lookup).
> - **② Responsibility boundary (missing param only响亮 in typecheck, IR only replay slot
>   binding)**: typecheck side `bind_named_args` (`expressions.rs:2854`)传 full required ⇒ missing
>   any value param E1010; native export arm (`:5653`)传真 optional position `i >= required_min` ⇒
>   only对 real required position E1010, `?msg` omission allowed. IR side this commit changed two
>   places (`ir_gen.rs:7926` RFC-004 binding path / `:8021` overload resolution method path)
>   **一律传 full optional, only replay slot binding** (comment明 writes "missing param goes to
>   typecheck" and "per declaration table重判 missing param会 report legal resolution false E1010
>   (`t.pick2(a = 2)` measured)").
> - **③ API shape**: environment.rs:990-993明 writes "this function **must not** itself parse param
>   names" and points to sole exit `export_signature_param_info`; :997-999 writes "callers that only
>   get `MonoType` should parse out signature string on their own side and then走 this API".
> - **④ Behavior change surface**: Code four places difference (same param name twice E1111→E1015
>   code change / unknown name E1014 text change / empty table E1014 text change / position actual
>   arg exceeds IR side declaration table: silently discard→照 emit), **measured可达 form上 no
>   diagnostic code / text change** (typecheck first judgment, same code), IR product in 5
>   self-built probes上 **byte-by-byte consistent** (p1 / p5走 this commit changed's branch and
>   product unchanged, p2 / p3 / p4 two states same code, p9 not constructable, p6–p8 source layer
>   unreachable); t15's 8 CLI probes two states per code consistent. Difference all falls in
>   **本来就 failed** programs (program still fails) ⇒ D59 "not change originally successful
>   program's external behavior" not touched.
> - **⑤ Test quality** (not just red green): 12 new tests (`typecheck/tests/native_param_names.rs`)
>   item-by-item对 `test-specification`——naming `test_<what>_<scenario>`, three-segment AAA comment,
>   assertion all with custom message, setup repeat ≥3 times已抽 helper, helper内 panic with
>   context; **no "only断 exit code"式 weak assertion** (first `assert!(`diagnostic non-empty`)`
>   then `assert_eq!(`per code equal`)`, both empty won't空洞 pass); 3 negative cases (legal named
>   call allowed / optional position omission allowed / explicitly give optional position allowed)
>   under M1 red state still pass ⇒ they **only防 false positive**, exactly their use. **M1**
>   (remove `native_param_names` injection)→ `cargo test --lib native_param_names` **9 failed / 3
>   passed**, TEST_EXIT=101 ⇒ new test **真 pin fix**. **M2** (whole段 revert `ir_gen` merge)→
>   `cargo test --lib` **2806 passed / 0 failed / 2 ignored**, exit 0 ⇒ merge surface **no test
>   coverage**. `pick2` pin (existing `named_args.rs:253-283`, this commit not changed)走 **real
>   full link** and assert **runtime value** `Int(20)` ⇒ **真 pin overload resolution** (not "no
>   error"), and it's live guard for "IR must not重判 missing param".
> - **⑥ Limitation register**: Host native only enters `TypeEnvironment`'s two tables, **doesn't
>   enter `ModuleRegistry`**⇒ IR generation per registry resolve namespace member, so host native in
>   IR stage inherently unresolvable (E3006), **earlier than this task's embedding surface
>   limitation**; this limitation and "why manually-annotated surface must observe in type check
>   stage" written in **test file doc comment** and `environment.rs` field doc (not only in report).
> - **Gate eight items all re-run all green on reviewed revision**: fmt --check / clippy -D warnings
>   / `cargo test --all` (**3220 passed, 0 failed**; lib 2806 + 2 ignored, integration 297) / corpus
>   difference 348-348 zero drift / IR snapshot 204 zero drift / docs-truth / check-boundary (0
>   exemption 0 violation) / check-fanout (new symbol no same-name multi-definition entry).
> - **Four observation items (copied as-is: verified, judged not constitute finding, no need to
>   handle, but registered in case日后 misjudged as untreated violation)**:
>   - **O1**: Commit message says "after this merge **warehouse only left shared criterion one
>     place**"——in terms of **slot binding** holds; but typecheck's **method overload arm** still
>     has one "per name lookup candidate table (E1014) + arity (E1010)" own judgment
>     (`inference/expressions.rs:5578-5600`), it uses **candidate group representative**'s param
>     table (`method_overload_param_names`), different from shared criterion's used **resolved
>     candidate** table. Judgment basis: that place **earlier than this commit, this commit not
>     touched** (`git show c3082a160 --name-status` only 4 files, no expressions.rs), and is
>     "candidate matching" concern (table different, semantics different), not slot binding.
>     **Wording too broad, register as observation**.
>   - **O2**: `add_native_signature` changed to receive signature string後, **no**
>     `MonoType → signature string`'s shared helper ⇒ future callers that only get MonoType can only
>     each write one serialization. Judgment basis: param name / required lower bound已被 doc locked
>     in sole exit `signature::export_signature_param_info`; currently **zero caller**; doc已 write
>     "callers that only get MonoType should parse out signature string on their own side and
>     then走 this API"⇒ residual risk only in "how串 is constructed" one section, **low**.
>   - **O3**: This commit's **merge part** (ir_gen deduplication) **no any test coverage** (M2:
>     whole段 revert ir_gen後 `cargo test --lib` still 2806 passed all green). Judgment basis:
>     "deduplication" no new behavior to pin, nature so; "single source" property由 grep +
>     `check-fanout.py` gate guarantee. **Optional strengthening**: in `native_param_names.rs` add
>     one **IR layer** case (cover ir_gen:7926 / :8021 path), let merge surface also have guard.
>   - **O4 (attribution correction)**: Commit message and report attribute d2's E1010 to "**method
>     arm** arity pre-check first triggered"——measured that E1010's help is generic text "check the
>     argument count of the function call", **not** shared criterion "missing param"'s help
>     ("缺少必需形参：…")⇒ trigger is **native arm**'s arity pre-check (`native_arity` table).
>     **Code behavior conclusion unchanged** (parent commit existing, not changed this time, already
>     reviewed), only **attribution wording needs correction**; and "`list.push` duplicate
>     name即 E1010" must be qualified as "**arity mismatch + duplicate name**"——only duplicate name
>     and arity match时 (`list.push(list = v, list = 9)`) two states are both **E1015** (duplicate
>     name prior to arity judgment).
>
> **V. Gate and resource channel (one external link)**: This task **first真 use**
> `python scripts/dev/testsuite.py run full` → **exit 2**——exclusive wave peak 8.11 GiB > local
> budget 7.51 GiB (gap 618 MiB), tool按 design **reject silent skip** ⇒ fall back
> `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 cargo test --all --quiet` → **exit 0** (3220 passed / 0
> failed). This channel state chain **#486**: `full` on local machine due to peak
> extrapolation偏 conservative and unavailable, **calibration (2600 tier由 measured calibration)
> belongs to WBS 0.5.2**.
>
> **VI. Count table: this round not moved**——`3.17.1` already on 2026-10-15 registered and计入 table
> header ("2026-10-15: +2 = 3.17.1 / 3.18.1"), this round only check + note, **three-level count
> unchanged** ⇒ `09` top's secondary 67 / three-level 176 **not moved**. This round **not new**
> count deviation (parent entry 3.17's "1 three-level task" note本来就 correct); previous section
> "XI" recorded 6 pre-existing deviation (parent entry count note lag) **unrelated to this round,
> not introduced by this round**, still per that section caliber left for accounting review.
>
> **VII. Batch tree recycling (gc and status original output)**: `python scripts/dev/scratch.py gc`
> → `[scratch] 已删除 verify-p3-t16（释放约 10235.4 MB）` /
> `[scratch] 已删除 verify-p3-t17（释放约 10079.2 MB）` / `[scratch] gc 完成，共释放约 20314.6 MB`;
> `python scripts/dev/scratch.py status` → `[scratch] 没有批次树`; `git worktree list` →
> `E:/git/YaoXiang c3082a160 [dev]` (only main mount left). **Gc also deleted two batch
> tree内 verification evidence** (`verify-p3-t16/`'s `EVIDENCE.md`, `verify-p3-t17/`'s
> `EVIDENCE-t17.md` and probes / patches / original logs)——they are **registered batch
> directories**,随 tree recycled; this record sections III, IV have put its measurement numbers,
> gate numbers, mutation table and four observation items **as-is into record** for evidence,
> conclusion doesn't失效 due to file deletion. **Unaffected archive** (not registered batch
> directory, gc doesn't touch): `wbs317` (implementer evidence: evidence-3.17.1.md / red-test.txt /
> trial-before-after.txt + fix-3171.patch / attribution table etc.; located in workspace
> `target/`下, **not in repository**; conclusion已 into this section, workspace path not as
> reference source). Acceptance statement (resolvable): `git status` clean;
> `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only main mount left;
> no this batch new directories outside warehouse.
>
> **VIII. Doc real build and commit hygiene**: This round's check / note / register **攒成 one** doc
> commit (AGENTS.md rule 8); before commit locally run `npm run docs:build` (AGENTS.md rule 10⑨) and
> `python scripts/ci/check-docs-truth.py`, markdownlint zero warning. **No push** (member doesn't
> push, see AGENTS.md rule 10⑧); **CI / Deploy Docs conclusion交给 unified check环节** backfill, not
> wait in session.
>
> **IX. Leftover items and registration**:
>
> 1. **Round 1 closure registration's leftover item ① closed**: `src/driver/arms.rs`'s "decision B1"
>    comment opposite to 3.4.6後 code fact one item,由 `537b46b74`
>    (`:memo: docs(driver): correct B1 comment opposite to fact (3.4.6 closing omission)`) in-place
>    rewrite to current state fact (behavior neutral, only comment line; wording aligned with
>    `src/driver/tests/program.rs`'s same fact statement).
> 2. **This round new registration**: O1–O4 four observation items (verified, not blocking,
>    registered); 0.5.2 (`full` channel's 2600 tier measured calibration + pre-commit / CI wiring);
>    #489 ureq 3 migration and `.cargo/audit.toml` exemption's hard removal condition; 3.8 / 3.12
>    pending decision, 3.18 prerequisite awaits P6——**all not in this team scope**.
