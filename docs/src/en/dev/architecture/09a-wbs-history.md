# WBS Historical Notes (Implementation Log · Audit · Ruling Records)

> This document is the **historical notes** for [09-execution-wbs.md](./09-execution-wbs.md)
> (construction task table): implementation logs, audit findings, ruling records, and errata for
> each phase. **Division of work**: 09 describes only the **plan** (kanban, three-level task table,
> dependencies, risks, acceptance); this document records only the **history** (what was done, what
> was found, how it was ruled), appended chronologically, without rewriting history.

## P2 Establishing Equivalence Criterion Baseline

> **P2 Implementation (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green.
> **D38 prophecy fulfilled**——the green-running process actually identified and fixed three real
> defects in ir_gen (see 06 §G6 for details): method locals table truncated to parameter count
> (local_count underestimated, runtime relies on Frame::set_slot fallback), unannotated parameters
> dropped by filter_map (three places, params metadata misaligned with frame layout), match result
> slot depended on implicit Void initialization of frame (now explicitly pre-initialized). The
> actual semantics of jump target is flattened instruction index (not label-keyed as in 07 text),
> CFG built at instruction level——registered in verify.rs file header. **Commit reference**: ir_gen
> fix = `a9ea199b`; verify main body = `d5b18390` (⚠ Parallel stream COMMIT_EDITMSG collision caused
> its message to be incorrectly marked as "docs(rfc): RFC-027a…", content is actually verify.rs +
> six test files, buried in history middle and unamendable, hereby registered).
>
> **P2 Narrowing Partial Implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> bleeding-control channel pre-unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; judgment contract uses the same `TestFileSpec` as single-file
>   corpus (library-side single point), binary positioning/subprocess launch extracted to
>   `tests/common/` and yx_runner migrated synchronously (prohibition three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. Trigger source uses the `SumUpTo(3, r)`
>   form nailed by rfc027 tests (return position/call site/binding position)——02's `Sorted(3)`
>   annotation example would be substituted and evaluated directly into Disproved in current code,
>   not presenting Unproven silently; the 07 pseudocode's Sorted(3) is for illustration only.
>   Red-state measured data see test file header.
> - Two red criteria hung with `#[ignore]` standing per 2.4.5 red skeleton precedent: default suite
>   all green, `-- --ignored` reproduces red state; **P3 fix implementation removes attribute to
>   turn green**.
> - The reason for comparison normalization's intentional deviation from 07 (code, file, line) table
>   (removing file/line, comparing only error code sets) is written in 2.4.1 test comments.
>
> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **moved out of P2**: the `Program` / `Obligations` types they reference only exist in P4, P2
> cannot compile. The former is P4's 4.1.4 (don't double-register), the latter's `#[ignore]` red
> skeleton merged into 4.3.1.
>
> **2.3.3 Implementation (2026-10-07)**: C4 mandatory coverage checklist item-by-item verified (07
> §143), 10 types of semantic corpus layer fully covered and all within corpus-parity.jsonl diff
> baseline (26 representative files 26/26 hits, grep evidence)——ref/borrow/move 05-ownership full
> directory (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep, etc.), closure
> capture closures + closure_arg_inference + spawn_capture, currying curry_value_fix +
> curry_tail_expr, spawn 04-concurrency ×13, iterator for spawn_for etc. 16 files, sum types
> sum_type__+ pattern_or_guard + match_call_scrutinee, existential mandatory points
> interface_dynamic_dispatch (Vec(Animal) existential type heterogeneous packaging triggers
> existential_coercions) + interface_rebind_dispatch, `?`/Try propagation question_propagation +
> option_try + try_methods + user_try_type, method overload method_overload (+
> method_overload_ambiguous_err in diagnostic diff surface), refinement constraint Drop sequence
> refined__ ×5 + 05-ownership driven ReleasePlan (span key contract has another 2.4.4 red criterion
> standing). Performance baseline `benches/pipeline.rs` checked in (07 §145 two of three smoke
> tests): `corpus_compile_frontend` full corpus compilation first test 17.6s, `cli_cold_start_hello`
> process cold start 169.7ms; interpreter throughput is already covered by benches/lib.rs hotpath
> group (interp_fib_recursive_27 / interp_loop_steady_10m), not duplicatively built. Regression gate
> check-perf-regression.sh unified to P9 checklist (2.5).

## P3 Correctness Bug Fixes (Minimum Plan)

> **Error type annotations for landed rows** (retrospective annotation, no history modification):
> 3.1 = B ｜ 3.2 = B (defensive empty arm, criterion turning green carried by 3.1) ｜ 3.3 =
> A ｜ 3.4.1 / 3.4.2 = A (new W code) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 /
> 3.4.7 = D (user-visible surface, already user-ruled) ｜ 3.4.8 ①② = C (main) + A (incidental three
> places) ｜ 3.5 / 3.6 / 3.7 / 3.8 = D (all user-ruled).
>
> **P3 Diagnosis (2026-10-12, user request)**: Address the perception of "P3 items continuing to
> grow" with an account breakdown.
>
> **One. First clarify the scale (perception amplified by two accounting defects)**
>
> 1. **Number reuse**: `3.5.x` is shared by two task families——(a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 three-level items: lexical and parser branches / AST `is_pub` /
>    checker side / dead code exemption convergence / peripheral echo / std and diagnostic text /
>    language reference convergence, all checked); (b) 2026-10-10 registered **3.5 higher-order type
>    parameter call path completion** (3 three-level items). Both share `3.5.x` ⇒ cross-reference
>    ambiguity (**2026-10-12 already renumbered per user ruling: 029g family changed to `029g.x`**),
>    and 029g's rows were miscounted into P3. **Pending ruling**: rename 029g family to `029g.x` (or
>    move into its phase), other document references synchronized.
> 2. **Missing checks**: `3.7`'s 4 three-level items (3.7.1–3.7.4) all completed (d922a551 /
>    38bd4b9e) but not checked ⇒ this item corrected this round.
>
> **Two. P3 Actual Account (12 second-level rows)**
>
> | Row                                                                     | Status                                                                                                                                                                             | Disposition Suggestion                                                                                                      |
> | ----------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                         | Completed                                                                                                                                                                          | —                                                                                                                           |
> | 3.4 Silent drop point additions (8 three-level)                         | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (prerequisite 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (prerequisite 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4/3.4.5 unlocked with 4.3.1                                                                  |
> | 3.5 Higher-order type parameter call path completion (3 three-level)    | Not started (declaration side live, call side broken)                                                                                                                              | Keep P3; **does not block refactor mainline**, but is "accepted feature not implemented", priority higher than pure cleanup |
> | 3.6 Function type value call path (1 three-level)                       | Not started (semantics already cut to (i))                                                                                                                                         | **Keep**: correctness defect, clear criteria                                                                                |
> | 3.7 init segment call name qualification (4 three-level)                | **All completed**                                                                                                                                                                  | Check and close this round                                                                                                  |
> | 3.8 native shadowing (1 three-level)                                    | Not started                                                                                                                                                                        | **Stuck on decision** (first determine name resolution priority), not scheduling issue                                      |
> | 3.9 / 3.10 / 3.11                                                       | Completed (2026-10-12)                                                                                                                                                             | —                                                                                                                           |
> | 3.12 Runtime pattern executor configuration propagation (1 three-level) | Not started (Class D, user ruling B: separate team)                                                                                                                                | **Suggest move out of P3 → P8 (execution layer)**, pending ruling                                                           |
>
> **Four. 4.3.1's Leverage (Main Line Landmark)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the prerequisite for **3.4.4**, **3.4.5**, and **4.5.1** (cross-layer contract
> PlanId-ization) ⇒ taking it once advances the mainline and clears two P3 stranded items. 3.5
> (HKT), 3.6 (function type value call), and 4.6.1/4.9.1 all have **no unmet prerequisites**, are
> do-able anytime; 3.8 stuck on semantic decision (user-defined priority vs. explicit diagnostic).
>
> **Three. Mechanism Suggestion (Prevent Recurrence)**: Newly registered second/third-level rows
> **must carry two fields**——"Belonging phase" (which P it truly belongs to) and "Whether blocks
> mainline"; `registered pending scheduling` no longer as default state, unannotated items
> considered unschedulable. This way inflation is visible and sortable, rather than undifferentiated
> accumulation. **Pending ruling**: whether to write this into this document's registration
> convention and `src/dev/coding-rules.md`.
>
> **P3 Implementation (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + sole getter
> (recorded as `pub(crate)` at the time; **2026-10-07 verified actual implementation is
> `pub(super)`**——the `#434` ruling referenced in code comments has no docs-side record, see 3.4.3;
> full private requires rewriting 11 construction literals, not taken at bleeding-control phase;
> will be closed when P4 Obligations migrates); execution mechanism extracted to
> `src/frontend/proof_execution.rs` (prohibition three single point——cannot be placed in typecheck/,
> that would create L3→L4 reverse dependency); 3.1.2 supplemented **four entry points** per 02
> evidence chain step 5 (WBS row omitted compile_project `:99`, must supplement and register per
> 3.1.1 acceptance "criterion turns green"); 3.2.1 empty arm changed to bookkeeping (proof_calls
> upthrows) + diagnostic (`into_result` established conversion path); 3.3.1 SOLVER slot Option-ized,
> Z3 missing conservatively degraded as `SMTResult::Unknown` (E2031 family diagnostic replaces
> process crash). Two bug criteria removed `#[ignore]` turning to normal green, supplemented corpus
> fixture `proof-obligation-honored`; check-fanout inventory migrated accordingly (wiring point
> 19=inventory). Measured: criteria 2.4.1/2.4.3 turned green, full suite lib 2494 / integration 272
> / corpus dual layer all green.
>
> **3.4 Audit (2026-10-07, P3 Post-Implementation Re-review)**: For the type of defect "failure
> produces no signal" (definition see 02 §41-52), re-inventoried on unmodified code, found 8 more,
> all registered as 3.4.x above. Method and evidence:
>
> - **Consumption surface inventory** (16 obligation fields grep all repo read points one by one):
>   `variant_ctor_calls` **zero consumers**——the only read `checker.rs:1468` is "self-pour into
>   `TypeCheckResult`", IR side uses form detection `ir_gen.rs:7374`; `rfc011b.rs:659/688` only
>   asserts production side, which is the "only test production side, naturally immune to
>   consumption side defects" form criticized in 02 §186 → 3.4.4. `method_overload_ir_names`
>   result-level field similarly zero external consumers, IR uses parallel `overload_resolutions` →
>   3.4.5.
> - **Phase coverage re-review**: 02 §91 table row 1 (four entry points "no proof_execution")
>   already fixed by 3.1.2, current state 5 places share one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 unchanged (multi-file `run` still doesn't
>   output W1003, doesn't run dead code family) → 3.4.6; LSP non-project-internal path (02 §77
>   "manual lex→parse") also doesn't run proof → 3.4.7.
> - **Same-shape empty arm scan** (`Unproven` all-repo consumers only 4 places): checker three
>   refinement branches (3.1 covered), 3.2.1 fixed ownership arm (**source unreachable**), and
>   `environment.rs:422` (**reachable, zero diagnostic, zero bookkeeping, and all-repo docs zero
>   registration**) → 3.4.1, the only "all unknown" item in this audit.
> - **Solver acquisition surface**: `default_solver()` three consumption forms vary
>   (`predicate.rs:32` process singleton / `checker.rs:1266` per `check_module` injection /
>   `ownership.rs:621` per back-edge determination new build)——3.3.1 only eliminated panic one item;
>   the other two's **silent surface** goes to 3.4.2, **singleleton and cache hit** goes to P4's
>   4.4.2 (don't double-register).
> - **Not counted into 3.4's existing attribution**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys not consumed, Drop silently lost) still P2 red state standing, attribution
>   4.5.1/D20 unchanged; 4.4.1 (layer order declaration opposite to actual, `equivalence` layer not
>   in pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) already registered in P4.
>
> **Execution Convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 prerequisites, can be landed in place
> anytime (suggest doing 3.4.1 first——the only "all-repo zero registration" living bug, smallest
> change surface); 3.4.4–3.4.8 follow P4 execution, their criteria depend on obligation ledger
> (4.3.1) or unified Driver (4.2.x). **3.4 does not create new criterion system**, all reuse
> existing C2/C3 and P9's 9.2 gate.
>
> **Batch 3.13–3.16 Implementation Log (2026-10-13 registered → 2026-10-15 closed)**: User
> 2026-10-14 ruling merged four related small items (#451 method binding panic / #452 named actual
> parameter into resolution / #453 dead inference entry / #454 native export named actual
> parameter——the last is a boundary finding when verifying 3.14, same day attribution experiment
> confirmed **not introduced by #452**, user ruling ① merged into this batch) into one team for
> processing. Closing state: all four completed, ruling registered as RFC-039 **D60** (six binding
> rules + shared criterion single-point).
>
> **One. Two Contract Corrections (Lesson: contracts themselves can be wrong)**
>
> 1. **Acceptance fixture self-contradiction**: #452 task contract simultaneously required "mixed
>    actual parameter output 20" and "position and name contending same position error", but the
>    nailed fixture `Temp.pick(self, a)` has only one value parameter——the same actual parameter
>    combination is both "mixed" and "conflicting". First implementer truthfully reported blocker
>    and marked task failed, overall revert (no green light in exchange for silence). Disposition:
>    task re-dispatch + contract revision——provide **six binding rules** and **8 value-level
>    matrix** (fixture changed to two value parameters `pick(self, n, a)`), then attempt 2 completed
>    by second implementer. Six rules full text verbatim incorporated into D60.
> 2. **inScope missing mechanically required surface**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed the generator-regenerated
>    `docs/src/reference/error-code/`**. After E1110 registered, only ran `code-tables --fix`, **did
>    not re-run** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth red ("source
>    registered/emitted but document missing 1: E1110", source 154 / document 153, run
>    **38518624964**——this batch's only CI red line). Fix = mechanical regeneration (8 files
>    +83/−70, only touching `docs/src/reference/`), reconciled 154/154, CI 38520209026 green.
>
> **Two. Review → Fix → Re-review Two Closed Loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    didn't narrow with resolution ⇒ calls containing named actual parameters in multi-candidate
>    zero-hit/single-candidate hit two paths entire chain silent) → fix round added external code
>    **E1111 (argument_order_invalid)** (code table + locales + RFC-013 + generator four-piece same
>    submission; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared parsing entry + native arm settling by name) →
>    independent verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations via
>    `scratch.py trial` mechanically reversibly replayed all captured, restoration byte-by-byte
>    consistent) → round-1 review needs_revision (F1 = typecheck native arm **inlined copy** of
>    binding criterion, missing rule ④ ⇒ method and native same-form call **ruling
>    divergence**——method reports E1111, native silently exit=0; ir_gen has another third existing
>    copy reporting old code E1015) → convergence round consolidated criterion to single
>    implementation `bind_args_to_params` (`arg_binding.rs:35`, four call sites
>    `expressions.rs:2727` / `:5416`, `ir_gen.rs:7776` / `:8279`) → round-2 review caught regression
>    introduced by convergence round (IR convergence block lost native omittable position ⇒
>    originally successful programs with omittable optional positions misreported E1010) → fix round
>    (optional position criterion and parameter name from same source `signature.rs:236` sole
>    export) → round-3 review PASS (independent reproduction red→green; mutation nailed fixtures
>    FAILED empirical criterion can fail). **Lesson**: review must cover "unmodified branches of
>    similar paths" (this batch's two needs_revision both due to probes all falling on hit surface);
>    fix round itself can also introduce regression (optional positions), caught by next round's
>    swapped reviewer.
>
> **Three. Process Events (Truthful Recording, Not Pointing to Individuals)**
>
> 1. **External whole-tree discard (2026-10-14 around 17:14)**: Working tree was silently restored
>    externally——implementer's fixes in progress (typecheck/ir_gen/code table/locales/RFC-013 etc.)
>    all disappeared, **simultaneously the two review documents the user themselves edited also
>    disappeared from git status** ⇒ determined as repository-level discard (most likely user-side
>    manual cleanup/IDE "discard all changes"). Three evidence exclude team members: stash empty, no
>    new commits, reflog no reset. Implementer replayed in-progress work from context and pushed;
>    discipline changed to "**save patch to scratch first, then capture red state**".
> 2. **Verifier ran out of context, swapped people**: Independent verifier verifying 3.16 ran out of
>    room midway (no closing message), swapped per load rule——successor continued using their ready
>    products (green state results, corpus equivalence script, mutation plan), verification
>    ultimately PASS. Positive precedent for partial product handoff.
> 3. **Review round 5 rotation and historical round return**: When rotating review per load cap
>    clause ⑤, `remove_member` returned both already-ruled terminal review/verification rounds **to
>    shared pool**, scheduler immediately re-ran (t3/t8 taken by new members again, duplicate work
>    blocking downstream). Disposition: captain took over and cancelled returned historical rounds
>    (t3/t8/t13), added AGENTS.md clause 12 accordingly (handle returned rounds immediately after
>    personnel swap). This batch's closing record registers this event per that clause.
> 4. **scratch governance (about 6.3 GB)**: Early batch had one external tree per task, 6
>    directories scattered outside repo (about 780 MB, cleaned) + two members' external verification
>    trees (about 2.2 GB / 3.3 GB, self-deleted after use). Produced governance tool
>    `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md clause 9
>    (batch tree sole entry), clause 10 (verification environment six rules: mechanically reversible
>    mutation / line number from referenced revision / CI-exclusive gates must run etc.), clause 11
>    (writer side load cap), clause 12 (handle returned rounds after personnel swap) and two top
>    meta-rules (no private content / verify before assert).
> 5. **Contract defect (not pointing to members)**: #454 fix round's contract inScope only circled 2
>    files, actual mechanically required surface 8 (clause 6 is exactly to prevent this——contract
>    template should pre-set mechanically required surface list); also that task's terminal field
>    was once occupied by probe payload, formal acceptance/command evidence supplemented by captain
>    per regulations.
>
> **Four. Residual Items (Not Fixed This Batch, Registered for Reference)**: round-3 review's three
> low-severity findings——① method path's `build_arg_slots` (`ir_gen.rs:521`) is still the
> criterion's **second** implementation (existing, per previous round's caliber registered with
> closure, not converged)——**2026-10-16 pre-release audit supplemental three points**: (a) it has
> **two** call sites: `ir_gen.rs:7888` (method binding path, using bound function
> `binding.function`'s parameter name) and `:7975` (overload resolution path, using resolved mixed
> name leaf segment), original registration only wrote "`ir_gen.rs:521` area"; (b) it **has already
> diverged from shared implementation `bind_args_to_params`**: missing rule ⑤ (duplicate name →
> E1015), rule ⑥ (missing required parameter → E1010) and "optional position" concept, and hardcodes
> receiver position as `1 + i`; two call sites treat empty slots uniformly as `continue` (comment
> says "diagnostic goes to typecheck (it guarantees resolution passes so no empty slots)"), a
> **dangling invariant written in comment with no assertion fallback**; (c) **RFC-039 D60's "all
> call sites converge to it" enumeration missed these two places**, declaration inconsistent with
> fact, and cited line numbers not re-fetched from referenced revision, D60 text corrected
> accordingly; ② batch verification tree (worktree) inner `cargo fmt --all` error is worktree
> workspace artifact, gate checklist should note fmt goes to main repo; ③ repair-type contract
> template should pre-set mechanically required surface into inScope (see 3.5).
>
> **Five. Final Gates (Closing Revision, Local Two Implementation Commits + One Document Commit
> Unified Push——This Batch's Only Push)**: fmt --check 0 / clippy -D warnings 0 / `cargo test --lib`
> **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** / `--test yx_runner`
> 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 / corpus_probe **342
> lines consistent line-by-line with baseline** (sole regenerated line: `named_arg_duplicate_err.yx`
> E1015@14 → E1111@16, expected code and line number synchronized, manually reviewed) / IR snapshot
> zero drift.
>
> **Six. Run Links (Push-Side Evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (same run's MSRV/Test cancelled by subsequent push
> per concurrency——cancel≠fail, covered by later run); governance tool push → CI 38546373435 /
> 38561405117; WBS registration (3.16 row + table header 56→57 / 147→148) → CI 38528560364. Closing
> push's own CI / Deploy Docs run registered after push to supplement task record.
>
> **3.4.8 ③④ Implementation (2026-10-11, 8ecf56f93)**: ③ `CHECK_STAGES` tail adds `Linking` (**pure
> merge**, entire block skips `is_bin_role` entry validation——E3020 criterion still belongs to
> `RoleClassification` to prevent double-report; also hard reason: run path judges "has main" by
> merged IR, collection semantics "entry itself errors → no IR" would be misjudged as missing main
> and hard-aborted) → `Monomorphization` (checks form); `ir_generation_check` keeps unit IR
> (**memory only**, for pure merge and mono consumption, not external/write/snapshot——if not kept
> then aggregation request is empty, mono with `NoObligations` false green); mono failure in Check
> goes per-file collection channel (diagnostic belongs to entry file + `failed_stages`, not
> failure), MultiFile keeps `fail_with`; `into_outcome` changed to "take collection first then judge
> failure". ④ **Empirically zero production change**: full program BFS already covers cross-unit
> mutual recursion (pathological → E3005, well-formed three-layer chain executes correct value),
> delivered four groups e2e + one mono unit test as living criterion.
>
> **Verification Chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized boundary),
> t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-form matrix (single/multi/check
> three forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review PASS; test count 2643
> → 2656 (only increased).
>
> **F1 (Found in Verification Phase, D59 Class A/B, Same Round Fixed)**: In standalone multi-file
> check without project root, aggregation request name (`lib.f`) and unit IR definition key (`f`)
> have different sources → mono drops "native no IR definition" silent `continue`, check reports 0
> error/exit 0. Fix: aggregation side adds reverse lookup by each unit's **true definition key**
> (precise enumeration + literal equality + ambiguity abandonment, not suffix heuristic; project
> form not registered, existing path unchanged). **Errata (Same Day)**: This reverse lookup was
> **deleted** in subsequent regression fix; root cause instead solved by `module_key` dual-use
> decoupling (added `script_mode`) from naming caliber——IR definition key = IR call name =
> aggregation request name = `{unit_key}.{name}`. **Regression Record**: After ③ landed, CI's
> `Check tutorial/guide code examples compile` red (258 blocks / 10 failures: 6 same-file same-name
> rebinding + 4 colliding with `<std/list>`, rootless check without qualification triggers
> `link_module_irs`'s conflict validation); fix = ir_gen split `script_mode` (qualified name ⊥
> Script/Bin, E3023 gate changed to look at it) + `ir_generation_check` per unit constantly pass
> unit key + `.with_script_mode(rootless)` + Script top-level same-name rebinding "latter wins"
> (consistent with run path's existing behavior); verification = doc gate 258/0, three-binary
> comparison (HEAD/③ tree/fix tree) three collision fixtures, check form E3005 no regression, E3023
> not triggered.
>
> **F2/F3 (Accepted)**: mono diagnostic belongs to entry file entry, span can cross unit (same
> caliber as run path `path=entry`); Check form is each unit holds one ModuleIR (memory only, freed
> when check ends).
>
> **Existing Issues for Follow-up (Not This Round's Regression)**: ① E3005's help text still writes
> "internal error; please report this issue", inconsistent with resource protection semantics
> (precedent: `e3xxx.rs:47` once moved similar "user writing error" out of E3005)——registered
> pending scheduling; ② multi-file E3005's span rendering label misalignment (reproduced with HEAD
> binary, pre-existing); ③ request collection surface has **type parameter position dependency**
> (generic calls with type parameters not in value parameter position don't issue instantiation
> request, mono records `NoObligations`, convergent mutual recursion is this form)——same surface as
> 3.5 (HKT call path), registered to that row.

## External Track RFC-029g: Remove `pub` and Auto-Binding (#399)

> **P3.5 Implementation (2026-10-08)**: Seven second-level tasks all completed, entire batch one
> commit (independently revertible).
>
> - **Measurement Inconsistent with Plan (Truthful Record)**: 029g.1.2/029g.1.3's acceptance
>   originally wrote "`pub f = ...` reports **parse error**"——actually after deleting `KwPub` `pub`
>   reverts to normal identifier, old writing falls to semantic layer reporting **E1001 Unknown
>   variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specially detecting removed
>   syntax is anti-pattern", **no new** migration-specific diagnostic added; corpus fixture
>   `pub_declaration_err.yx` nails E1001, document (`syntax.md`) truthfully states this consequence.
> - **Design Convergence (Beyond RFC Literal, Needs Review)**: `project_refs` and `cross_file_refs`
>   two fields after pub deletion have completely overlapping semantics (same "package-internal
>   reference union", prohibition one prohibits parallel representation) → merged into single
>   `project_refs`, `set_exempt_pub`/`set_cross_file_refs` two switches deleted. Behaviorally
>   Internal role's non-pub items changed from "file-internal reachability" to "reference pool
>   judgment" (loosened, false negative direction); Lib tightened from "pub absolute exemption" to
>   reference pool judgment; Script top-level bindings not referenced start reporting
>   W1001/W1002/W1004/W1005.
> - **Dead Table Cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports write-back all deleted (production-side
>   table-filling, all-repo zero consumption).
> - **Test Migration** (test count not decreasing): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub is recognized" assertion reversed to "pub is no
>   longer modifier"; `passes/tests/dead_code.rs` 7 role semantic cases rewritten (assertion
>   reversal 2); `tests/integration/cli_e2e.rs` 8 role cases rewritten (`lib`/`internal` exemption
>   assertions reversed to report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` not participating in IR); corpus diff baseline
>   only added 1 (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerated then manually reviewed.
> - **Measurement**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-Layer Contract PlanId-ization (D20)

> **4.5 Implementation Record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, closing
> (registration / WBS check / gates / push) completed by t7. Prerequisite = 4.3.1 obligation ledger
> (red state archive 173 files / 533 keys).
>
> **One. Carrier Ruling (A: `PlanId` walks with AST node)**: Added `PlanId` (`src/util/plan_id.rs`,
> `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` for synthesized nodes
> outside parser), issued by **sole allocator** `ParserState::alloc_plan_id`; carriers in two
> places——`Stmt.plan_id` (statement identity) and `Expr::Call.plan_id` (call site identity),
> `Expr::call` is repo's sole call constructor. Voted down is "both sides independently derive same
> identity by span" position-coincidence form (D20's original defect).
>
> **Two. Two-Level Verification and Review (Members Independent)**: Each functional commit paired
> with one independent verifier and one reviewer, **verifier/reviewer don't reuse implementer
> evidence**: 4.5.1 = t1 implement / t2 independent verify (external tree + 5 mutations) / t3
> review; 4.5.2 = t4 implement / t5 independent verify (external tree + 5 mutations) / t6 review.
> Review timepoint and reviewed revision both in immutable history (`git show` verifiable).
> **Provider rename caused t5 / t6 personnel swap and continuation**: original verifier/reviewer
> couldn't continue due to provider rename, swapped per orchestration convention and **wrote reason
> in record** (cross-round context preservation priority, only swap when unusable); continued
> dispatch result consistent with original round (PASS), didn't reuse implementer evidence.
>
> **Three. Two Defects Found and Fixed During Implementation Self-Check (t1, All Within inScope)**:
> ① consumption exemption criterion once hung on `UnitState.ir.is_some()`——Check form's Linking/Mono
> would take away `unit.ir`, real unconsumed in Check form **silently passed** (mutation
> measurement: D41 red but CLI check exit 0); changed to read **ledger's own completion marker**,
> supplemented regression case. ② Drop sequence drifted with process hash seed (partial order
> `sort_by` + `HashSet` iteration): snapshot criterion ran three times consecutively producing
> 106/120/114 line drift; changed to **total order sorting** (same name take max depth + name
> ascending) then two consecutive runs zero drift.
>
> **Four. Criteria and Evidence Numbers**: After 4.5.1 `test_release_plan_spans_consumed` removed
> `#[ignore]` standing (`examined ≥ 173` floor + no whitelist), 172 IR snapshots regenerated same
> submission (programmatic review: added 1083 lines Drop + 86 lines jump index translation, zero
> deletion, no reordering), corpus 342 diff entries empty; 4.5.2 added
> `test_overload_resolutions_consumed` (diff set empty + evidence surface floor `examined ≥ 1` /
> `consumed ≥ 2`; measured examined=1, consumption entries 2, no resolution skip 210), 9 resolution
> forms nailed, `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero drift**.
> Mutation-driven: consumption key offset by one ⇒ criterion red + real program
> `error [W1090] ... produced key not consumed` + corpus gate diff; restored then re-green.
>
> **Five. Semantic Correction Classification Basis (D59)**: `Type.method(位置实参, 名字 = 值)`
> before change **silently gave wrong answer** (measured compile clean, run output `1` not `20`) ⇒
> **Class C** (old behavior not successful), fix takes "loud": same program after change
> compile-time `error [W1090]`; "named actual parameter participates in resolution" terminal
> semantics is **Class D**, already registered §3.14 and left issue #452 for dedicated team.
> Additionally: `Drop` in interpreter is no-op (`backends/interpreter/executor/ops/control.rs`
> wildcard arm only `advance()`)——key migration doesn't change run behavior.
>
> **Six. F-1 / F-2 Disposition (Found by t5 Independent Verification)**: **F-1 (medium)** unbound
> consumption point (`ir_gen.rs:7766-7790`) unreachable in current corpus and test surface, key
> correctness has no red state to falsify ⇒ per "known boundary" registered in
> [02](02-stage-contract.md) §2 revision note fifth paragraph (keep reason + don't open new task);
> **F-2 (low)** pointer drift ⇒ `types.rs`'s `OverloadResolutions` doc comment changed to precise
> pointers (produce `inference/expressions.rs:4550`, consume `ir_gen.rs:7697` / `:7772`), single
> small commit, not mixed into document commit.
>
> **Seven. Closing (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settle marker); 07
> criteria table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG recorded one behavior change line; `check-obligations.py` and `check_tracking.py` green,
> CI and Deploy Docs green. **Closing-discovered accounting deviation**: t3 / t6 report's pointers
> `ir_gen.rs:357` / `:3102` / `:1096` differ from code fact by one to three lines (actually `:360` /
> `:3103` / `:1095`+`:1097`, due to 4.5.2 insertion)——registered per code fact, this note also
> preserves.

## P4 4.10.3 Standalone Run and MultiFile Orchestration Unification (Half of 4.2.5 Ruling A's Run)

> **Batch C Record (2026-10-15 registered → 2026-10-15 closed)**: User 2026-10-15 ruling C——this
> round only does 4.10.3 (same batch candidate 3.4.6 per B1 ([02](02-stage-contract.md) §Runtime
> Behavior Table "dead code family warnings don't enter compile path") deferred to after 4.9,
> prerequisite changed and annotated); implementation form = **complete-form single team** (user
> instruction no DAG, no team building, single-session implement + self-verify). Pre-batch debt
> first landed: #455 dependency announcement fix (rustls 0.23.46 + serde_yaml_ng, `cargo audit`
> recovered from exit 1 to exit 0), WBS registration debt (#456 registered as 3.17 + incomplete rows
> supplement "belonging phase / whether blocks mainline" fields).
>
> **One. Red First Then Green**: driver-level red criteria 3
> (`src/driver/tests/run_rootless.rs`——rootless merged IR must not carry entry_function (red state
> `Some("main.main")`), rootless top-level executable statement must not report E3023 (red state
> doesn't produce module), Bin control entry_function keeps (green state standing)) + CLI e2e 3
> nails (standalone relative import run (red state E5001), Script top-level statement execution and
> #413 hint two green state nails).
>
> **Two. Implementation Surface**: CLI `run` unified to `compile_project`
> (`run_file_with_diagnostics`'s project/standalone bifurcation deleted); Linking's `entry_function`
> gated by `is_bin_role` (#413 T4——Script doesn't auto-call main); #413 hint keying changed from
> `module.source_files.is_empty()` to call site project probe (after unification ModuleIR always
> carries source_files, original marker invalid); `script_main_never_called` adapted to qualified
> name form (function `{key}.main`, global slot measured also qualified name——with IR as evidence,
> not inferred from comment).
>
> **Three. Existing Gaps Exposed by Unification (Same Batch Incidental Fix, Class A/B/C——only fix
> failed programs to successful, original successful programs zero impact)**:
>
> 1. **Explicit type actual parameter pre-specialized name** (`mk(Int)(5)`): ir_gen per D6.3
>    directly emits specialized name (mono strips unspecialized original), `resolve_call_target`
>    three arms (rename / aliases / dot prefix) don't cover `mk(int64)` form ⇒ call site dangling
>    E6006. Fix = that single-point resolver add fourth arm (base name qualification, type parameter
>    string preserved as-is). **Bin project same-form reproduction (pre-existing defect, not
>    introduced by this batch)**.
> 2. **Script top-level `mut` assignment statement duplicate slot allocation**:
>    `extract_global_defs` and `register_global_slots` count `counter = counter + 1` (Assign same
>    form as declaration) statement by statement ⇒ same variable allocated 3 slots, initialization
>    writes later-allocated slot while read uses earlier-allocated slot (runtime Add type mismatch).
>    Fix = two places merge by name (same criterion same source).
> 3. **Same-module constraint generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidates + lookup table validation (precise hit →
>    generic's unit key qualification hit → fallback to bare name current; not suffix heuristic,
>    same form as `resolve_generic_name`).
>
> **Four. Registration Gap (Not Fixed This Batch)**: Constraint dispatch when **type and generic
> definition in different modules** still E6006 (rootless dual-file probe evidence; Bin same
> form)——root cause = MonoType user type name doesn't carry definition module, correct fix belongs
> to P6 type identity convergence; registered WBS **3.18** (belongs P3, prerequisite waiting P6,
> doesn't block mainline), `qualify_deferred_method` annotation same surface.
>
> **Five. Test Adaptation (Honest Record)**: `tests/integration/multifile.rs` fixture supplemented
> manifest——what it tests is RFC-029 project orchestration (Bin auto entry) not Script semantics;
> `cli_e2e` one nail (top-level diagnostic variable name visibility) and `script_main_hint` fixture
> (7 cases) synchronized per qualified name form——production consumption side only takes merged IR,
> fixture direct drives same form, no need for pipeline product substitute.
>
> **Six. Final Gates (Closing Revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new nails) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> diff baseline regeneration **12 lines**——line-by-line structured verification: exit / stdout /
> diagnostic code all item-by-item same, only stack frame function name and variable name qualified
> per unit key (manual review passed); after regeneration `check-corpus-parity.py` exit=0.
>
> **Seven. Process Events (Truthful Record)**: This batch once during pre-commit hook running period
> concurrently edited workspace document, was eaten by hook's stash rollback and that commit failed
> (concurrent form predicted in config comments)——discipline = don't touch workspace during hook
> running; #455's commit therefore split into two attempts (second fully green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closing)

> **4.6 Batch Record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 instructed to
> advance in order 4.6 → 4.9 → 4.4 → P5, first do 4.6; implementation form = **complete-form single
> team** (following previous batch user instruction: no DAG, no team building, single-session
> implement + self-verify). Task definition (09 three-level table): project-internal file's semantic
> data (SemanticDB) goes through orchestrator same pipeline, same source as diagnostic; cross-file
> references carry `resolves_to`. Acceptance: project-internal cross-file jump hits definition;
> semantic data and diagnostic from same compilation.
>
> **One. Current State Evidence (Verify Before Assert)**: didOpen/didChange triggers **two passes**
> of compilation——diagnostic goes through unified Driver pipeline (4.2.3/4.2.6 already unified),
> semantic data goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registry ⇒ project-internal
> `use` resolution fails, cross-file reference events not produced, jump/rename/reference lookup
> silently empties for imported symbols——D55 registered silent failure). Orchestration arm's checker
> unified naming `"<module>"` (SemanticDB file identity lost), `type_result` (containing
> semantic_db) discarded when diagnostic non-empty and never externally emitted; `checker.rs`
> discharge forcibly covers `resolves_to`'s DefId file identity to current file;
> `Export`/`ModuleInfo` both lack definition location surface.
>
> **Two. Red First Then Green**: LSP e2e nail (`src/lsp/tests/cross_file.rs`)——temp dual-file
> project didOpen main.yx then send GotoDefinition to `add_one` call site, **before fix returns None
> (red, empirically above-section silent failure)**, after fix hits lib.yx line 0 definition
> (green). One probe positioning in process: compiler-side chain (driver probe dump) already fully
> connected but e2e still empty——root cause is `resolve_reference` precisely matches by reference
> **start point** (D55 precise-only current caliber), test cursor falls on identifier middle; this
> tolerance is existing behavior surface, not touched this batch (see Seven).
>
> **Three. Implementation Surface**: (1) `Export.definition_span` (extract_module_info self-harvests
> from AST identifier node, same source same value as typecheck definition event's `def_span`;
> native std always None) + `ModuleInfo.definition_file` (Registry arm fills per DiscoveredFile,
> including vendor; std always None); (2) checker reference discharge via `import_def_locations`
> (use line binding span → definition file + definition span) redirects cross-file imported items'
> `resolves_to` out of current file; std import not registered, maintains falling to use line; (3)
> checker module identity changed from unified `"<module>"` to unit file path (verified
> `env.module_name` all 4 consumption points zero diagnostic consumption); (4)
> `DriverOutcome.semantic_db` channel——CollectAll form (Lsp target file **regardless of diagnostic
> success or failure** + SingleFile fallback) merged in, Lsp arm tail injects non-target file
> top-level export definition surface from registry (`external_definitions`, unopened files can also
> jump; type/submodule export not injected——typecheck side type registration doesn't produce
> definition event, both sides maintain consistent vacancy); (5)
> `orchestrator::analyze_source_in_project` (`check_source_in_project` transit takes diagnostic
> half); (6) LSP `analysis.rs` single compilation entry replaces manual pipeline
> (`update_semantic_db` deleted), World key unified normalized file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
> differences normalized to same key, cross-file DefId therefore stably hits); 5 handlers +
> semanticTokens dual requests connect `db_key`.
>
> **Four. Incidental Fix (D59 Diverted Class B——old behavior silently no action, original successful
> programs externally zero impact)**: ① project-internal file cross-file jump/rename/reference
> lookup from silently empty to hitting real definition; ② when lexical error, old manual pipeline
> deleted semantic information (coloring vanished) → residual AST coloring preserved (4.2.6 editor
> philosophy extension); ③ when target file has type error, semantic data no longer discarded with
> `type_result` (mid-typing coloring/jump not lost).
>
> **Five. Nails**: driver-level 6 (`src/driver/tests/lsp_semantic_db.rs`——target file path key +
> same-source diagnostic channel, cross-file resolves_to + external definition surface roundtrip,
> std import fallback use line, error file preserved, single file fallback path key, single file
> error preserved) + LSP e2e 1 (red first then green) + uri normalization 4
> (`src/lsp/tests/uri.rs`——drive letter conversion, space encoding, idempotent normalization,
> non-file scheme passthrough).
>
> **Six. Mutation Verification (scratch.py trial, Mechanically Reversible)**: Batch tree wbs-46 @
> closing revision 39546020, both mutations make hit nails turn red, after restore blob consistent
> with HEAD + git status CLEAN—— m1 remove `import_def_locations` registration (statements.rs) ⇒
> cross-file driver nail + LSP e2e nail red, other 7 green; m2 remove `external_definitions`
> injection (arms.rs) ⇒ same two nails red (roundtrip parsing loses anchor), other 7 green. Batch
> tree gc-ed (released 4.8 GB, `status` no batch tree).
>
> **Seven. Final Gates (Closing Revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new nails) / `--test integration` **287 passed** / corpus
> diff **byte-by-byte zero drift** / IR snapshot zero drift / check-obligations PASS (13·13·0·8,
> `semantic_db` non-obligation reason consumer anchor synchronized server.rs:482 with 02 §Obligation
> Field Table) / check-docs-truth 0 / docs-examples 258 entries 0 failures / docs-orphan 0 /
> markdownlint entire repo only remaining user-uncommitted 032 two MD056 (user's own edit, not
> touched) / pnpm docs:build 0. **Process Events (Truthful Record)**: en/09a-wbs-history.md:54 one
> MD037 introduced by auto-translate bot re-translating previous batch 09a (zh original no space),
> this batch fixed in passing——markdownlint red not this batch's change surface but in same file
> 09a, registered together. **Residual Observation (Registered Pending Discussion, Not Touched)**:
> `resolve_reference` only precisely matches by reference start point——when cursor falls on
> identifier middle, jump/hover/rename silently no action (D55 precise-only current state, entire
> file form same structure), if tolerance matching needed is independent behavior change, report to
> user for ruling.
>
> **Same Day Continued Batch (4.6.2, Implemented After User Ruling)**: After closing report
> delivered, user ruled on "residual observation ①" "can do, then change"——lookup key tolerance
> implemented per plan A: definition/hover/references/rename four handlers' lookup key from cursor
> position (via UTF-16 conversion) changed to use `find_identifier_at_position`'s located
> **identifier's own start point** (same coordinate system as semantic event, no conversion);
> `resolve_reference`'s start point precise match semantic unchanged (D55 precise-only unchanged).
> references/rename's definition point fallback judgment (start <= col < end) holds structurally for
> identifier start. Red first then green: definition middle cursor nail first red (None) then green;
> four handlers middle cursor each 1 nail + cross-file e2e start/middle dual position assertion,
> `--lib` 2748 (+4) all green, clippy -D warnings 0. Registered WBS 4.6.2 (three-level count
> 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction)— Round 1 Closing (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 Batch Record (2026-10-16 registered → 2026-10-16 closed)**: User 2026-10-16 ruling merged
> 4.9.1 / 4.9.2 / 4.9.3 three related three-level tasks **into one team one round** (reason: they
> all belong to "pool source and identity" surface, merged implementation avoids
> `collect_project_refs` deletion's intermediate state repeated rewrites; 4.9.4 / 4.9.5 belong to
> consumption refinement and test pool separation, depend on 4.6.1's `use` path resolution, forcibly
> same batch would expand gate surface). #450 attribution **pending ruling**——evidence indicates it
> belongs to "overload resolution then name not returned to reference count" class, still reproduced
> under round 1's landed form, registered per independent WBS item for disposition (captain
> recommends A: register as independent item when merging into round 2; **pending user ruling**).
> Frozen rev = `3863a9ff0` (parent commit `62f6e1a59` is pre-fix baseline); verification PASS
> attempt `aab92fe0` (i.e. t2); review pass attempt `96452696` (i.e. t3). **Closing registration
> commit = `38d30b846` (docs(docs): 4.9 reference pool round 1 closing registration)**——this commit
> actually written and pushed by closing member integration-refpool; its closing probe once
> incorrectly marked closing task as terminal failed and overwrote output, captain took over new
> attempt and changed back to completed per verified delivery facts, hereby correcting attribution
> and label. Evidence archived: `target/yx-scratch/yx49/EVIDENCE.md` (containing 8 mutation patches
> and warn_compare_v8.json).
>
> **One. Merge Ruling**
>
> 1. **Ruling itself**: Three related three-level tasks merged into round 1. **4.9.4 (consumption
>    refinement) / 4.9.5 (test pool separation) next round**——former depends on 4.6.1 delivered
>    `use` path resolution (already landed at `39546020`, can start); latter with RFC-029f table
>    incorporation same batch, registered at RFC-029f end section pending change.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 mutually prerequisite (4.9.2 changes pool type → 4.9.1
>    assembly surface needs rewrite → 4.9.3 only then has denominator judgment's role table), doing
>    in rounds inevitably writes one intermediate state then changes twice; (b) all three belong to
>    "pool source and identity" surface, 4.9.4 / 4.9.5 belong to another surface, batching is by
>    "cohesion" rather than "independently revertible" split.
> 3. **Merge Result**: Single commit `3863a9ff0` (one line `:recycle: refactor(driver)`), single
>    push, single batch gates. **Hash Note**: Before push `git pull --rebase` merged auto-translate
>    bot commit (`72f364169`); rebase landing point after opponent's commit, this batch's two
>    (implementation `3863a9ff0`, its pre-fix baseline兼 opponent document commit `62f6e1a59`) **not
>    rewritten, maintain original hash**——initial version mistakenly wrote this as
>    "`3863a9ff0`→`3863a9ff0`" identity mapping (looks like "happens to be same value after
>    rewrite", actually "not rewritten"), corrected at 2026-10-17 closing; opponent code commit
>    `33f14e8ab`→`b122b7ad1`. **External references all use post-push hash as standard** (rewritten
>    old hash unreachable in new clones, identity determined by patch blob comparison).
>
> **Two. Five Criteria Red/Green Measurement Caliber**
>
> | #   | Criterion                                              | Red State (Pre-fix / P1 directory scan regression patch)                                                                                         | Green State (Round 1 Landed)                                                                                                                                                                                                                                                                                                                                                                      |
> | --- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | ①   | Pool root = package target (#334)                      | Directory recursive scan ⇒ files under tests/ also enter pool, exempting production code unused definitions                                      | `collect_package_refs` (`helpers.rs:481`) calls `discover_with_used` per `ExplicitSurfaces` root; full root degenerate fallback `collect_compilation_refs` (`helpers.rs:454`)                                                                                                                                                                                                                     |
> | ②   | Pool granularity = `(module, name)`                    | Same-named local variable (`let foo = …`) accidentally exempts same-named top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` mixed | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) assembles sole entry; `arms.rs:603-609`'s `other_module_refs` unions "except this unit" all module entries then sends to `DeadCodeAnalyzer`; module's own names (top-level definitions and local bindings, `use` imported names exception) no longer exempt other modules' same-name definitions |
> | ③   | `FileRole::Script` doesn't use pool                    | Script affected by project-internal other files' references                                                                                      | `arms.rs:611-623`'s `match us.role` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`), Script/Test goes `_ => {}` default arm (judges per file-internal definition-use graph)                                                                                                                                                                              |
> | ④   | Fixture A: lib.yx line 4 definition must report W1001  | Pre-fix: reports `W1003@1` (actually "unused import"), not W1001                                                                                 | Red state measured (red 1 fixture red1) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                                        |
> | ⑤   | Fixture B: main.yx line 4 definition must report W1001 | Pre-fix: reports `W1003@1`                                                                                                                       | Same as above                                                                                                                                                                                                                                                                                                                                                                                     |
>
> **Criterion ② "Deductive Red"** (implementer self-stated + verifier closed): red2 fixture's
> `let foo = …` in helper.yx; old pool under bare name union treats `foo` as "used by helper",
> lib.yx's `foo` exempt. **Measured**: red2 fixture vs pre-fix binary → no W1001 (old caliber
> exempt); red2 vs post-fix binary → W1001@1.
>
> **Criterion ③** (Script doesn't use pool) red state fixture **first version failed**: first
> version fixture in same file places same-named local variable triggering E2010 (language doesn't
> allow same-module shadowing top-level definition), moved to another module (helper.yx) for
> deductive red state. This point t3 review and t2 verification both independently reproduced, **not
> contract defect**——same-module shadowing is language's hard constraint, fixture must cross-module.
>
> **Three. +8 and +16 Two Sets of Count Correspondence** (**t3 review proof as standard**)
>
> `target/yx-scratch/yx49/EVIDENCE.md` section C writes `+16/−0`, but missed "**new fixture
> record**" component——v8 instrument (`warn_compare_v8.json`, 331 files symmetric diff,
> `only_before=0`) caliber **includes** new fixtures; t1 implementer's self-built "+8" is
> **warn-before / warn-after (each 325 entries) aggregated per fixture** (each member file check
> once). Review item-by-item mapping:
>
> - **+8** (8 new warnings, all `check <non-entry file>`, `check <entry file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (Unified Attribution)**: These 8 fixtures all **didn't declare package target** ⇒
>     4.9.1 root degenerates to checked file's own compilation set, consumer main.yx not in it; this
>     is design-intended change (before, these files' check surface was already zero warning, not
>     exemption path). Repo root `yaoxiang.toml` is empty file ⇒ same degenerate path empirically
>     holds.
> - **+16 Residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files in t1's `warn-before.jsonl` have `exit=None` (capture timepoint
>     **didn't exist**——they're this batch's new fixtures). So +8's "+ 0 − 0" diff doesn't have
>     them, +16's v8 symmetric diff has them.
>   - `v8`'s `stray.yx` only has `W1001@2(probe)` one item: reason is pre-fix binary also reports
>     `probe@stray.yx:4` (this fixture runs `probe` once per file), **cancels** leaving only lib's 1
>     entry——three data sets (+8 / +16 / dual binary ABBA) consistent.
> - **New Fixtures 8** vs **+16 Residual 8**: same batch files (4.9.1 / 4.9.2 newly added
>   `ref-pool-compile-set/` and `ref-pool-module-identity/`), instrument treats their each check as
>   independent record——same file may trigger different diagnostic entries for multiple
>   `role/context`.
>
> **Conclusion**: **+16 = +8 (per fixture aggregation) ∪ new fixtures 8**——both counts correct,
> **different calibers**: t1 reported "8 new" is fixture layer caliber, t2 reported "+16" is file
> layer caliber (symmetric diff includes new fixtures). t3 review ① +8⊆+16 already proven; **review
> ② wrote "don't copy EVIDENCE.md section C"**——section C missed "new fixture record" component,
> this item rewritten per t3 review `pass` conclusion.
>
> **Four. Mutation Trials (Mechanically Reversible, All `scratch.py trial`)**
>
> 8 mutation patches (`target/yx-scratch/yx49/*.patch`, p1 directory scan, p2b use import name
> removal, p3b script connects pool, p3 flat consumption, p4 std connects pool, p7final self-built 4
> criteria, p8 std close, p9 vendor connects pool + p_stack2 / p_stack3 stack surface). Each trial
> output contains: HEAD blob → mutated blob → restored blob consistent with HEAD = yes +
> `git status` CLEAN. This machine `core.autocrlf=true` needs `GIT_CONFIG_*` injection `false`
> (`trial_env.ps1`), otherwise patch new lines convert to CRLF causing apply/restore double
> failure——**only affects patch mechanics, doesn't change tested behavior**.
>
> **Five. Process Events / Environment Lessons (Must Read for Next Round Handoff)**
>
> 1. **core.autocrlf Injection**: This machine `core.autocrlf=true` must be injected `false` via
>    `GIT_CONFIG_*` to guarantee patch mechanically reversible; see
>    `target/yx-scratch/yx49/trial_env.ps1`. **Next round don't reuse** any trial script without
>    injection.
> 2. **Same-Tree trial Serial**: On same tree, `trial` and `cargo build` **must be strictly
>    serial**. `trial`'s internal `git apply` and `cargo build` both touch worktree, parallel will
>    have "patch incomplete when build sees" or "build and apply deadlock" two races.
> 3. **trial Rebuild Binary Needs Sync Confirmation**: After `trial` completes it rebuilds
>    tree-internal binary (cargo's fingerprint includes source path, **switching tree = full
>    rebuild**), before subsequent measurement must confirm `bin/yaoxiang-rs` synced with HEAD.
>    **This round once used residual mutation binary to get fake data**, already rebuilt and
>    re-measured to correct.
> 4. **Second Tree yx49-b Tree-Building Reason**: Comparison needs pre-fix binary (62f6e1a59), no
>    ready-made in batch; `scratch.py init yx49-b` opened second tree (eats another full build).
>    **Tree already released** (see Six), deletion left batch closing `gc` disposition.
> 5. **cli_e2e Two Internal Cases Fixture Supplemented to Legal Package (Supplement `[run].main`,
>    Assertion Untouched) Reason**: Pre-fix `cli_e2e` two `Internal` role nail fixtures **didn't
>    declare `[run].main`** ⇒ after 4.9.1 landed `compile_project` goes `ExplicitSurfaces` root call
>    path, can't find `main` ⇒ entire batch 2 fixtures behavior shifted from "Script fallback" to
>    "can't find entry"——this is 4.9.1 introduced fallback behavior not defect. After supplementing
>    `[run].main` field two fixtures go normal package path, **assertions** (exit code / diagnostic
>    code / warning set) verbatim untouched.
> 6. **Second Round Tree Deletion Safe Way**: After release use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`), will verify "no mount" then delete.
>    Already released before this batch closing, next `gc` naturally cleans.
>
> **Six. Batch Tree State**
>
> Before closing: yx49 already released (4.8 GB released), yx49-b already released. Two trees no
> mount, state can be re-verified in `scratch.py status`. This task output attached `scratch.py gc`
> complete status.
>
> **Seven. RFC-029f "Known Defects" Table Reversal**
>
> Previous version (2026-10-09) described "reference pool's three design defects" in present tense
> (including `collect_project_refs` / `orchestrator.rs:603-648` / `orchestrator.rs:393-396` already
> deleted code locations). This batch closing synchronously rewritten as "**Known Defects: Landed
> (2026-10-16, WBS 4.9 Round 1)**": ① current pool's root = `collect_package_refs`
> (`helpers.rs:481`), parses per `ExplicitSurfaces`, doesn't scan directory; ② current pool type
> `HashMap<String, HashSet<String>>` (key = module key), assembly sole entry `collect_ref_pool`
> (`helpers.rs:429-449`), consumption end `arms.rs:603-609`; ③ current Script doesn't connect pool,
> `arms.rs:611-623`'s `match us.role` default arm. **All file:line re-fetched on referenced revision
> `3863a9ff0`**. `collect_project_refs` and `orchestrator.rs` already moved to
> `src/driver/{state,arms,helpers}.rs` with P4 4.2.9 split (`c59fb751`), table's old locations no
> longer hit——this is "declaration inconsistent with fact" fix (t3 review note ①).
>
> **Eight. Final Gates (Closing Revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 lines** (two new fixture
> behavior run, 338a339,340; other 342 lines byte-by-byte unchanged) / IR snapshot 204 zero drift /
> check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process Event**: This batch
> once cargo build due to `core.autocrlf` converted script line endings to CRLF, team scratch path
> `yaoxiang-rs` startup reported `BadImage`, empirically encoding inconsistent with git index
> ——passed after rebuild (parallel with 4.6 batch "external whole-tree discard" lesson, different
> reason).
>
> **Nine. Residual (Handoff Next Round)**
>
> 1. **#450 Attribution Ruling**: Evidence indicates "overload resolution then name not returned to
>    reference count" class, **still reproduced under round 1 landed form**——dispatch per user
>    ruling C flow. **#450 Probe**: dual binary same set `{W1002@2, W1005@4}` (measured, rootless
>    form), no direct relation with round 1 refactor, but #450's fix may need to wait for 4.9.4
>    (consumption refinement) together——because root cause is "overload resolution then name to
>    reference count" needs use item's precise consumption surface, 4.9.4's "truly used after
>    imported" is same kind criterion's precise version.
> 2. **dead_code.rs:683 Pointer Nit**: Comment writes "two pools' assembly caliber see driver side
>    collect_compilation_refs"——actually `collect_compilation_refs` is fallback path, main path is
>    `collect_package_refs` (`helpers.rs:481`). Comment inconsistent with code main path caliber
>    (**not this batch's write scope**), leave for 4.9.4 / 4.9.5 landing to fix together.
> 3. **4.9.4 / 4.9.5 Prerequisites Unlocked**: 4.6.1 (`use` path resolution) delivered at
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs RFC-029f table
>    incorporation first (already laid by this batch "Seven." reversal).
> 4. **`yaoxiang check` Multi-File `run` Never Reports Dead Code Family Warnings** (WBS 3.4.6,
>    2026-10-15 user ruling C deferred to after 4.9)——this batch round 1 not involved; next round
>    4.9.4 / 4.9.5 landing same batch handling.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction)— Round 2 Closing (4.9.4 / 4.9.5 / 4.9.6)

> **4.9 Round 2 Batch Record (2026-10-16 implemented → 2026-10-17 closed)**: Covers 4.9.4
> (consumption criterion = truly used after imported), 4.9.5 (test pool separation), 4.9.6 (method
> reachability caliber, issue #450, user 2026-10-16 ruling A merged into round 2). Closing fix
> (verification side remedy M1/M2/M4 compilable redo + F1 fix round + PA-2 dead branch delete) and
> registration gap issue #464 (WBS 4.9.7) settled together.
>
> **One. Judgment Chain and Commits (Post-push Truth)**
>
> | Step | Content                                                                                                                                                | Post-push Commit | Pre-push Local | Judgment Method                                         |
> | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------- | -------------- | ------------------------------------------------------- |
> | 1    | fix(util): check explicit path not blocked by default exclude table + --exclude always empty (issue #463, P0-0.4)                                      | 366bc5a69        | ef959ca55      | Patch blob same                                         |
> | 2    | Phase A = 4.9.4 + 4.9.5 (RefPool attributed by import + Test doesn't contribute consumption surface)                                                   | e6cf1aeaa        | 84a6f6978      | Patch blob same                                         |
> | 3    | Phase B = 4.9.6 (#450 per-candidate method reachability; including generated warning-codes.md)                                                         | ddcbb12d0        | cd629b5fe      | Patch blob same                                         |
> | 4    | docs(meta): AGENTS clause 10⑦ (stash window convention during commit)                                                                                  | ec94fa9d8        | 9a4ed5f3b      | Patch blob same                                         |
> | 5    | Fix round = F1 (overload sibling overload restored reporting W1005) + F2 (verification side M1/M2/M4 compilable redo) + F4 (snapshot generation noted) | b8380e6ab        | 3d9ef6eab      | Patch blob same (36962f318e42…, git range-diff no diff) |
> | 6    | Closing fix = PA-2 (delete strip_unit_prefix dead branch + 029f/CHANGELOG three misstatement corrections, register #464)                               | b940300ac        | e13984465      | Patch blob same (patch-id 9bda3cfe4c…)                  |
>
> **Hash Note**: Each pre-push `git pull --rebase` will rewrite local hash; table's "Judgment
> Method" column all "patch blob same" (`git diff <rev>~1 <rev> | git hash-object --stdin` both
> sides consistent + `git range-diff` no diff). This article doesn't reference any pre-push hash as
> authoritative source.
>
> **Two. Three Criteria Red/Green Measurement Caliber** (Verifier Self-Built Reproduction, Not
> Trusting Implementer Criteria)
>
> | Criterion                  | Red State (Before binary 366bc5a69 measured)                                                                                                                                   | Green State (ec94fa9d8 and after)                                                                                |
> | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
> | 4.9.4 import≠consumption   | Declaration surface project check a.yx only 1 W1001 (main's use b.{foo} + foo() saves a.yx same-name foo; guard only imported not used also saved)                             | Same fixture 2 reported truthfully (collision name not saved + unused import not consumption); ref_pool 8 passed |
> | 4.9.5 Test Pool Separation | src/util.yx's debug_print saved by tests/ reference (only 1)                                                                                                                   | Same fixture 2 (test references don't leak); ref_pool's test_ref_pool_excludes_test_role_references passed       |
> | 4.9.6 Method Reachability  | Self-built fixtures (2 disambiguation overload call + 1 uncalled) 3 W1005 false positive; method_call_args.yx 3 / method_overload.yx 1 (overload by full name coverage only 1) | Same fixture exactly reports uncalled orphan; method_unused 3 nails passed; corpus call_args 3→0 / overload 1→0  |
>
> **Fix Round (F1) Further Tightening**: Sibling overloads not hit by resolution restored reporting
> W1005 (b8380e6ab——same fixture call ->Float reports ->String sibling, call ->String reports
> ->Float sibling, bidirectional line number nail). **Boundary Fallback (Register #464)**:
> Cross-unit (registry resolution) method calls don't produce resolution entries, falls back to
> name-level exemption, cross-unit same name same order previously not reported——register 4.9.7.
>
> **Three. Process Events and Lessons (Four Hard Requirements)**
>
> 1. **PA-1 Changed from Blocker to Existing Capability Gap (medium)**: Round 2 review once
>    registered "cross-unit same name same order not reported" as blocker (PA-1); then two fact
>    reconciliations (implementer archive + verifier double binary re-measure same fixture set on
>    b8380e6ab/b940300ac, diagnostic set byte-by-byte consistent) confirmed it's **capability gap
>    existing before round 2** not this round's regression——mechanism = cross-unit method calls
>    don't produce resolution entries, fall back to name-level exemption; disposition = change to
>    medium, register issue #464, WBS 4.9.7 schedule next round.
> 2. **Reviewer Publicly Corrected Before Data and Withdrew Regression Ruling**: Reviewer originally
>    cited before data ("−42 entries include called methods") inconsistent with their original
>    output (W1005 count=0)——verifier confirmed after check before capture is 0, reviewer's
>    regression ruling **publicly withdrawn**; −42's correct caliber = 26 files' called methods
>    false positive elimination (Phase B), each item traceable to behavior-run exit=0 file (method
>    truly called executed).
> 3. **t13 Terminated Due to Contract Prerequisite Not Holding (Not Implementation Defect)**: This
>    task's acceptance prerequisite written "cross-unit same name same order will report W1005
>    regression fix", but measured (t14 dual binary) confirmed this behavior is registered gap
>    (#464) not regression——contract prerequisite doesn't exist, task terminated; its probe value
>    carried by t16 read-only judgment (confirmed `strip_unit_prefix` is dead branch).
> 4. **Document Commit's Real Criterion Is "Really Build Once"**: Round 2 closing's document commit
>    (`042c07860`) let Deploy Docs red on VitePress's Vue template compilation——in record angle
>    bracket placeholders like `<rev>` / `<before-exe>` / `<after-exe>` didn't enter backticks,
>    parsed as elements (reported `Element is missing end tag`, pointing to `<rev>` line);
>    markdownlint doesn't catch this type, and pre-commit's markdownlint hook in docs-only commit
>    judged `no files to check` skipped (measured). Disposition: placeholders all in backticks,
>    before push locally run `npm run docs:build` once (fix commit `0c837e1b4` evidenced by local
>    build exit=0); if using `--no-verify` commit, commit-msg verification needs manual supplement.
> 5. **Process Omission of Contract Description Not Synchronized**: t14's task description text
>    residual PA-1 old hypothesis (writes "b.yx reports W1005" expectation), verifier actively
>    pointed out in report "per contract field judgment, description text doesn't count" and asked
>    captain confirmation——disposition = use task contract field as judgment basis, description text
>    left as process lesson (**task description must sync with latest judgment chain when
>    re-dispatching/dispatching**).
>
> **Four. Mutation Trials (Mechanically Reversible, All scratch.py trial)**
>
> Round 2 verifier total 9 patches (target/yx-scratch/yx49/ m1b/m2b/m4b as F2 post-compilable redo
> versions——initial M1/M2/M4 commented out if condition lines leaving dangling blocks, t8 review F2
> judged "compile failure mis-recorded as criterion red" invalidated; redo version per patch cargo
> check exit=0 + trial red). M3 (collision name attribution regression) initial already true
> mutation, measured ref_pool 5 criteria red, keep original. M1b anchor re-fetched on b8380e6ab
> (after F1 fix probe segment refactored, old anchor invalid). All trials: HEAD blob → mutated blob
> → restored blob consistent with HEAD + git status CLEAN. Environment injection continues
> trial_env.ps1 (core.autocrlf=false via GIT_CONFIG_*).
>
> **Five. Warning Surface Full Comparison (Caliber First Clarified)**
>
> Instrument = warn_surface_r2.py (self-built parser, (file, code, line, col) 4-tuple symmetric
> diff, project mode channel cwd=file parent dir, full path key); command =
> `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>` (337 files × 2
> binaries). **Caliber**: This round before capture already contains all fixtures ⇒ matched set =
> full 337, doesn't have round 1's +8 (matched increment) / +16 (full set) dual caliber issue.
> Result: only_before = 42 W1005 (26 files, all called methods false positive elimination = Phase
> B's −42; each file verified all on behavior-run exit=0 baseline, method truly called
> executed——**no one real dead code saved**) / only_after = 3 W1001 (ref-pool-import-use's a.yx:4
> check surface ×2 + ref-pool-test-pool's ../src/util.yx:2 = Phase A criterion surface). **Snapshot
> Generation (F4)**: Phase A timepoint's after capture (Phase B before 57 residuals) and final
> version (Phase B after 15+2=17) are **two generations**, must note when citing; also 1 special
> case (interface_member_conflict.yx's Dog.name@18) is field access form, not in Phase B's W1005
> removal set, consistent before and after.
>
> **Six. Final Gates (Closing Revision b940300ac)**
>
> lib 2761 passed / integration 288 passed / IR snapshot zero drift (snapshot 7 passed) / corpus
> baseline zero drift (byte count three-way comparable: baseline = b8380e6ab = b940300ac) / clippy
> -D warnings 0 / docs-truth exit=0 / check_tracking exit=0 / generator run no product change.
> **Gate Verification Arrangement (Clause 13)**: This batch push's CI / Deploy Docs / Auto Translate
> run conclusions handed to scheduled verification for registration (run id + conclusion +
> push-coverage relationship: later run covers previous), don't wait in session.
>
> **Seven. Batch Tree State**
>
> yx49 and yx49-b both already released (closing-time gc unified delete); no this batch new
> directories outside repo.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Correction)— Round 3 Closing (4.9.7)

> **4.9 Round 3 Batch Record (2026-10-17 implemented → 2026-10-17 closed)**: Covers 4.9.7
> (cross-unit method call's resolution registration and receiver attribution, issue `#464`——WBS
> 4.9's last three-level task; after this round closes 4.9 entire second-level task kanban checked).
> Chain = production side t19 → independent verification t20 PASS → review t21 needs_revision (F1:
> injection surface false positive) → fix round t24 → independent verification t25 PASS → review t26
> PASS (increment, verdict=pass). All commit numbers are post-push truth (judgment method = patch
> blob same; `git diff <rev>~1 <rev> | git hash-object --stdin` both sides consistent).
>
> **One. Judgment Chain and Commits (Post-push Truth)**
>
> | Step | Content                                                                                                                                                                                   | Post-push Commit | Judgment Method         |
> | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ----------------------- |
> | 1    | Production side: cross-unit method calls produce D20 resolution entries (`feat(typecheck)`, t19)                                                                                          | `b8b3cc3f6`      | Patch blob same         |
> | 2    | Driver side attribution: method reachability marked by definition unit, names with resolution records stop using pool's name-level exemption (t20 verify PASS / t21 review catches F1)    | `d5c00235a`      | Patch blob same         |
> | 3    | Injection surface fix round F1: candidate injection by "name ← providing module" **per-pair** filter, eliminating E1107 false positive (t24, including F2 expression and F3 form landing) | `d5dd1a37b`      | Patch blob same         |
> | 4    | Nail test relocation: `src/frontend/core/typecheck/tests/environment.rs`'s F3 nail moved into declaration surface `src/driver/tests/overload_injection.rs`                                | `c2f887843`      | Patch blob same         |
> | 5    | dev tip                                                                                                                                                                                   | `e9675c9dd`      | merge commit, see "Two" |
>
> **Two. Hash Mapping and Two Merge Commits at dev Tip (Source Unconfirmed, Don't Attribute to
> Member)**: Round 3's four commits (table 1–4) all pushed to origin/dev; `git worktree list`'s main
> mount at `e9675c9dd`. dev tip has two **non-this-team-command-surface-generated** merge commits
> (`5da05b885` / `e9675c9dd`, reflog shows `pull --tags` + push form), source unconfirmed——per
> clause 10④ (references must be parseable by anyone) and clause 13 (gates hand to dedicated
> verification) same closing caliber, only register fact and reflog evidence here, no member
> attribution; also see "Five." lesson ② (push attribution convention must be verifiable).
>
> **Three. D20 Contract Extension's Actual Form (Key Form and Consumption Points)**: Production side
> packs "candidate / mixed IR name / declaration-period parameter name" three parallel tables **in
> same order** into `ModuleInfo.method_overloads` (`src/frontend/core/typecheck/module/mod.rs:157`)
> crossing unit with, registry deterministically merges per module key ascending
> (`src/frontend/core/typecheck/module/registry.rs:150`); typecheck arm injects per this unit import
> face pair-by-pair (`src/driver/arms.rs:354` → `visible_method_overloads`,
> `src/driver/arms.rs:1521-1579`: name ← providing module per-pair, same name conflict full name
> skip, own key position not externally merged), form landing single point `append_overload_group`
> (`src/frontend/core/typecheck/environment.rs:399`: three tables same-order append + equal length
> debug assertion); consumption end attributes D20 resolution product by **definition unit**
> (`MethodMarks`, `src/driver/helpers.rs:569-573`; `typed_owners` `:591`; `resolved_method_marks`
> `:621`), and stops name-level exemption for name groups with resolution records
> (`src/frontend/core/typecheck/passes/dead_code.rs:252-267`, `:825-841`). Key form = entry name
> `{Type}.{method}#k` (`{Type}` is definition-side bare name; `#k` declaration order).
>
> **Four. Key Scope and False Positive Defense Measurement (Fix Round F1 and Five Residual
> Boundaries)**: F1's red state is pre-fix `d5c00235a`'s injection surface "name set × module set"
> combination merging another module's same-name type candidates into same key position ⇒
> `use a.{Temp}` + `use b.{Other}` (a also defines Other, b also defines Temp, method same name same
> order) fixture `check` reports 2×E1107, exits 1 (pre-fix successful program compile fails);
> post-fix `check`/`run` both exit 0, 4 W1005 truthfully report two sides' **uncalled** same-name
> definitions (reverse false negative also verified). Entry fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`; driver-level nail
> `src/driver/tests/overload_injection.rs` 3 + `src/driver/tests/method_unused.rs` round 2's 5 #464
> criteria and 4 #450 old criteria all green; main acceptance fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` diagnostic set pre-fix and post-fix
> consistent item-by-item (not regressed). Five residual boundaries and per-item measurement (alone
> `check` definition file not report / name-level ambiguity E5006 full name skip / boundary (c) per
> t20 measured rewrite / inline alias and `ns.T` not inject candidate surface / full import `use M`
> and std candidate surface not produce entries) full text see
> [029f](../../rfc/accepted/029f-target-semantics.md) §Cross-Unit Method Reachability Attribution
> "Residual Boundaries", not repeated here.
>
> **Five. Process Events and Lessons (Four Continuations)**:
>
> 1. **Contract Wording Too Narrow Directly Becomes Defect**: F1's one root cause is contract only
>    wrote "only inject module surface that can be named by bare type name", didn't write "attribute
>    by name pair-by-pair"——implementation literally took "name set × module set" combination,
>    review caught E1107 false positive. When contract describes behavior boundary must write
>    **combinatorics** dead (per name pair-by-pair / full combination), can't leave to
>    implementation to understand.
> 2. **Convention Must Be Verifiable: "Push by Captain" Only Written in Dispatch Message ⇒ Can't
>    Attribute When Third Party Push/Merge Appears**. This round dev tip had two unconfirmed source
>    merge commits (see "Two"), ultimately used reflog to backtrack and withdraw one wrong
>    attribution. Verifiable form = members don't run `git push`/`fetch`/`pull`, sync all
>    `pull --rebase`, use reflog to verify.
> 3. **Infrastructure Unstable, Drop Patch Then Swap**: This batch had 5 unrecoverable failures
>    (3×TRANSPORT, 1×INVALID_REQUEST (thinking protocol), 1×background subagent no message exit);
>    disposition convention = produced evidence and patch first drop into repo-internal path
>    (`target/yx-scratch/<batch>/`) then swap to continue, same session two consecutive failures
>    don't bet third.
> 4. **Document Commit's Real Criterion Is "Locally Really Build Once"**: Round 2 closing's document
>    commit (`042c07860`) let Deploy Docs red on VitePress's Vue template compilation (angle bracket
>    placeholders `<rev>` etc. not in backticks parsed as elements, reported
>    `Element is missing end tag`); markdownlint doesn't catch this type, and pre-commit's
>    markdownlint hook in docs-only commit judged `no files to check` skipped (measured twice). Fix
>    commit `0c837e1b4` evidenced by local build exit=0. This round lesson's red→green run: Deploy
>    Docs **39011946793** (red) → **39014696984** (green, see "Six").
>
> **Six. Round 2 Gate Conclusion Supplemental Registration (Clause 13, Scheduled Verification
> Backfill)**: Round 2 closing commit `b940300ac` push——CI **39011946778** success / Auto Translate
> **39011946796** success / Deploy Docs **39011946793** **failure** (VitePress build red, i.e.
> "Five.4" angle bracket placeholders; already fixed by `0c837e1b4`); fix commit `0c837e1b4` push
> three green: CI **39014697000** / Auto Translate **39014697044** / Deploy Docs **39014696984**.
> Continuous push coverage: CI@`0c837e1b4`'s **39014356642** cancelled by later run per concurrency
> (cancel ≠ fail, later run covers previous, consistent with round 2 §Six caliber).
>
> **Seven. Final Gates (Closing Revision `e9675c9dd`, Local Numbers; CI Conclusion Hand to Scheduled
> Verification, See "Eight")**: lib 2770 passed / integration 289 passed / yx_multifile_runner 1
> passed / IR snapshot 7 passed zero drift / clippy `-D warnings` 0 / docs-truth 0 / generator
> idempotent (re-run `scripts/docs/gen-error-code-docs.py` then `git status` clean); corpus baseline
> 346 → 347 → 348 lines (each +1 behavior record, other lines byte-by-byte unchanged; baseline
> `tests/baselines/corpus-parity.jsonl` at `e9675c9dd` total 348 lines).
>
> **Eight. Gate Verification Arrangement and Conclusion (Clause 13)**: This batch push's CI / Auto
> Translate / Deploy Docs conclusion handed to scheduled verification for registration (not waited
> in session, also don't write "didn't wait" as "already green"). **Backfill (2026-10-17 Scheduled
> Verification)**: `d5c00235a`'s CI `39023417670` = **failure**——Docs Truth reports "error code
> document lags behind code table/locales", i.e. this commit missed generated product same
> submission; fixed by subsequent `d5dd1a37b` (regenerate generated product): `5da05b885` CI
> `39027788246` / Deploy Docs `39027788268` / Auto Translate `39027788340` all success, covering
> previous; `e9675c9dd` CI `39028827871` / Deploy Docs `39028827896` success; `d0cfc4e62` CI
> `39033963240` / Deploy Docs `39033963278` / Auto Translate `39033963261` success. Three's runs
> each ran to terminal, this round no concurrency cancellation. Lesson: **"Local generator
> idempotent" doesn't equal "generated product already same submission into repo"**——should verify
> CI conclusion per commit before closing, not just look at latest one.
>
> **Nine. Batch Tree State (Closing gc, status Output)**: Before closing `scratch.py status`——t25
> (holder=-) and yx49b (holder=-, previously held by deprecated member verify-r3, captain
> `release --force` released) two trees both no holder; `gc` deletes two, releases about 12.7 GB;
> after gc `status` = "no batch tree", `git worktree list` only main mount. Additionally:
> `target/yx-scratch/yx49r3/` and `yx49r4/` aren't batch trees (no meta.json), are member
> evidence/probe directories, not "residual batch tree". No this batch new directories outside repo.

## P4 4.4 Proof Layer and wasm Cleanup — Closing (4.4.1 / 4.4.2 / 4.4.3)

> **One. Batch Composition and Rounds (Batch name `yx44`)**: Three three-level tasks 4.4.1 / 4.4.2 /
> 4.4.3 with wasm property reachability judgment same batch; batch verification tree
> `target/yx-scratch/yx44/tree` (first full build). Roles and rounds: implement 4.4.2 = t1 (12
> files; local `1fc135437` → post-push `24028ffe8`) ｜ implement 4.4.1's layer order rewrite = t2
> (only doc/comment, zero behavior change, based on `b5e68b3c9`) ｜ implement 4.4.3 = t3 (11 files,
> including new code W1082; local `2d5e610cd` → post-push `52d2276ca`) ｜ wasm reachability judgment
> = t4 (only judgment, zero cleanup, judgment revision `b5e68b3c9`) ｜ 4.4.1 closing and one
> document commit = t8 (7 files 173+/43-; local `29ebd2145` → post-push `d07993157`) ｜ independent
> verification = t5 (frozen `29ebd2145`) ｜ round 1 review = t6 (revision `1b593e993`) ｜ patch = t9
> (3 files 9+/10-; local `1ebddaf95` → post-push `1b593e993`) ｜ fix round = t10 (3 files 23+/23-;
> local `657c3d5e3` → post-push `e184d88b6`) ｜ round 2 review = t11 (`657c3d5e3` = post-push
> `e184d88b6`, patch-id equal) ｜ closing = t7 (check / record / batch tree gc / document real build
> / one document commit; local `c8412565c` → post-push `da724940b`). **Failure and Redo**: t6
> terminal failed with `needs_revision` (review round per contract must fail and give findings, not
> accident); t7 first round interrupted at batch tree gc phase, completed product left in main repo
> working tree not committed, second round took over "patch gap + re-review"——didn't redo completed
> part (this round added: 4.4 check and evidence note, this record, three path rewrites pointing to
> deleted batch directory, document real build and closing commit). No cancelled rounds.
>
> **Two. Per-Task Output and Measurement Numbers (Original Commands and Output See Evidence Summary
> and Archive; Only Conclusions Here)**:
>
> - **4.4.1**: `layers/README.md` (including `layers/mod.rs` top comment)'s layer order table
>   rewritten to **actual call order** (predicate → termination → ownership, call point anchored
>   `checker.rs::check_module_impl`), `equivalence` per real identity described (no production call
>   point); same-source declaration 7 places closed (`01-routing` / `02-stage-contract` / RFC-039 /
>   `06` / typecheck README / layers two files); mechanical reconciliation script on batch tree
>   green baseline **11/11 PASS** (criterion 11 = README side 4: actual call order declaration / two
>   entry anchor / old order declaration vanished; code side 7: 5 line anchor + `check_module_impl`
>   no short circuit + `equivalence` zero production call; script archived
>   `target/yx-scratch/yx44-evidence/t5-441-recon.sh`), mutation (c) roll back old declaration →
>   reconciliation 2 FAIL i.e. red.
> - **4.4.2**: Solver acquisition surface unified to `proof/smt/backend.rs`'s process-level shared
>   singleton + sole closure port `with_shared_solver(consumer, f)`; three consumption points
>   (predicate / termination / ownership) all changed to shared port, **compilation production chain
>   `default_solver()` call points = 0** (sole backend selection point sunk to private
>   `new_backend()`). `wasm/src/lib.rs`'s `test_smt` self-check call point is **existing** (earlier
>   than this batch, since `b5e68b3c9`) non-test call point, belongs to "test and independent scene"
>   surface, doesn't constitute D58 prohibition violation. Independent verification self-built
>   reading (one compilation): predicate acq=2 hit=1 miss=1 ｜ termination acq=1 hit=2
>   miss=2 ｜ ownership acq=2 hit=1 miss=1, three points hit/miss sum (4,4) = shared instance
>   cumulative `Some((4,4))`; mutation (a) singleton degenerate → predicate hit=0 i.e. red. **Change
>   Word Surface Disclosure**: This commit contains `z3_backend.rs` 15 lines——`cache_stats` migrated
>   from intrinsic method into `Solver` trait block (returns `Some(instance internal count)`) + 1
>   line comment changed reference, no semantic coupling. D58 prohibition already mechanized:
>   same-thread reentry → panic contains D58, probe `recv_timeout(120s)` doesn't hang.
> - **4.4.3**: `body_checker` missing's silent degradation converged to
>   `TypeChecker::ownership_inputs` (sole criterion point) + new code **W1082** (no overlap with
>   existing W code fact: W1080 constraint degradation / W1081 solver missing / W1090 opposite
>   direction); nail test 2/2 green; healthy path corpus 348:348 zero diff; **degraded branch
>   production surface unreachable exhaustion holds** (CLI four entries + eval/repl + format + LSP +
>   wasm playground + all `ProgramKind` converge to `check_module_impl`: `init_body_checker`
>   unconditionally before ownership block, function inner 0 return/break/continue, `body_checker`
>   no `.take()`) ⇒ W1082 no user-visible surface. Mutation (b) delete emit → nail test i.e. red.
>   **W1082 belongs to external contract table change (Class E), user-ratified**; `locales/zh.json`
>   / `locales/en.json` same submission as code, other languages supplemented by i18n bot after push
>   (run see "Nine").
> - **wasm Property Reachability Judgment (t4)**: `orchestrator.rs` current tree **10 = 4 positive
>   arm + 6 negative arm** (caliber = same line inner `#[cfg(...)]` attribute text, positive and
>   negative both count; `cfg!` runtime macro 0 hits); 4 wasm positive arms unreachable at
>   playground runtime but **compile-time load-bearing** (call points not gated, alone delete arm ⇒
>   wasm compile fails), 6 `not(wasm32)` negative arms native three stage table load-bearing ⇒ D35
>   envisioned "delete unreachable" in this file **no applicable target**, real cleanup form =
>   entire project-mode subtree gated on wasm32 (register P10 S7, three unverified retained as-is).
>
> **Three. Independent Verification (t5, Frozen `29ebd2145` = Post-push `d07993157`'s Content——Two
> Revisions Per-Commit patch-id Equal, Entire Tree Diff Only in `docs/src/en/**`,
> `docs/src/.i18n-cache.json` etc. Bot Product Surface——`git range-diff` Three All `=`**): Gates
> **8/8 All Green**——`cargo fmt --check` 0 ｜
> `clippy --all --all-features --all-targets -- -D warnings` 0 ｜ `cargo test` **3128 passed / 0
> failed** (7 test targets total; baseline 3126 + this batch 2) ｜ Corpus Diff **348:348**
> line-by-line zero diff (sort diff 0 lines) ｜ IR Snapshot `UPDATE_SNAPSHOTS=1` re-run then
> `git status` zero drift ｜ `check-obligations.py` 21 = 13 + 8, containment 0 gap ｜
> `check-docs-truth.py` source 156 / document 156, CLI 21 commands consistent ｜ Error code
> generator re-run before and after tree both clean. Build warning only 4 bot language W1082 missing
> entries (known state) + Z3 download noise, zero code warning. **Contract Verify Caliber Correction
> (Must Two JSONL Paths)**: Corpus gate's call caliber =
> `python scripts/ci/check-corpus-parity.py tests/baselines/corpus-parity.jsonl <this generation>.jsonl`——**both
> JSONL paths must be given**; missing second parameter is comparing baseline to baseline (constant
> zero diff, criterion idle). This batch executed per two-path caliber and put down evidence (t5's
> `corpus-r44.jsonl` product).
>
> **Four. Mutation Evidence (3/3, Mechanically Reversible)**: All via
> `python scripts/dev/scratch.py trial` (`git apply` → run command → `git apply -R` → per-file blob
> compare with HEAD → require `git status` clean):
>
> - (a) Singleton degenerate → shared reading predicate hit=0 miss=2, test FAILED exit 101;
>   `backend.rs` HEAD=`3e2b2d150b24` → mutation `6d00bad4d4aa` → restore = HEAD.
> - (b) Delete W1082 emit → nail test FAILED exit 101; `checker.rs` HEAD=`70c04a41e020` → mutation
>   `db0a623c9b9a` → restore = HEAD.
> - (c) Roll back old layer order declaration → reconciliation 2 FAIL exit 1; `layers/README.md`
>   HEAD=`8b1ae875db32` → mutation `e88f9af6c42d` → restore = HEAD. Three trial restores
>   `git status` all CLEAN. trial (c) first run due to absolute path resolved by WSL failed exit 127
>   (not criterion red), changed to relative path re-run as standard.
>
> **Five. Review Rounds and Conclusions**: Round 1 (t6, revision `1b593e993`) **verdict =
> needs_revision**——3 findings all document line number anchor inaccuracy (F-01 medium: `02` §"wasm
> current state" 4 groups of outdated line numbers; F-02 low: `02` and `09`'s 4.4.3 delivery anchor
> `checker.rs:1283-1293`; F-03 low: `06`'s comparison statement), four substantive deliveries all
> pass. → Fix Round (t10, local `657c3d5e3` = post-push `e184d88b6`, 3 files 23+/23-, with two
> mechanical criteria) → Round 2 (t11, same revision) **verdict = pass, new finding = 0**. **Final
> Check Basis = Round 2's pass**. **Round 3 (Closing tip `da724940b` Upper)**: Independent
> verification t12 **verdict = pass** (bound `da724940b`: four rule text byte-by-byte, 3.19 / S7 /
> S8 field and scope complete, count table and judgment chain recalculated, dead reference zero,
> `check-docs-truth` 0, `markdownlint` 0); review t13 **verdict = needs_revision**, 4 findings——F-01
> medium (t10 / round 2 only gave rebase pre-hash, §Fourteen's list inconsistent with origin/dev
> actual) / F-02 low (`02:379`'s 09 4.2.8 anchor pushed by this batch itself at `09:144`/`:182` each
> insert one line to `:274`) / F-03 low (`09a` wrote local drive path) / F-04 low (three-level count
> row note 144+2+2+2+2+1+1+3 = 157 but 157 ≠ table header 156) → Fix Round t14 (this commit) closes
> item by item. **t10 Contract Caliber Ruling**: Fix surface based on **task record's 5 F-01…F-05**
> (F-04/F-05 are mechanical criteria), dispatch brief not criterion.
>
> **Six. User Ruling (2026-10-18) and Rule Solidification**:
>
> 1. **D58 Port Name In-Place Rewrite**: `with_shared_solver(f)` →
>    `with_shared_solver(consumer: SolverConsumer, f)`, in-place correct body, **no revision note**
>    (t8 completed in one commit within 4.4.1).
> 2. **wasm Judgment Registered as P10 S7** (Ruling B): with "belonging phase: P10; whether blocks
>    mainline: no" + three unverified **retained as-is**; `06`'s F2 "needs decision" line changed
>    reference to that registration; t4 produced "### F2 Judgment" subsection not touched.
> 3. **Crash Item Registered as P3 3.19** (Ruling A): symptom / cause points to
>    `proof/smt/translate.rs`'s sort inference / D59 **Class A/B** (crash) / acceptance line (this
>    form no longer abort——conservative degradation or diagnostic, corpus and IR snapshot zero
>    drift); **qualitative = pre-existing** (t5 two revision comparison, see "Seven" ⑤).
> 4. **Rule Solidification One** "Correction In-Place Write Body, No Old-New Comparison": Write
>    `coding-rules.md` (added clause), `07-equivalence-oracle.md` §Semantic Correction Diverted
>    Criteria Clause 3, RFC-039 **D59 Row** Clause 3 (same submission registration = WBS row
>    annotation error type + in-place correct affected body + PR required block "error type" one
>    line).
> 5. **Rule Solidification Two** "Contract `inScope` All Write Path Prefix Parseable by Path
>    Matcher": Write `AGENTS.md` team orchestration convention clause 6.
> 6. **New Code W1082 Ratified** (external contract table change, Class E).
> 7. **Register P10 S8** "Document Correction Form Convergence (In-Place Rewrite, Delete Revision
>    Note)": Contains 7 audit checklist scope and acceptance line.
> 8. `09` top count table: second-level **63** (60 + 3.19 / S7 / S8), third-level **157** (154 + 3),
>    with 2026-10-18 note; third-level item's account re-review see "Fifteen".
>
> **Seven. Process Facts That Must Be Truthfully Recorded**: ① 4.4.2 commit used
> `SKIP=cargo-fix,cargo-clippy-fix` to skip two `--fix` hooks——shared working tree had teammate
> in-progress work, `--fix` would incidentally modify others' files; **equivalent gate already
> manually fully run in final tree** (fmt --check / clippy `-D warnings` both 0). ② `pre-commit`'s
> stash window indeed occurred (`Stashing unstaged files…` / `Restored changes from…`), after commit
> `git status` check shows teammate in-progress work intact. ③ **4.4.3's inScope Lesson**:
> Prose-limited `docs/src/(limited to error code generated products)` rejected by `changedPaths`
> validation ⇒ `inScope` all write path prefix parseable by matcher (already solidified as rule, see
> "Six" 5). This task **actually 11 files**, of which 7 because declaration writing can't be parsed
> by path matcher not listed in `changedPaths`, already item-by-item listed in task result
> (`codes/w1xxx.rs`, RFC-013 code table, `reference/error-code/`'s index and each E section page
> etc.). ④ **W1082's locale Window Closed**: `locales/zh.json` / `locales/en.json` same submission
> as code, other languages supplemented by i18n bot after push (run see "Nine"). ⑤ **Z3
> `Sort mismatch` Crash Judgment (t5 Criterion ⑦)**: **Pre-existing**——same probe, same input on
> `b5e68b3c9` and frozen revision `29ebd2145` stdout **byte-by-byte consistent**
> (`Error: Sort mismatch at argument #1 for function (declare-fun not (Bool) Bool) supplied sort is Int`),
> both exit 1; static attribution consistent (4.4.2 to `translate.rs` 0 line change, to
> `z3_backend.rs` only `cache_stats` migration). **Degraded branch production surface unreachable
> exhaustion holds** (W1082 no user-visible surface). Probe three lines of source:
> `NonNegative: (x: Int) -> Type = { x >= 0 }` / `AtMost: (n: Int, x: Int) -> Type = { x <= n }` /
> `f: (x: NonNegative(x)) -> AtMost(x, x) = { return x }`. ⑥ **t5 Frozen Surface and t9's Coverage
> Difference**: t9 (three comment/doc patches, `1b593e993`) committed later than t5's frozen
> revision `29ebd2145`, belongs to zero behavior change, its acceptance covered by t9's own
> mechanical verification + t6 review (t6 on `1b593e993` **re-ran** t5's key measurement, not
> paraphrased). ⑦ `docs/src/en/**` not manually modified, regenerated by post-push auto-translate
> workflow (this batch bot landed two commits: `92d490695` i18n locales supplement W1082 other
> languages, `63b6cdf37` auto-translate document). ⑧ **t9 Out-of-Scope Observation Three
> Rulings**: ㈠ t9 incidentally corrected `02` same sentence's `wasm/src/lib.rs` line count and
> gating point line number (73 / 139·153·170 → 152 / 151·170·187)——**accepted** (same caliber same
> number required surface, commit message already listed); ㈡ RFC-039 **D35 Row** still writes "wasm
> 27 files' cfg branches"——**keep unchanged** (ruling row is historical ruling record, its premise
> correction carried by P10 S7 registration, don't rewrite history); ㈢ `docs/src/en/**`'s old port
> name translation aligned by auto-translate. ⑨ `09` / `09a` originally pointed to deleted batch
> directory's two paths (`…/yx44-evidence/EVIDENCE.md`, `…/probe-sort-mismatch.yx`) in closing round
> changed to point to `target/yx-scratch/close-44/evidence-summary.md`, probe three lines of source
> inlined in place.
>
> **Eight. t8 Acceptance ④ Caliber Ruling**: Acceptance original text requires
> `git diff | grep '修订注记\|原为\|现改为'` to be empty. **Measured default context caliber
> unreachable**——D58 row's adjacent D59 / D61 ruling rows themselves contain "修订注记 / 原为" words
> (not this task's change). Ruling: per **added lines / deleted lines / `-U0` increment lines**
> three calibers all empty accepted, and write **subsequent similar acceptance only grep
> added/deleted lines**——criterion should fall on this change's lines, not entire diff's context.
>
> **Nine. CI Gate Conclusion (AGENTS Clause 13①, Scheduled Verification 2026-10-18 17:26 Backfill;
> Details `target/yx-scratch/close-44/gate-runs.md`)**: First push (3 commits, tip = `d07993157`):
> CI **39110908163** success ｜ Deploy Docs **39110908155** success ｜ Auto Translate Docs
> **39110908193** success ｜ Auto Translate I18n **39110908144** success; this tip covers
> `24028ffe8` (4.4.2) / `52d2276ca` (4.4.3) / `d07993157` (4.4.1) three commits all content. Second
> push (1 commit, tip = `1b593e993`): CI **39112293391** success ｜ Deploy Docs **39112293062**
> success ｜ Auto Translate Docs **39112292903** success; covers all four commits including t9
> patch. **No concurrency cancellation**, latter covers former. Third push (2 commits, tip =
> `da724940b`): CI **39121538555** success ｜ Deploy Docs **39121538562** success ｜ Auto Translate
> Docs **39121538541** success——three runs' headSha all = `da724940b` and **all terminal**
> (2026-10-18 third round scheduled verification backfill); round 3 independent verification t12's
> "CI not terminal" observation closed. Unrelated: Nightly **39081392694** failure's headSha =
> `290f9ebc`, not this batch's commit surface.
>
> **Ten. Three `pull --rebase` (Auto-Commit on Opposite Side During Push)**: Opposite sides are
> auto-translate and i18n commits; six hashes rewritten and item-by-item **patch-id equal**——local
> `1fc135437` → `24028ffe8`, `2d5e610cd` → `52d2276ca`, `29ebd2145` → `d07993157`, `1ebddaf95` →
> `1b593e993`, `657c3d5e3` → `e184d88b6`, `c8412565c` → `da724940b` (after third rebase origin/dev
> head = `da724940b`); `git range-diff` verified **per-commit content unchanged** (three all `=`).
> Members didn't execute `git push` / `fetch` / `pull`, sync and push by captain unified. **Evidence
> Reminder**: Henceforth reference commit numbers all use **post-push hash** (above right values);
> `09`'s check rows per convention don't write commit numbers.
>
> **Eleven. Uncovered Surface (Can't Write "Didn't Test" as "Passed")**: ① doc-tests 0/0 not
> load-bearing criterion; ② **wasm32 target actual compile not done** (this surface covered by CI's
> `_build-wasm.yml`, this batch's wasm judgment is static argument); ③ mutation only covers 3 places
> named by contract, other gates no reverse break verification; ④ corpus diff not separately run on
> `b5e68b3c9` (only run on frozen revision); ⑤ i18n bot supplemented other languages W1082 text
> behavior not verified (only verified `zh.json` / `en.json`); ⑥ round 1 review and fix round
> reported **same kind but not in contract findings existing drift** not fixed (list see "Twelve"),
> attribution pending user ruling.
>
> **Twelve. `02` Entire Document Existing Drift Observation List (Reported to User, Pending Ruling
> Then Registration)**: Caliber = all **pre-existing drift** (not this batch's introduction),
> **doesn't constitute this batch's finding**; list found by fix round, round 2 review on
> `657c3d5e3` (= post-push `e184d88b6`) independently spot-checked and gave mapping, each item
> verbatim (full list archived `target/yx-scratch/close-44/s9-observations.md`):
>
> 1. `02` §(a) entry table 3a-3d / 5 / 5b: `02:72` cites `orchestrator.rs:273-399` → now `:155`;
>    `02:73` `:450` → `:250`; `02:74` `:1374` → `:985`; `02:78` `util/diagnostic/mod.rs:565` →
>    `:681`; `lsp/handlers/diagnostics.rs:146`'s `run_diagnostics` no longer there; **content
>    level**: `run_diagnostics` and "otherwise manual lex→parse" already replaced by 4.6.1 (delete
>    manual sequence).
> 2. `02:1016`: `compile_project :99-237` → now `:107-134` (28-line Program constructor).
> 3. `02:360-361`: "missing degrade to warning" → current tree `_build-wasm.yml:50-51` is
>    `::error` + `exit 1` (#435); `build.rs:65-72` already changed `panic!` hard fail;
>    `_build-wasm.yml:35-59` interval needs re-fetch.
> 4. `02:127` mechanism pointer: mechanism actually at `src/driver/arms.rs::typecheck_orchestrated`
>    (`:288-289` self-described); conclusion still holds (multi-file arm drops warnings), inaccurate
>    is "result's sole downstream use" one sentence (`type_result` also via `arms.rs:1389` →
>    `helpers.rs:259/:636` feeds `overload_resolutions`); suggested fix = pointer change to
>    `typecheck_orchestrated`, "sole downstream use" change to "warnings field no read point".
> 5. `02:128`: `check_project:385-395` / `250-252` needs re-fetch.
> 6. `02:334` (§S5 (d) investigation paragraph): cited old code snippet already replaced by 4.4.3's
>    `ownership_inputs` criterion point.
> 7. `02` change list table other rows: `predicate.rs:34-36`, `checker.rs:1303-1314` / `:1244`,
>    `backend.rs:67-72`, `wasm/src/lib.rs:30-36` / `:42-51`, `util/diagnostic/mod.rs:621-661`.
>    **This closing doesn't pre-set S9's WBS item** (user didn't rule); after obtaining ruling then
>    per registration form land `09`.
>
> **Thirteen. Batch Tree Reclaim (gc and status Original Output; Tree Path in Output Per Repo
> Relative Path)**: `python scripts/dev/scratch.py gc` →
> `[scratch] 已删除 yx44-evidence（释放约 0.0 MB）` / `[scratch] gc 完成，共释放约 0.0 MB`;
> `python scripts/dev/scratch.py status` → `yx44-evidence            holder=-` next line
> `target/yx-scratch/yx44/tree`——registry recorded path **doesn't exist**, this name is registry
> residue (`gc`'s `_remove` deletes `meta["path"]`'s parent directory i.e. `target/yx-scratch/yx44`,
> residual `meta.json` still in scan surface; its content batch=`yx44`, `base_rev=29ebd2145`,
> `lock=null`, contains t5 / t6 / t11 hold history); Closing round renamed that registry file to
> `meta.retired.json` to make it exit scan (**don't delete any evidence file, don't create any new
> tree**) → `python scripts/dev/scratch.py status` = `[scratch] 没有批次树`; re-run `gc` =
> `[scratch] 没有批次树，无需回收`; `git worktree list` only main mount (`dev`). Batch original
> evidence directory `target/yx-scratch/yx44-evidence/` (no `meta.json`, **not batch tree**) kept as
> archive, closing summary separately at `target/yx-scratch/close-44/`. Acceptance expression
> (parseable): `git status` clean; after batch tree gc `python scripts/dev/scratch.py status` = no
> batch tree; `git worktree list` only main mount; no this batch new directories outside repo.
>
> **Fourteen. Document Real Build and Commit Hygiene**: `npm run docs:build` locally **exit 0**
> (AGENTS clause 10⑨——markdownlint doesn't catch VitePress's Vue compile error, and docs-only commit
> will be judged `no files to check` skip markdownlint); this batch's check / note / registration /
> rule modification accumulated to **one** document commit (clause 8), at batch closing; members
> didn't execute `git push` / `fetch` / `pull`. **This batch's final two commits both pushed**
> (closing tip upper `git rev-list --count origin/dev..HEAD` = 0, origin/dev head = `da724940b`):
> `e184d88b6` (round 1 findings' document anchor fix, 3 files 23+/23-) + `da724940b` (closing
> document commit, 7 files 96+/10-; including check / record / three path rewrite / registration and
> rule modification).
>
> **Fifteen. Count Table Account Re-Review (Round 3 Fix F-04; Method and Result Both
> Re-Verifiable)**: Judgment object = `09` top "Position and Scope" table's three-level count row.
> **Method** (two independent calibers, per revision use `git show <rev>:<file>` to take bytes
> recalculate): ① **Sum Per Second-Level Entry**——take each second-level kanban row's parenthesis "N
> three-level tasks" (row start `- [x]`/`- [ ]` bold title first, row contains "个三级任务",
> including S7 row in-line written "1 个三级任务"): current tree 70 second-level entries, **sum =
> 157**. ② **Historical Self-Consistency**——same caliber on `366bc5a69^` got 66 entries, **sum = 151
> = current period header 151** (self-consistent); `366bc5a69` day newly registered `4.9.6` and
> `0.4.1` two three-level (sum 151 → 153), but header only writes 151 → 152 ⇒ **header from this
> commit missing 1**; thereafter each revision all differ by 1 (`92d490695` / `1b593e993` /
> `d07993157` / `e184d88b6`: sum 154 vs header 153; `da724940b`: sum 157 vs header 156). **Result**:
> Truth = **157**, so correct header (`09:13` third-level 156 → 157); note 144+2+2+2+2+1+1+3 = 157
> originally correct, not changed; this batch's previous base = **154** (not 153). **Second-level
> count row not touched**——this row's caliber never aligned with kanban entry count (`366bc5a69^`
> upper is 66 entries vs header 59), this time no verifiable evidence to overturn 53 + 10 = 63,
> don't change number to fit. Incidental correction (same round): §Two's 4.4.1 parenthesis breakdown
> per reconciliation script actual item count changed to "README side 4 + code side 7 = 11",
> consistent with **11/11** and script 11 `ck` calls.

## P3 Debt Round 1 Closing (3.5 / 3.6 / 3.4.6 / 0.5)

> **One. Batch Composition and Rounds**: This batch closes P3 side remaining debt——**3.5
> Higher-order type parameter call path** (3.5.1–3.5.3), **3.6 Function type value call path**
> (3.6.1–3.6.4), **3.4.6 compile path consumes warnings** (Class D, user 2026-10-19 ruling "do"),
> and registers **0.5 Test suite layering** (P0, issue #486 slice) and **3.20 Dead code analysis
> existing false positives** (3.20.1 / 3.20.2) together. Roles and rounds:
>
> - Implement: 3.5.1–3.5.3 = t1｜3.6.1 = t2｜3.4.6 = t6｜3.6.2 = t7｜3.6.3 = t8 (main + closing two
>   commits)｜3.6.4 = t9｜0.5.1 = t10 (#486 slice)
> - Independent verification: round 1 = t3 (frozen revision `81d5e414e`, covers 3.5 / 3.6.1–3.6.3 /
>   3.4.6)｜3.6.4 = t11 (round 2, frozen `8e32e8636`)
> - Review: round 1 = t4 (verdict = **needs_revision**, F1/F2/F3)｜fix round = t12 (one
>   commit)｜round 2 = t13 (verdict = **pass**)
> - Closing = t5 (check / register / this record / batch tree gc / document real build / one
>   document commit)
>
> **Failure and Redo**: t4 terminal failed with needs_revision (review round per contract must fail
> and give findings, not accident) → t12 truly fix → t13 review pass. t8 internally experienced one
> attempt interruption (process crash), previous round evidence kept in batch directory, continuing
> attempt supplemented two commits into repo. No cancelled rounds.
>
> **Two. Per-Task Output and Measurement Numbers (Conclusion Caliber; Original Commands and Output
> See Each Batch Evidence Directory)**:
>
> - **3.5.1** (Unifier matches constructor variable at head position): Criterion converged to **sole
>   implementation** `is_ctor_reference` (`pub(super)` free function, repo 1 definition 3
>   consumers)——this is t12's true fix for t4's F1 (originally `inference/statements.rs` had another
>   same-name method copy, constituting prohibition three sense same-name two implementations). Head
>   rename no longer inlined copy: `environment.rs` and `expressions.rs` each first traverse TypeRef
>   / variable table to construct head_map, then uniformly hand to shared implementation
>   `substitute_ctor_heads` (`types/substitute.rs`) execute——two places' head_map construction
>   criteria unchanged, rename only acts on head per name, empty table returns as-is (t12's F2 true
>   fix). Equivalence argument: traversal doesn't change Generic head name.
> - **3.5.2** (Explicit type actual parameter accepts type constructor, E1010 disappears): Red state
>   E1010 / E1002, green state exit 0.
> - **3.5.3** (Mono product replaces constructor variable): Symbolic type actual parameter criterion
>   narrows by container generic parameter name; product's constructor variable replaced by real
>   constructor (IR dump evidence: specialized function `Function probe(Box, int64)`).
> - **3.6.1** (Local function type value called by name): Criterion sole source =
>   `is_static_fn_name` (repo 1 definition 1 call), aligned with variable evaluation path's name
>   resolution priority (local → global), no new second resolution table; IR evidence: module init
>   segment changed from `LoadConst String("f")` + `CallNative` to `LoadGlobal` + `CallDyn`.
>   Regression surface: same name "function binding + value binding" unreachable (compile-time
>   E2010), non-function value called by name still E1065.
> - **3.6.2** (`use` alias imported function value called by name): Originally reported
>   `E6006 Native function not found: lib.f` (`use lib.{f}` and `use lib.{f as g}` two forms). Fix =
>   within `is_static_fn_name` **add an arm** (not copy), new sole implementation
>   `use_alias_value_slot` takes "use alias → qualified name → global slot" two-table join query, 2
>   consumption points (call form criterion + variable evaluation path).
> - **3.6.3** (Nested function body references outer frame value): **Criterion Re-Judged**——not "let
>   it run right", but this form per RFC-009 §2.3 + SPEC §12.3 should be **rejected at compile
>   time** (D59 Class C: change silent wrong value to loud; per `09` §P3's D59 row, "silently give
>   wrong answer" is Class C——this batch's closing brief records as "Class A/B", both refer to "old
>   behavior not successful", landing form both "red first then green + compile-time loud reject").
>   Original form = **no error, wrong value (void)**: red state stdout `void`, EXIT=0 (silent wrong
>   value archive); green state compile-time E1001 `Unknown variable`, EXIT=1, with existing
>   "unknown variable" path **same code same text**. Criterion sole source = capture barrier
>   (`enter_capture_barrier`), compile-time name (type parameter) not blocking; E1001 still only
>   emitted in typecheck, registered sole. **Assignment path covered by E2010** (t8's closing
>   commit: declaration / assignment judgment doesn't apply capture barrier)——otherwise this fix
>   would itself open a new "same name assignment silent wrong value" channel (c1 form: only barrier
>   degenerates to exit 0 print 10).
> - **3.6.4** (`use lib` full import then `lib.f(...)`): Goes through FieldAccess callee's namespace
>   branch, originally reported E6006 `lib.f`. Fix = namespace branch resolves qualified name then
>   first judges global slot layout → changes to indirect call; qualified name resolution and slot
>   query reuse `resolve_namespace_data_access` (same source as data access), no new resolution
>   table.
> - **3.4.6** (compile path consumes warnings, Class D): Before and after change external behavior =
>   **multi-file `run` previously never output any W code** (red state archive: multi-file [] vs
>   single-file [(W1001,3),(W1002,5),(W1003,1)]), now four paths (single-file run / multi-file run /
>   single-file check / multi-file check)'s W code set always = {W1003@1, W1001@3, W1002@5}
>   (non-empty set). Corpus baseline: 348 records in **46 changed**, added W code **118** (W1001 34
>   / W1002 12 / W1003 13 / W1004 42 / W1005 17), **118/118 per-item attributed to real existing
>   unused definitions in file (0 unlocated)**, pure increment (0 deleted, 0 exit / stdout / check
>   change). **D59 Four Fulfilled**: ① red first then green (red state archive + turn to nail test)
>   ② doesn't change "originally successful program"'s external behavior (pure increment, 0 exit /
>   stdout / check change) ③ same submission register (this record + `09`'s 3.4.6 row) ④ baseline
>   same submission regenerate and manually review diff. **Superset Approved**: W1004 / W1005 /
>   W1006 also enter `run` (user 2026-10-19 approved), reason = **one criterion, one channel**——only
>   putting W1001/W1002/W1003 is rebuilding second filter judgment. **External Contract Surface
>   Change**: `orchestrator::compile_project` return value from `Result<ModuleIR, _>` changed to
>   `Result<(ModuleIR, FileDiagnostics), _>` (repo consumers 2 + 2 test direct-drive points
>   synchronously adapted).
> - **0.5.1** (test resource profile marker + exclusive lock + fast/full minimum consumption
>   surface): Added `scripts/dev/testsuite.py` (`markers --check` and `cargo test --list`
>   bidirectional reconciliation, `plan` budget binning, `run fast|full` budget insufficient reports
>   gap not silently skip, `manual` file never implicitly execute) and `src/util/test_support.rs`
>   (cross-process file lock, test body takes lock ⇒ any call path mutually exclusive); in-place
>   `cost:` marker 3 (2600 file exclusive, peak from 5000 file measured linear extrapolation, **not
>   measured**; 5000 file exclusive, 15.6 GiB / 72.6 s measured; snapshot update manual, peak and
>   time both extrapolated). Self-verification numbers: `markers --check` verified 3149 `#[test]`, 3
>   mechanical mutations each exit 1, `run fast` batch wave 2793 passed / 47.7 s + exclusive wave 1
>   passed / 14.6 s, budget 1 GiB `run full` lists gap 7.11 GiB, two-process lock contention
>   measured `held→blocked→held(waited 2.0s)`. **Independent verification belongs to #486 subsequent
>   scheduling** (this batch only implementer self-verify). **0.5.2 pending scheduling** (pre-commit
>   / CI wiring + 2600 file actual measurement calibration + `test-specification` detailed section).
>
> **Three. Independent Verification (t3, Frozen `81d5e414e`; Red State Baseline `c599de7bb` /
> `1725d0215` / `ef2334de5` All HEAD Ancestors, `git merge-base --is-ancestor` Verified)**: Probes
> all verifier self-built (batch tree `target/yx-scratch/verify-p3-t3/`, probes and patches in its
> `probes/` / `patches/`), not paraphrasing implementer conclusion; this batch tree along with its
> in-tree evidence already deleted in this batch closing's gc (see "Nine"), so this section puts
> measurement conclusion and numbers **verbatim into record** for evidence. 3.5 red state four
> probes all report E1003+E1002 / E1010, green state exit 0 and value correct (stdout 1 / 7 / 1),
> with IR value-level evidence (specialized function `Function probe(Box, int64)`); 3.6.1 green
> state output 6 / 10, control "value position call" green, "non-function value called by name"
> compile-time E1065; 3.6.2 red state two forms E6006, green state exit 0 (probe inner `assert`
> failure is non-0, exit 0 is value correct); 3.6.3 red state stdout `void` / EXIT=0 (silent wrong
> value verifiable), green state compile-time E1001 / EXIT=1, and **expression position inline
> lambda capturing this function parameter** allowed (EXIT=0, output 6)——allow surface not injured.
> 3.4.6's C2 normalization by verifier self-built fixture independently reproduced (four-way W set
> identical, non-empty), baseline diff independently recalculated (changed=46, added=0, removed=0,
> expectation / run.exit / run.stdout / check change = 0, added W code heads 118 consistent with
> implementer), attribution sample 10 all hit real definitions. Gates on frozen tree all green: fmt
> / clippy (`-D warnings`) / `cargo test --all` (**lib 2790 passed + 2 ignored, integration 297
> passed**, 0 failed) / corpus diff 348:348 diff empty / IR snapshot 204 zero drift / obligation
> ledger / docs-truth. **t11 (Round 2, Frozen `8e32e8636`)** independently reproduced 3.6.4's
> red→green (red state parent commit 16e2a5c88: E6006 `lib.f`, EXIT=1; green state EXIT=0, stdout
> 27, probe inner 4 assertions all pass), criterion sole source (`resolve_namespace_data_access` 1
> definition 2 consumers, no third "qualified name → slot" join query), no regression eight forms
> all pass, verdict = **pass**.
>
> **Four. Review Rounds and Conclusions**: Round 1 (t4, judged `20abd3ebc`'s parent revision)
> verdict = **needs_revision**, three findings all true defects: F1 = `is_ctor_reference` same-name
> two copies (`inference/statements.rs` method copy), F2 = head rename in `environment.rs` and
> `expressions.rs` each inlined one copy, F3 = visibility over-boundary (should-private not private,
> should `pub(super)` not converged). → Fix Round t12 (one commit, 4 files +96/−85) → Round 2 (t13,
> reviewed revision `20abd3ebc`) verdict = **pass**, new finding = 0: 8 self-built probes' value and
> exit code per-item same as pre-fix, 4 IR dumps with round 1 green state **byte-by-byte
> consistent**, corpus 348/348 zero drift, IR snapshot 204 zero drift, `cargo test --all` EXIT=0
> (lib 2794 passed + 2 ignored, integration 297 passed, 0 failed), fmt / clippy / docs-truth all
> exit 0, `check-boundary`'s `pub(crate)` count 167 / frontend-core 26 (= 3.5 pre-baseline),
> `check-fanout` same-name definition zero hits. **Final check basis = Round 2's pass**.
>
> **Five. Mutation Evidence (All Via `python scripts/dev/scratch.py trial`, apply → run command →
> apply -R → per-file blob compare + require `git status` clean)**:
>
> | #   | Target                                        | Mutated Blob    | Capture Result                                                                                                                                                   |
> | --- | --------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | M1  | 3.6.1 `is_static_fn_name`'s global lookup arm | `022986cbea65…` | Two probes reproduce E6006, EXIT=1 (restored blob = HEAD, CLEAN)                                                                                                 |
> | M2  | 3.5.2 explicit type actual parameter branch   | `abbcc5ecf116…` | Probe check reports E1002, EXIT=1 (control function form still green ⇒ attribution precise to 3.5.1)                                                             |
> | M3  | 3.4.6's `MULTI_FILE_STAGES` dead code arm     | `372f7b92755d…` | run channel W code all extinguished (C2 red); M3b attribution shows check channel criterion sound, broken is exactly 3.4.6 newly opened compile external channel |
> | M4  | 3.6.3 capture barrier entry                   | `7a7204c60383…` | Capture class turn nail test 4 failed, TEST_EXIT=101                                                                                                             |
> | M5  | 3.6.4 value slot → CallDyn block              | `25853376639b…` | Probe reproduces E6006, EXIT=1 (blob chain `0d837e9d30de…` → `25853376639b…` → `0d837e9d30de…`)                                                                  |
> | M8  | 3.5.1 turn nail test                          | —               | Post-fix re-run still can fail (E1003, TEST_EXIT=101, mechanically reversible restore CLEAN) ⇒ criterion not lost fail-ability                                   |
>
> **Six. User Ruling and This Batch Registration**:
>
> 1. **3.4.6 = Class D** (externally observable: multi-file `run` added W code), user 2026-10-19
>    ruling "do"; **superset (W1004 / W1005 / W1006) also authorized**, reason = one criterion / one
>    channel. Original 2026-10-15 ruling C "defer per decision B1"'s **prerequisite** (4.9 pool
>    semantic correction) already closed 2026-10-17.
> 2. **3.6.3 Criterion Re-Judged**: from "let it run right" to "nested function body references
>    outer frame value → compile-time E1001"; assignment path covered by E2010.
> 3. **Two "Pre-This-Task" Silent Forms Registration Not Changed**: ① same block sibling nested
>    function reference——baseline already E1001 (outer block local family), this batch not expand
>    scope; ② nested function annotation references outer type parameter still print `void` (t1
>    registration item). Both **not** this batch's introduction, both retained as-is.
> 4. **3.20 "Dead Code Analysis Existing False Positives" Registered** (user 2026-10-19 agreed to
>    register per C disposition after): 3.20.1 = W1001 doesn't track closure inner reference
>    (`docs/src/reference/stdlib/test.md`'s `t_add` form: referenced in closure but reports unused);
>    3.20.2 = W1003 judges variant destructuring required import as unused
>    (`docs/src/reference/stdlib/weak.md`'s `use std.option` form: delete then E1002). Both forms
>    reproduce on 3.4.6's pre-baseline, **not 3.4.6 introduced**. **Rollback Obligation (Written
>    Dead)**: After this group fixed must restore `weak.md` / `test.md` modified to bypass false
>    positive's examples **back to natural writing** and **regenerate baseline** (baseline will
>    change once more).
> 5. **0.5 Test Suite Layering** (P0, issue #486): 0.5.1 completed; 0.5.2 (pre-commit / CI wiring +
>    2600 file actual measurement calibration + `test-specification` detailed section) pending
>    scheduling.
> 6. **#489 ureq 3 Migration**: dependency surface tracking item, **not entering WBS**;
>    `.cargo/audit.toml`'s `RUSTSEC-2026-0381` ignore **with hard removal condition**——after ureq
>    migrated to >= 3 then delete that ignore, if still red then can't extend deadline, must first
>    fix dependency. 0.26 line no fix version (0.26.12 doesn't exist), ureq 2.x no update version.
> 7. **3.20 Number Conflict Disposition (This Round New Registration)**: 3.20 number originally
>    occupied by 2026-10-18 registered "pre-release audit (v0.8.4) supplemental gap"
>    (3.20.1–3.20.4), and user 2026-10-19 ruling registered "dead code analysis existing false
>    positives" needs to use 3.20. Entire document number uniqueness rule doesn't allow two 3.20, so
>    **moved pre-release audit group entirely to 3.21** (third-level changed to 3.21.1–3.21.4, issue
>    anchor #468–#471 unchanged, entry content unchanged), 3.20 number given to "dead code analysis
>    existing false positives". Migration only changes number, doesn't change any registered entry's
>    content and attribution; historical commit information not rewritten (migration fact recorded
>    in this record and `09`'s count table note).
>
> **Seven. Process Facts That Must Be Truthfully Recorded**:
>
> 1. **Two inScope Writing Narrow Each Exchanged for One Round Rework**: ① 3.6 series' implementer
>    wrote inScope narrow to `ir_gen.rs` predicate layer, didn't cover `types/`'s shared replacement
>    implementation → review round 1's F2 when landing just discovered head rename in two files each
>    inlined one copy, need one more round of convergence; ② 3.5's fix round when moving changes
>    between `types/` shared implementation and two call sites, again hit inScope and actual change
>    surface mismatch (AGENTS.md clause 6: inScope must cover mechanically required surface, path
>    prefix parseable by matcher). Both reworks due to "implementer inScope wrote narrow", **not
>    implementation defect**——but cost is two rounds of rework and one review findings.
> 2. **External Sync (2026-10-19 13:08 `pull --rebase --autostash origin dev`, 13:19
>    `pull --tags origin dev`) and V0.8.5 Release Commit**: Per user 2026-10-19 confirmed as
>    **normal release flow, completed, doesn't affect this team**; member commits intact not lost.
>    Evidence (`git reflog --date=iso` / `git reflog show origin/dev --date=iso`): 13:08:43's
>    `pull --rebase --autostash` picked up release chain's "supplement CHANGELOG retention policy
>    criterion and self-check", 13:19:34's `pull --tags` was fast-forward; V0.8.5's release commit
>    (`:twisted_rightwards_arrows: V0.8.5: …` and its `:bookmark:` companion commit, CHANGELOG
>    generation and retention policy two commits) all in `v0.8.5` tag reachable range.
> 3. **Three rebase Caused Local Hash Rewrite (Reference Old Number Will Miss)**: `b3245aac9` →
>    `56ae469e1` (3.6.3 main), `415f0154b` → `ef2334de5` (3.6.2), `bdbe859e1` → `8e32e8636` (3.6.4).
>    Verifier used "src/, tests/ tree diff empty" to prove three are **pure rebase equivalent**
>    (content unchanged, only parent chain changed), this record all use **currently reachable**
>    hash.
> 4. **F2's Unfixable Part**: 3.5.1 that commit's message wrote "sole implementation" belongs to
>    history, **not rewrite**; correction can only carry by "shared implementation convergence that
>    commit + code comment per fact correct"——this record keeps this sentence, lest reader
>    mistakenly think nobody discovered this problem at the time.
> 5. **Three Observation Items (Verified, Don't Block, But Registered to Avoid Being Treated as
>    Unhandled Violation Later)**: **O1** head rename convergence to shared implementation coverage
>    drills down `MetaType` / `AssocType` / `Literal`, but three coverage probes **all parse fail**
>    (`Pipe` / `Ampersand` / old `Type[...]` syntax; `ast::Type::Union` no construction point in
>    parser) ⇒ **design stronger, currently unreachable**, not behavior change; **O2**
>    `check-fanout.py` after fix round new report `add_type_param: 2 files`, introducer is 3.6.3's
>    closing commit (`inference/scope.rs`) **rather than** this batch's fix round, review judged
>    **heuristic false positive** (delegate wrapper + sole implementation division of labor, same
>    type as existing `add_param`); **O3** `09`'s 3.6 row original text still writes "1 three-level
>    task" but actually has 3.6.1–3.6.4, 3.5 three rows missing completion note——by this closing
>    round supplement together (see `09`).
> 6. **`src/driver/arms.rs`'s "Decision B1" Comment Not Changed with Code Same Submission (Residual,
>    Not Changed This Batch)**: 3.4.6 already changed `typecheck_orchestrated`'s FailFast branch to
>    **temporarily store** `result.warnings` (originally directly discarded), but this function's
>    upper doc comment still writes "`result.warnings` discarded——Decision B1", opposite to code
>    fact after 3.4.6; same source another comment (`src/driver/tests/program.rs`'s stage table
>    description) **already changed with 3.4.6 that commit same submission** to "Decision B1 'dead
>    code family warnings don't enter compile path' hereby revoked". This batch `src/**` out of
>    scope, so **not changed** that comment, registered for next team (belongs to AGENTS.md clause 6
>    "ruling directly named declaration carrier" class debt). `02`'s runtime behavior table B1 row
>    already in-place correct to current state (dead code family warnings now through same criterion
>    enter MultiFile `run` path).
> 7. **Members Didn't Execute `git push` / `fetch` / `pull`**: sync and push by captain unified (in
>    `git reflog show origin/dev` each entry is "update by push", member side no push record).
>
> **Eight. Uncovered Surface (Verbatim, Can't Write "Didn't Test" as "Passed")**:
>
> - **t3's 7 items**: ① wasm32 target actual compile not done; ② performance impact not measured
>   (3.4.6 added two stages, 3.6.x's indirect call overhead no benchmark); ③ **W1005 per-item
>   attribution not independently reproduced** (verifier's message form regex didn't cover its text
>   form, only verified added W code total 118 consistent); ④ 3.6.3 closing commit (declaration /
>   assignment not apply barrier) not separately mutated, c1 form not independently re-tested; ⑤
>   t8's 29 probe matrix not fully re-run (only re-tested three representative forms + M4 turn
>   nail); ⑥ 3.6.4 belongs to round 2 scope, round 1 not independently measured; ⑦ CI gate per
>   convention not waited in session, hand to closing round unified verification.
> - **t11's 4 items**: ① qualified name callee with generic actual parameter (`lib.g(Int)(3)`) and
>   named actual parameter call function type value——no sample in repo, this round not constructed
>   re-test; ② under `use lib` full import without item import's `p.get_x()` (existing E6006
>   boundary, earlier than this task) not re-tested mutation state; ③ on `8e32e8636` not repeated
>   full gate (by t9's gate log and t3 on `81d5e414e` containing this commit's full green coverage);
>   ④ CI hand to closing round verification.
> - **This Closing Round New Registration**: This round's document change **didn't re-run
>   `cargo test --all`** (change surface is `docs/**` three files, zero code surface); closing
>   round's gate is `npm run docs:build` + `check-docs-truth.py` + batch tree reclaim + `git status`
>   check.
>
> **Nine. Batch Tree Reclaim (gc and status Original Output)**: `python scripts/dev/scratch.py gc` →
> `[scratch] 已删除 verify-p3-t3（释放约 13220.5 MB）` /
> `[scratch] 已删除 verify-p3-t4（释放约 14428.0 MB）` / `[scratch] gc 完成，共释放约 27648.5 MB`;
> `python scripts/dev/scratch.py status` → `[scratch] 没有批次树`; `git worktree list` →
> `E:/git/YaoXiang 20abd3ebc [dev]` (only main mount). **Also deleted by gc are two batch
> tree-internal verification evidence** (`verify-p3-t3/` contains independent verification's probes,
> patches, original logs and conclusion text; `verify-p3-t4/` contains review round 2's
> evidence)——they are **registered batch directories**, reclaimed with tree; this record's third,
> fourth, fifth sections have already verbatim recorded their conclusions, mutation table and gate
> numbers, conclusion not invalidated by file deletion. **Unaffected Archive** (non-registered batch
> directories, gc not touched): `target/yx-scratch/wbs35/`, `wbs35repair2/`, `wbs346/`, `wbs36/`,
> `wbs362/`, `wbs363b/`, `wbs364/`, `wbs486/`. Acceptance expression (parseable): `git status`
> clean; `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only main
> mount; no this batch new directories outside repo.
>
> **Ten. Document Real Build and Commit Hygiene**: This batch's check / note / registration /
> document correction **accumulated to one** document commit (AGENTS.md clause 8), at batch closing;
> generator regenerated document merged into its functional commit. Before commit locally run
> `npm run docs:build` (AGENTS.md clause 10⑨——markdownlint doesn't catch VitePress's Vue compile
> error, and docs-only commit will be judged `no files to check` skip markdownlint) and
> `python scripts/ci/check-docs-truth.py`. **Commit Number After Push**: This record per historical
> record convention records each round actually occurred commit and hash (**note division of labor
> with `09` check rows**——`09`'s check / completion registration only writes "task id + evidence
> (gate values, issue number, file:line)" not commit number, because `pull --rebase` will rewrite
> local hash; this record is current round process's historical record, keeps hash and
> simultaneously gives rebase before/after comparison, see "Seven" 3). **CI / Deploy Docs conclusion
> hand to "unified verification" round**, not wait in session (AGENTS.md clause 13)——this batch's CI
> run id and conclusion backfilled by captain in unified verification.
>
> **Eleven. Count Table Account Re-Review (This Round Caliber)**: `09` top's second-level /
> third-level count table continues "**base + increment**" caliber (same as 2026-10-18
> registration). This round increment = second-level +2 (0.5 / 3.20, of which 3.20 is after
> number-move new occupied, pre-release audit group changed 3.21 is net zero), third-level +7 (3.6.2
> / 3.6.3 / 3.6.4 / 0.5.1 / 0.5.2 / 3.20.1 / 3.20.2). **Residual Pre-Existing Deviation (This Round
> Not Correct, For Subsequent Account Re-Review Reference)**: taking "sum per second-level entry
> parenthesis 'N three-level tasks'" as caliber recalculate, this tree got 162, differs from header
> by 7——difference comes from 2026-10-18 registration when added third-level but didn't
> synchronously rewrite parent entry count note's 5 places (4.9 wrote 7 actually 8, 3.14 wrote 1
> actually 2, 4.10 wrote 3 actually 5, 4.4 wrote 3 actually 4, S2 wrote 1 actually 2) and 1 reverse
> deviation (7b wrote 6 actually 5), also 1.2 / 1.3 / 9.1 / S7 four parent entries originally don't
> carry that note (not counted in caliber). This difference **not this batch's introduction**, this
> batch doesn't change number for others' entries, only registers here.
>
> **Twelve. Residual Pending (This Round Registered, Not Fixed)**: ① `src/driver/arms.rs`'s
> "Decision B1" comment opposite to code fact after 3.4.6 (see "Seven" 6, `src/**` out of scope); ②
> 3.20.1 / 3.20.2 two existing false positives (after fix must restore `weak.md` / `test.md`'s
> natural writing and regenerate baseline); ③ 0.5.2 (pre-commit / CI wiring + 2600 file actual
> measurement calibration + `test-specification` detailed section); ④ #489 ureq 3 migration
> (dependency surface, ignore with hard removal condition); ⑤ 3.8 (native to user function name
> shadowing) and 3.12 (runtime pattern executor configuration propagation) pending ruling, 3.18
> prerequisite waiting P6——**all not in this team's scope**.
