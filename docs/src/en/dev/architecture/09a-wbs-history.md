# WBS Historical Notes (Implementation Record · Audit · Ruling Trail)

> This document is the **historical notes** for [09-execution-wbs.md](./09-execution-wbs.md)
> (Construction Task Table): implementation records, audit findings, ruling trails, and corrections
> for each phase. **Division of labor**: 09 only describes the **plan** (kanban, three-level task
> table, dependencies, risks, acceptance); this document only records the **history** (what was
> done, what was found, how it was ruled), appended chronologically without rewriting history.

## P2 Establishing Equivalence Criterion Baseline

> **P2 Implementation (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus passing
> green. **D38 prophecy fulfilled**—the green run process actually measured and fixed three real
> defects in ir_gen (see 06 §G6 for details): method locals table truncated to parameter count
> (local_count underestimation, runtime tolerated by Frame::set_slot), unannotated parameters
> dropped by filter_map (three places, params metadata misaligned with frame layout), match result
> slot depends on frame's implicit Void initialization (now explicitly pre-initialized). The actual
> semantics of jump target is flattened instruction index (not the label-keyed text in 07), CFG is
> built at instruction level—registered in the verify.rs file header. **Commit comparison**: ir_gen
> fix = `a9ea199b`; verify main body = `d5b18390` (⚠ parallel flow COMMIT_EDITMSG collision caused
> its message to be incorrectly labeled as "docs(rfc): RFC-027a…", the content is actually
> verify.rs + six test files, buried in history and unamendable, hereby recorded).
>
> **P2 narrowed portion implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> hemostasis channel preconditions unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; judgment contract uses the same `TestFileSpec` as single-file
>   corpus (single point on library side), binary location/subprocess launch extracted to
>   `tests/common/` with yx_runner migrated synchronously (rule 3).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. Trigger source uses the form
>   `SumUpTo(3, r)` pinned by rfc027 tests (return position/call site/binding position)—the
>   `Sorted(3)` annotation example in 02 would be substituted and evaluated directly as Disproved in
>   current code, not presenting Unproven silently; the Sorted(3) pseudocode in 07 is only
>   illustrative. Red-state measurement data is in the test file header.
> - Two red criteria follow the 2.4.5 red skeleton precedent with `#[ignore]` always-resident:
>   default suite all green, `-- --ignored` reproduces red state; **remove attribute when P3 fix
>   lands to turn green**.
> - The reason for the deliberate deviation from the 07 (code, file, line) table in comparison
>   normalization (removing file/line, comparing only error code sets) is written in the 2.4.1 test
>   comments.
>
> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **moved out of P2**: the `Program` / `Obligations` types they reference don't exist until P4, P2
> cannot compile. The former becomes P4's 4.1.4 (no double registration), the latter's `#[ignore]`
> red skeleton merges into 4.3.1.
>
> **2.3.3 Implementation (2026-10-07)**: C4 mandatory coverage checklist item-by-item verification
> (07 §143), 10 categories of semantic corpus layers fully covered and all within
> corpus-parity.jsonl diff baseline (26 representative files 26/26 hit, grep
> verified)—ref/borrow/move 05-ownership entire directory (ref_shared,
> borrow_immutable/mutable/return, move_basic, ownership_deep, etc.), closure captures closures +
> closure_arg_inference + spawn_capture, currying curry_value_fix + curry_tail_expr, spawn
> 04-concurrency ×13, iterator for spawn_for etc. 16 files, sum types sum_type_* +
> pattern_or_guard + match_call_scrutinee, existential coercion point interface_dynamic_dispatch
> (Vec(Animal) existential type heterogeneous wrapping triggers existential_coercions) +
> interface_rebind_dispatch, `?`/Try propagation question_propagation + option_try + try_methods +
> user_try_type, method overload method_overload (+ method_overload_ambiguous_err in diagnostic diff
> surface), refinement constraint Drop sequence refined_* ×5 + 05-ownership driven ReleasePlan (span
> key contract has additional 2.4.4 red criterion always-resident). Performance baseline
> `benches/pipeline.rs` included (07 §145 three smokes two): `corpus_compile_frontend` full corpus
> first compile test 17.6s, `cli_cold_start_hello` process cold start 169.7ms; interpreter
> throughput is covered by benches/lib.rs hotpath group (interp_fib_recursive_27 /
> interp_loop_steady_10m), no duplicate construction. Regression gate check-perf-regression.sh
> returns to P9 unified checklist (2.5).

## P3 Fixing Correctness Vulnerabilities (Minimal Plan)

> **Error type annotation for landed lines** (retrospective annotation, not changing history): 3.1 =
> B | 3.2 = B (defensive empty arm, criterion turning green borne by 3.1) | 3.3 = A | 3.4.1 / 3.4.2
> = A (new W code) | 3.4.3 = equivalent refactor (manual review) | 3.4.6 / 3.4.7 = D (user-visible
> surface, already user-ruled) | 3.4.8 ①② = C (main) + A (incidental three places) | 3.5 / 3.6 / 3.7
> / 3.8 = D (all user-ruled).
>
> **P3 Diagnosis (2026-10-12, user requested)**: A diagnostic accounting for the perception of "P3
> items continuously growing".
>
> **1. First clarify the scale (perception amplified by two accounting defects)**
>
> 1. **Duplicate numbering**: `3.5.x` is shared by two task families—(a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 three-level: lexical and parser branches / AST `is_pub` / checker
>    side / dead code exemption convergence / peripheral echo / std and diagnostic text / language
>    reference convergence, all checked); (b) 2026-10-10 registered **3.5 higher-order type
>    parameter call path completion** (3 three-level). The two share `3.5.x` ⇒ cross-reference
>    ambiguity (**2026-10-12 already renumbered per user ruling: 029g family changed to `029g.x`**),
>    and caused 029g's lines to be miscounted into P3. **Pending ruling**: Rename 029g family to
>    `029g.x` (or move into their phase), with synchronized references in the document.
> 2. **Missed checks**: `3.7`'s 4 three-level (3.7.1–3.7.4) are all completed (d922a551 / 38bd4b9e)
>    but unchecked ⇒ this item is corrected this time.
>
> **2. P3 actual accounting (12 second-level lines)**
>
> | Line                                                                          | Status                                                                                                                                                                             | Disposition suggestion                                                                                                          |
> | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                               | Completed                                                                                                                                                                          | —                                                                                                                               |
> | 3.4 Silent drop point additions (8 three-level)                               | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (precondition 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (precondition 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4/3.4.5 unlock with 4.3.1                                                                        |
> | 3.5 Higher-order type parameter call path completion (3 three-level)          | Not started (declaration side live, call side broken)                                                                                                                              | Keep in P3; **does not block main refactor line**, but is "accepted feature not implemented", higher priority than pure cleanup |
> | 3.6 Function type value call path (1 three-level)                             | Not started (semantics cut (i))                                                                                                                                                    | **Keep**: correctness defect, clear criterion                                                                                   |
> | 3.7 init segment call name qualification (4 three-level)                      | **All completed**                                                                                                                                                                  | Check and close this time                                                                                                       |
> | 3.8 native shadowing (1 three-level)                                          | Not started                                                                                                                                                                        | **Stuck on decision** (priority order of name resolution), not a scheduling issue                                               |
> | 3.9 / 3.10 / 3.11                                                             | Completed (2026-10-12)                                                                                                                                                             | —                                                                                                                               |
> | 3.12 Executor configuration propagation between runtime modes (1 three-level) | Not started (Class D, user ruling B: separate team)                                                                                                                                | **Suggest moving out of P3 → P8 (execution layer)**, pending ruling                                                             |
>
> **4. 4.3.1's leverage (main line marker)**: `4.3.1 obligations.rs + assert_drained()` is also the
> precondition for **3.4.4**, **3.4.5**, and **4.5.1** (cross-layer contract PlanId-ization) ⇒
> taking it once simultaneously advances the main line and clears two P3 stranded items. 3.5 (HKT),
> 3.6 (function type value call), and 4.6.1/4.9.1 have **no unsatisfied preconditions**, can be done
> anytime; 3.8 stuck on semantic decision (user definition priority vs. explicit diagnostic).
>
> **3. Mechanism suggestion (prevent recurrence)**: Newly registered second/third-level lines **must
> carry two fields**—"belonging phase" (which P it truly belongs to) and "whether it blocks the main
> line"; `registered pending scheduling` is no longer the default state, unmarked is considered
> unschedulable. This way, expansion is visible and sortable, not indiscriminately accumulated.
> **Pending ruling**: Whether to write this clause into the registration agreement of this document
> and `src/dev/coding-rules.md`.
>
> **P3 Implementation (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + unique getter (then
> noted `pub(crate)`; **2026-10-07 verified actual implementation is `pub(super)`**—the `#434`
> ruling referenced by code comments has no record on docs side, see 3.4.3; fully private requires
> rewriting 11 constructor literals, not taken during hemostasis phase; closed when P4 Obligations
> migrates); execution mechanism extracted to `src/frontend/proof_execution.rs` (rule 3 single
> point—cannot be placed in typecheck/, that would create L3→L4 reverse dependency); 3.1.2 added
> **four entry points** per 02 evidence chain step 5 (WBS line number missing compile_project `:99`,
> must be supplemented and registered per 3.1.1 acceptance "criterion turning green"); 3.2.1 empty
> arm changed to accounting (proof_calls throws up) + diagnostic (`into_result` established
> conversion path); 3.3.1 SOLVER slot Option-ized, Z3 missing uses `SMTResult::Unknown` conservative
> degradation (E2031 family diagnostic replaces process crash). Two vulnerability criteria removed
> `#[ignore]` to turn normal green, supplementary corpus fixture `proof-obligation-honored`;
> check-fanout inventory migrated accordingly (wiring point 19=inventory). Measured: criteria
> 2.4.1/2.4.3 turned green, full suite lib 2494 / integration 272 / corpus two-layer all green.
>
> **3.4 Audit (2026-10-07, P3 post-implementation review)**: Targeting "failure produces no signal"
> type defects (definition in 02 §41-52), re-inventoryed on unchanged code, located 8 more places,
> all registered as 3.4.x in the above table. Methods and evidence:
>
> - **Consumer-side inventory** (16 obligation fields grep all repository read points):
>   `variant_ctor_calls` **zero consumers**—the only read at `checker.rs:1468` is "self-poured into
>   `TypeCheckResult`", IR side uses form detection `ir_gen.rs:7374`; `rfc011b.rs:659/688` only
>   asserts the production side, which is the 02 §186 criticized "only test production side,
>   naturally immune to consumer side defects" form → 3.4.4. `method_overload_ir_names` result-level
>   field is also zero external consumers, IR uses parallel `overload_resolutions` → 3.4.5.
> - **Phase coverage review**: 02 §91 table row 1 (four entry points "no proof_execution") fixed by
>   3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 state unchanged (multi-file `run` still does
>   not output W1003, does not run dead code family) → 3.4.6; LSP non-project-internal path (02 §77
>   "manual lex→parse") also does not run proof → 3.4.7.
> - **Same-form empty arm scan** (`Unproven` all-repository consumers only four places): checker
>   three refinement branches (3.1 already covered), ownership arm fixed by 3.2.1 (**source code
>   unreachable**), and `environment.rs:422` (**reachable, zero diagnostic, zero accounting, and
>   zero registry in all repository docs**) → 3.4.1, the only "all-unknown" live vulnerability in
>   this audit.
> - **Solver acquisition side**: `default_solver()` three consumer forms vary (`predicate.rs:32`
>   process singleton / `checker.rs:1266` each `check_module` injection / `ownership.rs:621` each
>   back-edge decision new)——3.3.1 only eliminated the panic one; the **silent surface** of the
>   other two belongs to 3.4.2, **singleton and cache hit** belongs to P4's 4.4.2 (no double
>   registration).
> - **Not counted in 3.4 existing attribution**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys unconsumed, Drop silently lost) still P2 red state always-resident, attribution
>   4.5.1/D20 unchanged; 4.4.1 (layer order declaration opposite to actual, `equivalence` layer not
>   in pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) already registered in P4.
>
> **Execution agreement**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 preconditions, can be landed in place
> anytime (suggest doing 3.4.1 first—the only "all-repository zero registration" live vulnerability,
> smallest change surface); 3.4.4–3.4.8 follow P4 execution, their criteria depend on obligation
> ledger (4.3.1) or unified Driver (4.2.x). **3.4 does not establish a new criterion system**,
> uniformly reusing existing C2/C3 and P9's 9.2 gate.
>
> **Batch 3.13–3.16 Implementation Record (2026-10-13 registered → 2026-10-15 closed)**: User
> 2026-10-14 ruled to merge four relevance sub-items (#451 method binding panic / #452 named
> argument parameter resolution / #453 dead inference entry / #454 native export named argument—the
> last item is a boundary finding when verifying 3.14, same day attribution experiment verified
> **not introduced by #452**, user ruling ① merged into this batch) into one team. Closure status:
> all four completed, ruling registered as RFC-039 **D60** (six binding rules + shared criterion
> single-point).
>
> **1. Two contract corrections (lesson: contracts themselves can be wrong)**
>
> 1. **Acceptance fixture self-contradictory**: #452 task contract simultaneously required "mixed
>    arguments output 20" and "position and name competing for same position reports error", but the
>    pinned fixture `Temp.pick(self, a)` has only one value parameter—the same argument combination
>    is both "mixed" and "conflicting". First implementer truthfully reported blocker, set task as
>    failed, overall revert (no silent green for green light). Disposition: task reassigned +
>    contract revision—gave **six binding rules** and **8 value-level matrix** (fixture changed to
>    two value parameters `pick(self, n, a)`), attempt 2 completed by second implementer. Six rules
>    full text has been verbatim incorporated into D60.
> 2. **inScope missed mechanically necessary surface**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed the regenerated
>    `docs/src/reference/error-code/`**. After E1110 was registered, only ran `code-tables --fix`,
>    **did not re-run** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth red ("source
>    registered/emitted but document missing 1: E1110", source 154 / document 153, run
>    **38518624964**—the only CI red line in this batch). Fix = mechanical regeneration (8 files
>    +83/−70, only touched `docs/src/reference/`), reconciled 154/154, CI 38520209026 green.
>
> **2. Review → Fix → Re-review two closed loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed with resolution ⇒ calls with named arguments silently in the entire chain of
>    multi-candidate zero-hit/single-candidate hit paths) → fix round added external code **E1111
>    (argument_order_invalid)** (code table + locales + RFC-013 + generator four-piece set same
>    submission; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared resolution entry + native arm settled by name) →
>    independent verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations
>    mechanically reversibly replayed by `scratch.py trial` all caught, restoration byte-by-byte
>    consistent) → round-1 review needs_revision (F1 = typecheck native arm **inline copied** a
>    binding criterion, missing rule ④ ⇒ **ruling divergence** for method and native isomorphic
>    calls—method reports E1111, native silent exit=0; ir_gen has another third existing copy
>    reporting old code E1015) → convergence round converged criteria to single implementation
>    `bind_args_to_params` (`arg_binding.rs:35`, four call sites `expressions.rs:2727` / `:5416`,
>    `ir_gen.rs:7776` / `:8279`) → round-2 review caught the **regression** introduced by
>    convergence round (IR convergence block lost native omittable position ⇒ previously successful
>    programs with omitted optional positions were falsely reported as E1010) → fix round (optional
>    position criterion and parameter name from single exit `signature.rs:236`) → round-3 review
>    PASS (independent reproduction red→green; mutation under board pin FAILED empirical criterion
>    can fail). **Lesson**: Review must cover "unfixed branches of similar paths" (both
>    needs_revision in this batch were caused by probes all landing on hit surface); fix round
>    itself can introduce regression (omittable position), caught by next round of replacement
>    reviewer.
>
> **3. Process events (truthfully recorded, not pointing to individuals)**
>
> 1. **External whole tree discard (2026-10-14 about 17:14)**: Working tree was externally restored
>    silently—implementer's fix in-progress (typecheck/ir_gen/code table/locales/RFC-013 etc.) all
>    disappeared, **at the same moment the two review documents edited by user himself also
>    disappeared from git status** ⇒ judged as repository-level discard (most likely user-side
>    manual cleanup/IDE "discard all changes"). Three evidences exclude group members: stash empty,
>    no new commits, no reset in reflog. Implementer replayed in-progress work by context and
>    pushed; discipline subsequently changed to "**first save patch to scratch, then take red
>    state**".
> 2. **Verification member context exhausted replaced**: The independent verifier verifying 3.16 ran
>    out of room midway (no closing message), replaced by load rule—the successor continued using
>    their ready products (green state results, corpus equivalence script, mutation plan),
>    verification finally PASS. Positive precedent for partial product handover.
> 3. **Review 5th round rotation and historical round withdrawal**: When rotating the reviewer per
>    load limit rule ⑤, `remove_member` **withdrew already-ruled terminal review/verification rounds
>    back to shared pool**, scheduler immediately re-ran (t3/t8 were reassigned to new members,
>    duplicate work and blocked downstream). Disposition: captain took over and cancelled withdrawn
>    historical rounds (t3/t8/t13), accordingly added AGENTS.md rule 12 (handle withdrawn rounds
>    immediately after replacement). This batch closure record is registered per this rule.
> 4. **scratch governance (about 6.3 GB)**: Each task had an external tree in the early stage of the
>    batch, 6 directories scattered outside the repository (about 780 MB, cleaned) + two members'
>    external verification trees (about 2.2 GB / 3.3 GB, self-deleted after use). Output governance
>    tool `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md
>    rule 9 (batch tree single entry), rule 10 (verification environment six items: mechanically
>    reversible mutation / line number from referenced revision / CI unique gate must run, etc.),
>    rule 11 (writer side load limit), rule 12 (handle withdrawn rounds after replacement) and top
>    meta rules two (no private content / verify before assert).
> 5. **Contract defect (not pointing to members)**: #454 fix round's contract inScope only circled 2
>    files, actually mechanically necessary surface 8 (rule 6 is to prevent this—contract template
>    should pre-set mechanically necessary surface list); also this task's terminal state field was
>    occupied by probe payload, formal acceptance/command evidence was supplemented by captain per
>    specification.
>
> **4. Outstanding items (this batch does not fix, registered for future reference)**: Three
> low-severity findings from round-3 review—① Method path's `build_arg_slots` (`ir_gen.rs:521`) is
> still criterion's **second copy** (existing, per last round's caliber registered with closure, not
> converged)—**2026-10-16 pre-release audit supplementary three points**: (a) It has **two** call
> sites: `ir_gen.rs:7888` (method binding path, using bound function `binding.function`'s parameter
> names) and `:7975` (overload resolution path, using resolved mixed name leaf segment), original
> registration only wrote "`ir_gen.rs:521` area"; (b) It has **already diverged from shared
> implementation `bind_args_to_params`**: missing rule ⑤ (duplicate naming → E1015), rule ⑥ (missing
> required parameter → E1010) and "optional position" concept, and hardcodes receiver position as
> `1 + i`; both call sites uniformly `continue` for empty slots (comment says "diagnostic belongs to
> typecheck (it guarantees no empty slots after resolution passes)"), this is a **dangling invariant
> written in comments, without assertion safety net**; (c) **RFC-039 D60's "all call sites converged
> to it" enumeration missed these two places**, belongs to declaration-fact mismatch, and cited line
> numbers were not re-taken on referenced revision, D60 text corrected along with this item; ② Batch
> verification tree (worktree) `cargo fmt --all` error is a worktree workspace artifact, gate list
> should note fmt goes to main repo; ③ repair-type contract template should pre-set mechanically
> necessary surface into inScope (see 3.5).
>
> **5. Final gate (closure revision, local two implementation commits + one document commit pushed
> together—the only push in this batch)**: fmt --check 0 / clippy -D warnings 0 / `cargo test --lib`
> **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** / `--test yx_runner`
> 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 / corpus_probe **342
> lines consistent with baseline line-by-line** (only regenerated line: `named_arg_duplicate_err.yx`
> E1015@14 → E1111@16, expected code and line number synchronized, manually reviewed) / IR snapshot
> zero drift.
>
> **6. Run links (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (MSRV/Test of same run cancelled by subsequent push
> per concurrency—cancellation≠failure, covered by successor run); governance tool push → CI
> 38546373435 / 38561405117; WBS registration (3.16 line + table header 56→57 / 147→148) →
> CI 38528560364. Closure push's own CI / Deploy Docs run supplemented task record after push.
>
> **3.4.8 ③④ Implementation (2026-10-11, 8ecf56f93)**: ③ Added `Linking` to end of `CHECK_STAGES`
> (**pure merge**, entirely skipping `is_bin_role` entry check—E3020 criterion still belongs to
> `RoleClassification` to prevent double report; another hard reason: run path determines "has main"
> by merged IR, under collection semantics "entry itself errors→no IR" would be misjudged as missing
> main and hard-aborted) → `Monomorphization` (check form); `ir_generation_check` retained unit IR
> (**memory only**, for pure merge and mono consumption, not emitted/written/produced snapshot—no
> retention means aggregation request is empty, mono gives `NoObligations` false green); mono
> failure in Check goes per-file collection channel (diagnostic attributed to entry file +
> `failed_stages`, not set failure), MultiFile keeps `fail_with`; `into_outcome` changed to "first
> take collection then judge failure". ④ Verified by **zero production change**: full program BFS
> already covers cross-unit mutual recursion (pathological → E3005, well-formed three-layer chain
> executes correct value), delivered four groups e2e + one mono unit test as living criterion.
>
> **Verification chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross-comparison with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-form matrix
> (single/multi/check three forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increase no decrease).
>
> **F1 (found during verification, D59 class A/B, same round fix)**: Without project root,
> standalone multi-file check, aggregation request name (`lib.f`) and unit IR definition key (`f`)
> come from different sources → mono falls into "native has no IR definition" silent `continue`,
> check reports 0 error/exit 0. Fix: aggregation side added lookup jump by each unit's **actual
> definition key** (precise enumeration + exact match + ambiguity abandon, not suffix heuristic;
> project form not registered, existing path unchanged). **Errata (same day)**: This lookup jump was
> **deleted** in subsequent regression fix; root cause changed to `module_key` dual-use decoupling
> (added `script_mode`) solved from naming caliber—IR definition key = IR call name = aggregation
> request name = `{unit key}.{name}`. **Regression record**: After ③ landed, CI's
> `Check tutorial/guide code examples compile` red (258 blocks / 10 failures: 6 same-file same-name
> rebinding + 4 colliding with `<std/list>`, rootless check doesn't qualify triggers
> `link_module_irs` conflict check); fix = ir_gen split `script_mode` (qualified name ⊥ Script/Bin,
> E3023 gate changed to look at it) + `ir_generation_check` per-unit always pass unit key +
> `.with_script_mode(rootless)` + Script top-level same-name rebinding "latter takes effect"
> (consistent with run path existing behavior); verification = doc gate 258/0, three binary
> comparison (HEAD/③ tree/fix tree) three collision fixtures, check form E3005 no regression, E3023
> not opened.
>
> **F2/F3 (accepted)**: mono diagnostic attributed to entry file entry, span can cross units (same
> caliber as run path `path=entry`); Check form holds one ModuleIR per unit (memory only, released
> when check ends).
>
> **Issues left for follow-up (not this round's regression)**: ① E3005's help text still writes
> "internal error; please report this issue", inconsistent with resource protection semantics
> (precedent: `e3xxx.rs:47` once moved similar "user writing error" out of E3005)—registered pending
> scheduling; ② multi-file E3005's span rendering label misalignment (already reproduced with HEAD
> binary, pre-existing); ③ Request collection side has **type parameter position dependency**
> (generic calls where type parameter is not at value parameter position don't issue instantiation
> request, mono records `NoObligations`, converging mutual recursion is exactly this form)—same
> surface as 3.5 (HKT call path), registered to that line.

## External Track RFC-029g: Remove `pub` and Auto-binding (#399)

> **P3.5 Implementation (2026-10-08)**: All seven second-level tasks completed, entire batch in one
> commit (independently revertable).
>
> - **Measurement inconsistent with plan (truthfully recorded)**: 029g.1.2/029g.1.3's acceptance
>   originally wrote "`pub f = ...` reports **parse error**"—actually after removing `KwPub`, `pub`
>   falls back to normal identifier, old writing falls to semantic layer reporting **E1001 Unknown
>   variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specifically detecting removed
>   syntax is anti-pattern", **do not add** migration-specific diagnostic; corpus fixture
>   `pub_declaration_err.yx` pins E1001, document (`syntax.md`) truthfully states the consequence.
> - **Design convergence (beyond RFC literal, requires review)**: `project_refs` and
>   `cross_file_refs` two fields have completely overlapping semantics after pub removal (same
>   "package-internal reference union", rule 1 prohibits parallel representation) → merged into
>   single `project_refs`, `set_exempt_pub`/`set_cross_file_refs` two switches deleted. Behaviorally
>   Internal role's non-pub items change from "file-internal reachability" to "reference pool
>   judgment" (relaxed, prefer false negative direction); Lib tightens from "pub absolute exemption"
>   to reference pool judgment; Script top-level bindings not referenced start reporting
>   W1001/W1002/W1004/W1005.
> - **Dead table cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports writeback all deleted (production-side table
>   filling, all-repository zero consumption).
> - **Test migration** (test count not decreased): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub recognized" assertion reversed to "pub no longer
>   modifier"; `passes/tests/dead_code.rs` 7 role semantic cases rewritten (assertion reversed 2);
>   `tests/integration/cli_e2e.rs` 8 role cases rewritten (`lib`/`internal` exemption assertions
>   reversed to report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` doesn't participate in IR); corpus diff baseline
>   only added 1 entry (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerated then manually reviewed.
> - **Measurement**: lib 2556 / integration 272 / corpus 322 entries baseline all green; `cargo fmt`
>   / `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-layer Contract PlanId-ization (D20)

> **4.5 Implementation Record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, closure
> (registration / WBS check / gate / push) completed by t7. Precondition = 4.3.1 obligation ledger
> (red state archive 173 files / 533 keys).
>
> **1. Carrier ruling (A: `PlanId` follows AST node)**: Added `PlanId` (`src/util/plan_id.rs`, `u32`
> monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` represents synthesized nodes
> outside parser), issued by **unique allocator** `ParserState::alloc_plan_id`; carriers in two
> places—`Stmt.plan_id` (statement identity) and `Expr::Call.plan_id` (call site identity),
> `Expr::call` is the only call constructor in the entire repository. The denied one is "both sides
> derive same identity from span" position coincidence form (D20's original defect).
>
> **2. Two-level verification and review (members independent)**: Each functional commit has an
> independent verifier and a reviewer, **verification/review do not reuse implementer evidence**:
> 4.5.1 = t1 implement / t2 independent verification (external tree + 5 mutations) / t3 review;
> 4.5.2 = t4 implement / t5 independent verification (external tree + 5 mutations) / t6 review.
> Review time points and reviewed revision both fall on immutable history (`git show` verifiable).
> **Provider rename caused t5 / t6 member replacement**: original verification/review members could
> not continue due to provider rename, replaced per orchestration agreement and **clearly noted in
> record** (cross-round context preservation priority, replacement only when context unavailable);
> replacement conclusion consistent with original round (PASS), no implementer evidence reuse.
>
> **3. Two defects found and fixed during implementation (t1, both within inScope)**: ① Consumption
> exemption criterion was hung on `UnitState.ir.is_some()`—`check` form's Linking/Mono would take
> away `unit.ir`, real unconsumption in check form **silently passed** (mutation test: D41 red but
> CLI check exit 0); changed to read **ledger's own completion marker**, and added regression case.
> ② Drop sequence drifted with process hash seed (partial order `sort_by` + `HashSet` iteration):
> snapshot criterion ran three consecutive times yielding 106/120/114 line count drift; changed to
> **total order sort** (same name takes max depth + name ascending) then ran twice zero drift.
>
> **4. Criterion and evidence numbers**: After 4.5.1, `test_release_plan_spans_consumed` removed
> `#[ignore]` always-resident (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated
> same submission (programmatic review: added 1083 lines Drop + 86 lines jump index translation,
> zero deletion, no rearrangement), corpus 342 entries diff empty; 4.5.2 added
> `test_overload_resolutions_consumed` (difference set empty + evidence surface floor `examined ≥ 1`
> / `consumed ≥ 2`; measured examined=1, consumption entries 2, no resolution skip 210), resolution
> form 9 pins, `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero drift**. Mutation
> driven: consumption key offset by one ⇒ criterion red + real program
> `error [W1090] ... produced key not consumed` + corpus gate difference; after restoration green
> again.
>
> **5. Semantic correction classification basis (D59)**: `Type.method(positional, name = value)`
> **silently gave wrong answer** before change (measured compile clean, run output `1` not `20`) ⇒
> **Class C** (old behavior not successful), fix takes "loud": same program after change
> compile-time `error [W1090]`; "named argument participates in resolution" final state semantics
> belongs to **Class D**, registered in §3.14 and left issue #452 for special team to take over.
> Also: `Drop` in interpreter is no-op (`backends/interpreter/executor/ops/control.rs` wildcard arm
> only `advance()`)—key migration doesn't change runtime behavior.
>
> **6. F-1 / F-2 disposition (t5 independent verification found)**: **F-1 (medium)** No binding
> consumption point (`ir_gen.rs:7766-7790`) unreachable in current corpus and test surface, key
> correctness has no red state to falsify ⇒ registered as "known boundary" in
> [02](02-stage-contract.md) §2 revision note fifth paragraph (keep reason + no new task); **F-2
> (low)** Pointer drift ⇒ `types.rs`'s `OverloadResolutions` document comment changed to precise
> pointer (production `inference/expressions.rs:4550`, consumption `ir_gen.rs:7697` / `:7772`),
> separate small commit, not mixed into doc commit.
>
> **7. Closure (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settlement marker);
> 07 criterion table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 /
> 4.5.2; CHANGELOG recorded one line behavior change; `check-obligations.py` and `check_tracking.py`
> green, CI and Deploy Docs green. **Closure found accounting deviation**: t3 / t6 report's pointers
> `ir_gen.rs:357` / `:3102` / `:1096` differ from code fact by one to three lines (actually `:360` /
> `:3103` / `:1095`+`:1097`, due to 4.5.2 insertion)—code fact as standard, this note also keeps
> record.

## P4 4.10.3 Standalone run and MultiFile Orchestration Unification (4.2.5 Ruling A's run half)

> **Third batch implementation record (2026-10-15 registered → 2026-10-15 closed)**: User 2026-10-15
> ruling third—this round only does 4.10.3 (same batch candidate 3.4.6 per B1
> ([02](02-stage-contract.md) §Runtime behavior table "dead code family warnings don't enter compile
> path") deferred until after 4.9, preconditions re-attached and noted); implementation form =
> **complete single team** (user instructed no DAG, no team, single session implementation +
> self-verification). Pre-batch debt first landed: #455 dependency announcement fix (rustls
> 0.23.46 + serde_yaml_ng, `cargo audit` from exit 1 recovered to exit 0), WBS registration debt
> (#456 registered as 3.17 + incomplete lines supplement "belonging phase / whether blocks main
> line" field).
>
> **1. Red before green**: driver-level red criteria 3 entries
> (`src/driver/tests/run_rootless.rs`—rootless merged IR must not carry entry_function (red state
> `Some("main.main")`), rootless top-level executable statement must not report E3023 (red state
> produces no module), Bin control entry_function kept (green state always-resident)) + CLI e2e 3
> pins (standalone relative import run (red state E5001), Script top-level statement execution and
> #413 hint two green state pins).
>
> **2. Implementation side**: CLI `run` uniformly goes through `compile_project` (deleted
> `run_file_with_diagnostics`'s project/standalone bifurcation); Linking's `entry_function` gated by
> `is_bin_role` (#413 T4—Script doesn't auto-call main); #413 hint key changed from
> `module.source_files.is_empty()` to call site project probe (after unification ModuleIR always
> carries source_files, original marker fails); `script_main_never_called` adapted to qualified name
> form (function `{key}.main`, global slot also measured as qualified name—IR evidence-based, not
> inferred from comments).
>
> **3. Existing gaps exposed by unification (same batch road repairs, class A/B/C—only fix failed
> programs to successful, original successful programs zero impact)**:
>
> 1. **Explicit type argument pre-specialized name** (`mk(Int)(5)`): ir_gen directly sends
>    specialized name per D6.3 (mono will strip unspecialized original), `resolve_call_target` three
>    arms (rename / aliases / dot prefix) don't cover `mk(int64)` form ⇒ call site dangling E6006.
>    Fix = add fourth arm at this single point parser (base name qualified, type parameter string
>    preserved as-is). **Bin project isomorphic reproduction (pre-existing defect, not introduced by
>    this batch)**.
> 2. **Script top-level `mut` assignment statement duplicate slot assignment**:
>    `extract_global_defs` and `register_global_slots` count `counter = counter + 1` (same-shape
>    Assign as declaration) per statement ⇒ same variable assigned 3 slots, initialization writes
>    the later-assigned slot while read uses the earlier-assigned slot (runtime Add type mismatch).
>    Fix = merge by name in both places (same criterion same source).
> 3. **Same-module constraint generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidate + table lookup verification (precise hit →
>    generic's unit key qualified hit → fall back to bare name status quo; not suffix heuristic,
>    same form as `resolve_generic_name`).
>
> **4. Registration gap (this batch does not fix)**: Constraint dispatch in **type and generic
> definition in different modules** still E6006 (rootless two-file probe verified; Bin
> isomorphic)—root cause = MonoType user type name doesn't carry definition module, correct fix
> belongs to P6 type identity convergence; registered WBS **3.18** (belongs to P3, preconditions
> pending P6, doesn't block main line), `qualify_deferred_method` noted same surface.
>
> **5. Test adaptation (honestly recorded)**: `tests/integration/multifile.rs` fixture supplemented
> manifest—it tests RFC-029 project orchestration (Bin auto entry) not Script semantics; `cli_e2e`
> one pin (top-level diagnostic variable name visibility) and `script_main_hint` fixture (7 cases)
> synchronized by qualified name form—production consumption side only eats merged IR, fixtures
> directly drive same form, no need to use pipeline product stand-in.
>
> **6. Final gate (closure revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new pins) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> diff baseline regeneration **12 lines**—line-by-line structured verification: exit / stdout /
> diagnostic code all line-by-line same, only stack frame function name and variable name qualified
> by unit key (manual review passed); after regeneration `check-corpus-parity.py` exit=0.
>
> **7. Process events (truthfully recorded)**: This batch once concurrent editing workspace document
> during pre-commit hook run, was eaten by hook's stash rollback and that commit failed (concurrent
> form predicted in config comment)—discipline = don't touch workspace during hook run; #455's
> commit therefore split into two attempts (second fully green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closure)

> **4.6 batch implementation record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16
> instructed to advance 4.6 → 4.9 → 4.4 → P5 in order, first do 4.6; implementation form =
> **complete single team** (following previous batch user instruction: no DAG, no team, single
> session implementation + self-verification). Task definition (09 three-level table):
> project-internal file semantic data (SemanticDB) goes through orchestrator same pipeline, same
> source as diagnostic; cross-file references carry `resolves_to`. Acceptance: project-internal
> cross-file jump hits definition; semantic data and diagnostic come from same compilation.
>
> **1. Current state empirical (verify before assert)**: didOpen/didChange triggers **two trips** of
> compilation—diagnostic goes through unified Driver pipeline (4.2.3/4.2.6 already unified),
> semantic data goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registry ⇒ project-internal
> `use` resolution fails, cross-file reference events not produced, jump/rename/reference lookup for
> imported symbols silently fails—D55 registered silent failure). Orchestration arm's checker
> unified naming `"<module>"` (SemanticDB file identity lost), `type_result` (containing
> semantic_db) discarded when diagnostic non-empty and never emitted; `checker.rs`排空 covers
> `resolves_to`'s DefId file identity to current file; `Export`/`ModuleInfo` have no definition
> position surface.
>
> **2. Red before green**: LSP e2e pin (`src/lsp/tests/cross_file.rs`)—temp two-file project didOpen
> main.yx then send GotoDefinition on `add_one` call site, **returns None before fix (red,
> empirically silent failure from above section)**, hits lib.yx line 0 definition after fix (green).
> Probe positioning during process: compiler-side chain (driver probe dump) was fully connected but
> e2e still empty—root cause is `resolve_reference` precisely matches by reference **start point**
> (D55 precise-only current caliber), test cursor falls in identifier middle; this tolerance is
> existing behavior surface, this batch doesn't touch (see 7).
>
> **3. Implementation side**: (1) `Export.definition_span` (extract_module_info self-harvests from
> AST identifier node, same source same value as typecheck definition event's `def_span`; native std
> always None) + `ModuleInfo.definition_file` (Registry arm fills by DiscoveredFile, including
> vendor; std always None); (2) checker reference排空 via `import_def_locations` (use line binding
> span → definition file + definition span) redirects cross-file imported items' `resolves_to` out
> of this file; std import not registered, maintains falling to use line; (3) checker module
> identity changed from unified `"<module>"` to unit file path (verified `env.module_name` all 4
> consumer points zero diagnostic consumption); (4) `DriverOutcome.semantic_db` channel—CollectAll
> form (Lsp target file **regardless of diagnostic success or failure** + SingleFile fallback)
> merged in, Lsp arm tail injects non-target file top-level export definition surface from registry
> (`external_definitions`, unopened files can also jump; type/submodule exports not
> injected—typecheck side type registration doesn't produce definition events, both sides maintain
> consistent gap); (5) `orchestrator::analyze_source_in_project` (`check_source_in_project` retrans
> to take diagnostic half); (6) LSP `analysis.rs` single compilation entry replaces manual pipeline
> (`update_semantic_db` deleted), World key uniformly normalized file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
> differences normalized to same key, cross-file DefId therefore stably hits); 5 handlers +
> semanticTokens dual requests connect `db_key`.
>
> **4. Road repairs (D59分流 class B—old behavior silent no-action, original successful programs
> zero impact on external performance)**: ① Project-internal file cross-file jump/rename/reference
> lookup changed from silent failure to hitting real definition; ② When lexical error, old manual
> pipeline deleted semantic information (highlight disappeared) → residual AST highlight preserved
> (4.2.6 editor philosophy extension); ③ When target file has type error, semantic data no longer
> discarded along with `type_result` (typing intermediate state highlight/jump doesn't disappear).
>
> **5. Pins**: driver-level 6 (`src/driver/tests/lsp_semantic_db.rs`—target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface round-trip,
> std import fallback to use line, error file preserved, single file fallback path key, single file
> error preserved) + LSP e2e 1 (red before green) + uri normalization 4
> (`src/lsp/tests/uri.rs`—drive letter conversion, space encoding, idempotent normalization,
> non-file scheme passthrough).
>
> **6. Mutation verification (scratch.py trial, mechanically reversible)**: Batch tree wbs-46 @
> closure revision 39546020, two mutations both caused hit pin to turn red, after restoration blob
> consistent with HEAD + git status CLEAN—m1 removed `import_def_locations` registration
> (statements.rs) ⇒ cross-file driver pin + LSP e2e pin red, other 7 green; m2 removed
> `external_definitions` injection (arms.rs) ⇒ same two pins red (round-trip resolution lost
> anchor), other 7 green. Batch tree gc'd (released 4.8 GB, `status` no batch tree).
>
> **7. Final gate (closure revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new pins) / `--test integration` **287 passed** / corpus
> diff **byte-by-byte zero drift** / IR snapshot zero drift / check-obligations PASS (13·13·0·8,
> `semantic_db` non-obligation reason consumer anchor synchronized server.rs:482 and 02 §obligation
> field table) / check-docs-truth 0 / docs-examples 258 entries 0 failures / docs-orphan 0 /
> markdownlint only remaining user-uncommitted 032 two MD056s (user's own edit, untouched) / pnpm
> docs:build 0. **Process events (truthfully recorded)**: en/09a-wbs-history.md:54 one MD037 was
> introduced by auto-translate bot retranslating previous batch 09a (zh original has no space), this
> batch fixed it back—markdownlint red not this batch's change surface but same file as 09a,
> registered together. **Outstanding observation (registered pending discussion, untouched)**:
> `resolve_reference` only precisely matches by reference start point—cursor falling in identifier
> middle jump/hover/rename silently no-action (D55 precise-only current state, all-file form
> isomorphic), if tolerance matching needed is independent behavior change, report to user for
> ruling.
>
> **Same day continuation batch (4.6.2, landed after user ruling)**: After delivery report, user on
> "outstanding observation ①" ruled "OK, then change"—reference key tolerance implemented per plan
> A: definition/hover/references/rename four handlers' reference key changed from cursor position
> (after UTF-16 conversion) to `find_identifier_at_position` already located **identifier's own
> start point** (same coordinate system as semantic event, no conversion needed);
> `resolve_reference`'s start point precise match semantics unchanged (D55 precise-only unchanged).
> references/rename's definition point fallback judgment (start <= col < end) holds for identifier
> start point isomorphic. Red before green: definition middle cursor pin first red (None) then
> green; four handlers middle cursor each 1 pin + cross-file e2e start/middle dual position
> assertion, `--lib` 2748 (+4) all green, clippy -D warnings 0. Registered WBS 4.6.2 (three-level
> count 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction)—Round 1 Closure (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 batch implementation record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16
> ruled to merge three related three-level tasks **into one team one round** (reason: they all
> belong to "pool's source and identity" surface, combined landing can avoid repeated rewriting of
> intermediate state after `collect_project_refs` deletion; 4.9.4 / 4.9.5 belong to consumption
> refinement and test pool separation respectively, depend on 4.6.1's `use` path resolution, forcing
> into same batch would expand gate surface). #450 attribution **pending ruling**—evidence indicates
> belongs to "name not passed back to reference count after overload resolution" class, still
> reproduces in round 1 landed form, registered per independent WBS item for disposition (captain
> recommends A: register as independent item in round 2; **pending user ruling**). Frozen rev =
> `3863a9ff0` (parent commit `62f6e1a59` is pre-fix baseline); verification PASS attempt `aab92fe0`
> (i.e. t2); review pass attempt `96452696` (i.e. t3). **Closure registration commit = `38d30b846`
> (docs(docs): 4.9 reference pool round 1 closure registration)**—this was actually written and
> pushed by closure member integration-refpool; its closing probe once mislabeled closure task as
> terminal failed and overwrote output, captain took over new attempt and changed back to completed
> per verified delivery fact, hereby correcting attribution and label. Evidence archive: Evidence
> summary (8 mutation patch list, `warn_compare_v8.json` caliber and conclusion) inlined in this
> section "3. +8 and +16 two count sets correspondence" and "4. mutation experiment"; original files
> recycled with workspace, **not included in repo, must not be used as document reference source**
> (AGENTS meta rule ①).
>
> **1. Merge ruling**
>
> 1. **Ruling itself**: Three related three-level tasks merged into round 1. **4.9.4 (consumption
>    refinement) / 4.9.5 (test pool separation) next round**—former depends on 4.6.1 delivered `use`
>    path resolution (already landed at `39546020`, can start); latter same batch as RFC-029f table
>    compilation, registered at RFC-029f end section pending.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 are mutually preconditional (4.9.2 changes pool type →
>    4.9.1 assembly surface needs rewrite → 4.9.3 has denominator judgment role table), doing in
>    rounds inevitably writes intermediate state then changes twice; (b) All three belong to "pool's
>    source and identity" surface, 4.9.4 / 4.9.5 belong to another surface, batch division is by
>    "cohesion" not "independently revertable".
> 3. **Merge result**: Single commit `3863a9ff0` (one line `:recycle: refactor(driver)`), single
>    push, single batch gate. **Hash explanation**: Before push `git pull --rebase` merged
>    auto-translate bot commit (`72f364169`); rebase fall point after the other's commit, this
>    batch's two (implementation `3863a9ff0`, its pre-fix baseline兼 other's doc commit `62f6e1a59`)
>    **not rewritten, maintained original hash**—initial version mistakenly wrote here as
>    "`3863a9ff0`→`3863a9ff0`" identity mapping (looks like "after rewrite same value", actually
>    "not rewritten"), corrected at 2026-10-17 closure; other's code commit `33f14e8ab`→`b122b7ad1`.
>    **External references uniformly use post-push hash** (rewritten old hash unreachable in new
>    clone, identity determined by patch blob comparison).
>
> **2. Five criteria red/green measurement caliber**
>
> | #   | Criterion                                              | Red state (pre-fix / P1 directory scan regression patch)                                                                                                   | Green state (round 1 landed)                                                                                                                                                                                                                                                                                                                                                                       |
> | --- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | ①   | Pool's root = package target (#334)                    | Directory recursive scan ⇒ tests/ files' identifiers also enter pool, exempting production code's unused definitions                                       | `collect_package_refs` (`helpers.rs:481`) by `ExplicitSurfaces` root calls `discover_with_used`; all roots degenerate fallback `collect_compilation_refs` (`helpers.rs:454`)                                                                                                                                                                                                                       |
> | ②   | Pool granularity = `(module, name)`                    | Same-name local variable (`let foo = …`) accidentally exempts same-name top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` cross-contaminate | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) unique assembly entry; `arms.rs:603-609`'s `other_module_refs` takes all module entries except this unit union then sends to `DeadCodeAnalyzer`; module's own names (top-level definitions and local bindings, `use` import name exception) no longer exempt other modules' same-name definitions |
> | ③   | `FileRole::Script` doesn't use pool                    | Script affected by other files' references in project                                                                                                      | `arms.rs:611-623`'s `match us.role` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`), Script/Test goes `_ => {}` default arm (judged by file-internal definition-usage graph)                                                                                                                                                                              |
> | ④   | Fixture A: lib.yx line 4 definition must report W1001  | Pre-fix: reports `W1003@1` (actually "unused import"), not W1001                                                                                           | Red state measurement (red 1 fixture red1) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                                      |
> | ⑤   | Fixture B: main.yx line 4 definition must report W1001 | Pre-fix: reports `W1003@1`                                                                                                                                 | Same as above                                                                                                                                                                                                                                                                                                                                                                                      |
>
> **Criterion ②'s "deduction red"** (implementer self-stated + verifier closed): red2 fixture's
> `let foo = …` is in helper.yx; old pool under bare name union treats `foo` as "used by helper",
> lib.yx's `foo` exempts. **Measurement**: red2 fixture vs pre-fix binary → no W1001 (old caliber
> exemption); red2 vs post-fix binary → W1001@1.
>
> **Criterion ③** (Script doesn't use pool) red state fixture **first version failed**: First
> version fixture put same-name local variable in same file triggering E2010 (the language doesn't
> allow same-module shadowing top-level definition), after moving to another module (helper.yx) it's
> deduction red. This point t3 review and t2 verification both independently reproduced, **not
> contract defect**—same-module shadowing is the language's hard constraint, fixture must
> cross-module.
>
> **3. +8 and +16 two count sets correspondence** (**with t3 review verification as standard**)
>
> 4.9 round 1 evidence summary's section C writes `+16/−0`, but missed "**new fixture records**"
> component—v8 instrument (`warn_compare_v8.json`, 331 files symmetric diff, `only_before=0`)
> caliber **includes** new fixtures; t1 implementer's self-built "+8" is **warn-before / warn-after
> (each 325 entries) aggregated by fixture** (each member file checked once). Review item-by-item
> mapping:
>
> - **+8** (8 new warnings, all `check <non-entry file>`, `check <entry file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (unified attribution)**: These 8 fixtures all **didn't declare package target** ⇒
>     4.9.1 root degenerates to checked file's own compilation set, consumer main.yx not in it; this
>     is design-intended change (before these files' check surface had zero warnings, not exemption
>     path). Repository root `yaoxiang.toml` is empty file ⇒ same degenerate path empirically holds.
> - **+16 residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files in t1's `warn-before.jsonl` have `exit=None` (capture time point
>     **doesn't exist**—is this batch's newly added fixture). So +8's "+ 0 − 0" diff doesn't have
>     them, +16's v8 symmetric diff does.
>   - `v8`'s `stray.yx` only has `W1001@2(probe)` left: because pre-fix binary also reports
>     `probe@stray.yx:4` (this fixture runs `probe` once per file), **cancels** leaving only lib's 1
>     entry—three data sources (+8 / +16 / dual binary ABBA) mutually consistent.
> - **8 new fixtures** vs **+16 residual 8**: Same batch of files (4.9.1 / 4.9.2 newly added
>   `ref-pool-compile-set/` and `ref-pool-module-identity/`), instrument treats each of their checks
>   as independent record—same file may trigger different diagnostic entries for different
>   `role/context`.
>
> **Conclusion**: **+16 = +8 (by fixture aggregation) ∪ 8 new fixture entries**—both counts are
> correct, **different calibers**: t1 reported "added 8 entries" is fixture-level caliber, t2
> reported "+16 entries" is file-level caliber (symmetric diff includes new fixtures). t3 review ①
> +8⊆+16 verified; **review ② writes "do not copy EVIDENCE.md section C"**—section C missed "new
> fixture records" component, this item rewritten per t3 review `pass` conclusion.
>
> **4. Mutation experiment (mechanically reversible, all `scratch.py trial`)**
>
> 8 mutation patches (p1 directory scan, p2b use import name removal, p3b script joins pool, p3 flat
> consumption, p4 std joins pool, p7final self-built 4 criteria, p8 std close, p9 vendor joins
> pool + p_stack2 / p_stack3 overlay surface). Each trial output contains: HEAD blob → mutated blob
> → restored blob consistent with HEAD = yes + `git status` CLEAN. Local `core.autocrlf=true` needs
> `GIT_CONFIG_*` injection `false` (`trial_env.ps1`), otherwise patch added lines convert to CRLF
> causing apply/restore dual failure—**only affects patch mechanicality, doesn't change tested
> behavior**.
>
> **5. Process events / environment lessons (must read for handoff)**
>
> 1. **core.autocrlf injection**: Local `core.autocrlf=true` must inject `false` via `GIT_CONFIG_*`
>    to ensure patch mechanical reversibility; injection script at that time recycled with workspace
>    (criteria listed in previous item, can be rewritten accordingly). **Next round do not reuse**
>    any trial script without injection.
> 2. **Same tree trial serial**: `trial` and `cargo build` on the same tree **must be strictly
>    serial**. `trial`'s internal `git apply` and `cargo build` both touch worktree, parallel will
>    see two kinds of races: "patch incomplete seen by build" or "build and apply deadlock".
> 3. **Trial rebuilt binary needs sync confirmation**: After `trial` completes, will rebuild
>    tree-internal binary (cargo's fingerprint contains source path, **tree change = full
>    rebuild**), subsequent measurement must confirm `bin/yaoxiang-rs` is synchronized with HEAD.
>    **This round once used residual mutation binary to measure false data**, already rebuilt
>    re-measured and corrected.
> 4. **Second tree yx49-b's tree creation reason**: Need pre-fix binary (62f6e1a59) for
>    cross-comparison, no ready product in batch; `scratch.py init yx49-b` opened second tree (one
>    more full build). **Tree released** (see 6), deletion left batch closure `gc` disposition.
> 5. **cli_e2e two Internal cases fixture supplemented to legal package (supplement `[run].main`,
>    assertion unchanged) reason**: Pre-fix `cli_e2e` two `Internal` role pin fixtures **didn't
>    declare `[run].main`** ⇒ after 4.9.1 landing `compile_project` goes `ExplicitSurfaces` root
>    call path, can't find `main` ⇒ entire batch 2 fixtures behavior drifted from "Script fallback"
>    to "can't find entry"—is 4.9.1 introduced fallback behavior not defect. After supplementing
>    `[run].main` field, two fixtures go normal package path, **assertions** (exit code / diagnostic
>    code / warning set) **unchanged verbatim**.
> 6. **Second round tree deletion safe way**: After release use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`), will verify "no mount" then delete.
>    This batch before closure already released, next step `gc` naturally cleans up.
>
> **6. Batch tree status**
>
> Before closure: yx49 released (4.8 GB released), yx49-b released. Two trees no mount, status
> verifiable in `scratch.py status`. This task output ends with `scratch.py gc` complete status.
>
> **7. RFC-029f "Known Defects" table flip**
>
> Previous version (2026-10-09) used present tense to describe "three design defects in reference
> pool" (including `collect_project_refs` / `orchestrator.rs:603-648` / `orchestrator.rs:393-396`
> etc. deleted code locations). This batch closure synchronously rewritten as "**Known Defects:
> Landed (2026-10-16, WBS 4.9 Round 1)**": ① Current pool's root = `collect_package_refs`
> (`helpers.rs:481`), parses by `ExplicitSurfaces`, not directory scan; ② Current pool type
> `HashMap<String, HashSet<String>>` (key = module key), unique assembly entry `collect_ref_pool`
> (`helpers.rs:429-449`), consumer end `arms.rs:603-609`; ③ Current Script doesn't join pool,
> `arms.rs:611-623`'s `match us.role` default arm. **All file:line retaken on referenced revision
> `3863a9ff0`**. `collect_project_refs` and `orchestrator.rs` moved with P4 4.2.9 split (`c59fb751`)
> into `src/driver/{state,arms,helpers}.rs`, old locations in table no longer hit—this is
> "declaration-fact mismatch" fix (t3 review note ①).
>
> **8. Final gate (closure revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 lines** (two new fixtures
> behavior run, 338a339,340; other 342 lines byte-by-byte unchanged) / IR snapshot 204 zero drift /
> check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process event**: This batch
> once cargo build failed because `core.autocrlf` converted script line endings to CRLF, team
> scratch path `yaoxiang-rs` start reported `BadImage`, empirically encoding and git index
> inconsistent—after rebuild passed (parallel with 4.6 batch "external whole tree discard" lesson,
> different cause).
>
> **9. Outstanding (handoff next round)**
>
> 1. **#450 attribution ruling**: Evidence indicates belongs to "name not passed back to reference
>    count after overload resolution" class, **still reproduces in round 1 landed form**—assigned
>    per user ruling third process. **#450 probe**: dual binary same set `{W1002@2, W1005@4}`
>    (measured, rootless form), no direct relationship with round 1 transformation, but #450's fix
>    may need to wait for 4.9.4 (consumption refinement) together—because root cause "name passed to
>    reference count after overload resolution" needs use item's precise consumption surface,
>    4.9.4's "real use after import" is precise version of same criterion.
> 2. **dead_code.rs:683 pointer nit**: Comment writes "two pools' assembly caliber see driver side
>    collect_compilation_refs"—actually `collect_compilation_refs` is fallback path, main path is
>    `collect_package_refs` (`helpers.rs:481`). Comment and code main path caliber inconsistent
>    (**not in this batch's write scope**), left to be corrected when 4.9.4 / 4.9.5 land.
> 3. **4.9.4 / 4.9.5 preconditions unlocked**: 4.6.1 (`use` path resolution) already delivered by
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs to define RFC-029f
>    table compilation first (already paved by this batch "7." flip).
> 4. **`yaoxiang check` multi-file `run` never reports dead code family warnings** (WBS 3.4.6,
>    2026-10-15 user ruling third deferred until after 4.9)—this batch round 1 not involved; next
>    round 4.9.4 / 4.9.5 land at same time handle.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction)—Round 2 Closure (4.9.4 / 4.9.5 / 4.9.6)

> **4.9 Round 2 batch implementation record (2026-10-16 implementation → 2026-10-17 closure)**:
> Covers 4.9.4 (consumption criterion = real use after import), 4.9.5 (test pool separation), 4.9.6
> (method reachability caliber, issue #450, user 2026-10-16 ruling A merged into round 2). Closure
> fix (verification side remediation M1/M2/M4 compilable redo + F1 fix round + PA-2 dead branch
> deletion) and registration gap issue #464 (WBS 4.9.7) all settled.
>
> **1. Judgment chain and commits (uniformly post-push truth)**
>
> | Step | Content                                                                                                                                                   | Post-push commit | Pre-push local value | Judgment method                                               |
> | ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- | ------------------------------------------------------------- |
> | 1    | fix(util): check explicit path not blocked by default exclude table + --exclude always empty (issue #463, P0-0.4)                                         | 366bc5a69        | ef959ca55            | Patch blob same                                               |
> | 2    | Phase A = 4.9.4 + 4.9.5 (RefPool by import attribution + Test doesn't contribute consumption surface)                                                     | e6cf1aeaa        | 84a6f6978            | Patch blob same                                               |
> | 3    | Phase B = 4.9.6 (#450 per-candidate method reachability; including generated warning-codes.md)                                                            | ddcbb12d0        | cd629b5fe            | Patch blob same                                               |
> | 4    | docs(meta): AGENTS rule 10⑦ (commit period stash window agreement)                                                                                        | ec94fa9d8        | 9a4ed5f3b            | Patch blob same                                               |
> | 5    | Fix round = F1 (overload sibling overload recovery reports W1005) + F2 (verification side M1/M2/M4 compilable redo) + F4 (snapshot generation annotation) | b8380e6ab        | 3d9ef6eab            | Patch blob same (36962f318e42…, git range-diff no difference) |
> | 6    | Closure fix = PA-2 (delete strip_unit_prefix dead branch + 029f/CHANGELOG three untruthful statement corrections, register #464)                          | b940300ac        | e13984465            | Patch blob same (patch-id 9bda3cfe4c…)                        |
>
> **Hash explanation**: Each push's pre-push `git pull --rebase` will rewrite local hash; "judgment
> method" column above is all "patch blob same" (`git diff <rev>~1 <rev> | git hash-object --stdin`
> both sides consistent + `git range-diff` no difference). This document doesn't reference any
> pre-push hash as authoritative source.
>
> **2. Three criteria red/green measurement caliber** (verifier self-built reproduction, not
> trusting implementer criterion)
>
> | Criterion                  | Red state (before binary 366bc5a69 measured)                                                                                                                                                              | Green state (ec94fa9d8 and after)                                                                                                            |
> | -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.9.4 import≠consumption   | Declaration surface project check a.yx only 1 W1001 (main's use b.{foo} + foo() revives a.yx same-name foo; guard only imported not used also revived)                                                    | Same fixture 2 entries truthfully reported (same-name collision not revived + unused import doesn't count as consumption); ref_pool 8 passed |
> | 4.9.5 test pool separation | src/util.yx's debug_print revived by tests/ reference (only 1 entry)                                                                                                                                      | Same fixture 2 entries (test reference doesn't spill); ref_pool's test_ref_pool_excludes_test_role_references passed                         |
> | 4.9.6 method reachability  | Self-built fixture (2 groups disambiguation overload call + 1 uncalled) 3 W1005 false reports; method_call_args.yx 3 entries / method_overload.yx 1 entry (overload by full name override only reports 1) | Same fixture exactly reports uncalled orphan; method_unused 3 pins passed; corpus call_args 3→0 / overload 1→0                               |
>
> **Fix round (F1) further tightening**: Sibling overloads that didn't hit resolution recover
> reporting W1005 (b8380e6ab—same fixture call ->Float reports ->String sibling, call ->String
> reports ->Float sibling, bidirectional line number pin). **Falloff boundary (registered #464)**:
> Cross-unit (registry resolution) method call doesn't produce resolution entries, falls back to
> name-level exemption, cross-unit same-name same-order previously not reported—registered 4.9.7.
>
> **3. Process events and lessons (four hard requirements correspond)**
>
> 1. **PA-1 changed from blocker to existing capability gap (medium)**: Round 2 review once
>    registered "cross-unit same-name same-order not reporting W1005" as blocker (PA-1); subsequent
>    two fact reconciliations (implementer archive + verifier dual binary re-measurement of same
>    fixture set in b8380e6ab/b940300ac, diagnostic set byte-by-byte consistent) confirmed it is
>    **capability gap existing before round 2** not this round's regression—mechanism = cross-unit
>    method call doesn't produce resolution entries, falls back to name-level exemption; disposition
>    = reclassify as medium, register issue #464, WBS 4.9.7 schedule next round.
> 2. **Reviewer publicly corrected before data and withdrew regression judgment**: Reviewer's
>    originally cited before data ("−42 entries contain called methods") didn't match their original
>    output (W1005 count=0)—verifier verified before capture was 0 entries, reviewer's regression
>    judgment **publicly withdrew**; −42 correct caliber = 26 files' called method false report
>    elimination (phase B), each item traceable to behavior-run exit=0 file (method really called
>    executed).
> 3. **t13 terminated due to contract premise not holding (not implementation defect)**: That task's
>    acceptance premise wrote "cross-unit same-name same-order will report W1005 regression fix",
>    but measurement (t14 dual binary) confirmed this behavior is registered gap (#464) not
>    regression—contract premise doesn't exist, task terminated; its probe value taken over by t16
>    read-only judgment (confirmed `strip_unit_prefix` is dead branch).
> 4. **Document commit's real criterion is "really build once"**: Round 2 closure's document commit
>    (`042c07860`) caused Deploy Docs red at VitePress's Vue template compilation—angle bracket
>    placeholders like `<rev>` / `<before-exe>` / `<after-exe>` etc. didn't enter backticks, were
>    parsed as elements (reported `Element is missing end tag`, pointing to `<rev>` line);
>    markdownlint can't catch this class, and pre-commit's markdownlint hook judged
>    `no files to check` to skip in doc-only commits (measured). Disposition: placeholders uniformly
>    enter backticks, before push locally run `npm run docs:build` once (fix commit `0c837e1b4`
>    evidenced by local build exit=0); if use `--no-verify` commit, commit-msg check needs manual
>    supplementary run.
> 5. **Contract description not synchronized process omission**: t14's task description text
>    retained PA-1 old hypothesis (wrote "b.yx reports W1005" expectation), verifier in report
>    proactively pointed out "judging by contract field, description text doesn't count" and asked
>    captain to confirm—disposition = task contract field as judgment basis, description text left
>    as process lesson (**when reassigning/assigning, task description must be synchronized with
>    latest judgment chain**).
>
> **4. Mutation experiment (mechanically reversible, all scratch.py trial)**
>
> Round 2 verifier total 9 patches (m1b/m2b/m4b are F2 post-compilable redo version—initial M1/M2/M4
> commented if condition line leaving dangling block, was voided by t8 review F2 judging "compile
> failure misremembered as criterion red"; redo version via per-patch cargo check exit=0 + trial
> turn red). M3 (same-name collision attribution regression) initial version was real mutation,
> measured ref_pool 5 criteria turn red, kept original judgment. M1b anchor retaken on b8380e6ab
> (after F1 fix probe segment refactor, old anchor invalid). All trials: HEAD blob → mutated blob →
> restored blob consistent with HEAD + git status CLEAN. Environment injection continues
> trial_env.ps1 (core.autocrlf=false via GIT_CONFIG_*).
>
> **5. Warning surface full cross-comparison (caliber explained first)**
>
> Instrument = warn_surface_r2.py (self-built parser, (file, code, line, column) four-tuple
> symmetric diff, project state channel cwd=file parent directory, full path key); command =
> `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>` (337 files × 2
> binaries). **Caliber**: This round before capture already includes all fixtures ⇒ matched set =
> full set 337, no round 1's +8 (matched increment)/+16 (full set) dual caliber problem. Result:
> only_before = 42 W1005 (26 files, all called method false report elimination = phase B's −42; per
> file check all on behavior-run exit=0 baseline, method really called executed—**no real dead code
> revived**) / only_after = 3 W1001 (ref-pool-import-use's a.yx:4 check surface ×2 +
> ref-pool-test-pool's ../src/util.yx:2 = phase A criterion surface). **Snapshot generation (F4)**:
> Phase A time point's after capture (phase B pre's 57 residual) and final version (phase B post's
> 15+2=17) are **two generations**, must be noted when citing; also 1 exception
> (interface_member_conflict.yx's Dog.name@18) belongs to field access form, not in phase B's W1005
> removal set, consistent before and after.
>
> **6. Final gate (closure revision b940300ac)**
>
> lib 2761 passed / integration 288 passed / IR snapshot zero drift (snapshot 7 passed) / corpus
> baseline zero drift (byte count three-way comparable: baseline = b8380e6ab = b940300ac) / clippy
> -D warnings 0 / docs-truth exit=0 / check_tracking exit=0 / generator already run no product
> change. **Gate check arrangement (rule 13)**: This batch's push CI / Deploy Docs / Auto Translate
> run conclusions submitted to scheduled check session for registration (run id + conclusion +
> consecutive push coverage: successor run covers previous one), not waiting in session.
>
> **7. Batch tree status**
>
> yx49 and yx49-b both released (closure time gc unified deletion); no this batch new directories
> outside repository.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction)—Round 3 Closure (4.9.7)

> **4.9 Round 3 batch implementation record (2026-10-17 implementation → 2026-10-17 closure)**:
> Covers 4.9.7 (cross-unit method call's resolution registration and receiver attribution, issue
> `#464`—WBS 4.9's last three-level task; after this round's closure 4.9 entire second-level task
> kanban checked). Chain = production side t19 → independent verification t20 PASS → review t21
> needs_revision (F1: injection surface false report) → fix round t24 → independent verification t25
> PASS → review t26 PASS (increment, verdict=pass). All commit numbers are post-push truth (judgment
> method = patch blob same; `git diff <rev>~1 <rev> | git hash-object --stdin` both sides
> consistent).
>
> **1. Judgment chain and commits (post-push truth)**
>
> | Step | Content                                                                                                                                                                                          | Post-push commit | Judgment method       |
> | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------- | --------------------- |
> | 1    | Production side: cross-unit method call produces D20 resolution entries (`feat(typecheck)`, t19)                                                                                                 | `b8b3cc3f6`      | Patch blob same       |
> | 2    | Driver side attribution: method reachability by definition unit gives mark, names with resolution records stop pool's name-level exemption (t20 verification PASS / t21 review catches F1)       | `d5c00235a`      | Patch blob same       |
> | 3    | Injection surface fix round F1: candidate injection by "name ← provider module" **per pair** filter, eliminate E1107 false report (t24, including F2 statement and F3 landing form single point) | `d5dd1a37b`      | Patch blob same       |
> | 4    | Pin test return: `src/frontend/core/typecheck/tests/environment.rs`'s F3 pin moved to declaration surface `src/driver/tests/overload_injection.rs`                                               | `c2f887843`      | Patch blob same       |
> | 5    | dev tip                                                                                                                                                                                          | `e9675c9dd`      | merge commit, see "2" |
>
> **2. Hash mapping and dev tip's two merges (source unconfirmed, must not write as member
> action)**: Round 3's four commits (above table 1-4) all pushed to origin/dev;
> `git worktree list`'s main mount at `e9675c9dd`. dev tip has two
> **non-this-team-command-surface-produced** merge commits (`5da05b885` / `e9675c9dd`, reflog shows
> `pull --tags` + push form), source unconfirmed—per rule 10④ (references must be resolvable by
> anyone) and rule 13 (gate submitted to dedicated check) same closure caliber, here only register
> fact and reflog evidence, no member attribution; also see "5." lesson ② (push attribution
> agreement must be verifiable).
>
> **3. D20 contract extension actual form (key form and consumption points)**: Production side
> packages "candidates / mixed IR name / declaration-period parameter name" three parallel tables
> **in same order** into `ModuleInfo.method_overloads` (`src/frontend/module/mod.rs:157`) following
> unit boundary, registry merges by module key ascending deterministic
> (`src/frontend/module/registry.rs:150`); typecheck arm injects per pair by this unit's import face
> (`src/driver/arms.rs:354` → `visible_method_overloads`, `src/driver/arms.rs:1521-1579`: name ←
> provider module per pair, same-name conflict full name skip, own key position not external merge),
> landing form single point `append_overload_group`
> (`src/frontend/core/typecheck/environment.rs:399`: three tables same order append + equal length
> debug assertion); consumer side attributes D20 resolution product by **definition unit**
> (`MethodMarks`, `src/driver/helpers.rs:569-573`; `typed_owners` `:591`; `resolved_method_marks`
> `:621`), and makes names with resolution records stop pool's name-level exemption
> (`src/frontend/core/typecheck/passes/dead_code.rs:252-267`, `:825-841`). Key form = entry name
> `{Type}.{method}#k` (`{Type}` is definition side bare name; `#k` declaration order).
>
> **4. Key scope and false report defense measurement (fix round F1 and five residual boundaries)**:
> F1's red state is pre-fix `d5c00235a`'s injection surface "name set × module set" combination
> merges another module's same-name type candidates into same key position ⇒ `use a.{Temp}` +
> `use b.{Other}` (a also defines Other, b also defines Temp, method same name same order) fixture
> `check` reports 2×E1107, exit 1 (previously successful program compile fails); after fix
> `check`/`run` both exit 0, 4 W1005 truthfully report both sides' **uncalled** same-name
> definitions (reverse false negative surface also verified). Included fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`; driver-level pin
> `src/driver/tests/overload_injection.rs` 3 entries + `src/driver/tests/method_unused.rs` round 2's
> 5 #464 criteria and 4 #450 old criteria all green; main acceptance fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` diagnostic set before and after fix
> line-by-line consistent (no regression). Five residual boundaries and per-item measurement
> (standalone `check` definition file not report / name level ambiguity E5006 full name skip /
> boundary(c) rewritten per t20 measurement / inline alias and `ns.T` not inject candidate surface /
> whole import `use M` and std candidate surface not produce entry) full text see
> [029f](../../rfc/accepted/029f-target-semantics.md) §Cross-unit method reachability attribution
> "Residual Boundaries", not repeated here.
>
> **5. Process events and lessons (four categories continuation)**:
>
> 1. **Contract wording too narrow directly becomes defect**: One of F1's root causes is contract
>    only wrote "only inject module surface that can be named by bare type name", didn't write "by
>    name per pair attribution"—implementation literally took "name set × module set" combination,
>    review only caught E1107 false report. When contract describes behavior boundary,
>    **combinatorics** must be written to death (by name per pair / full combination), can't be left
>    to implementation to understand.
> 2. **Agreement must be verifiable: "push by captain" only written in assignment message ⇒ when
>    third-party push/merge appears cannot attribute**. This round dev tip had two
>    source-unconfirmed merges (see "2"), finally relied on reflog reverse-inferring and withdrew
>    one wrong attribution. Verifiable form = members don't run `git push`/`fetch`/`pull`,
>    synchronization uniformly `pull --rebase`, use reflog to verify.
> 3. **Infrastructure unstable, first land patch then replace**: This batch total 5 unrecoverable
>    failures (3×TRANSPORT, 1×INVALID_REQUEST (thinking protocol), 1×background sub-agent no message
>    exit); disposition convention = produced evidence and patches first land in **workspace** batch
>    directory (under `target/`, not in repo), but **conclusions and numbers must land in this
>    record on site**—workspace path must not be used as document reference source (AGENTS meta rule
>    ①). Then replace person to continue, same session consecutive two failures don't bet on third.
> 4. **Document commit's real criterion is "really build locally once"**: Round 2 closure's document
>    commit (`042c07860`) caused Deploy Docs red at VitePress's Vue template compilation (angle
>    bracket placeholders like `<rev>` etc. didn't enter backticks were parsed as elements, reported
>    `Element is missing end tag`); markdownlint can't catch this class, and pre-commit's
>    markdownlint hook judged `no files to check` to skip in doc-only commits (measured twice). Fix
>    commit `0c837e1b4` evidenced by local build exit=0. This round's lesson red→green run: Deploy
>    Docs **39011946793** (red) → **39014696984** (green, see "6" for details).
>
> **6. Round 2 gate conclusion supplementary registration (rule 13, scheduled check session
> backfill)**: Round 2 closure commit `b940300ac`'s push—CI **39011946778** success / Auto Translate
> **39011946796** success / Deploy Docs **39011946793** **failure** (VitePress build red, "5.4"'s
> angle bracket placeholder; fixed by `0c837e1b4`); fix commit `0c837e1b4` push three green: CI
> **39014697000** / Auto Translate **39014697044** / Deploy Docs **39014696984**. Consecutive push
> coverage: CI@`0c837e1b4`'s **39014356642** cancelled by successor run per concurrency
> (cancellation ≠ failure, successor run covers previous one, consistent with round 2 §6 caliber).
>
> **7. Final gate (closure revision `e9675c9dd`, local numbers; CI conclusion submitted to scheduled
> check, see "8")**: lib 2770 passed / integration 289 passed / yx_multifile_runner 1 passed / IR
> snapshot 7 passed zero drift / clippy `-D warnings` 0 / docs-truth 0 / generator idempotent (rerun
> `scripts/docs/gen-error-code-docs.py` after `git status` clean); corpus baseline 346 → 347 → 348
> lines (each commit +1 behavior record, other lines byte-by-byte unchanged; baseline
> `tests/baselines/corpus-parity.jsonl` at `e9675c9dd` total 348 lines).
>
> **8. Gate check arrangement and conclusion (rule 13)**: This batch's post-push CI / Auto Translate
> / Deploy Docs conclusions submitted to scheduled check session for registration (not waiting in
> session, also don't write "didn't wait" as "already green"). **Backfill (2026-10-17 scheduled
> check)**: `d5c00235a`'s CI `39023417670` = **failure**—Docs Truth reported "error code document
> behind code table/locales", i.e. this commit missed generated product same submission; fixed by
> its subsequent `d5dd1a37b` (regenerate generated product): `5da05b885` CI `39027788246` / Deploy
> Docs `39027788268` / Auto Translate `39027788340` all success, covering previous one; `e9675c9dd`
> CI `39028827871` / Deploy Docs `39028827896` success; `d0cfc4e62` CI `39033963240` / Deploy Docs
> `39033963278` / Auto Translate `39033963261` success. Three commits' runs each ran to terminal
> state, this round no concurrency cancellation. Lesson: **"local generator idempotent" ≠ "generated
> product already included same commit"**—before closure should per-commit check CI conclusion, not
> just look at latest one.
>
> **9. Batch tree status (closure gc, status output)**: Before closure `scratch.py status`—t25
> (holder=-) and yx49b (holder=-, previously held by stopped member verify-r3 obsolete, captain
> `release --force` released) two trees have no holders; `gc` deletes two, releases about 12.7 GB;
> after gc `status` = "no batch tree", `git worktree list` only has main mount. Also: `yx49r3` and
> `yx49r4` are not batch trees (no meta.json), are members' **workspace** directories for
> evidence/probes, not "residual batch trees". No this batch new directories outside repository.

## P4 4.4 Proof Layer and wasm Wrap-up—Closure (4.4.1 / 4.4.2 / 4.4.3)

> **1. Batch composition and rounds (batch name `yx44`)**: Three three-level tasks 4.4.1 / 4.4.2 /
> 4.4.3 and wasm attribute reachability judgment same batch; batch verification tree built by
> `scripts/dev/scratch.py init` in workspace `target/` (first full build; deleted after closure gc).
> Roles and rounds: implement 4.4.2 = t1 (12 files; local `1fc135437` → post-push `24028ffe8`) |
> implement 4.4.1's layer order rewrite = t2 (only doc/comments, zero behavior change, based on
> `b5e68b3c9`) | implement 4.4.3 = t3 (11 files, including new code W1082; local `2d5e610cd` →
> post-push `52d2276ca`) | wasm reachability judgment = t4 (only judgment, zero cleanup, judgment
> revision `b5e68b3c9`) | 4.4.1 closure and one document commit = t8 (7 files 173+/43-; local
> `29ebd2145` → post-push `d07993157`) | independent verification = t5 (frozen `29ebd2145`) | round
> 1 review = t6 (revision `1b593e993`) | supplement = t9 (3 files 9+/10-; local `1ebddaf95` →
> post-push `1b593e993`) | fix round = t10 (3 files 23+/23-; local `657c3d5e3` → post-push
> `e184d88b6`) | round 2 review = t11 (`657c3d5e3` = post-push `e184d88b6`, patch-id all equal) |
> closure = t7 (check / record / batch tree gc / document real build / one document commit; local
> `c8412565c` → post-push `da724940b`). **Failure and redo**: t6 ended with `needs_revision` failure
> (review round per contract must fail and give findings, not accident); t7 first round interrupted
> at batch tree gc stage, completed products left in main repo working tree uncommitted, second
> round took over "supplement gap + recheck"—didn't redo completed part (this round's new: 4.4 check
> and evidence note, this record, three path rewrites pointing to deleted batch directory, document
> real build and closure commit). No cancelled rounds.
>
> **2. Per-task output and measurement numbers (original commands and output see evidence summary
> and archive; only conclusion here)**:
>
> - **4.4.1**: `layers/README.md`'s (including `layers/mod.rs` top comment) layer order table
>   rewritten as **actual call order** (predicate → termination → ownership, call point anchored to
>   `checker.rs::check_module_impl`), `equivalence` described by actual identity (no production call
>   point); same source declaration 7 places closed (`01-routing` / `02-stage-contract` / RFC-039 /
>   `06` / typecheck README / layers two files); mechanical reconciliation script on batch tree
>   green baseline **11/11 PASS** (criteria 11 = README side 4: actual call order declaration / two
>   entry anchors / old order declaration disappeared; code side 7: 5 line anchors +
>   `check_module_impl` no short circuit + `equivalence` zero production call; reconciliation script
>   is one-time, recycled with workspace, criteria 11 listed can be rewritten accordingly), mutation
>   (c) rollback old declaration → reconciliation 2 FAIL i.e. red.
> - **4.4.2**: Solver acquisition surface unified to `proof/smt/backend.rs`'s process-level shared
>   singleton + unique closure entry `with_shared_solver(consumer, f)`; three consumption points
>   (predicate / termination / ownership) all changed to shared entry, **compile production chain
>   `default_solver()` call point = 0** (only backend selection point sunk to private
>   `new_backend()`). `wasm/src/lib.rs`'s `test_smt` self-check call point is **existing** (earlier
>   than this batch, from `b5e68b3c9`) non-test call point, belongs to "test and independent
>   scenario" surface, doesn't constitute D58 prohibition violation. Independent verification
>   self-built reading (one compile): predicate acq=2 hit=1 miss=1 | termination acq=1 hit=2 miss=2
>   | ownership acq=2 hit=1 miss=1, three points hit/miss sum (4,4) = shared instance cumulative
>   `Some((4,4))`; mutation (a) singleton degenerate → predicate hit=0 i.e. red. **Change verb
>   surface disclosure**: This commit contains `z3_backend.rs` 15 lines—`cache_stats` migrated from
>   inherent method into `Solver` trait block (returns `Some(instance internal count)`) + 1 line
>   comment changed pointer, no semantic coupling. D58 prohibition has been mechanism-ized: same
>   thread reentry → panic contains D58, probe `recv_timeout(120s)` doesn't hang.
> - **4.4.3**: `body_checker` missing's silent degradation converged to
>   `TypeChecker::ownership_inputs` (only criterion point) + new code **W1082** (doesn't overlap
>   with existing W code facts: W1080 constraint degradation / W1081 solver missing / W1090 opposite
>   direction); pin test 2/2 green; health path corpus 348:348 zero diff; **degradation branch
>   production surface unreachable exhaustion holds** (CLI four entries + eval/repl + format + LSP +
>   wasm playground + all `ProgramKind` merge into `check_module_impl`: `init_body_checker`
>   unconditionally before ownership block, function in 0 return/break/continue, `body_checker` no
>   `.take()`) ⇒ W1082 no user-visible surface. Mutation (b) delete emission → pin test i.e. red.
>   **W1082 belongs to external contract table change (class E), user ratified**; `locales/zh.json`
>   / `locales/en.json` same commit as code, other languages supplemented by i18n bot after push
>   (run see "9").
> - **wasm attribute reachability judgment (t4)**: `orchestrator.rs` current tree **10 entries = 4
>   positive arms + 6 negative arms** (caliber = same line `#[cfg(...)]` attribute text, positive
>   and negative both counted; `cfg!` runtime macro 0 hits); 4 wasm positive arms unreachable at
>   playground runtime but **compile-time load-bearing** (call point not gated, deleting arm alone ⇒
>   wasm compile fails), 6 `not(wasm32)` negative arms native three phase table load-bearing ⇒ D35
>   envisioned "delete unreachable ones" in this file **has no applicable target**, real cleanup
>   form = whole project-mode subtree gated on wasm32 (registered P10 S7, three unverified items
>   preserved as is).
>
> **3. Independent verification (t5, frozen `29ebd2145` = post-push `d07993157` content—two
> revisions per commit patch-id all equal, whole tree difference only in `docs/src/en/**`,
> `docs/src/.i18n-cache.json` etc. bot product surface—`git range-diff` three commits all `=`**):
> Gate **8/8 all green**—`cargo fmt --check` 0 |
> `clippy --all --all-features --all-targets -- -D warnings` 0 | `cargo test` **3128 passed / 0
> failed** (7 test targets total; baseline 3126 + this batch 2) | corpus diff **348:348**
> line-by-line zero difference (sort diff 0 lines) | IR snapshot `UPDATE_SNAPSHOTS=1` rerun after
> `git status` zero drift | `check-obligations.py` 21 = 13 + 8, containment 0 gap |
> `check-docs-truth.py` source 156 / document 156, CLI 21 commands consistent | error code generator
> rerun before and after tree clean. Build warnings only 4 bot language W1082 missing entries (known
> state) + Z3 download noise, zero code warnings. **Contract verify caliber correction (must give
> two JSONL paths)**: Corpus gate's call caliber =
> `python scripts/ci/check-corpus-parity.py tests/baselines/corpus-parity.jsonl <this generation>.jsonl`—**two
> JSONL paths both needed**; missing second parameter is comparing baseline to baseline (always zero
> difference, criterion no-op). This batch executed per two-path caliber and recorded evidence (t5's
> `corpus-r44.jsonl` product).
>
> **4. Mutation evidence (3/3, mechanically reversible)**: All via
> `python scripts/dev/scratch.py trial` (`git apply` → run command → `git apply -R` → per file blob
> vs HEAD comparison → require `git status` clean):
>
> - (a) Singleton degenerate → shared reading predicate hit=0 miss=2, test FAILED exit 101;
>   `backend.rs` HEAD=`3e2b2d150b24` → mutation `6d00bad4d4aa` → restore = HEAD.
> - (b) Delete W1082 emission → pin test FAILED exit 101; `checker.rs` HEAD=`70c04a41e020` →
>   mutation `db0a623c9b9a` → restore = HEAD.
> - (c) Rollback old layer order declaration → reconciliation 2 FAIL exit 1; `layers/README.md`
>   HEAD=`8b1ae875db32` → mutation `e88f9af6c42d` → restore = HEAD. Three trials after restore
>   `git status` all CLEAN. trial (c) first run failed due to absolute path parsed by WSL exit 127
>   (not criterion red), changed to relative path rerun as standard.
>
> **5. Review rounds and conclusion**: Round 1 (t6, revision `1b593e993`) **verdict =
> needs_revision**—3 findings all document line number anchor inaccuracies (F-01 medium: `02` §"wasm
> current state" 4 groups of outdated line numbers; F-02 low: `02` and `09`'s 4.4.3 delivery anchor
> `checker.rs:1283-1293`; F-03 low: `06`'s comparison statement), four substantive deliveries all
> pass. → Fix round (t10, local `657c3d5e3` = post-push `e184d88b6`, 3 files 23+/23-, with two
> mechanical criteria) → Round 2 (t11, same revision) **verdict = pass, new finding = 0**. **Final
> check basis = round 2's pass**. **Round 3 (closure tip `da724940b`)**: Independent verification
> t12 **verdict = pass** (bind `da724940b`: four rule text byte-by-byte, 3.19 / S7 / S8 field and
> scope complete, count table and judgment chain recheck, zero dead references, `check-docs-truth`
> 0, `markdownlint` 0); review t13 **verdict = needs_revision**, 4 findings—F-01 medium (t10 / round
> 2 only gave rebase pre-hash, §14 list not matching origin/dev actual), F-02 low (`02:379`'s 09
> 4.2.8 anchor pushed by this batch itself in `09:144`/`:182` each insert one line to `:274`), F-03
> low (`09a` writes local drive letter path), F-04 low (three-level count line note and 157 ≠ table
> header 156) → fix round t14 (this one) closes item by item. **t10 contract caliber ruling**: Fix
> surface based on **task record's 5 F-01…F-05** (F-04/F-05 are mechanical criteria), assignment
> brief not criterion.
>
> **6. User ruling (2026-10-18) and rule solidification**:
>
> 1. **D58 entry name rewritten in place**: `with_shared_solver(f)` →
>    `with_shared_solver(consumer: SolverConsumer, f)`, correct body in place, **don't write
>    revision note** (t8 completed in one commit within 4.4.1).
> 2. **wasm judgment registered as P10 S7** (ruling B): with "belonging phase: P10; whether blocks
>    main line: no" + three unverified **preserved as is**; `06`'s F2 "needs decision" line changed
>    to that registration; t4's "### F2 Judgment" section not touched.
> 3. **Crash item registered as P3 3.19** (ruling A): symptom / cause pointing to
>    `proof/smt/translate.rs`'s sort inference / D59 **class A/B** (crash) / acceptance line (this
>    shape no longer abort—conservative degradation or diagnostic, corpus and IR snapshot zero
>    drift); **characterization = pre-existing** (t5 two revision comparison, see "7" ⑤).
> 4. **Rule solidification 1** "correction written in body in place, no old-new comparison left":
>    land in `coding-rules.md` (new clause), `07-equivalence-oracle.md` §Semantic
>    correction分流 criterion item 3, RFC-039 **D59 line** item 3 (same submission registration =
>    WBS line marks error type + in-place correct affected body + PR required block "error type" one
>    line).
> 5. **Rule solidification 2** "contract `inScope` uniformly write path prefix that can be parsed by
>    path matcher": land in `AGENTS.md` team orchestration agreement item 6.
> 6. **New code W1082 ratified** (external contract table change, class E).
> 7. **Register P10 S8** "document correction form convergence (in-place rewrite, delete revision
>    note)": including 7 audit checklist scope and acceptance line.
> 8. `09` top count table: second-level **63** (60 + 3.19 / S7 / S8), three-level **157** (154 + 3),
>    with 2026-10-18 note; three-level item's accounting review see "15".
>
> **7. Process facts that must be truthfully recorded**: ① 4.4.2 commit used
> `SKIP=cargo-fix,cargo-clippy-fix` to skip two `--fix` hooks—shared working tree had teammate
> in-progress products, `--fix` would incidentally rewrite others' files; **equivalent gate already
> manually fully run on final tree** (fmt --check / clippy `-D warnings` both 0). ② `pre-commit`'s
> stash window indeed happened (`Stashing unstaged files…` / `Restored changes from…`), after commit
> `git status` check shows teammate in-progress products intact. ③ **4.4.3's inScope lesson**: Prose
> limitation `docs/src/(only error code generated products)` was rejected by `changedPaths` check ⇒
> `inScope` uniformly write path prefix that can be matched (already solidified as rule, see "6" 5).
> This task **actually 11 files**, 7 of which because of declaration writing couldn't be parsed by
> path matcher weren't listed in `changedPaths`, already item-by-item listed in task result
> (`codes/w1xxx.rs`, RFC-013 code table, `reference/error-code/`'s index and each E section page
> etc.). ④ **W1082's locale window closed**: `locales/zh.json` / `locales/en.json` same commit as
> code, other languages supplemented by i18n bot after push (run see "9"). ⑤ **Z3 `Sort mismatch`
> crash judgment (t5 criterion ⑦)**: **pre-existing**—same probe, same input on `b5e68b3c9` and
> frozen revision `29ebd2145` stdout **byte-by-byte consistent**
> (`Error: Sort mismatch at argument #1 for function (declare-fun not (Bool) Bool) supplied sort is Int`),
> both exit 1; static attribution consistent (4.4.2 0 line change to `translate.rs`, only
> `cache_stats` migration to `z3_backend.rs`). **Degradation branch production surface unreachable
> exhaustion holds** (W1082 no user-visible surface). Probe three lines source:
> `NonNegative: (x: Int) -> Type = { x >= 0 }` / `AtMost: (n: Int, x: Int) -> Type = { x <= n }` /
> `f: (x: NonNegative(x)) -> AtMost(x, x) = { return x }`. ⑥ **t5 frozen surface and t9's coverage
> difference**: t9 (three comment/doc supplements, `1b593e993`) commit later than t5's frozen
> revision `29ebd2145`, belongs to zero behavior change, its acceptance covered by t9's own
> mechanical check + t6 review (t6 on `1b593e993` **reran** t5's key measurement, not transcribed).
> ⑦ `docs/src/en/**` not hand-edited, regenerated by post-push auto-translate workflow (this batch's
> bot landed two commits: `92d490695` i18n locales supplement W1082 other languages, `63b6cdf37`
> auto-translate documents). ⑧ **t9 overstepping observation three rulings**: (i) t9 incidentally
> corrected `02` same sentence's `wasm/src/lib.rs` line count and gating point line numbers (73 /
> 139·153·170 → 152 / 151·170·187)—**accepted** (same caliber same number's necessary surface,
> commit message already listed); (ii) RFC-039 **D35 line** still writes "wasm 27 files' cfg
> branches"—**keep unchanged** (resolution line is historical ruling record, its premise correction
> already carried by P10 S7 registration, no rewrite history); (iii) `docs/src/en/**`'s old entry
> name translation aligned by auto-translate. ⑨ `09` / `09a` originally pointing to deleted batch
> directory's two paths in closure round changed to **inline in place** (evidence summary merged
> into this record this section, probe three lines source inlined). **2026-10-21 correction**: What
> was changed to point to at that time was still workspace path, equal to moving violation down one
> level; from this round that summary's delivery commit, gate numbers and CI run id all inline,
> document no longer references any workspace path (AGENTS meta rule ① / rule 10④).
>
> **8. t8 acceptance ④ caliber ruling**: Acceptance original text required
> `git diff | grep '修订注记\|原为\|现改为'` to be empty. Measurement **default context caliber
> unreachable**—D58 line's adjacent D59 / D61 resolution line itself contains "修订注记 / 原为"
> words (not this task's change). Ruling: per **added lines / deleted lines / `-U0` increment
> lines** three calibers all empty accepted, and note **subsequent same-type acceptance only grep
> added/deleted lines**—criterion should fall on this change's lines, not whole diff's context.
>
> **9. CI gate conclusion (AGENTS rule 13①, scheduled check 2026-10-18 17:26 backfill; run id and
> conclusion inlined in place)**: First push (3 commits, tip = `d07993157`): CI **39110908163**
> success | Deploy Docs **39110908155** success | Auto Translate Docs **39110908193** success | Auto
> Translate I18n **39110908144** success; this tip covers `24028ffe8` (4.4.2) / `52d2276ca` (4.4.3)
> / `d07993157` (4.4.1) three commits all content. Second push (1 commit, tip = `1b593e993`): CI
> **39112293391** success | Deploy Docs **39112293062** success | Auto Translate Docs
> **39112292903** success; covers all four commits including t9 supplement. **No concurrency
> cancellation**, latter covers former. Third push (2 commits, tip = `da724940b`): CI
> **39121538555** success | Deploy Docs **39121538562** success | Auto Translate Docs
> **39121538541** success—three runs' headSha all = `da724940b` and **all in terminal state**
> (2026-10-18 third round scheduled check backfill); round 3 independent verification t12 at that
> time noted "CI not terminal state" observation closed. Unrelated: Nightly **39081392694**
> failure's headSha = `290f9ebc`, not this batch's commit surface.
>
> **10. Three `pull --rebase` (auto commits at push period)**: Counterparts were auto-translate and
> i18n commits; six hashes rewritten and item-by-item **patch-id all equal**—local `1fc135437` →
> `24028ffe8`, `2d5e610cd` → `52d2276ca`, `29ebd2145` → `d07993157`, `1ebddaf95` → `1b593e993`,
> `657c3d5e3` → `e184d88b6`, `c8412565c` → `da724940b` (after third rebase origin/dev head =
> `da724940b`); `git range-diff` verified **per commit content unchanged** (three commits all `=`).
> Members didn't execute `git push` / `fetch` / `pull`, synchronization and push unified by captain.
> **Evidence reminder**: Henceforth reference commit numbers uniformly use **post-push hash** (above
> right value); `09`'s check line per agreement doesn't write commit number.
>
> **11. Uncovered surface (must not write "not tested" as "passed")**: ① doc-tests 0/0 not bearing
> criterion; ② **wasm32 target actual compile not done** (this surface covered by CI's
> `_build-wasm.yml`, this batch's wasm judgment is static argument); ③ mutation only covers contract
> named 3 places, other gates not done reverse corruption verification; ④ corpus diff not separately
> run on `b5e68b3c9` (only run on frozen revision); ⑤ i18n bot supplemented other languages W1082
> text behavior not verified (only verified `zh.json` / `en.json`); ⑥ round 1 review and fix round
> reported **same class but not in contract findings' existing drift** not fixed (list see "12"),
> attribution pending user ruling.
>
> **12. `02` full document existing drift observation list (user 2026-10-18 agreed to register;
> 2026-10-21 landed as S10)**: Caliber = all **pre-existing drift** (not introduced by this batch),
> **doesn't constitute this batch's finding**; list found by fix round, round 2 review on
> `657c3d5e3` (= post-push `e184d88b6`) independently spot-checked and gave mapping, item-by-item
> recorded (list inlined as follows, original files recycled with workspace):
>
> 1. `02` §(a) entry table 3a-3d / 5 / 5b: `02:72` cites `orchestrator.rs:273-399` → now `:155`;
>    `02:73` `:450` → `:250`; `02:74` `:1374` → `:985`; `02:78` `util/diagnostic/mod.rs:565` →
>    `:681`; `lsp/handlers/diagnostics.rs:146`'s `run_diagnostics` no longer there; **content
>    level**: `run_diagnostics` and "otherwise manual lex→parse" replaced by 4.6.1 (delete manual
>    sequence).
> 2. `02:1016`: `compile_project :99-237` → now `:107-134` (28 lines Program constructor).
> 3. `02:360-361`: "missing degrade to warning" → current tree `_build-wasm.yml:50-51` is
>    `::error` + `exit 1` (#435); `build.rs:65-72` changed to `panic!` hard fail;
>    `_build-wasm.yml:35-59` interval needs re-take.
> 4. `02:127` mechanism pointer: mechanism actually at `src/driver/arms.rs::typecheck_orchestrated`
>    (`:288-289` self-statement); conclusion still holds (multi-file arm loses warnings), inaccurate
>    is "result only downstream use" sentence (`type_result` also via `arms.rs:1389` →
>    `helpers.rs:259/:636` feeds `overload_resolutions`); suggestion fix = pointer change to
>    `typecheck_orchestrated`, "only downstream use" change to "warnings field no read point".
> 5. `02:128`: `check_project:385-395` / `250-252` need re-take.
> 6. `02:334` (§S5 (d) investigation paragraph): cited old code snippet replaced by 4.4.3's
>    `ownership_inputs` criterion point.
> 7. `02` change list table other rows: `predicate.rs:34-36`, `checker.rs:1303-1314` / `:1244`,
>    `backend.rs:67-72`, `wasm/src/lib.rs:30-36` / `:42-51`, `util/diagnostic/mod.rs:621-661`.
>    **This closure doesn't pre-set S9's WBS item** (user not ruled); after getting ruling then per
>    registration form land `09`. **Follow-up (2026-10-21 landed)**: User 2026-10-18 agreed to
>    register, this list as `S10` "02 body and fact alignment (line number anchor + content level
>    outdated)" landed `09` (second-level +1 / three-level +1 = S10.1)—**originally planned S9
>    number already occupied by "pre-release audit's abstraction closure"** (S9.1 = issue #473 /
>    S9.2 = issue #474), per 09 "number must be unique in whole document" registration agreement
>    deferred to S10. **Registration timing deviated from original plan** (originally wrote "with
>    P5's closure document commit land together"), per user 2026-10-21 instruction this batch land
>    immediately.
>
> **13. Batch tree recycle (gc and status original output; output's tree path per repository
> relative path record)**: `python scripts/dev/scratch.py gc` →
> `[scratch] deleted yx44-evidence (released about 0.0 MB)` /
> `[scratch] gc complete, total released about 0.0 MB`; `python scripts/dev/scratch.py status` →
> `yx44-evidence            holder=-` next line `<workspace>/yx44/tree`—registry record's path
> **doesn't exist**, this name is registry residual (`gc`'s `_remove` deletes `meta["path"]`'s
> parent directory i.e. workspace's `yx44`, residual `meta.json` still in scan surface; its content
> batch=`yx44`, `base_rev=29ebd2145`, `lock=null`, including t5 / t6 / t11 holding history); Closure
> round renamed that registry file to `meta.retired.json` to exit scan (**don't delete any evidence
> files, don't create any trees**) → `python scripts/dev/scratch.py status` =
> `[scratch] no batch tree`; then run `gc` = `[scratch] no batch tree, no need to recycle`;
> `git worktree list` only has main mount (`dev`). Batch original evidence directory (no
> `meta.json`, **not batch tree**) kept as archive at that time; **from 2026-10-21 workspace
> directory no longer as evidence source**—closure summary inlined in this section, original
> directory deleted with cleanup (AGENTS meta rule ①). Acceptance statement (resolvable):
> `git status` clean; after batch tree gc `python scripts/dev/scratch.py status` = no batch tree;
> `git worktree list` only has main mount; no this batch new directories outside repository.
>
> **14. Document real build and commit hygiene**: `npm run docs:build` local **exit 0** (AGENTS rule
> 10⑨—markdownlint can't catch VitePress's Vue compile error, and doc-only commit judged
> `no files to check` skip markdownlint); this batch's check / note / registration / rule
> modification accumulated into **one** document commit (rule 8), landed at batch closure; members
> didn't execute `git push` / `fetch` / `pull`. **This batch's final two both pushed** (closure tip
> on `git rev-list --count origin/dev..HEAD` = 0, origin/dev head = `da724940b`): `e184d88b6` (round
> 1 findings' document anchor fix, 3 files 23+/23-) + `da724940b` (closure document commit, 7 files
> 96+/10-; including check / record / three path rewrites / registration and rule modification).
>
> **15. Count table accounting review (round 3 fix F-04; method and result both verifiable)**:
> Judgment object = `09` top "positioning and scope" table's three-level count line. **Method** (two
> independent calibers, per revision use `git show <rev>:<file>` take byte recalculate): ① **Sum per
> second-level item**—take each second-level kanban line parentheses' "N three-level tasks" (line
> start `- [x]`/`- [ ]` bold title start, line contains "个三级任务", including S7 line inner
> written "1 个三级任务"): current tree 70 second-level entries, **sum = 157**. ② **Historical
> self-consistency**—same caliber on `366bc5a69^` gets 66 entries, **sum = 151 = current period
> table header 151** (self-consistent); `366bc5a69` that day newly registered `4.9.6` and `0.4.1`
> two three-level (sum 151 → 153), but table header only wrote 151 → 152 ⇒ **table header since that
> commit undercounted 1**; subsequent revisions all off by 1 (`92d490695` / `1b593e993` /
> `d07993157` / `e184d88b6`: sum 154 vs table header 153; `da724940b`: sum 157 vs table header 156).
> **Result**: True value = **157**, so correct table header (`09:13` three-level 156 → 157); note
> 144+2+2+2+2+1+1+3 = 157 was originally correct, not changed; pre-batch's base = **154** (not 153).
> **Second-level count line not touched**—that line's caliber never aligned with kanban entry count
> (`366bc5a69^` is 66 entries vs table header 59), this time no verifiable evidence to overturn 53 +
> 10 = 63, don't change number to fit. Attached correction (same round): §2's 4.4.1 parenthesis
> decomposition per reconciliation script actual item count changed to "README side 4 + code side 7
> = 11", consistent with **11/11** and script 11 `ck` calls.

## P3 Debt Round 1 Closure (3.5 / 3.6 / 3.4.6 / 0.5)

> **1. Batch composition and rounds**: This batch closes P3 side remaining debt—**3.5 Higher-order
> type parameter call path** (3.5.1–3.5.3), **3.6 Function type value call path** (3.6.1–3.6.4),
> **3.4.6 compile path consume warnings** (class D, user 2026-10-19 ruling "do"), and registers
> **0.5 test suite layering** (P0, issue #486 slice) and **3.20 dead code analysis existing false
> reports** (3.20.1 / 3.20.2). Roles and rounds:
>
> - Implement: 3.5.1–3.5.3 = t1 | 3.6.1 = t2 | 3.4.6 = t6 | 3.6.2 = t7 | 3.6.3 = t8 (main body +
>   closing two commits) | 3.6.4 = t9 | 0.5.1 = t10 (#486 slice)
> - Independent verification: Round 1 = t3 (frozen revision `81d5e414e`, covering 3.5 / 3.6.1–3.6.3
>   / 3.4.6) | 3.6.4 = t11 (round 2, frozen `8e32e8636`)
> - Review: Round 1 = t4 (verdict = **needs_revision**, F1/F2/F3) | Fix round = t12 (one commit) |
>   Round 2 = t13 (verdict = **pass**)
> - Closure = t5 (check / registration / this record / batch tree gc / document real build / one
>   document commit)
>
> **Failure and redo**: t4 ended with needs_revision failure (review round per contract must fail
> and give findings, not accident) → t12 real fix → t13 review pass. t8 internally experienced one
> attempt interruption (process crash), previous round's evidence kept in batch directory,
> subsequent attempt supplemented then two commits included. No cancelled rounds.
>
> **2. Per-task output and measurement numbers (conclusion caliber; original commands and output see
> each batch's evidence directory)**:
>
> - **3.5.1** (unifier matches constructor variable in head position): Criterion converged to
>   **single implementation** `is_ctor_reference` (`pub(super)` free function, whole repo 1
>   definition 3 consumption)—this is t12's F1 real fix for t4 (originally `inference/statements.rs`
>   had a same-name method copy, constituting same-name two implementations in rule 3 sense). Head
>   rename no longer inline copy: `environment.rs` and `expressions.rs` each first iterate TypeRef /
>   variable table to construct head_map, then uniformly hand to shared implementation
>   `substitute_ctor_heads` (`types/substitute.rs`) to execute—two places' head_map construction
>   criterion unchanged, rename only acts on head by name, empty table returns as is (t12's F2 real
>   fix). Equivalence argument: iteration doesn't change Generic head name.
> - **3.5.2** (explicit type argument accepts type constructor, E1010 disappears): Red state E1010 /
>   E1002, green state exit 0.
> - **3.5.3** (mono product replaces constructor variable): Symbolic type argument criterion
>   narrowed by container generic parameter name; constructor variable in product replaced with real
>   constructor (IR dump verified: specialized function `Function probe(Box, int64)`).
> - **3.6.1** (local function type value call by name): Criterion single source =
>   `is_static_fn_name` (whole repo 1 definition 1 call), aligned with variable evaluation path's
>   name resolution priority (local → global), no new second resolution table; IR verified: module
>   initialization segment changed from `LoadConst String("f")` + `CallNative` to `LoadGlobal` +
>   `CallDyn`. Regression surface: same-name "function binding + value binding" unreachable
>   (uniformly compile-time E2010), non-function value call by name still E1065.
> - **3.6.2** (`use` alias imported function value call by name): Originally reported
>   `E6006 Native function not found: lib.f` (`use lib.{f}` and `use lib.{f as g}` two forms). Fix =
>   in `is_static_fn_name` **add one arm** (not copy), new unique implementation
>   `use_alias_value_slot` takes "use alias → qualified name → global slot" two-table joint query,
>   consumption point 2 places (call form criterion + variable evaluation path).
> - **3.6.3** (nested function body references outer frame value): **Criterion judgment
>   changed**—not "let it run correctly", but this form per RFC-009 §2.3 + SPEC §12.3 should be
>   **compile-time rejected** (D59 class C: change silent wrong value to loud; per `09` §P3's D59
>   line, "silently give wrong answer" goes to class C—this batch's closure brief noted as "class
>   A/B", both refer to "old behavior not successful", landing way both are "red before green +
>   compile-time loud rejection"). Original form = **no error, compute wrong value (void)**: red
>   state stdout `void`, EXIT=0 (silent wrong value archived); green state compile-time E1001
>   `Unknown variable`, EXIT=1, same code same text as existing "unknown variable" path. Criterion
>   single source = capture barrier (`enter_capture_barrier`), compile-time name (type parameter)
>   not blocked; E1001 still only emitted at typecheck, registered unique. **Assignment path caught
>   by E2010** (t8's closing commit: declaration / assignment judgment not wrapped with capture
>   barrier)—otherwise this fix itself would open new "same-name assignment silent wrong value"
>   channel (c1 form: only with barrier degrades to exit 0 print 10).
> - **3.6.4** (`use lib` whole import then `lib.f(...)`): Goes through FieldAccess callee's
>   namespace branch, originally reported E6006 `lib.f`. Fix = namespace branch extracts qualified
>   name then first judges global slot layout → changes to indirect call; qualified name resolution
>   and slot query reuse `resolve_namespace_data_access` (same source as data access), no new
>   resolution table.
> - **3.4.6** (compile path consume warnings, class D): Change before and after external performance
>   = **multi-file `run` previously always output no W code** (red state archive: multi-file [] vs
>   single-file [(W1001,3),(W1002,5),(W1003,1)]), now four paths (single-file run / multi-file run /
>   single-file check / multi-file check)'s W code set uniformly {W1003@1, W1001@3, W1002@5}
>   (non-empty set). Corpus baseline: in 348 records **46 entries changed**, new W code **118
>   entries** (W1001 34 / W1002 12 / W1003 13 / W1004 42 / W1005 17), **118/118 item-by-item
>   attributed to real existing unused definitions in files (0 unlocated)**, pure increment (0
>   deleted, 0 exit / stdout / check changes). **D59 four fulfillment**: ① red before green (red
>   state archive + turn to pin test) ② don't change "original successful program"'s external
>   performance (pure increment, 0 exit / stdout / check changes) ③ same submission registration
>   (this record + `09`'s 3.4.6 line) ④ baseline same submission regeneration and manual review
>   diff. **Superset approved**: W1004 / W1005 / W1006 also enter `run` (user 2026-10-19 approved),
>   reason = **one criterion, one channel**—only putting W1001/W1002/W1003 equals creating second
>   filter ruling. **External contract surface change**: `orchestrator::compile_project` return
>   value from `Result<ModuleIR, _>` changed to `Result<(ModuleIR, FileDiagnostics), _>` (in-repo
>   consumers 2 + 2 test direct drive points synchronously adapted).
> - **0.5.1** (test resource profile mark + exclusive lock + fast/full minimum consumption surface):
>   Added `scripts/dev/testsuite.py` (`markers --check` and `cargo test --list` bidirectional
>   reconciliation, `plan` budget boxing, `run fast|full` budget insufficient reports gap not
>   silently skip, `manual` file never implicitly executed) and `src/util/test_support.rs`
>   (cross-process file lock, test body takes lock ⇒ any call path mutually exclusive); in-place
>   `cost:` mark 3 places (2600 file exclusive, peak extrapolated linearly from 5000 file measured,
>   **not measured**; 5000 file exclusive, 15.6 GiB / 72.6 s measured; snapshot update manual, peak
>   and time both extrapolated). Self-verification numbers: `markers --check` reconciled 3149
>   `#[test]`, 3 mechanical mutations each exit 1, `run fast` batch wave 2793 passed / 47.7 s +
>   exclusive wave 1 passed / 14.6 s, budget 1 GiB `run full` lists gap 7.11 GiB, two processes lock
>   contention measured `held→blocked→held(waited 2.0s)`. **Independent verification belongs to #486
>   subsequent scheduling** (this batch only implementer self-verification). **0.5.2 pending
>   scheduling** (pre-commit / CI wiring + 2600 file measurement calibration + `test-specification`
>   detailed section).
>
> **3. Independent verification (t3, frozen `81d5e414e`; red state baseline `c599de7bb` /
> `1725d0215` / `ef2334de5` all HEAD ancestors, `git merge-base --is-ancestor` verified)**: Probes
> all built by verifier (batch tree built by `scripts/dev/scratch.py init` in workspace, probes and
> patches in its `probes/` / `patches/` subdirectory), not transcribing implementer conclusion; that
> batch tree together with its in-tree evidence deleted in this batch's closure gc (see "9"), so
> this section puts measurement conclusion and numbers **as is into record** for evidence. 3.5 red
> state four probes all report E1003+E1002 / E1010, green state exit 0 and value correct (stdout 1 /
> 7 / 1), and has IR value-level evidence (specialized function `Function probe(Box, int64)`); 3.6.1
> green state output 6 / 10, control "value position call" green, "non-function value call by name"
> compile-time E1065; 3.6.2 red state two forms E6006, green state exit 0 (probe internal `assert`
> failure non 0, exit 0 means value correct); 3.6.3 red state stdout `void` / EXIT=0 (silent wrong
> value verifiable), green state compile-time E1001 / EXIT=1, while **expression position inline
> lambda capture this function parameter** released (EXIT=0, output 6)—release surface not
> accidentally injured. 3.4.6's C2 normalization independently reproduced by verifier's self-built
> fixture (four paths W set identical, non-empty), baseline diff independently recalculated
> (changed=46, added=0, removed=0, expectation / run.exit / run.stdout / check changes = 0, new W
> code total 118 consistent with implementer), attribution sampling 10 all hit real definition. Gate
> on frozen tree all green: fmt / clippy (`-D warnings`) / `cargo test --all` (**lib 2790 passed + 2
> ignored, integration 297 passed**, 0 failed) / corpus diff 348:348 difference empty / IR snapshot
> 204 zero drift / obligation ledger / docs-truth. **t11 (round 2, frozen `8e32e8636`)**
> independently reproduced 3.6.4's red→green (red state parent commit 16e2a5c88: E6006 `lib.f`,
> EXIT=1; green state EXIT=0, stdout 27, probe internal 4 assertions all passed), criterion single
> source (`resolve_namespace_data_access` 1 definition 2 consumption, no third "qualified name →
> slot" joint query), no regression eight forms all pass, verdict = **pass**.
>
> **4. Review rounds and conclusion**: Round 1 (t4, judged `20abd3ebc`'s parent revision) verdict =
> **needs_revision**, three findings all real defects: F1 = `is_ctor_reference` same-name two copies
> (`inference/statements.rs` method copy), F2 = head rename each inline copy in `environment.rs` and
> `expressions.rs`, F3 = visibility overstep (what should be private not private, what should be
> `pub(super)` not converged). → Fix round t12 (one commit, 4 files +96/−85) → Round 2 (t13,
> reviewed revision `20abd3ebc`) verdict = **pass**, new finding = 0: 8 self-built probes' value and
> exit code each item same as before fix, 4 IR dumps with round 1 green state **byte-by-byte
> consistent**, corpus 348/348 zero drift, IR snapshot 204 zero drift, `cargo test --all` EXIT=0
> (lib 2794 passed + 2 ignored, integration 297 passed, 0 failed), fmt / clippy / docs-truth all
> exit 0, `check-boundary`'s `pub(crate)` count 167 / frontend-core 26 (= 3.5 pre-baseline),
> `check-fanout` same-name definition zero hit. **Final check basis = round 2's pass**.
>
> **5. Mutation evidence (all via `python scripts/dev/scratch.py trial`, apply → run command → apply
> -R → per file blob comparison + require `git status` clean)**:
>
> | #   | Target                                        | Mutated blob    | Capture result                                                                                                                                               |
> | --- | --------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
> | M1  | 3.6.1 `is_static_fn_name`'s global lookup arm | `022986cbea65…` | Two probes reproduce E6006, EXIT=1 (after restore blob = HEAD, CLEAN)                                                                                        |
> | M2  | 3.5.2 explicit type argument branch           | `abbcc5ecf116…` | Probe check reports E1002, EXIT=1 (control function form still green ⇒ attribution precise to 3.5.1)                                                         |
> | M3  | 3.4.6's `MULTI_FILE_STAGES` dead code arm     | `372f7b92755d…` | run channel W code all extinct (C2 red); M3b attribution shows check channel criterion intact, broken exactly is 3.4.6 newly opened compile external channel |
> | M4  | 3.6.3 capture barrier entry                   | `7a7204c60383…` | Capture class turn pin test 4 failed, TEST_EXIT=101                                                                                                          |
> | M5  | 3.6.4 value slot → CallDyn block              | `25853376639b…` | Probe reproduces E6006, EXIT=1 (blob chain `0d837e9d30de…` → `25853376639b…` → `0d837e9d30de…`)                                                              |
> | M8  | 3.5.1 turn pin test                           | —               | After fix rerun still can fail (E1003, TEST_EXIT=101, mechanically reversible restore CLEAN) ⇒ criterion not lost fail-ability                               |
>
> **6. User ruling and this batch's registration**:
>
> 1. **3.4.6 = class D** (externally observable: multi-file `run` new W code), user 2026-10-19
>    ruling "do"; **superset (W1004 / W1005 / W1006) also approved**, reason = one criterion / one
>    channel. Original 2026-10-15 ruling third "per decision B1 defer" **precondition** (4.9 pool
>    semantic correction) closed on 2026-10-17.
> 2. **3.6.3 criterion judgment changed**: from "let it run correctly" to "nested function body
>    references outer frame value → compile-time E1001"; assignment path caught by E2010.
> 3. **Two "earlier than this task" silent form registration not changed**: ① Same block sibling
>    nested function reference—baseline already E1001 (outer block local family), this batch doesn't
>    expand scope; ② Nested function annotation reference outer type parameter still print `void`
>    (t1 registered item). Both are **not** introduced by this batch, both preserved as is.
> 4. **3.20 "dead code analysis existing false report" registration** (user 2026-10-19 agreed per
>    third disposition then register): 3.20.1 = W1001 doesn't track closure-internal reference
>    (`docs/src/reference/stdlib/test.md`'s `t_add` form: referenced inside closure yet reports
>    unused); 3.20.2 = W1003 judges variant destructuring required import as unused
>    (`docs/src/reference/stdlib/weak.md`'s `use std.option` form: delete i.e. E1002). Both forms
>    also reproduce on baseline before 3.4.6, **not introduced by 3.4.6**. **Rollback obligation
>    (written to death)**: After this group is fixed must rewrite `weak.md` / `test.md` modified to
>    work around false report **back to natural writing** and **regenerate baseline** (baseline will
>    move again).
> 5. **0.5 test suite layering** (P0, issue #486): 0.5.1 completed; 0.5.2 (pre-commit / CI wiring +
>    2600 file measurement calibration + `test-specification` detailed section) pending scheduling.
> 6. **#489 ureq 3 migration**: Dependency surface trace item, **not entering WBS**;
>    `.cargo/audit.toml`'s `RUSTSEC-2026-0381` exemption **with hard removal condition**—after ureq
>    migrated to >= 3 delete that ignore, if still red then can't extend deadline, must fix
>    dependency first. 0.26 line has no fix version (0.26.12 doesn't exist), ureq 2.x has no update
>    version.
> 7. **3.20 number conflict disposition (this round's new registration)**: 3.20 number originally
>    occupied by 2026-10-18 registered "pre-release audit (v0.8.4) supplementary gap"
>    (3.20.1–3.20.4), while user 2026-10-19 ruling registered "dead code analysis existing false
>    report" needs 3.20. Whole document number uniqueness rule doesn't allow two 3.20, so **move
>    pre-release audit group entirely to 3.21** (three-level change 3.21.1–3.21.4, issue anchors
>    #468–#471 unchanged, entry content unchanged), 3.20 number given to "dead code analysis
>    existing false report". Migration only changes number, doesn't change any registered entry's
>    content and attribution; historical commit message not rewritten (migration fact recorded in
>    this record and `09`'s count table note).
>
> **7. Process facts that must be truthfully recorded**:
>
> 1. **Two times inScope written too narrow each exchanged for one round rework**: ① 3.6 series'
>    implementer wrote inScope too narrow to `ir_gen.rs` predicate layer, didn't cover `types/`'s
>    shared substitution implementation → review round 1's F2 when landing discovered head rename
>    each inline copy in two files, must open one more round convergence; ② 3.5's fix round between
>    `types/` shared implementation and two call sites moving changes, again hit inScope and actual
>    change surface mismatch (AGENTS.md rule 6: inScope must cover mechanically necessary surface,
>    path prefix can be parsed by matcher). Both reworks have "implementer inScope written too
>    narrow" as cause, **not implementation defect**—but cost is two rounds rework and one review
>    findings.
> 2. **External sync (2026-10-19 13:08 `pull --rebase --autostash origin dev`, 13:19
>    `pull --tags origin dev`) and V0.8.5 release commit**: Per user 2026-10-19 confirmed as
>    **normal release process, completed, doesn't affect this team**; member commits intact no loss.
>    Evidence (`git reflog --date=iso` / `git reflog show origin/dev --date=iso`): 13:08:43's
>    `pull --rebase --autostash` picked up release chain's "supplement CHANGELOG retention policy's
>    criterion and self-check", 13:19:34's `pull --tags` was fast-forward; V0.8.5's release commit
>    (`:twisted_rightwards_arrows: V0.8.5: …` and its `:bookmark:` companion commit, CHANGELOG
>    generation and retention policy two commits) all in `v0.8.5` tag reachable range.
> 3. **Three rebase led to local hash rewrite (referencing old number will be empty)**: `b3245aac9`
>    → `56ae469e1` (3.6.3 main body), `415f0154b` → `ef2334de5` (3.6.2), `bdbe859e1` → `8e32e8636`
>    (3.6.4). Verifier with "`src/`, `tests/` tree diff empty" proves three are **pure rebase
>    equivalent** (content unchanged, only parent chain changed), this record uniformly uses
>    **currently reachable** hash.
> 4. **F2's unfixable part**: 3.5.1's commit message wrote "unique implementation" is historical,
>    **not rewritten**; correction can only be carried by "shared implementation convergence's one
>    commit + code comment per fact correction"—this record keeps this sentence, to avoid readers
>    mistakenly thinking nobody found this problem at that time.
> 5. **Three observation items (verified, doesn't block, but registered to avoid being treated as
>    unresolved violation later)**: **O1** Head rename convergence to shared implementation after
>    coverage drills down `MetaType` / `AssocType` / `Literal`, but three coverage probes **all
>    parse fail** (`Pipe` / `Ampersand` / old `Type[...]` syntax; `ast::Type::Union` has no
>    construction point in parser) ⇒ **design stronger, currently unreachable**, not behavior
>    change; **O2** `check-fanout.py` after fix round newly reports `add_type_param: 2 files`,
>    introducer is 3.6.3's closing commit (`inference/scope.rs`) **not** this batch's fix round,
>    review judged as **heuristic false report** (delegation wrapper + unique implementation
>    division of labor, same pattern as existing `add_param`); **O3** `09`'s 3.6 line original text
>    still writes "1 three-level task" but actually has 3.6.1–3.6.4, 3.5 three lines missing
>    completion note—supplemented by this closure round together (see `09`).
> 6. **`src/driver/arms.rs`'s "Decision B1" comment not rewritten with code same submission
>    (outstanding item, this batch not changed)**: 3.4.6 already changed `typecheck_orchestrated`'s
>    FailFast branch to **temporarily store** `result.warnings` (originally directly discarded), but
>    that function's above document comment still writes "`result.warnings` discard—Decision B1",
>    opposite to code fact after 3.4.6; same source's another comment
>    (`src/driver/tests/program.rs`'s phase table description) **already rewritten with 3.4.6 that
>    commit same submission** to "Decision B1 'dead code family warnings don't enter compile path'
>    hereby revoked". This batch `src/**` out of scope, so **not changed** that comment, registered
>    for next team (belongs to AGENTS.md rule 6 "declaration carrier directly named by ruling" class
>    debt). `02`'s runtime behavior table B1 line already corrected in place to current state (dead
>    code family warnings now enter MultiFile `run` path through same criterion).
> 7. **Members didn't execute `git push` / `fetch` / `pull`**: Synchronization and push unified by
>    captain (in `git reflog show origin/dev` each commit is "update by push", member side no push
>    record).
>
> **8. Uncovered surface (verbatim transcription, must not write "not tested" as "passed")**:
>
> - **t3's 7 items**: ① wasm32 target actual compile not done; ② Performance impact not measured
>   (3.4.6 added two phases, 3.6.x's indirect call overhead all no benchmark numbers); ③ **W1005
>   item-by-item attribution not independently reproduced** (verifier's message form regex didn't
>   cover its text form, only verified new W code total 118 consistent); ④ 3.6.3 closing commit
>   (declaration / assignment not wrapped with barrier) not separately mutated, c1 form not
>   independently retested; ⑤ t8's 29 probe matrix not fully rerun (only retested three
>   representative forms + M4 turn pin); ⑥ 3.6.4 belongs to round 2 scope, round 1 not independently
>   measured; ⑦ CI gate per agreement not waited in session, submitted to closure session unified
>   check.
> - **t11's 4 items**: ① Generic argument qualified name callee (`lib.g(Int)(3)`) and named argument
>   call function type value—no in-repo sample, this round no construction retest; ② `use lib` whole
>   import under no item import's `p.get_x()` (existing E6006 boundary, earlier than this task) not
>   retest mutation state; ③ On `8e32e8636` not repeated full gate (covered by t9's gate log and t3
>   on `81d5e414e` containing this commit's all green); ④ CI submitted to closure session check.
> - **This closure round's new registration**: This round's document change **didn't rerun
>   `cargo test --all`** (change surface `docs/**` three files, zero code surface); closure round's
>   gate is `npm run docs:build` + `check-docs-truth.py` + batch tree recycle + `git status` check.
>
> **9. Batch tree recycle (gc and status original output)**: `python scripts/dev/scratch.py gc` →
> `[scratch] deleted verify-p3-t3 (released about 13220.5 MB)` /
> `[scratch] deleted verify-p3-t4 (released about 14428.0 MB)` /
> `[scratch] gc complete, total released about 27648.5 MB`; `python scripts/dev/scratch.py status` →
> `[scratch] no batch tree`; `git worktree list` → only main mount (`dev`). **Deleted together with
> gc are also two batch trees' verification evidence** (`verify-p3-t3/` containing independent
> verification's probes, patches, original logs and conclusion text; `verify-p3-t4/` containing
> review round 2's evidence)—they are **registered batch directories**, recycled with tree; this
> record's third, fourth, fifth sections have put their conclusion, mutation table and gate numbers
> **as is into record**, conclusion doesn't fail due to file deletion. **Unaffected archive** (not
> registered batch directory, gc doesn't touch): each batch's evidence directory `wbs35` /
> `wbs35repair2` / `wbs346` / `wbs36` / `wbs362` / `wbs363b` / `wbs364` / `wbs486` (in workspace
> `target/`, **not in repo**; conclusion already item-by-item in this record, workspace path not as
> reference source). Acceptance statement (resolvable): `git status` clean;
> `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only has main mount;
> no this batch new directories outside repository.
>
> **10. Document real build and commit hygiene**: This round's check / note / registration /
> document correction **accumulated into one** document commit (AGENTS.md rule 8), landed at batch
> closure; generator regenerated documents merged into its functional commit. Before commit locally
> run `npm run docs:build` (AGENTS.md rule 10⑨—markdownlint can't catch VitePress's Vue compile
> error, and doc-only commit judged `no files to check` skip markdownlint) and
> `python scripts/ci/check-docs-truth.py`. **Commit number after push**: This record per historical
> record's convention records each round actually occurred commits and hashes (**note division of
> labor with `09` check line**—`09`'s check / completion registration only writes "task id +
> evidence (gate values, issue number, file:line)" doesn't write commit number, because
> `pull --rebase` will rewrite local hash; this record is current round process's historical record,
> keep hash and simultaneously give rebase before and after comparison, see "7" 3). **CI / Deploy
> Docs conclusion submitted to "unified check" session**, not waited in session (AGENTS.md rule
> 13)—this batch's CI run id and conclusion backfilled by captain at unified check.
>
> **11. Count table accounting review (this round's caliber)**: `09` top's second-level /
> three-level count table continues "**base + increment**" caliber (same as 2026-10-18's
> registration). This round's increment = second-level +2 (0.5 / 3.20, where 3.20 is new number
> after move, pre-release audit group change to 3.21 is net zero), three-level +7 (3.6.2 / 3.6.3 /
> 3.6.4 / 0.5.1 / 0.5.2 / 3.20.1 / 3.20.2). **Remaining pre-existing deviation (this round not
> corrected, for subsequent accounting review)**: With "per second-level item parentheses' 'N
> three-level tasks' sum" as caliber recalculate, this tree gets 162, differs from table header by
> 7—difference comes from 2026-10-18 registration time newly added three-level but not synchronously
> rewritten parent entry count note's 5 places (4.9 wrote 7 actually 8, 3.14 wrote 1 actually 2,
> 4.10 wrote 3 actually 5, 4.4 wrote 3 actually 4, S2 wrote 1 actually 2) and 1 place reverse
> deviation (7b wrote 6 actually 5), additionally 1.2 / 1.3 / 9.1 / S7 four parent entries
> themselves don't have this note (not counted in caliber). This difference **not introduced by this
> batch**, this batch doesn't change number for others' entries, only registered here.
>
> **12. Outstanding todo (this batch registered, not fixed)**: ① `src/driver/arms.rs`'s "Decision
> B1" comment opposite to code fact after 3.4.6 (see "7" 6, `src/**` out of scope); ② 3.20.1 /
> 3.20.2 two existing false reports (after fix must restore `weak.md` / `test.md`'s natural writing
> and regenerate baseline); ③ 0.5.2 (pre-commit / CI wiring + 2600 file measurement calibration +
> `test-specification` detailed section); ④ #489 ureq 3 migration (dependency surface, exemption
> with hard removal condition); ⑤ 3.8 (native shadowing of user function name) and 3.12 (executor
> configuration propagation between runtime modes) pending ruling, 3.18 precondition pending
> P6—**all not in this team's scope**.

## P3 3.17 Closure (Round 2: 3.17.1 Hand-annotated FFI Named Argument + Second Binding Criterion Merge)

> **1. Scope and round relationship**: This section is **round 2**'s closure registration, scope =
> **3.17.1** (issue [#456](https://github.com/ChenXu233/YaoXiang/issues/456)), not including other
> second-level tasks. Round 1 (3.5 / 3.6 / 3.4.6)'s closure see previous section "P3 Debt Round 1
> Closure"; relationship between two rounds written here: **Round 1's review round 1 (t4) that
> needs_revision (F1 / F2 / F3) already covered by t12 fix round + t13 review round 2 (verdict =
> pass), not repeated in this section**. This section's own criterion chain = implement **t15**
> (attempt 2 delivered; **previous attempt due to provider timeout, zero product**) → independent
> verification **t16** (verdict = pass) → adversarial review **t17** (verdict = pass, **no
> unresolved findings**, registered 4 observation items).
>
> **2. Per-item output and measurement numbers (3.17.1)**:
>
> - **Parameter name table same source**: `add_native_signature` changed to take **signature
>   string** and return `Result`—`signature::parse_signature` outputs type, unique exit
>   `signature::export_signature_param_info` same fills "parameter name + required minimum"; key
>   same as signature table (same name). In-repo zero caller so signature can be changed; document
>   clearly writes "callers that can only get `MonoType` should parse signature string on their own
>   side then go through this API", and this function **must not** parse parameter name itself.
> - **Red before green** (red state measured on parent commit `537b46b74`): hand-annotated surface
>   four forms + duplicate naming + legal call total 6, **5 failed, diagnostic all `[]`** (named
>   argument silently discarded); green state same file **12 all pass**—four forms E1014 / E1010 /
>   E1111 / E1111, duplicate naming E1015, legal named call and omitted optional position released.
> - **Three call surfaces same criterion same code**: Hand-annotated FFI (typecheck phase) / regular
>   user function (IR phase) / stdlib native export (`assert.assert`) three surfaces **per code
>   equal assertion** (E1014 / E1010 / E1111; E1015 see hand-annotated surface and user surface).
>   Observation phase different only reflects "who finds first"; host native doesn't enter registry,
>   IR phase originally unresolvable (**earlier than this task's embedding surface limit**).
> - **Criterion single source**: `bind_args_to_params` **1 definition**
>   (`inference/arg_binding.rs:35`) + **6 calls** (typecheck 2 + ir_gen 4); second criterion
>   `build_arg_slots` **0 places** (deleted, `ir_gen.rs:516` only has comment explaining its
>   history); `ir_gen` only remains pure carry `slot_order_arg_indices` (`ir_gen.rs:521`, 1
>   definition 2 calls, zero judgment). Line count: `ir_gen` **+58 / −92 (net −34)**; whole repo 4
>   files **+448 / −102** (among which 353 lines are new tests).
> - **No regression**: 8 CLI probes (u1–u4 / d1 / d2 / n1 / n3) before and after mutation **per code
>   consistent**; `ir_gen` blob `0d837e9d…` ↔ `ffe1af8e…` check. Merge initial version's one
>   self-injury fixed: IR side re-judge "missing parameter" misreports `t.pick2(a = 2)` legal
>   overload as E1010 (named_args unit test caught) ⇒ missing parameter judgment goes to typecheck,
>   IR only replays slot binding; after fix 19 named_args + 12 new unit tests all green. Corpus
>   348-348 zero drift, IR snapshot zero drift.
>
> **3. Independent verification (t16, frozen `c3082a160`; red state baseline `537b46b74`, both
> `git merge-base --is-ancestor` verified as HEAD ancestors; probes all self-built)**:
>
> - **Red→green independent reproduction** (hand-annotated FFI surface): Self-built independent Rust
>   unit test `verify_host_ffi.rs` (4: unknown name E1014 / competing slot E1111 / named then
>   positional E1111 / legal named call released), via `scratch.py trial` temporarily hung in
>   `tests/mod.rs` (apply → test → apply -R → CLEAN). Two states each use **their own interface
>   form**'s probe variant (parent commit `add_native_signature(name, MonoType)` old signature / fix
>   state `(name, &str) -> Result` new signature—interface itself is fix surface): parent commit **3
>   failed / 1 passed**, 3 failure diagnostic **all `[]`** (`got []`), TEST_EXIT=101; fix state **4
>   passed / 0 failed**, TEST_EXIT=0. Red state another on fix tree undid fix's mutation state
>   reproduction (`M7v2` merge patch: undo fix + undo implementer test + hang self-built old API
>   probe)—3 failed `got []`, TEST_EXIT=101, after restore four files blob consistent with HEAD,
>   `git status` CLEAN, after undo `ir_gen.rs` blob = `0d837e9d30de` (i.e. parent commit original
>   blob). CLI surface (user function + stdlib) two states per code consistent—as expected: these
>   two surfaces 3.16.1 already loud, this commit's fix surface only in hand-annotated FFI.
> - **Four forms item by item** (CLI self-built probe, `c3082a160` measured): parameter name error
>   E1014 `has no parameter named x (available: a, b)`, missing required position E1010, position
>   and name competing slot E1111, named then positional E1111; additional: duplicate naming E1015,
>   stdlib unknown name E1014, stdlib missing required position E1010 (help point name cond), stdlib
>   competing slot E1111, legal named call EXIT=0 output 3/3, omitted optional position EXIT=0.
> - **Criterion really deleted not renamed**: `git show c3082a160:src/middle/core/ir_gen.rs`
>   `build_arg_slots` **0 hit** (parent commit same file 3 hit, definition at :521);
>   `slot_order_arg_indices` (fix state :521) line by line =
>   `sort_by_key(|&arg_idx| positions[arg_idx])` pure sort, no position/name/optional position
>   judgment residue, document comment states "not judge position, not judge name, not judge
>   optional position"; deleted 71 lines judgment block (parent commit :521-593, six rules: empty
>   table loud / position occupy slot / order illegal E1111 / unknown name E1014 / competing slot
>   E1111 / enter slot) no longer exists; `bind_args_to_params` 1 definition 6 calls (typecheck 2:
>   `expressions.rs:2854` / `:5653`; ir_gen 4: :7828 / :7926 / :8021 / :8351), consistent with
>   implementer report; parameter name unique exit = `export_signature_param_info` (environment.rs
>   hand-annotated / typecheck::mod stdlib / ir_gen lookup), no second self-parse.
> - **Self-injury regression pinned** (`t.pick2(a = 2)`): `git show c3082a160 --name-only` **doesn't
>   contain `named_args.rs`** (4 files: environment.rs / tests/mod.rs / native_param_names.rs /
>   ir_gen.rs) ⇒ pin test not changed; independent rerun that case 1 passed, named_args full set
>   **19 passed / 0 failed**.
> - **No regression gate** (`c3082a160` tree): `cargo test --all --quiet` (4×4 caliber) EXIT=0—lib
>   **2806** + 2 ignored, integration **297**, 52/44/9/2/1/9 all green 0 failed; `cargo fmt --check`
>   / `clippy -D warnings` both EXIT=0; corpus diff (dual parameter) 348/348 difference empty;
>   `check-snapshot-drift.sh` PASS (204 snapshot zero drift); `check-docs-truth.py` EXIT=0; 3.16.1
>   four forms control (stdlib surface) E1014 / E1010 / E1111 / release.
> - **Uncovered surface (t16 five items, verbatim transcription)**: ① `testsuite.py run full` (CI
>   caliber resource profile channel) not rerun—implementer already fell back per ruling (exclusive
>   wave peak 8.11 GiB > budget 7.51 GiB, exit 2 is tool by design refuse); this round covered with
>   4×4 and 2×1 two calibers' `cargo test --all`. ② Hand-annotated FFI surface (host-FFI) can only
>   be driven via Rust test (`add_native_signature` in-repo zero caller, no CLI surface)—"real FFI
>   call in host embedding process" not tested. ③ Variadic (`...`) named argument form no
>   construction sample (no in-repo criterion sample). ④ `check-boundary.py` / `check-fanout.py` not
>   rerun (implementer already passed; no cross with behavior criterion). ⑤ CI (GitHub Actions) per
>   agreement not waited, submitted to closure session check.
>
> **4. Adversarial review (t17, reviewed revision `c3082a16051cb1bfb8b84bdc62ec6bd29d90ff2d2`,
> parent = `537b46b7488a60b42cdc35110e0a413c80a58169`; all content with `git show <rev>:<file>`
> take, line numbers re-taken on referenced revision)**:
>
> - **① Criterion single source reverse check**: `slot_order_arg_indices` (ir_gen.rs:521-527)
>   **doesn't reorder by name** (sort key is shared criterion given slot number, name only in
>   `arg_binding::bind_args_to_params`), **missing parameter doesn't fall back** (absent slot
>   doesn't produce argument index at all), **doesn't distinguish optional position** (IR two places
>   pass all optional, other two places pass real value `i >= required_min`). Whole repo "find
>   position by parameter name / find member" grep item by item checked: **slot calculation only
>   `arg_binding.rs:76` one place**, other 7 hits each belong to other concern (generic type
>   parameter's constraint interface check / const generic argument substitution / generic type
>   variable binding / generic type parameter substitution / const generic parameter lookup / method
>   definition lookup / struct field lookup).
> - **② Responsibility boundary (missing parameter only loud at typecheck, IR only replays slot
>   binding)**: Typecheck side `bind_named_args` (`expressions.rs:2854`) pass all required ⇒ missing
>   any value parameter E1010; native export arm (`:5653`) pass real optional position
>   `i >= required_min` ⇒ only for real required position E1010, `?msg` omission released. IR side
>   this commit changed two places (`ir_gen.rs:7926` RFC-004 binding path / `:8021` overload
>   resolution method path) **uniformly pass all optional, only replay slot binding** (comment
>   clearly writes "missing parameter belongs to typecheck" and "re-judge missing parameter per
>   declaration table will misreport legal resolution as E1010 (`t.pick2(a = 2)` measured)").
> - **③ API shape**: environment.rs:990-993 clearly writes "this function **must not** parse
>   parameter name itself" and points to unique exit `export_signature_param_info`; :997-999 writes
>   "callers that can only get `MonoType` should parse signature string on their own side then go
>   through this API".
> - **④ Behavior change surface**: Code four differences (same parameter name twice E1111→E1015 code
>   change / unknown name E1014 text change / empty table E1014 text change / positional argument
>   exceeds IR side declaration table: silent discard→send as is), **measured reachable form no
>   diagnostic code / text change** (typecheck judges first, same code), IR product on 5 self-built
>   probes **byte-by-byte consistent** (p1 / p5 goes this commit changed branch and product
>   unchanged, p2 / p3 / p4 two states same code, p9 unconstructable, p6–p8 source layer
>   unreachable); t15's 8 CLI probes two states per code consistent. Differences all in **already
>   failing** programs (programs still fail) ⇒ D59 "don't change original successful program's
>   external performance" not touched.
> - **⑤ Test quality** (not just look at red green): 12 new tests
>   (`typecheck/tests/native_param_names.rs`) item by item to `test-specification`—naming
>   `test_<what>_<scenario>`, three-part AAA comment, assertions all with custom message, setup
>   repeated ≥3 times extracted helper, helper internal panic with context; **no "only break exit
>   code" type weak assertion** (first `assert!(diagnostic non-empty)` then
>   `assert_eq!(per code equal)`, both sides empty won't pass through); 3 negative cases (legal
>   named call release / optional position omission release / explicitly give optional position
>   release) still pass under M1 red state ⇒ they **only prevent false report**, exactly their use.
>   **M1** (remove `native_param_names` filling) → `cargo test --lib native_param_names` **9 failed
>   / 3 passed**, TEST_EXIT=101 ⇒ new test **really pinned fix**. **M2** (entire segment rollback
>   `ir_gen` merge) → `cargo test --lib` **2806 passed / 0 failed / 2 ignored**, exit 0 ⇒ merge
>   surface **no test coverage**. `pick2` pin (existing `named_args.rs:253-283`, this commit not
>   changed) goes **real full chain** and asserts **runtime value** `Int(20)` ⇒ **really pinned
>   overload resolution** (not "no error"), and it's live gatekeeper for "IR must not re-judge
>   missing parameter".
> - **⑥ Limit registration**: Host native only enters `TypeEnvironment`'s two tables, **doesn't
>   enter `ModuleRegistry`** ⇒ IR generation resolves namespace members by registry, so host native
>   in IR phase originally unresolvable (E3006), **earlier than this task's embedding surface
>   limit**; this limit and "why hand-annotated surface must be observed at typecheck phase" written
>   in **test file document comment** and `environment.rs` field document (not only in report).
> - **Gate eight items in reviewed revision all rerun all green**: fmt --check / clippy -D warnings
>   / `cargo test --all` (**3220 passed, 0 failed**; lib 2806 + 2 ignored, integration 297) / corpus
>   diff 348-348 zero drift / IR snapshot 204 zero drift / docs-truth / check-boundary (0 exemption
>   0 violation) / check-fanout (new symbol no same-name multiple definition entries).
> - **Four observation items (verbatim registration: verified, judgment doesn't constitute finding,
>   no need to dispose, but registered to avoid being treated as unresolved violation later)**:
>   - **O1**: Commit message says "after this merge **whole repo only has one shared
>     criterion**"—for **slot binding** it holds; but typecheck's **method overload arm** still has
>     a "find candidate table by name (E1014) + arity (E1010)" own judgment
>     (`inference/expressions.rs:5578-5600`), it uses **candidate group representative**'s parameter
>     table (`method_overload_param_names`), different from shared criterion's **already resolved
>     candidate** table. Judgment basis: this place **existed before this commit, this commit not
>     touched** (`git show c3082a160 --name-status` only 4 files, no expressions.rs), and belongs to
>     "candidate matching" concern (different table, different semantics), not slot binding.
>     **Wording too broad, registered as observation**.
>   - **O2**: `add_native_signature` changed to take signature string, **no**
>     `MonoType → signature string` shared helper ⇒ future callers that can only get MonoType can
>     only each write one serialization. Judgment basis: parameter name / required minimum already
>     locked in document in unique exit `signature::export_signature_param_info`; currently **zero
>     caller**; document already writes "callers that can only get `MonoType` should parse signature
>     string on their own side then go through this API" ⇒ residual risk only in "how to construct
>     string" section, **low**.
>   - **O3**: This commit's **merge part** (ir_gen deduplication) **no any test coverage** (M2:
>     entire segment rollback ir_gen then `cargo test --lib` still 2806 passed all green). Judgment
>     basis: "deduplication" no new behavior to pin, by nature; "single source" nature guaranteed by
>     grep + `check-fanout.py` gate. **Optional enhancement**: Add one **IR layer** case in
>     `native_param_names.rs` (cover ir_gen:7926 / :8021 path), let merge surface also have
>     guardrail.
>   - **O4 (attribution correction)**: Commit message and report attribute d2's E1010 to "**method
>     arm** arity pre-check first triggers"—measured that E1010's help is generic text "check the
>     argument count of the function call", **not** shared criterion "missing parameter"'s help
>     ("缺少必需形参：…") ⇒ trigger is **native arm**'s arity pre-check (`native_arity` table).
>     **Code behavior conclusion unchanged** (parent commit existing, this time not changed, already
>     reviewed), only **attribution wording needs correction**; and "`list.push` duplicate naming
>     i.e. E1010" must limit to "**arity mismatch + duplicate naming**"—only duplicate naming and
>     arity match (`list.push(list = v, list = 9)`) two states are both **E1015** (duplicate naming
>     before arity).
>
> **5. Gate and resource channel (one external link)**: This task **first real use**
> `python scripts/dev/testsuite.py run full` → **exit 2**—exclusive wave peak 8.11 GiB > local
> budget 7.51 GiB (gap 618 MiB), tool by design **refuses silent skip** ⇒ fall back
> `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 cargo test --all --quiet` → **exit 0** (3220 passed / 0
> failed). This channel status chain **#486**: `full` locally due to peak extrapolation too
> conservative unavailable, **calibration (2600 file from measured calibration) belongs to WBS
> 0.5.2**.
>
> **6. Count table: this round not changed**—`3.17.1` already 2026-10-15 registered and counted in
> table header ("2026-10-15：+2 = 3.17.1 / 3.18.1"), this round only check + note, **three-level
> count unchanged** ⇒ `09` top's second-level 67 / three-level 176 **not changed**. This round
> **doesn't add** count deviation (parent entry 3.17's "1 three-level task" note originally
> correct); previous section "11" recorded 6 pre-existing deviations (parent entry count note
> lagging) **unrelated to this round, not introduced by this round**, still left per that section's
> caliber for accounting review.
>
> **7. Batch tree recycle (gc and status original output)**: `python scripts/dev/scratch.py gc` →
> `[scratch] deleted verify-p3-t16 (released about 10235.4 MB)` /
> `[scratch] deleted verify-p3-t17 (released about 10079.2 MB)` /
> `[scratch] gc complete, total released about 20314.6 MB`; `python scripts/dev/scratch.py status` →
> `[scratch] no batch tree`; `git worktree list` → only main mount (`dev`). **Deleted together with
> gc are also two batch trees' verification evidence** (`verify-p3-t16/`'s `EVIDENCE.md`,
> `verify-p3-t17/`'s `EVIDENCE-t17.md` and probes / patches / original logs)—they are **registered
> batch directories**, recycled with tree; this section's third, fourth sections have put their
> measurement numbers, gate values, mutation table and four observation items **as is into record**
> for evidence, conclusion doesn't fail due to file deletion. **Unaffected archive** (not registered
> batch directory, gc doesn't touch): `wbs317` (implementer evidence: evidence-3.17.1.md /
> red-test.txt / trial-before-after.txt + fix-3171.patch / attribution table etc.; in workspace
> `target/`, **not in repo**; conclusion already in this section, workspace path not as reference
> source). Acceptance statement (resolvable): `git status` clean;
> `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only has main mount;
> no this batch new directories outside repository.
>
> **8. Document real build and commit hygiene**: This round's check / note / registration
> **accumulated into one** document commit (AGENTS.md rule 8); before commit locally run
> `npm run docs:build` (AGENTS.md rule 10⑨) and `python scripts/ci/check-docs-truth.py`,
> markdownlint zero warning. **Don't push** (members don't push, see AGENTS.md rule 10⑧); **CI /
> Deploy Docs conclusion submitted to unified check session** backfill, not waited in session.
>
> **9. Outstanding items and registration**:
>
> 1. **Round 1 closure registered outstanding item ① closed**: `src/driver/arms.rs`'s "Decision B1"
>    comment opposite to code fact after 3.4.6, by `537b46b74`
>    (`:memo: docs(driver): correct opposite-to-fact B1 comment (3.4.6 closing omission)`) in-place
>    rewritten to current state fact (behavior neutral, only comment line; wording aligned with
>    `src/driver/tests/program.rs`'s same fact's statement).
> 2. **This round's new registration**: O1–O4 four observation items (verified, doesn't block,
>    registered); 0.5.2 (`full` channel's 2600 file measured calibration + pre-commit / CI wiring);
>    #489 ureq 3 migration and `.cargo/audit.toml` exemption's hard removal condition; 3.8 / 3.12
>    pending ruling, 3.18 precondition pending P6—**all not in this team's scope**.
>
> **10. CI gate conclusion (AGENTS rule 13①, scheduled check 2026-10-21 backfill)**: This batch's
> last commit push tip = `1f5eb8665`
> (`:memo: docs(meta): verify checklist supplement three CI unique gates, and write "emission point" caliber`)
> → CI **39604109534 success**, Deploy Docs **39603305398 success**; this tip covers `e7a3fd4b1`
> (regenerate error code generated product) and all commits before it. `e7a3fd4b1` itself's that
> round CI **39603305171 cancelled**—per rule 10③ "consecutive push will cancel previous round", its
> conclusion taken over by successor `1f5eb8665`'s success. Failure chain for tracing: `c3082a160`
> (**39587784184 failure**) and `190a0e7a3` (**39599653111 failure**) both same job "Error-code docs
> up to date (generator idempotent)", root cause = 3.17.1 delete judgment + line number translation
> after didn't rerun generator; by `e7a3fd4b1` regenerate, `1f5eb8665` closure then turn green,
> cumulative red 5 rounds, led to AGENTS rule 10⑤ wording patch (`1f5eb8665` is that patch).
>
> Check command and original output (2026-10-21 13:19Z rerun):
> `gh run list --workflow ci.yml --limit 4` → `39604109534 success` (`1f5eb8665`) /
> `39603305171 cancelled` (`e7a3fd4b1`) / `39599653111 failure` (`190a0e7a3`) /
> `39587784184 failure` (`c3082a160`); `gh run list --workflow docs-deploy.yml --limit 3` →
> `39603305398` / `39599653238` / `39587784210` all success.

## Document Reference Governance and "Unresolvable References" Gate (2026-10-21)

> **Trigger**: When cleaning workspace evidence directory, found in `09` / `09a` / `CHANGELOG` /
> branch maintenance guide there are 24 places treating "directories in workspace, excluded by
> `.gitignore`" (batch tree / probe / evidence directory) as **evidence source**—such directories
> don't exist after clone, external contributors can't verify any one number (AGENTS.md meta rule ①
> and rule 10④). One of these is 4.4 closure round's fix: changed old path to point to another local
> path, equal to moving violation down one layer not eliminating.
>
> **1. Reference inlined in place (47 places = zh 24 + en 23)**: Conclusion, numbers, CI run id,
> delivery commit written in place into this document and `09`, source changed to in-repo path
> (record section anchor) or revision / run id / issue number. CI run id details and S9 seven-group
> list **already inlined**, deleted only redundant path pointer. en mirror regenerated by
> auto-translate bot from zh source.
>
> **2. New mechanical gate** `scripts/ci/check-docs-refs.py` (hooked into `ci.yml`'s docs job):
> track path-form tokens in text files' backticks and markdown links, if hit by `.gitignore` with
> specific subpath then red; placeholder form (rule statement) and bare directory name not judged;
> system runtime directory (package manager cache, build product, playground publish directory,
> local agent workbench) takes **item-by-item write reason** exemption table (26; details written in
> gate's own runtime report—uploaded as artifact in CI, not in repo). **Criterion can red can
> green**: self-test 9 cases (4 negative cases must red / 5 positive cases must green, including
> cross-batch boundary one); end-to-end mutation verification—roll back one violation reference →
> gate red → restore → green → that file with pre-mutation byte-by-byte consistent.
>
> **3. Rule wording supplement**: `AGENTS.md` rule 10④ clearly states "'in-repo path' means
> git-tracked in-repo files", rule 9 supplements "things in workspace are not evidence source", rule
> 10⑤'s local verify checklist supplements this gate (user 2026-10-21 approved).
>
> **4. Two platform consistency (first version gate's defect and fix)**: First version gate
> **Windows local green, CI (Linux) red**—`git check-ignore` for "directory form path ending with
> `/`" two platforms behave differently (Linux reports real rule, Windows reports one **empty
> pattern** = `.gitignore`'s empty line), only recognizing non-empty pattern when Windows side
> missed `docs/public/wasm/` (playground publish directory, RFC-037 reference). Fix: each token
> query three forms (with slash / without slash / without slash + probe subitem), any form hits
> **non-empty** pattern counts as hit. After fix Windows and Linux dual run **same number**: 1122
> tracked text files / 5533 path-form tokens / 0 violations / 26 exemptions / selftest 9/9. Lesson:
> Gate script **at least run once on two OS and checkout forms** before hooking CI (CI's job log
> needs admin permission, when not available use "Linux real clone reproduce" to locate).
>
> **5. CI evidence (AGENTS.md rule 13①, scheduled check backfill)**: Fix commit `c7a0dd417` → CI run
> **39626722284 success**; previous commit `fff9580f8` → CI run **39623038087 failure** (red at
> newly added "Check doc references are resolvable", i.e. above platform difference, fixed by
> `c7a0dd417`), same commit Deploy Docs **39623038228 success**, Auto Translate Docs **39623038027
> success**; earlier commits (`ec2893e0d` / `ff2104b0c` / `ffd43b1d0`)'s CI runs cancelled by
> concurrency (**cancelled** 39621547581 / 39620372322 / 39619834870), per "use successor run's
> conclusion to cover previous one" take conclusion.
>
> **5.2, This section's in-repo commit's CI evidence (2026-10-22 scheduled check backfill)**: This
> section's in-repo commit `612c65322` → CI run **39798235412 success** (7 jobs all green, including
> newly added "Check doc references are resolvable" and "Error-code docs up to date" two steps),
> Deploy Docs **39798235484 success**; same commit Auto Translate Docs **39798235413 failure**,
> failed at last step "Commit translations"—translation itself success (Translate documents / Format
> / Lint all success), but push time remote already pushed forward by this batch's code commit and
> bot's own previous auto-translate commit, `git pull --rebase` + `git push` blocked, **result is
> this document's en mirror not yet containing this section** (zh as only source; next push touching
> `docs/src/**/*.md` will again trigger that workflow to supplement translation). Parallel 4
> bugfixes (`eb4124cf0` / `14e9b2704` / `96638ec9f` / `06ac6d687`)'s CI run **39797917813
> cancelled** (consecutive push cancelled by concurrency), per "use successor run's conclusion to
> cover previous one" take conclusion.
>
> **5.3, Gap closure and this batch's closure (2026-10-23 scheduled check backfill)**: After
> supplement push bot's Auto Translate Docs run **39904034074 success** (bot commit
> `449360a5a`)—**this document's en mirror already contains this section**, previous round's gap
> closed; same commit CI run **39904034164 success**, Deploy Docs **39904034075 success**. Previous
> round 39798235413's failure is bot's own race: its previous translation commit and this batch's
> code commit simultaneously push forward remote, `git pull --rebase` + `git push` rejected.
> **Operation lesson**: Don't immediately push after pushing—leave landing window for bot (this time
> about 15 minute interval succeeded).
>
> **6. Registration caliber**: This section is **document governance** record, doesn't add WBS
> kanban line, `09` top count table unchanged.

## P4 4.10 Multi-file Semantic Gap Closure (Round 1: 4.10.4 / 4.10.5 + Add-frame 3.6.5)

> **1. Scope and round relationship**: This section = **4.10** second-level task's first round
> closure, scope = **4.10.4** (issue [#467](https://github.com/ChenXu233/YaoXiang/issues/467),
> rootless multi-file initialization by `use` dependency topology order), **4.10.5** (issue
> [#476](https://github.com/ChenXu233/YaoXiang/issues/476), rootless defines `main` uncalled exit
> code 1), **add-frame merged into 3.6.5** (issue
> [#479](https://github.com/ChenXu233/YaoXiang/issues/479), selective import's function value after
> top-level rebind silent `void`; **add-frame from 4.10 team, user 2026-10-21 approved**). This
> batch also contains one **cross-second-level add-frame** std fix (`src/std/json.yx`'s `\uXXXX`
> subscript + `src/std/tests/json_ops.yx` long-term zero coverage), it doesn't belong to 4.10's task
> surface, only registered in "9" and left evidence as "test false pass" instance.
>
> **2. Four fix's landing content and evidence** (commit number take **post-push** `origin/dev`
> hash; `09`'s check line per agreement doesn't write commit number):
>
> - **4.10.4 (#467)**: Root cause = each unit's `init` segment spliced by **source file path
>   lexicographic order** (discovery period's path lexicographic sort, **not** "discovery
>   order"—`09` original line wrote "discovery order" already corrected this round; measured
>   renaming dependency file flips result, so "dependency first" was never invariant). Fix = in
>   Linking arm by `use` dependency solve **Tarjan SCC topology order** (in-ring by path
>   lexicographic as deterministic agreement, between SCCs by minimum unit index priority dequeue)
>   then reorder unit order, `link_module_irs` split into "file table order" and "initialization
>   execution order" two concepts, **zero IR shape change**. Red before green = change execution
>   order back to identity (revert to discovery order) then 4 order cases all red, CLI reproduce
>   "entry reads dependency top-level value = `void`, exit 0" and "runtime E6007, exit 1" two forms;
>   after restore byte-by-byte return, 7/7 green. Corpus = added 2 multi-file fixtures, baseline
>   **+2 lines** and original 348 lines byte-by-byte unchanged; **IR snapshot zero drift**.
> - **4.10.5 (#476)**: Run entry changed to return **outcome enum** (completed / defined but not
>   called), CLI only based on outcome set exit code **1** (2 reserved for CLI / config usage
>   error); two hint texts **word unchanged**, no new second diagnostic; value-type `main` already
>   evaluated at initialization period not affected. Pin test 2 entries land
>   `tests/integration/cli_e2e.rs`.
> - **3.6.5 (#479)**: Value path's alias fallback arm by "**module source + export kind**" fork—user
>   module exported function → materialize to closure; std constant / native function → keep
>   original zero-arg call form; **no new diagnostic code, `src/std` zero change**. Pin test 3
>   (selective rebind / selective direct call / whole import rebind). Generated product:
>   `docs/src/reference/error-code/E1xxx.md` and `E8xxx.md` each 1 line line number translation
>   (1789→1821 / 1976→2008), **generator regenerate, not hand-edited**.
> - **std side (cross-second-level add-frame, user approved)**: `src/std/json.yx`'s `parse_string`
>   in `e == "u"` branch passes **backslash position** to `parse_uchar`, while `hex4`'s contract
>   requires reading 4 hex digits from `'u'` position ⇒ all `\uXXXX` report "invalid \u escape";
>   change to pass "`'u'` position" then BMP case adv=6, surrogate pair adv=12 arithmetic unchanged,
>   error position still takes backslash (user visible position unchanged). Same commit fix
>   `src/std/tests/json_ops.yx` this **never run test**, and also fix fixture's own three errors
>   (low surrogate segment extra hex digit, trailing `\u0039` never spliced into source string,
>   error position assertion written as "contains 2" but actual message is "invalid literal at line
>   1, column 7"). **IR snapshot same commit regenerate**
>   (`src/middle/core/tests/snapshots/src/std/tests/json_ops.snap`), corpus baseline 1 line change.
>
> **3. Review judgment (batch review round 1, different model family) and three uncovered branches'
> registration**: Review gave two committed implementations **#467 = pass, #479 = pass** (two
> implementations themselves after independent check both hold), at same time pointed out **two
> high-level uncovered branches originally only in code comments, not in WBS**—this round per review
> opinion registered as **4.10.6** (import ring no warning, still report misleading E6007) and
> **4.10.7** (native function as value silent no-action), also registered **4.10.8** (pure yx's std
> export as value; review marked medium, **not verified**) and **3.6.6** (same family criterion
> convergence, refactor item). Review also pointed out **defer reason's classification error**:
> defer reason wrote "need new diagnostic code ⇒ class D pending ruling", but D59's分流 lists
> "silent mismatch / silent no-action" in **class C** (old behavior not successful, can fix with
> refactor) ⇒ must not silently defer with unregistered class D label. Accordingly, three items on
> 2026-10-23 got user ruling (issue [#491](https://github.com/ChenXu233/YaoXiang/issues/491) /
> [#492](https://github.com/ChenXu233/YaoXiang/issues/492) /
> [#493](https://github.com/ChenXu233/YaoXiang/issues/493) /
> [#494](https://github.com/ChenXu233/YaoXiang/issues/494)): **4.10.6 = compile-time W code warning,
> ring must not be silent** (warning surface "ring exists" or "ring reads uninitialized value",
> whether allow `#[allow]` suppression, whether align with existing W10xx family's "evidence
> level"—three **pending consideration**, must finalize before landing); **4.10.7 = really support**
> (materialize native function to value, **no diagnostic code**, end state =
> `use std.io.{println}` + `f = println` + `f("hello")` really print hello); **4.10.8 = class C, no
> code, can directly fix**; **3.6.6 = refactor item** (behavior unchanged).
>
> **4. Refuted assumption (left evidence to avoid duplicate labor)**: "Same file two `use` spellings
> ⇒ dependency edge lost" **doesn't hold**—this form reports existing diagnostic **E5001** (`use`
> target not in unit set), not "silently run by discovery order". Review listed it as refuted
> assumption, so 4.10.4's "no dependency information three situations" each have deterministic
> disposition, no fourth silent channel.
>
> **5. Task list and final state**:
>
> | Task | Subject                                                                                    | Final state                                                                      |
> | ---- | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
> | t1   | Reproduce and locate: two P1's red state evidence collection + #467 fix feasibility ruling | completed                                                                        |
> | t2   | #467 initialization topology order (implementation + pin test + corpus)                    | completed (3rd attempt closure)                                                  |
> | t3   | #467 independent verification                                                              | completed, verdict = pass                                                        |
> | t4   | #479 selective import + top-level rebind (implementation + pin test)                       | completed (2nd attempt closure)                                                  |
> | t5   | #479 independent verification                                                              | completed, verdict = pass                                                        |
> | t6   | #476 exit code 1                                                                           | **failed** (provider side stream interruption accumulation, 6 attempts)          |
> | t7   | #476 independent verification                                                              | **cancelled** (cross-block + add-frame timing, captain took over then cancelled) |
> | t8   | Batch review (#467 / #479 two implementations)                                             | completed, verdict = pass ×2                                                     |
> | t9   | Batch closure (check + 3.6.5 add-frame + record + gate + recycle batch tree)               | completed                                                                        |
>
> **6. Per-role attempt count and provider side failure disposition**:
>
> | Role         | Took on                                                    | Attempt count                                                                                                                                                                                                       |
> | ------------ | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | Probe        | t1 (only read evidence), t6 (#476 implementation takeover) | t1 one success; t6 **6 attempts not succeeded**—during period multiple provider peak stream interruption (`PI_AI_ERROR`, 2064 "server temporarily busy, usually 1-5 minute recovery"), task end state judged failed |
> | Implement A  | t2 first assignment, t4 takeover, t6 review takeover       | t4 one success (attempt 2); t6's implementation finally reviewed and supplemented by them, landed with t4's "clean tree full amount" plan                                                                           |
> | Implement B  | t4 first assignment                                        | attempt 1 due to provider side **400 `invalid_request_error`** (in thinking mode `reasoning_content` must return) judged failed → reassign                                                                          |
> | Implement C  | t6 period implementation takeover                          | consecutive two same **400 `invalid_request_error`** ⇒ per AGENTS rule 11 "replace" disposition: **removed from team**, their task returned to shared pool then reassigned                                          |
> | Implement D  | t2 takeover                                                | attempt 2                                                                                                                                                                                                           |
> | Implement E  | t2 closure, t6 one attempt                                 | t2 3rd attempt completed; t6's one attempt also due to same 400 stream interruption judged failed                                                                                                                   |
> | Verification | t3, t5                                                     | each 1 time, verdict both pass                                                                                                                                                                                      |
> | Review       | t8                                                         | 1 time, verdict = pass ×2                                                                                                                                                                                           |
> | Closure      | t9                                                         | 1 time                                                                                                                                                                                                              |
>
> **Provider side stream interruption and replace disposition caliber** (this round empirically
> reusable): ① **Stream interruption is not defect evidence**—tasks interrupted by stream
> interruption can't judge "zero product" as failed conclusion, can only judge "incomplete" and
> reassign; ② After replace **don't rerun rounds with existing conclusions** (AGENTS rule 12:
> replace will return all tasks of removed member to shared pool, already-ruled rounds shouldn't be
> rerun by scheduler), this round directly reassigned or cancelled returned t6 / t7, no duplicate
> labor; ③ **Zero product during stream interruption** members uniformly first check disk status
> then decide takeover side (this batch multiple times "implementation already in working tree, just
> nobody claiming").
>
> **7. Repository concurrent write and pre-commit stash window (this round two instances)**:
>
> 1. **Concurrent write**: During batch period main working tree was **another concurrent session**
>    written, its staging area already contains `.github/workflows/ci.yml` and
>    `scripts/ci/check-docs-refs.py` etc. **others' files**. Disposition = **commit authority closed
>    to captain**: during batch period all members **must not execute `git commit`** (`git add`
>    still per explicit path), changes only stay in working tree, commit unified by
>    captain—otherwise one party's commit will roll in others' staging content. This batch's four
>    fixes finally by captain with four independent commits landed originate from this.
> 2. **Batch directory deleted by third party**: `scratch.py`'s `rm` / `gc` will **entire batch
>    directory** recursively delete (that directory's reports and probes disappear together). This
>    round empirically: batch directory together with its internal location report and all probe
>    logs deleted during implementation (**attribution not verified**: executor unknown).
>    Disposition = reports and probes moved to **outside batch directory**, and key numbers and
>    conclusion **directly written into task output** (don't depend on workspace file as source);
>    batch tree rebuilt then measurement side takes over by frozen revision.
> 3. **pre-commit stash window**: This round instance = commit std side that commit (amend
>    supplement IR snapshot) period, pre-commit hook **all unstaged changes** stashed, run then
>    restore ⇒ parallel reading working tree's member sees "only staged files" illusion (this round
>    member reported "changes lost" accordingly). Disposition per AGENTS rule 10⑦: ① **before commit
>    first leave patch backup for unstaged changes** (this round instance's three files sha256 =
>    `02e89141…` / `3736961f…` / `98eee1d7…`, after restore one by one same); ② during commit period
>    others only measure by **frozen sha**, don't read working tree; ③ after each commit use
>    `git status` check remaining change integrity.
>
> **8. Local hash rewrite comparison** (this batch's four commits before push experienced
> `git rebase` and `git commit --amend` two kinds of rewrite; `09`'s check line per agreement
> doesn't write commit number, this record keeps **post-push** `origin/dev` hash and gives rewrite
> chain):
>
> | Content                             | Pre-push local → Post-push `origin/dev`                                                 |
> | ----------------------------------- | --------------------------------------------------------------------------------------- |
> | 4.10.4 (#467)                       | `950a5838e` → (rebase) → `eb4124cf0`                                                    |
> | 3.6.5 (#479)                        | `0ef2e8887` → (rebase + amend) → `14e9b2704`                                            |
> | std side (`\uXXXX` + `json_ops.yx`) | `48bc37af0` → (**amend supplement IR snapshot**) → `c67de7e2f` → (rebase) → `96638ec9f` |
> | 4.10.5 (#476)                       | `47c2d5292` → (rebase) → `06ac6d687`                                                    |
>
> `48bc37af0` → `c67de7e2f` this amend's content = supplement **regenerated IR snapshot**
> (`src/middle/core/tests/snapshots/src/std/tests/json_ops.snap`) into same commit: original commit
> only contained `src/std/json.yx` / `src/std/tests/json_ops.yx` / corpus baseline three files,
> after amend four files—i.e. "changing IR behavior must same commit regenerate baseline" red line
> fulfilled in **same one commit**, not leaving "baseline debt". Evidence = `git reflog --date=iso`
> (`commit` → `commit (amend)` → `rebase (pick)` three segments can read one by one).
>
> **9. Corpus / IR baseline in current byte reverification + this round's two new findings**:
>
> - **Current byte reverification** (rerun on `origin/dev` head before closure):
>   `tests/baselines/corpus-parity.jsonl` = **350 lines**, sha256 =
>   `76bd7a1cc82797fe0c2fb4f3406506ad65e7fe268d15b7d807448f4a30b5118b`; on this byte rerun corpus
>   diff probe, product and baseline **byte-by-byte same** (diff empty) ⇒ current byte zero drift.
>   **Note**: Two verification rounds (t3 / t5)'s judgment binding is **earlier** corpus byte
>   (`41cc0343…`), different from current `76bd7a1c…`—review already pointed out this difference,
>   this round per current byte reverification and record; hereafter reference corpus numbers must
>   first confirm cited byte.
> - **New finding ①: `src/std/tests/json_ops.yx` long-term zero coverage**—this file defines `main`
>   but **never called**; in script mode top-level statement is program body ⇒ 15 `check_*`'s
>   assertions **never executed once**, exit code always 0, i.e. **test false pass** typical
>   instance. `std.json`'s `\uXXXX` defect therefore long-term unexposed. This round fixed
>   (supplement `main()` call + fix fixture's own three errors + fix `std.json`'s subscript), after
>   fix this file's 15 check assertions all really run and pass (exit 0). **Lesson**: "Has test
>   file" ≠ "criterion executed"—in script form "defined `main` but not called" test file is **false
>   green** source, should be independent check item (control: #476's "define `main` not called" in
>   rootless form exit 0, also lets such file long-term use exit 0 as filler).
> - **New finding ②: Ring import no W code warning**—4.10.4 ring's order **deterministic-ized**, but
>   ring itself still silently tolerated: ring's unit read uninitialized top-level value ⇒
>   misleading E6007 "type mismatch". Already registered as **4.10.6** (issue
>   [#491](https://github.com/ChenXu233/YaoXiang/issues/491); user 2026-10-23 ruling A =
>   compile-time W code warning, **must not be silent**; W's warning surface pending consideration).
>
> **10. Gate values**:
>
> - **Before closure on current `origin/dev` head local rerun**: `npm run docs:build` → exit 0
>   (document commit's **real build**, AGENTS rule 10⑨—markdownlint can't catch VitePress's Vue
>   compile error); `python scripts/ci/check-docs-truth.py` → exit 0;
>   `python scripts/ci/check-boundary.py` → exit 0 (violation 0, `pub(crate)` count guard baseline);
>   `python scripts/ci/check-fanout.py` → exit 0; `python scripts/ci/check-docs-refs.py` → exit 0
>   (1124 tracked text files, 5535 path-form tokens, no unresolvable reference); error code
>   generator rerun after `git diff --quiet` → exit 0 (generated product zero drift); corpus diff
>   rerun → same as baseline byte-by-byte (350 lines).
> - **Each fix surface's own gate** (each verification round measured on frozen revision): #467——fmt
>   0 / clippy 0 / `cargo test --all` exit 0 (lib **2810 passed** + 2 ignored, integration 297
>   passed, 0 failed) / docs-truth 156/156 pages / boundary 0 / fanout 0 / docs-refs 0 / generator
>   rerun zero drift / **IR snapshot diff empty**; #479——same caliber exit 0 (lib 2810, integration
>   **300** = 297 + new 3 pin tests each name pass); #476——**clean tree** full amount exit 0 (9 test
>   target all ok, **3227 passed / 0 failed**, including
>   `tests/yx_runner.rs::test_all_yx_files_pass`).
> - **Gate discipline one instance**: Full amount `cargo test --all` when "shared working tree with
>   uncommitted changes" once red at `tests/yx_runner.rs::test_all_yx_files_pass`
>   (`src/std/tests/json_ops.yx` exit 1), once recorded as "this batch's defect"; on **clean frozen
>   revision** retest that case passed ⇒ that red belongs to "tree with #476 patch state"'s
>   measurement, not fix surface's attribute. **Lesson**: Full amount gate must measure on
>   **isolated clean tree**, otherwise will misjudge "shared tree state" as "delivery defect".
>
> **11. Batch tree recycle (gc and status original output)**:
>
> `python scripts/dev/scratch.py gc` → `[scratch] deleted wbs410 (released about 13268.2 MB)` /
> `[scratch] gc complete, total released about 13268.2 MB`; `python scripts/dev/scratch.py status` →
> `[scratch] no batch tree`; `git worktree list` → only main mount (`dev`). **Deleted together with
> gc are also batch tree's verification evidence** (each verification round's probes, mutation
> patches, original logs and conclusion text)—they are **registered batch directories**, recycled
> with tree; this section's second, third sections have put their conclusion, numbers, mutation
> table and gate values **as is into record**, conclusion doesn't fail due to file deletion.
> **Unaffected archive**: non-registered batch directory's evidence directory (not in repo;
> conclusion already item-by-item in this record, workspace path not as reference source).
> Acceptance statement (resolvable): `git status` clean (after closure document commit);
> `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only has main mount;
> no this batch new directories outside repository.
>
> **12. Document real build and commit hygiene**: This round's check / note / registration
> **accumulated into one** document commit (AGENTS rule 8); before commit locally run
> `npm run docs:build` (AGENTS rule 10⑨) and `python scripts/ci/check-docs-truth.py`, markdownlint
> zero warning. **Don't push** (members don't push, see AGENTS rule 10⑧); **CI / Deploy Docs
> conclusion submitted to unified check session** backfill, not waited in session (AGENTS rule 13).
> Also: This round incidentally changed two historical record's residual **local absolute paths**
> (`git worktree list` output's references) to expression not containing local path (meta rule ①).
>
> **13. Outstanding items and registration**:
>
> 1. **Pending landing (already got user 2026-10-23 ruling, registered in `09`)**: 4.10.6 (issue
>    #491, compile-time W code warning, must not be silent; W's warning surface three pending
>    consideration), 4.10.7 (issue #492, really support native function as value, no code), 4.10.8
>    (issue #493, class C, no code, can directly fix), 3.6.6 (issue #494, criterion convergence
>    refactor item, behavior unchanged).
> 2. **Known imperfect items**: ① **t6 task record end state failed, t7 cancelled** (provider stream
>    interruption + add-frame timing) ⇒ #476's **independent verification round not separately
>    run**; its verification evidence from "clean tree full amount exit 0 (3227 passed / 0 failed)"
>    and that round's exit code pin test (`tests/integration/cli_e2e.rs` two). ② Two verification
>    rounds (t3 / t5)'s judgment binding is earlier corpus byte, this round already per current byte
>    reverification (see "9"), but **verification report itself's byte binding still old
>    value**—must explain when citing. ③ Review **didn't personally rerun** two verification rounds'
>    mutation construction (batch tree already released by multiple people, rerun needs exclusive
>    tree + one incremental build), with "product check + blob byte-by-byte check + self-built
>    probe" conversion.
> 3. **Uncovered surface** (each verification round self-listed, this round not supplement): ① Ring
>    import's W code warning not implemented (already registered 4.10.6); ② `check` and LSP form not
>    separately probed (with `run` share Linking arm, library-level pin test indirect coverage); ③
>    Mutation only did "revert discovery order" one direction—ring's path order / minimum index
>    tie-break each failure form not done counter-proof mutation (its behavior covered by pin test
>    assertion); ④ Multi-package / workspace form not in CLI probe; ⑤ #479 side: native function as
>    value, three-segment module path, non-existent export name, `ExportKind` other variants,
>    mutation round not run full amount, LSP path; ⑥ Review's self-built 7 probes, "pure yx's std
>    module function as value" one marked **not verified** (already registered 4.10.8).
> 4. **Commit message scope rejected twice**: This batch's two commits' initial scope took from
>    **source directory name** (`ir_gen` / `cli`), twice rejected by
>    `scripts/hooks/check-commit-msg.py` (its scope allow table doesn't have these two words),
>    changed to **subsystem name** `codegen` / `util` then passed (allow table: `frontend` /
>    `middle` / `backends` / `std` / `formatter` / `lsp` / `package` / `util` / `proof` / `driver` /
>    `codegen` / `monomorphize` / `lifetime` etc.). **Lesson**: scope take **subsystem** not source
>    directory name.
> 5. **This batch's untouched existing debt** (not in this team's scope, transferred as is): 3.8
>    (native shadowing of user function name), 3.12 (executor configuration propagation between
>    runtime modes), 3.18 (cross-module type identity, precondition pending P6), 0.5.2 (pre-commit /
>    CI wiring + resource file measured calibration), #489 ureq 3 migration.
