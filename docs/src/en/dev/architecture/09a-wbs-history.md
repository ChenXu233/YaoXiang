# WBS Historical Notes (Implementation Record · Audit · Ruling Trail)

> This document is the **historical notes** for [09-execution-wbs.md](./09-execution-wbs.md)
> (construction task table): implementation records, audit findings, ruling trails and errata for
> each phase. **Division of labor**: 09 only describes the **plan** (kanban, three-level task table,
> dependencies, risks, acceptance); this document only records the **history** (what was done, what
> was found, how it was ruled), appended chronologically, without rewriting history.

## P2 Establishing Equivalence Criterion Baseline

> **P2 implementation (2026-10-06, 2.1)**: verify.rs dual mode + 7 invariants + all corpora green.
> **D38 prophecy fulfilled**—the green-running process actually measured and fixed three real
> defects in ir_gen (see 06 §G6 for details): method locals table truncated to parameter count
> (local_count underestimated, runtime tolerates via Frame::set_slot), unannotated parameters
> dropped by filter_map (three places, params metadata misaligned with frame layout), match result
> slot relies on frame implicit Void initialization (now explicitly pre-initialized). The actual
> semantics of jump target is flattened instruction index (not the label-keyed form in 07 text), CFG
> built at instruction level—recorded in verify.rs file header. **Commit comparison**: ir_gen fix =
> `a9ea199b`; verify main body = `d5b18390` (⚠ parallel stream COMMIT_EDITMSG collision caused its
> message to be mislabeled as "docs(rfc): RFC-027a…", actual content is verify.rs + six test files,
> buried in history middle and unamendable, hereby recorded).
>
> **P2 narrow portion implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> stop-bleeding channel preconditions unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; judgment contract uses the same `TestFileSpec` as single-file
>   corpora (single point on library side), binary location / subprocess spawn abstracted to
>   `tests/common/` and yx_runner migrated simultaneously (prohibition three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. Trigger source uses the `SumUpTo(3, r)`
>   form nailed by rfc027 tests (return position/call site/binding position)—02's `Sorted(3)`
>   annotation example is substituted and evaluated in current code directly Disproved, does not
>   show Unproven silence, 07's pseudo-code Sorted(3) is illustrative only. Red state measurement
>   data see test file header.
> - The two red criteria are hung with `#[ignore]` as permanent residents per 2.4.5 red skeleton
>   precedent: default suite all green, `-- --ignored` reproduces red state; **P3 fix lands when
>   attribute is removed to turn green**.
> - The reason for deliberate deviation from 07 (code, file, line) table in comparison normalization
>   (excluding file/line, comparing only error code set) is written in 2.4.1 test comments.
>
> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **moved out of P2**: the `Program` / `Obligations` types they reference don't exist until P4,
> cannot compile at P2. The former is P4's 4.1.4 (no double registration), the latter's `#[ignore]`
> red skeleton merged into 4.3.1.
>
> **2.3.3 implementation (2026-10-07)**: C4 mandatory coverage checklist item-by-item verification
> (07 §143), 10 categories of semantic corpus layers fully covered and all within
> corpus-parity.jsonl diff baseline (26 representative files 26/26 hit, grep
> evidence)—ref/borrow/move entire 05-ownership directory (ref_shared,
> borrow_immutable/mutable/return, move_basic, ownership_deep etc.), closures closures +
> closure_arg_inference + spawn_capture, currying curry_value_fix + curry_tail_expr, spawn
> 04-concurrency ×13, iterators for spawn_for etc. 16 files, sum types sum_type_* +
> pattern_or_guard + match_call_scrutinee, existential coercion point interface_dynamic_dispatch
> (Vec(Animal) existential type heterogeneous packaging triggers existential_coercions) +
> interface_rebind_dispatch, `?`/Try propagation question_propagation + option_try + try_methods +
> user_try_type, method overload method_overload (+ method_overload_ambiguous_err in diagnostic
> diff), refined constraint Drop sequence refined_* ×5 + 05-ownership driven ReleasePlan (span key
> contract has another 2.4.4 red criteria permanent), performance baseline `benches/pipeline.rs`
> checked in (07 §145 three smokes two of three): `corpus_compile_frontend` full corpus compilation
> first measurement 17.6s, `cli_cold_start_hello` process cold start 169.7ms; interpretation
> throughput covered by benches/lib.rs hotpath group (interp_fib_recursive_27 /
> interp_loop_steady_10m), no duplicate construction. Regression gate check-perf-regression.sh falls
> to P9 unified list (2.5).

## P3 Fixing Correctness Vulnerabilities (Minimal Solution)

> **Error type labels for landed rows** (retrospective labels, not altering history): 3.1 = B ｜ 3.2
> = B (defensive empty arm, criterion turn green carried by 3.1) ｜ 3.3 = A ｜ 3.4.1 / 3.4.2 = A
> (new W code) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 / 3.4.7 = D (user-visible
> surface, already user-ruled) ｜ 3.4.8 ①② = C (main) + A (incidental three places) ｜ 3.5 / 3.6 /
> 3.7 / 3.8 = D (all user-ruled).
>
> **P3 diagnosis (2026-10-12, user request)**: A diagnostic accounting for the perception that "P3
> entries keep growing".
>
> **First, clarify the scale (the perception is amplified by two accounting defects)**
>
> 1. **Number collision**: `3.5.x` is shared by two task families—(a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 three-level: lexical and parser branches / AST `is_pub` / checker
>    side / dead code exemption convergence / peripheral echo / std and diagnostic text / language
>    reference convergence, all checked); (b) 2026-10-10's registered **3.5 Higher-kinded type
>    parameter call path completion** (3 three-level). Both share `3.5.x` ⇒ cross-reference
>    ambiguity (**renamed per user ruling on 2026-10-12: 029g family changed to `029g.x`**), and
>    029g's rows were miscounted into P3. **Pending ruling**: rename 029g family to `029g.x` (or
>    move to its phase), and synchronize other references in the document.
> 2. **Missing checks**: `3.7`'s 4 three-level (3.7.1–3.7.4) all completed (d922a551 / 38bd4b9e) but
>    not checked ⇒ this entry corrected now.
>
> **Two, P3 actual accounting (12 secondary rows)**
>
> | Row                                                                   | Status                                                                                                                                                                             | Disposition suggestion                                                                                                                  |
> | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                       | Completed                                                                                                                                                                          | —                                                                                                                                       |
> | 3.4 Silent discard point supplements (8 three-level)                  | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (precondition 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (precondition 4.3.1 not started) | 3.4.6 **now doable**; 3.4.4/3.4.5 unlocked with 4.3.1                                                                                   |
> | 3.5 Higher-kinded type parameter call path completion (3 three-level) | Not started (declaration side active, call side broken)                                                                                                                            | Keep in P3; **does not block main refactor**, but belongs to "accepted features unimplemented", higher priority than pure cleanup items |
> | 3.6 Function type value call path (1 three-level)                     | Not started (semantics already cut (i))                                                                                                                                            | **Keep**: correctness defect, criteria clear                                                                                            |
> | 3.7 init segment call name qualification (4 three-level)              | **All completed**                                                                                                                                                                  | Check and close this time                                                                                                               |
> | 3.8 native shadowing (1 three-level)                                  | Not started                                                                                                                                                                        | **Stuck at decision** (define name resolution priority first), not a scheduling issue                                                   |
> | 3.9 / 3.10 / 3.11                                                     | Completed (2026-10-12)                                                                                                                                                             | —                                                                                                                                       |
> | 3.12 Runtime mode executor configuration propagation (1 three-level)  | Not started (D class, user ruling B: form separate team)                                                                                                                           | **Suggest move out of P3 → P8 (execution layer)**, pending ruling                                                                       |
>
> **Four, 4.3.1's lever (main line waypoint)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the precondition for **3.4.4**, **3.4.5** and **4.5.1** (cross-layer contract
> PlanId-ization) ⇒ taking it once advances the main line and clears two P3 stranded items. 3.5
> (HKT), 3.6 (function type value call) and 4.6.1/4.9.1 have **no unsatisfied preconditions**, can
> be done at any time; 3.8 is stuck at semantic decision (user-defined priority vs explicit
> diagnostic).
>
> **Three, mechanism suggestion (preventing recurrence)**: Newly registered secondary/three-level
> rows **must carry two fields**—"Belonging phase" (which P it truly belongs to) and "Whether it
> blocks the main line"; `registered pending scheduling` is no longer a default state, unnoted is
> considered unschedulable. This way expansion is visible and sortable, not undifferentiated
> accumulation. **Pending ruling**: whether to write this item into the registration convention of
> this document and `src/dev/coding-rules.md`.
>
> **P3 implementation (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + unique getter
> (recorded as `pub(crate)` at the time; **2026-10-07 verified actual landing as `pub(super)`**—the
> `#434` ruling referenced in code comments has no entry on docs side, see 3.4.3; full private
> requires rewriting 11 construction literal sites, not taken during stop-bleeding phase; P4
> Obligations migration will close); execution mechanism abstracted to
> `src/frontend/proof_execution.rs` (prohibition three single point—cannot be placed in typecheck/,
> that would create L3→L4 reverse dependency); 3.1.2 per 02 evidence chain step 5 completed **four
> entries** (WBS row number missing compile_project `:99`, per 3.1.1 acceptance "criterion turns
> green" necessary to complete and register); 3.2.1 empty arm changed to bookkeeping (proof_calls
> throw up) + diagnostic (`into_result` established conversion path); 3.3.1 SOLVER slot
> Option-ified, Z3 missing degraded conservatively per `SMTResult::Unknown` (E2031 family diagnostic
> replaces process crash). Two vulnerability criteria removed `#[ignore]` to turn normal green,
> supplemented corpus fixture `proof-obligation-honored`; check-fanout inventory migrated
> accordingly (wiring point 19 = inventory). Measured: criteria 2.4.1/2.4.3 turned green, full suite
> lib 2494 / integration 272 / corpus two layers all green.
>
> **3.4 audit (2026-10-07, P3 post-implementation review)**: For the class of defects "failure
> produces no signal" (definition see 02 §41-52), re-inventoried on unchanged code, found 8 more
> places, all registered as 3.4.x in above table. Method and evidence:
>
> - **Consumer-side inventory** (16 obligation fields grep full repo read sites one by one):
>   `variant_ctor_calls` **zero consumers**—the only read `checker.rs:1468` is "self-injected into
>   `TypeCheckResult`", IR side uses form detection `ir_gen.rs:7374`; `rfc011b.rs:659/688` only
>   asserts producer side, exactly the form criticized in 02 §186 "only tests producer side,
>   naturally immune to consumer side defects" → 3.4.4. `method_overload_ir_names` result-level
>   field likewise zero external consumers, IR uses parallel `overload_resolutions` → 3.4.5.
> - **Phase coverage review**: 02 §91 table row 1 (four entries "no proof_execution") fixed by
>   3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 state unchanged (multi-file `run` still
>   doesn't output W1003, doesn't run dead code family) → 3.4.6; LSP non-project-internal path (02
>   §77 "manual lex→parse") likewise doesn't run proof → 3.4.7.
> - **Same-form empty arm scan** (`Unproven` full repo consumers only four places): checker three
>   refined branches (3.1 covered), 3.2.1 fixed ownership arm (**source unreachable**), and
>   `environment.rs:422` (**reachable, zero diagnostic, zero bookkeeping, and full repo docs zero
>   entries**) → 3.4.1, the only "all-unknown" living vulnerability in this audit.
> - **Solver acquisition side**: `default_solver()` three consumer forms vary (`predicate.rs:32`
>   process singleton / `checker.rs:1266` inject per `check_module` / `ownership.rs:621` create new
>   per backedge judgment)—3.3.1 only eliminated panic one; the **silent surface** of the other two
>   is 3.4.2, **singleton-ization and cache hit** goes to P4's 4.4.2 (no double registration).
> - **Not counted in 3.4 with existing attribution**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys unconsumed, Drop silently lost) still permanent P2 red state, attribution
>   4.5.1/D20 unchanged; 4.4.1 (phase order declaration opposite to reality, `equivalence` layer not
>   in pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) already in P4.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 no P4 preconditions, can land in place at any time
> (suggest 3.4.1 first—the only "full repo zero entry" living vulnerability, smallest change
> surface); 3.4.4–3.4.8 follow P4 execution, their criteria depend on obligation ledger (4.3.1) or
> unified Driver (4.2.x). **3.4 does not create new criteria system**, all reuse existing C2/C3 and
> P9's 9.2 gate.
>
> **Batch 3.13–3.16 record (2026-10-13 registered → 2026-10-15 closed)**: User's 2026-10-14 ruling
> merged four correlation minor items (#451 method binding panic / #452 named actual parameter in
> resolution / #453 dead inference entry / #454 native export named actual parameter—the last item
> is boundary finding when verifying 3.14, same day proven **not introduced by #452** via
> attribution experiment, user ruling ① merged into this batch) into one team for processing.
> Closure status: all four items completed, ruling registered as RFC-039 **D60** (six binding
> rules + shared criteria single-point-ization).
>
> **One, Two contract corrections (lesson: contracts themselves can be wrong)**
>
> 1. **Acceptance fixture self-contradiction**: #452 task contract simultaneously required "mixed
>    actual parameters output 20" and "position and naming contending same position error", but the
>    nailed fixture `Temp.pick(self, a)` has only one value parameter—the same actual argument
>    combination is both "mixed" and "conflicting". First implementer honestly reported blocker, set
>    task failed, overall revert (not trading silence for green light). Disposition: task
>    reassigned + contract revision—given **six binding rules** and **8 value-level matrix**
>    (fixture changed to two value parameter `pick(self, n, a)`), attempt 2 completed by second
>    implementer. Six rules full text verbatim incorporated into D60.
> 2. **inScope missing mechanical essentials**: #452 contract inScope listed
>    `docs/src/rfc/accepted/013-error-code-specification.md`, **missed generator-regenerated
>    `docs/src/reference/error-code/`**. E1110 registered only ran `code-tables --fix`, **didn't
>    re-run** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth red ("source registered/emitted
>    but document missing 1: E1110", source 154 / document 153, run **38518624964**—the only CI red
>    line in this batch). Fix = mechanical regeneration (8 files +83/−70, only touched
>    `docs/src/reference/`), reconciliation 154/154, CI 38520209026 green.
>
> **Two, Review → fix → re-review two closed loops**
>
> 1. **#452 line**: round-1 review needs_revision (F1–F5; F1 = arity guard `named_args.is_empty()`
>    not narrowed with resolution ⇒ named-actual-parameter-containing calls in multi-candidate
>    zero-hit/single-candidate hit two path full chain silent) → fix round added external code
>    **E1111 (argument_order_invalid)** (code table + locales + RFC-013 + generator four-piece suite
>    in same commit; code count 154→155) → round-2 review pass.
> 2. **#454 line**: implementation (shared parse entry + native arm by-name settlement) →
>    independent verification PASS (value-level matrix 12 cells, corpus 342/342, 5 mutations via
>    `scratch.py trial` mechanical reversible replay all caught, restoration byte-by-byte
>    consistent) → round-1 review needs_revision (F1 = typecheck native arm **inlined copy** of
>    binding criteria, missing rule ④ ⇒ method and native same-form call **ruling
>    divergence**—method reports E1111, native silent exit=0; ir_gen has another third existing
>    duplicate reporting old code E1015) → convergence round consolidated criteria into single
>    implementation `bind_args_to_params` (`arg_binding.rs:35`, four call sites
>    `expressions.rs:2727` / `:5416`, `ir_gen.rs:7776` / `:8279`) → round-2 review caught
>    **regression** introduced by convergence round (IR convergence block lost native omittable
>    position ⇒ originally successful program with omittable position mistakenly reported E1010) →
>    fix round (omittable position criteria and parameter name from same `signature.rs:236` unique
>    exit) → round-3 review PASS (independently reproduced red→green; mutation nailed fixture FAILED
>    proved criteria can fail). **Lesson**: review must cover "unmodified branches of similar paths"
>    (this batch's two needs_revision were both probes all landing on hit surface); fix round itself
>    can also introduce regression (omittable position), caught by next round's personnel change
>    review.
>
> **Three, Process events (faithful registration, not pointing to individuals)**
>
> 1. **External whole-tree discard (2026-10-14 around 17:14)**: Working tree was externally restored
>    in silence—implementer's fix in-progress (typecheck/ir_gen/code table/locales/RFC-013 etc.) all
>    disappeared, **at the same moment the two review documents edited by the user themselves also
>    disappeared from git status** ⇒ judged as repository-level discard (most likely user-side
>    manual cleanup/IDE "discard all changes"). Three evidence excluded team members: stash empty,
>    no new commits, reflog no reset. Implementer replayed in-progress artifacts from context and
>    pushed; discipline changed to "**first save patch to scratch, then sample red state**".
> 2. **Verifier context exhausted personnel change**: Independent verifier verifying 3.16 ran out of
>    room midway (no closing message), personnel changed per load rule—successor continued with
>    ready products (green state result, corpus equivalence script, mutation plan), verification
>    finally PASS. Positive precedent for partial product handover.
> 3. **Round 5 review rotation and historical rounds sent back**: When rotating review per load cap
>    item ⑤, `remove_member` sent back the already-ruled terminal review/verification rounds
>    **together to shared pool**, scheduler immediately reran (t3/t8 were re-claimed by new members,
>    duplicate work and blocked downstream). Disposition: team leader took over and cancelled
>    sent-back historical rounds (t3/t8/t13), based on which added AGENTS.md item 12 (handle
>    sent-back rounds immediately after personnel change). This batch closure record registers this
>    event per that item.
> 4. **scratch governance (about 6.3 GB)**: Early batch had one external tree per task, 6
>    directories scattered outside repository (about 780 MB, cleaned) + two members' external
>    verification trees (about 2.2 GB / 3.3 GB, self-deleted after use). Produced governance tool
>    `scripts/dev/scratch.py` (init/use/build/release/**trial**/status/rm/gc) and AGENTS.md item 9
>    (batch tree unique entry), item 10 (verification environment six rules: mechanical reversible
>    mutation / line number from referenced revision / CI unique gate must run etc.), item 11
>    (writer load cap), item 12 (handle sent-back rounds after personnel change) and top meta rules
>    two items (no private content / verify before assert).
> 5. **Contract defect (not pointing to members)**: #454 fix round's contract inScope only circled 2
>    files, actual mechanical essentials 8 (item 6 is exactly to prevent this—contract template
>    should preset mechanical essentials list); also that task's terminal state field was once
>    occupied by probe payload, formal acceptance/command evidence supplemented by team leader per
>    specification.
>
> **Four, Remaining items (not fixed in this batch, registered for reference)**: round-3 review
> three low-severity findings—① method path's `build_arg_slots` (`ir_gen.rs:521`) is still the
> **second copy** of criteria (existing, per last round's caliber registered at closure, not
> consolidated)—**2026-10-16 pre-release audit added three points**: (a) it has **two** call sites:
> `ir_gen.rs:7888` (method binding path, uses bound function `binding.function`'s parameter name)
> and `:7975` (overload resolution path, uses resolved compiled name leaf segment), original
> registration only wrote "`ir_gen.rs:521` area"; (b) it **has diverged from shared implementation
> `bind_args_to_params`**: missing rule ⑤ (duplicate naming → E1015), rule ⑥ (missing required
> parameter → E1010) and "omittable position" concept, and hardcodes receiver position to `1 + i`;
> two call sites uniformly `continue` for empty slots (comment writes "diagnostic goes to typecheck
> (it guarantees resolution passes so no empty slots)"), is a **dangling invariant written in
> comments, no assertion backstop**; (c) **RFC-039 D60's "all call sites consolidated into it"
> enumeration missed these two places**, is declaration not matching facts, and referenced line
> numbers not re-fetched on referenced revision, D60 text corrected with this item; ② batch
> verification tree (worktree) `cargo fmt --all` error is worktree workspace artifact, gate list
> should note fmt uses main repo; ③ repair-type contract template should preset mechanical
> essentials into inScope (see 3.5).
>
> **Five, Final gate (closure revision, local two implementation commits + one document commit
> unified push—the only push in this batch)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** /
> `--test yx_runner` 2 passed / check-obligations **PASS (13·13·0·8)** / check-docs-truth exit=0 /
> corpus_probe **342 lines byte-by-byte consistent with baseline** (only regenerated line:
> `named_arg_duplicate_err.yx` E1015@14 → E1111@16, expected code and line number synchronized,
> manually reviewed) / IR snapshot zero drift.
>
> **Six, Run links (push-side evidence)**: 3.16 implementation push (cb45de1fc) → CI **38550504517**
> (8/8 job success, including Docs Truth + Obligation Gate) / Deploy Docs **38550504587**; #452 fix
> round (9ec7d89a8) → Docs Truth job 38545762349 (same run's MSRV/Test was cancelled by subsequent
> push per concurrency—cancel≠failure, later run covers); governance tool push → CI 38546373435 /
> 38561405117; WBS registration (3.16 row + table header 56→57 / 147→148) → CI 38528560364. Closure
> push's own CI / Deploy Docs run was supplementarily registered to task record after push.
>
> **3.4.8 ③④ implementation (2026-10-11, 8ecf56f93)**: ③ `CHECK_STAGES` tail added `Linking` (**pure
> merge**, whole block skips `is_bin_role` entry check—E3020 criteria still goes to
> `RoleClassification` to prevent double reporting; another hard reason: run path judges "has main"
> per merged IR, collection semantics "entry itself errors→no IR" would be misjudged as missing main
> and hard-abort) → `Monomorphization` (check form); `ir_generation_check` retains unit IR (**memory
> only**, for pure merge and mono consumption, no external / no write to disk / no snapshot—if not
> retained aggregate request is empty, mono false-greens with `NoObligations`); mono failure goes
> through per-file collection channel in Check (diagnostic to entry file + `failed_stages`, not set
> failure), MultiFile keeps `fail_with`; `into_outcome` changed to "first take collection then judge
> failure". ④ Empirically **zero production change**: full program BFS already covers cross-unit
> mutual recursion (pathological → E3005, well-formed three-layer chain executes correct value),
> delivered four groups e2e + one mono unit test as living criteria.
>
> **Verification chain** (all independent of implementation): t3 (③, 9 self-built fixtures + HEAD
> binary byte-by-byte cross-check), t4 (④, 10 fixtures + `max_depth` parameterized boundary), t10
> (F1 fix, 11 examples + dual binary comparison) all pass; t5 cross-form matrix (single/multi/check
> three forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review PASS; test count 2643
> → 2656 (only add no remove).
>
> **F1 (found during verification, D59 A/B class, same round fixed)**: In standalone multi-file
> check without project root, aggregate request name (`lib.f`) and unit IR definition key (`f`) are
> different sources → mono falls into "native no IR definition" silent `continue`, check reports 0
> error/exit 0. Fix: aggregate side added reverse lookup per each unit's **real definition key**
> (precise enumeration + char-for-char equality + ambiguity abandon, not suffix heuristic; project
> form not registered, existing path unchanged). **Errata (same day)**: This reverse lookup was
> **deleted** in subsequent regression fix; root cause solved by `module_key` one-duty-two-use
> decoupling (added `script_mode`) from naming caliber—IR definition key = IR call name = aggregate
> request name = `{unit key}.{name}`. **Regression record**: ③ implementation post-CI's
> `Check tutorial/guide code examples compile` red (258 blocks / 10 failures: 6 same-file same-name
> rebinding + 4 collisions with `<std/list>`, rootless check doesn't do qualification triggers
> `link_module_irs` collision check); fix = ir_gen split `script_mode` (qualified name ⊥ Script/Bin,
> E3023 gate changed to look at it) + `ir_generation_check` per unit always passes unit key +
> `.with_script_mode(rootless)` + Script top-level same-name rebinding "latter takes effect"
> (consistent with existing run path behavior); verification = document gate 258/0, three binary
> comparison (HEAD/③ tree/fix tree) three collision fixtures, check form E3005 not regression, E3023
> not opened.
>
> **F2/F3 (accepted)**: mono diagnostic to entry file entry, span can cross units (same caliber as
> run path `path=entry`); Check form holds one ModuleIR per unit (memory only, released after
> check).
>
> **Issues left for follow-up (not this round's regression)**: ① E3005's help text still writes
> "internal error; please report this issue", not matching resource protection semantics (precedent:
> `e3xxx.rs:47` once moved similar "user writing error" out of E3005)—registered pending scheduling;
> ② multi-file E3005's span rendering label misalignment (reproduced with HEAD binary,
> pre-existing); ③ request collection surface has **type parameter position dependency** (generic
> call with type parameter not in value parameter position doesn't issue instantiation request, mono
> records `NoObligations`, convergent mutual recursion is this form)—same surface as 3.5 (HKT call
> path), registered to that row.

## External Track RFC-029g: Delete `pub` and Auto-binding (#399)

> **P3.5 implementation (2026-10-08)**: Seven secondary tasks all completed, one commit for the
> whole batch (independently revertable).
>
> - **Measurement not matching plan (faithful registration)**: 029g.1.2/029g.1.3's acceptance
>   originally wrote "`pub f = ...` reports **parse error**"—actually after deleting `KwPub`, `pub`
>   falls back to ordinary identifier, old writing lands at semantic layer with **E1001 Unknown
>   variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specifically probing removed
>   syntax is anti-pattern", **do not add** migration-specific diagnostic; corpus fixture
>   `pub_declaration_err.yx` nails E1001, document (`syntax.md`) honestly writes this consequence.
> - **Design convergence (beyond RFC literal, needs review)**: `project_refs` and `cross_file_refs`
>   two fields after pub deletion have completely overlapping semantics (same "package-internal
>   reference union", prohibition one prohibits parallel representation) → merged into single
>   `project_refs`, `set_exempt_pub`/`set_cross_file_refs` two switches deleted. Behaviorally
>   Internal role's non-pub items changed from "file-internal reachability" to "reference pool
>   judgment" (relaxed, miss-reporting direction); Lib tightened from "pub absolute exemption" to
>   reference pool judgment; Script top-level bindings not referenced start reporting
>   W1001/W1002/W1004/W1005.
> - **Dead table cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports writeback all deleted (producer side feeds
>   table, full repo zero consumption).
> - **Test migration** (test count not decreasing): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub is recognized" assertion reversed to "pub is no
>   longer a modifier"; `passes/tests/dead_code.rs` 7 role semantic cases rewritten (assertion
>   reversed 2); `tests/integration/cli_e2e.rs` 8 role cases rewritten (`lib`/`internal` exemption
>   assertions reversed to report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` not in IR); corpus diff baseline only added 1
>   entry (`pub_declaration_err.yx` → E1001), command `cargo run --quiet --example corpus_probe`
>   regenerated then manually reviewed.
> - **Measured**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-layer Contract PlanId-ization (D20)

> **4.5 implementation record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, closure
> (registration / WBS check / gate / push) by t7. Precondition = 4.3.1 obligation ledger (red state
> archive 173 files / 533 keys).
>
> **One, Carrier ruling (A: `PlanId` travels with AST node)**: Added `PlanId`
> (`src/util/plan_id.rs`, `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX`
> represents synthetic node outside parser), issued by **unique allocator**
> `ParserState::alloc_plan_id`; carrier two places—`Stmt.plan_id` (statement identity) and
> `Expr::Call.plan_id` (call site identity), `Expr::call` is the only call constructor in repo.
> Rejected is "both sides separately derive same identity by span" position-coincidence form (D20's
> original defect).
>
> **Two, Two-level verification and review (members independent)**: Each functional commit paired
> with one independent verifier and one reviewer, **verifier/reviewer do not reuse implementer's
> evidence**: 4.5.1 = t1 implement / t2 independent verify (external tree + 5 mutations) / t3
> review; 4.5.2 = t4 implement / t5 independent verify (external tree + 5 mutations) / t6 review.
> Review time and reviewed revision both on immutable history (`git show` verifiable). **Provider
> rename caused t5 / t6 member change and reassignment**: original verify/review members couldn't
> continue due to provider rename, reassigned per orchestration convention and **wrote reason in
> record** (cross-round context retention priority, only change personnel when compression
> unavailable); after reassignment conclusion consistent with original round (PASS), no implementer
> evidence reused.
>
> **Three, Two defects found and fixed during implementation self-check (t1, both within inScope)**:
> ① Consumption exemption criteria was hung on `UnitState.ir.is_some()`—`check` form's Linking/Mono
> would take `unit.ir` away, real unconsumed in check form **silently passed** (mutation
> measurement: D41 red but CLI check exit 0); changed to read **ledger's own completion flag**, and
> added regression cases. ② Drop sequence drifted with process hash seed (partial order `sort_by` +
> `HashSet` iteration): snapshot criteria ran 106/120/114 line drift three consecutive times;
> changed to **total order sort** (same name takes max depth + name ascending) then ran zero drift
> twice in a row.
>
> **Four, Criteria and evidence numbers**: After 4.5.1 `test_release_plan_spans_consumed` removed
> `#[ignore]` permanent (floor `examined ≥ 173` + no whitelist), 172 IR snapshots regenerated same
> commit (programmatic review: added 1083 lines Drop + 86 lines jump index translation, zero
> deletion, no reorder), corpus 342 diff empty; 4.5.2 added `test_overload_resolutions_consumed`
> (difference set empty + evidence floor `examined ≥ 1` / `consumed ≥ 2`; measured examined=1,
> consumer entries 2, no resolution skip 210), resolution form 9 items nailed, `cargo test --lib`
> 2712 passed / 0 failed, IR snapshot **zero drift**. Mutation driven: consumer key offset one
> position ⇒ criteria red + real program `error [W1090] ... produced key not consumed` + corpus gate
> difference; restored to green.
>
> **Five, Semantic correction classification basis (D59)**:
> `Type.method(positional arg, name = value)` **silently gave wrong answer before change** (measured
> compile clean, run output `1` instead of `20`) ⇒ **C class** (old behavior not successful), fix
> takes "loud": after change same program compile time `error [W1090]`; "named actual parameter
> participates in resolution" terminal semantics is **D class**, registered in §3.14 with issue #452
> pending dedicated team. Also: `Drop` is no-op in interpreter
> (`backends/interpreter/executor/ops/control.rs` wildcard arm only `advance()`)—key migration
> doesn't change runtime behavior.
>
> **Six, F-1 / F-2 disposition (t5 independent verification found)**: **F-1 (medium)** no-binding
> consumer site (`ir_gen.rs:7766-7790`) unreachable in current corpus and test surface, key
> correctness has no red state falsifiable ⇒ per "known boundary" register into
> [02](02-stage-contract.md) §2 revision annotation fifth paragraph (retain reason + don't open new
> task); **F-2 (low)** pointer drift ⇒ `types.rs`'s `OverloadResolutions` doc comment changed to
> precise pointer (produced `inference/expressions.rs:4550`, consumed `ir_gen.rs:7697` / `:7772`),
> one small commit, not mixed into doc commit.
>
> **Seven, Closure (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumer line number (`:360` read / `:3103` use / `:1095` and `:1097` settlement flag); 07
> criteria table and 04-ssa defect class 3 changed to past tense; 09 checked 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG recorded one row behavior change; `check-obligations.py` and `check_tracking.py` green,
> CI and Deploy Docs green. **Closure discovered bookkeeping deviation**: t3 / t6 report's pointers
> `ir_gen.rs:357` / `:3102` / `:1096` differ from code fact by 1-3 lines (actually `:360` / `:3103`
> / `:1095`+`:1097`, due to 4.5.2 insertion)—code fact is authoritative, this note keeps record.

## P4 4.10.3 Standalone run and MultiFile Orchestration Unification (Half of 4.2.5 Ruling A)

> **Batch C record (2026-10-15 registered → 2026-10-15 closed)**: User's 2026-10-15 ruling C—this
> round only does 4.10.3 (same batch candidate 3.4.6 per B1 ([02](02-stage-contract.md) §Runtime
> Behavior Table "dead code family warnings don't enter compile path") held until after 4.9,
> precondition changed and noted); implementation form = **full single team** (user instructed no
> DAG, no team, single-session implement + self-verify). Pre-batch debts landed first: #455
> dependency announcement fix (rustls 0.23.46 + serde_yaml_ng, `cargo audit` from exit 1 restored
> exit 0), WBS registration debt (#456 registered as 3.17 + incomplete rows supplemented "belonging
> phase / whether blocks main line" field).
>
> **One, Red first then green**: Driver-level red criteria 3
> (`src/driver/tests/run_rootless.rs`—rootless merged IR must not carry entry_function (red state
> `Some("main.main")`), rootless top-level executable statement must not report E3023 (red state
> doesn't produce module), Bin control entry_function maintained (green state permanent)) + CLI e2e
> 3 nails (standalone relative import run (red state E5001), Script top-level statement execution
> with #413 hint two green state nails).
>
> **Two, Implementation surface**: CLI `run` uniformly goes through `compile_project` (deleted
> `run_file_with_diagnostics`'s project/standalone bifurcation); Linking's `entry_function` gated by
> `is_bin_role` (#413 T4—Script doesn't auto-call main); #413 hint keying changed from
> `module.source_files.is_empty()` to call site project probe (after unification ModuleIR always
> carries source_files, original marker invalid); `script_main_never_called` adapted to qualified
> name form (function `{key}.main`, global slot empirically same qualified name—IR evidence is
> authoritative, not inferred from comment).
>
> **Three, Existing gaps exposed by unification (same batch fix, A/B/C class—only fix failing
> programs to succeed, originally successful programs zero impact)**:
>
> 1. **Explicit type argument pre-specialized name** (`mk(Int)(5)`): ir_gen per D6.3 directly emits
>    specialized name (mono would strip unspecialized original), `resolve_call_target` three arms
>    (rename / aliases / dot prefix) don't cover `mk(int64)` form ⇒ call site dangling E6006. Fix =
>    that single point parser adds fourth arm (base name qualified, type parameter string kept
>    as-is). **Bin project same form reproduces (pre-existing defect, not introduced by this
>    batch)**.
> 2. **Script top-level `mut` assignment statement repeated slot allocation**: `extract_global_defs`
>    and `register_global_slots` count `counter = counter + 1` (same form as declaration Assign) per
>    statement ⇒ same variable allocated 3 slots, initialization writes to later slot but read uses
>    earlier slot (runtime Add type mismatch). Fix = two places merge by name (same criteria same
>    source).
> 3. **Same-module constraint generic operator dispatch** (RFC-011 §5.2): mono dispatch rewrite
>    produces bare name `Vec3.add`, merged table actually `{key}.Vec3.add`. Fix =
>    `qualify_deferred_method` deterministic candidate + table lookup validation (precise hit →
>    generic's owning unit key qualified hit → fallback bare name current state; not suffix
>    heuristic, same form as `resolve_generic_name`).
>
> **Four, Registration gap (not fixed in this batch)**: Constraint dispatch when **type and generic
> definition in different modules** still E6006 (rootless dual-file probe evidence; Bin same
> form)—root cause = MonoType user type name doesn't carry definition module, correct fix belongs to
> P6 type identity convergence; registered WBS **3.18** (belongs to P3, precondition pending P6,
> does not block main line), `qualify_deferred_method` annotation same surface.
>
> **Five, Test adaptation (honest registration)**: `tests/integration/multifile.rs` fixture
> supplemented manifest—what it tests is RFC-029 project orchestration (Bin auto entry) not Script
> semantics; `cli_e2e` one nail (top-level diagnostic variable name visibility) and
> `script_main_hint` fixture (7 cases) synchronized per qualified name form—production consumer side
> only takes merged IR, fixture directly drives same form, no need for pipeline product substitute.
>
> **Six, Final gate (closure revision 084eaf87b)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed** (+3 new nails) / `--test integration` **287 passed** /
> check-obligations PASS (13·13·0·8) / check-docs-truth exit=0 / check_tracking exit=0 / boundary
> three gates report-only / markdownlint 0 / pnpm docs:build 0 / IR snapshot zero drift / corpus
> diff baseline regenerated **12 lines**—line-by-line structured check: exit / stdout / diagnostic
> code all per item identical, only stack frame function name and variable name qualified by unit
> key (manual review passed); after regeneration `check-corpus-parity.py` exit=0.
>
> **Seven, Process events (faithful registration)**: This batch once during pre-commit hook running
> concurrently edited workspace document, was eaten by hook's stash rollback and that commit failed
> (concurrent form previewed in config comments)—discipline = don't touch workspace during hook
> running; #455's commit therefore split into two attempts (second all green).

## P4 4.6.1 LSP Semantic Data Pipeline Unification (D55 Closure)

> **4.6 batch record (2026-10-16 registered → 2026-10-16 closed)**: User's 2026-10-16 instruction to
> advance in order 4.6 → 4.9 → 4.4 → P5, do 4.6 first; implementation form = **full single team**
> (per last batch user instruction: no DAG, no team, single-session implement + self-verify). Task
> definition (09 three-level table): project-internal file's semantic data (SemanticDB) goes through
> orchestrator same pipeline, same source as diagnostic; cross-file reference carries `resolves_to`.
> Acceptance: project-internal cross-file jump hits definition; semantic data and diagnostic from
> same compilation.
>
> **One, Current state empirical (verify before assert)**: didOpen/didChange triggered **two
> passes** of compilation—diagnostic goes through unified Driver pipeline (4.2.3/4.2.6 already
> unified), semantic data goes through `src/lsp/server.rs`'s manual
> lex→parse→`TypeChecker::new(uri).check_module_collect_all` (no module registry ⇒ project-internal
> `use` resolution fails, cross-file reference event not produced, jump/rename/reference lookup
> silently falls through for imported symbols—D55 registered silent failure). Orchestration arm's
> checker uniformly named `"<module>"` (SemanticDB file identity lost), `type_result` (containing
> semantic_db) was discarded when diagnostic non-empty and never externally sent; `checker.rs`
> emptying always overwrites `resolves_to`'s DefId file identity to current file;
> `Export`/`ModuleInfo` both have no definition position surface.
>
> **Two, Red first then green**: LSP e2e nail (`src/lsp/tests/cross_file.rs`)—temp dual-file project
> after didOpen main.yx, on `add_one` call site send GotoDefinition, **before fix returns None (red,
> empirically silent failure above)**, after fix hits lib.yx line 0 definition (green). Process once
> probe positioning: compiler side link (driver probe dump) already fully connected but e2e still
> empty—root cause is `resolve_reference` matches exactly by reference **start point** (D55
> precise-only current caliber), test cursor lands in identifier middle; this tolerance is existing
> behavior surface, this batch doesn't touch (see seven).
>
> **Three, Implementation surface**: (1) `Export.definition_span` (extract_module_info harvests from
> AST identifier node, same source same value as typecheck definition event's `def_span`; native std
> always None) + `ModuleInfo.definition_file` (Registry arm fills per DiscoveredFile, including
> vendor; std always None); (2) checker reference emptying via `import_def_locations` (use line
> binding span → definition file+definition span) redirects cross-file imported items' `resolves_to`
> out of this file; std import no registration, maintains falling to use line; (3) checker module
> identity changed from unified `"<module>"` to unit file path (empirically `env.module_name` all 4
> consumer points zero diagnostic consumption); (4) `DriverOutcome.semantic_db` channel—CollectAll
> form (Lsp target file **regardless of diagnostic success/failure** + SingleFile fallback)
> converges in, Lsp arm tail injects non-target file top-level export definition surface from
> registry (`external_definitions`, unopened files can also jump; type/submodule exports not
> injected—typecheck side type registration doesn't produce definition event, two sides maintain
> consistent gap); (5) `orchestrator::analyze_source_in_project` (`check_source_in_project`
> redirected to take diagnostic half); (6) LSP `analysis.rs` single compilation entry replaces
> manual pipeline (`update_semantic_db` deleted), World key uniformly normalizes file URI (`uri.rs`:
> `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
> differences normalized to same key, cross-file DefId therefore stable hit); 5 handlers +
> semanticTokens dual request connect to `db_key`.
>
> **Four, Fix along the way (D59 branch B class—old behavior silently did nothing, originally
> successful program external performance zero impact)**: ① project-internal file cross-file
> jump/rename/reference lookup changed from silent fall-through to hitting real definition; ② when
> lexical error old manual pipeline deleted semantic info (highlighting disappeared) → partial AST
> highlighting preserved (4.2.6 editor philosophy extension); ③ target file with type error no
> longer discards semantic data along with `type_result` (typing intermediate state
> highlighting/jump not lost).
>
> **Five, Nails**: Driver-level 6 (`src/driver/tests/lsp_semantic_db.rs`—target file path
> key+same-source diagnostic channel, cross-file resolves_to+external definition surface roundtrip,
> std import fallback to use line, error file preserved, single-file fallback path key, single-file
> error preserved) + LSP e2e 1 (red first then green) + uri normalization 4
> (`src/lsp/tests/uri.rs`—drive letter conversion, space encoding, idempotent normalization,
> non-file scheme passthrough).
>
> **Six, Mutation verification (scratch.py trial, mechanical reversible)**: Batch tree wbs-46 @
> closure revision 39546020, two mutations both made hit nail turn red, after restoration blob
> consistent with HEAD + git status CLEAN—m1 removed `import_def_locations` registration
> (statements.rs) ⇒ cross-file driver nail + LSP e2e nail red, other 7 green; m2 removed
> `external_definitions` injection (arms.rs) ⇒ same two nails red (roundtrip resolution lost
> anchor), other 7 green. Batch tree gc'd (freed 4.8 GB, `status` no batch tree).
>
> **Seven, Final gate (closure revision 39546020)**: fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed** (+11 new nails) / `--test integration` **287 passed** / corpus
> diff **byte-by-byte zero drift** / IR snapshot zero drift / check-obligations PASS (13·13·0·8,
> `semantic_db` non-obligation reason consumer anchor synchronized server.rs:482 and 02 §Obligation
> Field Table) / check-docs-truth 0 / docs-examples 258 items 0 failures / docs-orphan 0 /
> markdownlint full repo only remaining user uncommitted 032 two MD056 (user's own editing, not
> touched) / pnpm docs:build 0. **Process event (faithful registration)**: en/09a-wbs-history.md:54
> one MD037 was introduced by auto-translate bot retranslating last batch 09a (zh original has no
> space), this batch fixed on the way—markdownlint red not this batch's change surface but same file
> as 09a, registered together. **Remaining observation (registered pending discussion, not
> touched)**: `resolve_reference` only matches exactly by reference start—when cursor lands in
> identifier middle jump/hover/rename silently does nothing (D55 precise-only current state, full
> file form isomorphic), if tolerance matching needed is independent behavior change, report to user
> for ruling.
>
> **Same day continued batch (4.6.2, landed after user ruling)**: After delivery of wrap-up report,
> user on "remaining observation ①" ruled "OK, then change"—lookup key tolerance implemented per
> plan A: definition/hover/references/rename four handlers' lookup key changed from cursor position
> (via UTF-16 conversion) to use `find_identifier_at_position` already located **identifier's own
> start** (same coordinate system as semantic event, no conversion); `resolve_reference`'s start
> exact match semantics unchanged (D55 precise-only unchanged). references/rename's definition point
> fallback judgment (start <= col < end) is isomorphic for identifier start. Red first then green:
> definition middle cursor nail first red (None) then green; four handlers middle cursor each 1
> nail + cross-file e2e start/middle dual position assertion, `--lib` 2748 (+4) all green, clippy -D
> warnings 0. Registered WBS 4.6.2 (three-level count 150→151).

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 1 Closure (4.9.1 / 4.9.2 / 4.9.3)

> **4.9 batch record (2026-10-16 registered → 2026-10-16 closed)**: User's 2026-10-16 ruling to
> merge three related three-level tasks **4.9.1 / 4.9.2 / 4.9.3** into one team one round (reason:
> they belong to "pool's source and identity" side, merged landing avoids `collect_project_refs`
> deletion intermediate state repeated rewriting; 4.9.4 / 4.9.5 belong to consumption refinement and
> test pool separation, depend on 4.6.1's `use` path resolution, forcibly same batch would expand
> gate surface). #450 attribution **pending ruling**—evidence shows it belongs to "overload
> resolution name not passed back to reference count" class, still reproduces under round 1 landed
> form, registered per independent WBS entry disposition (team leader recommends A: merge into round
> 2 then register as independent entry; **pending user ruling**). Frozen rev = `3863a9ff0` (parent
> commit `62f6e1a59` is pre-fix baseline); verification PASS attempt `aab92fe0` (i.e. t2); review
> pass attempt `96452696` (i.e. t3). **Closure registration commit = `38d30b846` (docs(docs): 4.9
> reference pool round 1 closure registration)**—this commit actually written and pushed by closure
> member integration-refpool; its closing probe once mislabeled closure task as terminal failed and
> overwrote output, team leader took over new attempt, changed back to completed per verified
> delivery facts, hereby corrected attribution and label. Evidence archive: evidence summary (8
> mutation patch list, `warn_compare_v8.json` caliber and conclusion) inlined in this section
> "Three, +8 and +16 two counts correspondence" and "Four, mutation experiments"; original files
> recycled with workspace, **not checked in, must not be used as document reference source** (AGENTS
> meta rule ①).
>
> **One, Merge ruling**
>
> 1. **The ruling itself**: Three related three-level tasks merged into round 1. **4.9.4
>    (consumption refinement) / 4.9.5 (test pool separation) next round**—former depends on 4.6.1's
>    delivered `use` path resolution (already landed in `39546020`, can start); latter with RFC-029f
>    table acceptance same batch, registered RFC-029f last section to be changed.
> 2. **Reason**: (a) 4.9.1 / 4.9.2 / 4.9.3 are mutual preconditions (4.9.2 changes pool type → 4.9.1
>    assembly surface needs rewrite → 4.9.3 has denominator-judged role table), splitting rounds
>    inevitably write intermediate state then change twice; (b) all three belong to "pool's source
>    and identity" side, 4.9.4 / 4.9.5 belong to another side, batch split is by "cohesion" not
>    "independently revertable" cutting.
> 3. **Merge result**: Single commit `3863a9ff0` (one line `:recycle: refactor(driver)`), single
>    push, single batch gate. **Hash explanation**: Before push `git pull --rebase` merged
>    auto-translate bot commit (`72f364169`); rebase landed after the other party's commit, this
>    batch's two commits (implementation `3863a9ff0`, its pre-fix baseline兼 other party document
>    commit `62f6e1a59`) **not rewritten, kept original hash**—initial version mistakenly wrote here
>    "`3863a9ff0`→`3863a9ff0`" identity mapping (looks like "rewrote to same value", actually "not
>    rewritten"), corrected at 2026-10-17 closure; other party code commit `33f14e8ab`→`b122b7ad1`.
>    **External references uniformly use post-push hash** (rewritten old hash unreachable in new
>    clone, sameness determined by patch blob comparison).
>
> **Two, Five criteria red/green measurement caliber**
>
> | #   | Criterion                                              | Red state (pre-fix / P1 directory scan regression patch)                                                                                                   | Green state (round 1 landed)                                                                                                                                                                                                                                                                                                                                                                     |
> | --- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
> | ①   | Pool root = package target (#334)                      | Directory recursive scan ⇒ files under tests/ also enter pool, exempt production code unused definitions                                                   | `collect_package_refs` (`helpers.rs:481`) per `ExplicitSurfaces` root calls `discover_with_used`; full root degenerate falls back to `collect_compilation_refs` (`helpers.rs:454`)                                                                                                                                                                                                               |
> | ②   | Pool granularity = `(module, name)`                    | Same-name local variable (`let foo = …`) accidentally exempts same-name top-level definition (`fn foo()`); a.yx's `foo` and b.yx's `foo` cross-contaminate | `HashMap<String, HashSet<String>>` (key = module key); `collect_ref_pool` (`helpers.rs:429-449`) assembly unique entry; `arms.rs:603-609`'s `other_module_refs` takes "except this unit" all module entries union then sends to `DeadCodeAnalyzer`; module self-name (top-level definition and local binding, `use` imported name exception) no longer exempts other module same-name definition |
> | ③   | `FileRole::Script` doesn't use pool                    | Script affected by other project-internal files' references                                                                                                | `arms.rs:611-623`'s `match us.role` only for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`) assembles, Script/Test goes through `_ => {}` default arm (per file-internal definition-use graph judgment)                                                                                                                                                                   |
> | ④   | Fixture A: lib.yx line 4 definition must report W1001  | Pre-fix: reports `W1003@1` (actually "unused import"), not W1001                                                                                           | Red state measurement (red 1 fixture red1) reports `W1001@3`; green state all 6 fixtures PASS                                                                                                                                                                                                                                                                                                    |
> | ⑤   | Fixture B: main.yx line 4 definition must report W1001 | Pre-fix: reports `W1003@1`                                                                                                                                 | Same as above                                                                                                                                                                                                                                                                                                                                                                                    |
>
> **Criterion ② "deduction red"** (implementer self-stated + verifier closed): red2 fixture's
> `let foo = …` in helper.yx; old pool under bare name union treats `foo` as whole "used by helper",
> lib.yx's `foo` exempted. **Measured**: red2 fixture vs pre-fix binary → no W1001 (old caliber
> exemption); red2 vs post-fix binary → W1001@1.
>
> **Criterion ③** (Script doesn't use pool) red state fixture **first version failed**: first
> version fixture put same-name local variable in same file triggered E2010 (this language doesn't
> allow same-module shadowing top-level definition), moved to another module (helper.yx) for
> deduction red. This point t3 review and t2 verification both independently reproduced, **not
> contract defect**—same-module shadowing is this language's hard constraint, fixture must be
> cross-module.
>
> **Three, +8 and +16 two counts correspondence** (**t3 review's closure as authoritative**)
>
> 4.9 round 1 evidence summary section C writes `+16/−0`, but missed "**new fixture records**"
> component—v8 instrument (`warn_compare_v8.json`, 331 files symmetric diff, `only_before=0`)
> caliber **includes** new fixtures; t1 implementer's self-built "+8" is **warn-before / warn-after
> (each 325 items) aggregated by fixture** (each member file checked once). Review item-by-item
> mapping:
>
> - **+8** (8 new warnings, all `check <non-entry file>`, `check <entry file>` zero change)
>   - `cross-file-method/lib.yx`: `W1002@2(Point)`, `W1005@4(get_x)`, `W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`: `W1001@2(double)`
>   - `type-error-in-lib/lib.yx`: `W1004@2(value)`
>   - `use-constant/lib.yx`: `W1004@2(value)`
>   - `use-type-and-function/lib.yx`: `W1001@4(distance)`, `W1002@2(Point)`
>   - **Cause (unified attribution)**: These 8 fixtures all **didn't declare package target** ⇒
>     4.9.1 root degenerated to checked file's own compilation set, consumer main.yx not in it; this
>     is design-intended change (before, these files' check surface was already zero warnings, not
>     exemption path). Repo root `yaoxiang.toml` is empty file ⇒ same degenerate path empirically
>     holds.
> - **+16 residual 8** (check surface = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` +
>   `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`)
>   - These check surface files in t1's `warn-before.jsonl` have `exit=None` (capture time point
>     **doesn't exist**—this batch's newly added fixtures). So +8's "+ 0 − 0" diff doesn't include
>     them, +16's v8 symmetric diff does.
>   - `v8`'s `stray.yx` only has `W1001@2(probe)` remaining: reason is pre-fix binary also reports
>     `probe@stray.yx:4` (this fixture runs `probe` per file), **cancels out** leaving only lib's
>     1—three data sets (+8 / +16 / dual binary ABBA) mutually consistent.
> - **New fixtures 8** vs **+16 residual 8**: Same batch of files (4.9.1 / 4.9.2 added
>   `ref-pool-compile-set/` and `ref-pool-module-identity/`), instrument treats their each check as
>   independent record—same file may trigger different diagnostic entries for multiple
>   `role (role) / context`.
>
> **Conclusion**: **+16 = +8 (aggregated by fixture) ∪ new fixtures 8**—both counts are correct,
> **different caliber**: t1's "added 8" is fixture-level caliber, t2's "+16" is file-level caliber
> (symmetric diff includes new fixtures). t3 review ① +8⊆+16 already closed; **review ② wrote "don't
> copy EVIDENCE.md section C"**—section C missed "new fixture records" component, this item
> rewritten per t3 review's `pass` conclusion.
>
> **Four, Mutation experiments (mechanical reversible, all `scratch.py trial`)**
>
> 8 mutation patches (p1 directory scan, p2b use import name elimination, p3b script joining pool,
> p3 flat consumption, p4 std joining pool, p7final self-built 4 criteria, p8 std closing, p9 vendor
> joining pool + p_stack2 / p_stack3 stacking surface). Each trial output includes: HEAD blob →
> mutated blob → restored blob consistent with HEAD = yes + `git status` CLEAN. Local
> `core.autocrlf=true` needs `GIT_CONFIG_*` injection `false` (`trial_env.ps1`), otherwise patch new
> lines convert to CRLF causing apply/restore double failure—**only affects patch mechanics, doesn't
> change tested behavior**.
>
> **Five, Process events / environment lessons (must read for next round handover)**
>
> 1. **core.autocrlf injection**: Local `core.autocrlf=true` must inject `false` via `GIT_CONFIG_*`
>    to ensure patch mechanical reversibility; injection script at that time was recycled with
>    workspace (criteria listed in previous item, can be rewritten based on it). **Next round don't
>    reuse** any trial script without injection.
> 2. **Same tree trial serial**: `trial` and `cargo build` on the same tree **must be strictly
>    serial**. `trial`'s internal `git apply` and `cargo build` both touch worktree, parallel
>    produces "patch incomplete but seen by build" or "build and apply deadlock" two kinds of races.
> 3. **Trial rebuild binary needs sync confirmation**: After `trial` completes it rebuilds tree
>    internal binary (cargo's fingerprint includes source path, **switch tree = full rebuild**),
>    before subsequent measurement must confirm `bin/yaoxiang-rs` synced with HEAD. **This round
>    once used residual mutation binary to measure false data**, rebuilt and remeasured to correct.
> 4. **Second tree yx49-b's build reason**: Cross-comparison needs pre-fix binary (62f6e1a59), no
>    ready product within batch; `scratch.py init yx49-b` opened second tree (eat full build again).
>    **Tree released** (see six), delete leaves batch closure `gc` disposition.
> 5. **cli_e2e two Internal cases fixture supplemented to legal package (supplement `[run].main`,
>    assertion unchanged) reason**: Pre-fix `cli_e2e`'s two `Internal` role nail fixtures **didn't
>    declare `[run].main`** ⇒ after 4.9.1 landing `compile_project` goes through `ExplicitSurfaces`
>    root call path, can't find `main` ⇒ whole batch 2 fixtures behavior from "Script fallback"
>    drifted to "can't find entry"—is 4.9.1 introduced fallback behavior not defect. After
>    supplementing `[run].main` field two fixtures go through normal package path, **assertion**
>    (exit code / diagnostic code / warning set) verbatim unchanged.
> 6. **Second round tree deletion safe way**: After release use
>    `python scripts/dev/scratch.py rm yx49-b` (not `rm -rf`), will verify "no mount" then delete.
>    This batch before closure already released, next step `gc` natural cleanup.
>
> **Six, Batch tree state**
>
> Before closure: yx49 released (4.8 GB freed), yx49-b released. Two trees no mount, state can
> verify in `scratch.py status`. This task output end attaches `scratch.py gc` complete status.
>
> **Seven, RFC-029f "Known Defects" table flip**
>
> Previous version (2026-10-09) described in present tense "three design defects of reference pool"
> (including already deleted code locations like `collect_project_refs` / `orchestrator.rs:603-648`
> / `orchestrator.rs:393-396` etc.). This batch closure synchronized rewrite to "**Known defects:
> landed (2026-10-16, WBS 4.9 round 1)**": ① current pool root = `collect_package_refs`
> (`helpers.rs:481`), per `ExplicitSurfaces` parse, no directory scan; ② current pool type
> `HashMap<String, HashSet<String>>` (key = module key), assembly unique entry `collect_ref_pool`
> (`helpers.rs:429-449`), consumer side `arms.rs:603-609`; ③ current Script doesn't join pool,
> `arms.rs:611-623`'s `match us.role` default arm. **All file:line re-fetched on referenced revision
> `3863a9ff0`**. `collect_project_refs` and `orchestrator.rs` were moved to
> `src/driver/{state,arms,helpers}.rs` with P4 4.2.9 split (`c59fb751`), old positions in table no
> longer hit—this is "declaration not matching facts" fix (t3 review note ①).
>
> **Eight, Final gate (closure revision `3863a9ff0`)**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287
> passed / `yx_multifile_runner` all green / corpus baseline **only +2 lines** (two new fixture
> behavior runs, 338a339,340; another 342 lines byte-by-byte unchanged) / IR snapshot 204 zero drift
> / check-obligations PASS (13·13·0·8) + selftest / check-docs-truth 0. **Process event**: This
> batch once cargo build due to `core.autocrlf` converting script line endings to CRLF, team scratch
> path `yaoxiang-rs` launch reported `BadImage`, empirically code encoding inconsistent with git
> index—rebuilt and passed (parallel to 4.6 batch "external whole-tree discard" lesson, different
> cause).
>
> **Nine, Remaining (handover next round)**
>
> 1. **#450 attribution ruling**: Evidence shows it belongs to "overload resolution name not passed
>    back to reference count" class, **still reproduces under round 1 landed form**—dispense per
>    user ruling C flow. **#450 probe**: Dual binary same set `{W1002@2, W1005@4}` (measured,
>    rootless form), not directly related to round 1 change, but #450's fix may need to wait for
>    4.9.4 (consumption refinement) to do together—because root cause is "overload resolution name
>    passing reference count" needs use item's precise consumption surface, 4.9.4's "imported then
>    actually used" is same kind criteria's precise version.
> 2. **dead_code.rs:683 pointer nit**: Comment writes "two pool assembly caliber see driver side
>    collect_compilation_refs"—actually `collect_compilation_refs` is fallback path, main path is
>    `collect_package_refs` (`helpers.rs:481`). Comment not matching code main path caliber (**not
>    in this batch's writing scope**), leave to 4.9.4 / 4.9.5 landing to fix together.
> 3. **4.9.4 / 4.9.5 preconditions unlocked**: 4.6.1 (`use` path resolution) already delivered by
>    `39546020`; 4.9.4 can start immediately. 4.9.5 (test pool separation) needs to first define
>    RFC-029f table acceptance (already laid by this batch's "seven," flip).
> 4. **`yaoxiang check` multi-file `run` never reports dead code family warnings** (WBS 3.4.6,
>    2026-10-15 user ruling C held until after 4.9)—this batch round 1 didn't involve; next round
>    4.9.4 / 4.9.5 landing same batch processing.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 2 Closure (4.9.4 / 4.9.5 / 4.9.6)

> **4.9 round 2 batch record (2026-10-16 implement → 2026-10-17 closure)**: Covering 4.9.4
> (consumption criteria = imported then actually used), 4.9.5 (test pool separation), 4.9.6 (method
> reachability caliber, issue #450, user 2026-10-16 ruling A merged into round 2). Closure fix
> (verifier-side remediation M1/M2/M4 compilable redo + F1 fix round + PA-2 delete dead branch) and
> registration gap issue #464 (WBS 4.9.7) settled together.
>
> **One, Judgment chain and commits (all post-push true value)**
>
> | Step | Content                                                                                                                                        | Post-push commit | Pre-push local value | Judgment method                                              |
> | ---- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------- | ------------------------------------------------------------ |
> | 1    | fix(util): check explicit path not blocked by default exclusion table + --exclude always empty (issue #463, P0-0.4)                            | 366bc5a69        | ef959ca55            | Patch blob identical                                         |
> | 2    | Phase A = 4.9.4 + 4.9.5 (RefPool per import attribution + Test doesn't contribute consumption surface)                                         | e6cf1aeaa        | 84a6f6978            | Patch blob identical                                         |
> | 3    | Phase B = 4.9.6 (#450 per-candidate method reachability; including generated warning-codes.md)                                                 | ddcbb12d0        | cd629b5fe            | Patch blob identical                                         |
> | 4    | docs(meta): AGENTS item 10⑦ (commit period stash window convention)                                                                            | ec94fa9d8        | 9a4ed5f3b            | Patch blob identical                                         |
> | 5    | Fix round = F1 (overload sibling overload recovery report W1005) + F2 (verifier-side M1/M2/M4 compilable redo) + F4 (snapshot generation note) | b8380e6ab        | 3d9ef6eab            | Patch blob identical (36962f318e42…, git range-diff no diff) |
> | 6    | Closure fix = PA-2 (delete strip_unit_prefix dead branch + 029f/CHANGELOG three places inaccurate statement correction, register #464)         | b940300ac        | e13984465            | Patch blob identical (patch-id 9bda3cfe4c…)                  |
>
> **Hash explanation**: Each push's pre-push git pull --rebase rewrites local hash; above "Judgment
> method" column all "patch blob identical" (`git diff <rev>~1 <rev> | git hash-object --stdin` both
> sides consistent + `git range-diff` no diff). This article doesn't reference any pre-push hash as
> authoritative source.
>
> **Two, Three criteria red/green measurement caliber** (verifier self-built reproduce, not trusting
> implementer's criteria)
>
> | Criterion                  | Red state (before binary 366bc5a69 measured)                                                                                                                                           | Green state (ec94fa9d8 and after)                                                                                            |
> | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
> | 4.9.4 import≠consumption   | Declaration surface project check a.yx only 1 W1001 (main's use b.{foo} + foo() rescues a.yx same-name foo; guard only imported not used also rescued)                                 | Same fixture 2 as reported (same-name collision not rescued + unused import doesn't count as consumption); ref_pool 8 passed |
> | 4.9.5 test pool separation | src/util.yx's debug_print rescued by tests/ reference (only 1)                                                                                                                         | Same fixture 2 (test references don't spill over); ref_pool's test_ref_pool_excludes_test_role_references passed             |
> | 4.9.6 method reachability  | Self-built fixture (2 disambiguation overload calls + 1 uncalled) 3 W1005 false reports; method_call_args.yx 3 / method_overload.yx 1 (overload per full name coverage only reports 1) | Same fixture accurately reports uncalled orphan; method_unused 3 nails passed; corpus call_args 3→0 / overload 1→0           |
>
> **Fix round (F1) tightens again**: Unhit resolution's sibling overload recovers reporting W1005
> (b8380e6ab—same fixture call ->Float reports ->String sibling, call ->String reports ->Float
> sibling, bidirectional line number nail). **Fallback boundary (register #464)**: Cross-unit
> (registry parse) method call doesn't produce resolution entry, falls back name-level exemption,
> cross-unit same-name same-order previously not reported—register 4.9.7.
>
> **Three, Process events and lessons (four hard requirements corresponding)**
>
> 1. **PA-1 changed from blocker to existing capability gap (medium)**: Round 2 review once
>    registered "cross-unit same-name same-order not reported" as blocker (PA-1); subsequent two
>    fact reconciliations (implementer archive + verifier dual binary remeasure same fixture set on
>    b8380e6ab/b940300ac, diagnostic set byte-by-byte consistent) confirmed it's **pre-existing
>    capability gap before round 2** not this round's regression—mechanism = cross-unit method call
>    doesn't produce resolution entry, falls back name-level exemption; disposition = change to
>    medium, register issue #464, WBS 4.9.7 schedule next round.
> 2. **Reviewer publicly corrects before data and withdraws regression judgment**: Reviewer's
>    originally cited before data ("−42 items includes called methods") doesn't match its original
>    output (W1005 count=0)—verifier after reconciliation confirmed before capture is 0 items,
>    **reviewer's regression judgment publicly withdrawn** based on this; −42's correct caliber = 26
>    files' called methods false report elimination (Phase B), each traceable to behavior-run exit=0
>    file (method really called and executed).
> 3. **t13 terminated due to contract premise not holding (not implementation defect)**: That task's
>    acceptance premise wrote "cross-unit same-name same-order will report W1005 regression fix",
>    but measured (t14 dual binary) confirmed that behavior is already registered gap (#464) not
>    regression—contract premise doesn't exist, task terminated; its probe value carried by t16
>    read-only judgment (confirm `strip_unit_prefix` is dead branch).
> 4. **Document commit's real criteria is "really build once"**: Round 2 closure's document commit
>    (`042c07860`) made Deploy Docs red on VitePress's Vue template compilation—angle bracket
>    placeholders like `<rev>` / `<before-exe>` / `<after-exe>` etc. not in backticks, treated as
>    elements to parse (reported `Element is missing end tag`, pointing to `<rev>` line);
>    markdownlint can't catch this kind, and pre-commit's markdownlint hook in commit only
>    containing docs was judged `no files to check` skip (measured). Disposition: placeholders
>    uniformly in backticks, before push locally run `npm run docs:build` once (fix commit
>    `0c837e1b4` with local build exit=0 as evidence); if commit with `--no-verify`, commit-msg
>    check needs manual re-run.
> 5. **Contract description not synchronized process oversight**: t14's task description text
>    residual PA-1 old hypothesis (wrote "b.yx reports W1005" expectation), verifier in report
>    proactively pointed out "per contract field judgment, description text doesn't count" and asked
>    team leader to confirm—disposition = task contract field as judgment basis, description text
>    kept as process lesson (**reassign/dispatch task description must synchronize with latest
>    judgment chain**).
>
> **Four, Mutation experiments (mechanical reversible, all scratch.py trial)**
>
> Round 2 verifier total 9 patches (m1b/m2b/m4b are F2 post-compilable redo version—initial version
> M1/M2/M4 commented out if condition lines leaving dangling blocks, judged "compile failure
> misremembered as criteria turning red" by t8 review F2, voided; redo version per patch cargo check
> exit=0 + trial turning red). M3 (same-name attribution regression) initial version is real
> mutation, measured ref_pool 5 criteria turn red, keep original judgment. M1b anchor re-fetched on
> b8380e6ab (after F1 fix probe segment refactor, old anchor invalid). All trial: HEAD blob →
> mutated blob → restored blob consistent with HEAD + git status CLEAN. Environment injection
> follows trial_env.ps1 (core.autocrlf=false via GIT_CONFIG_*).
>
> **Five, Warning surface full comparison (caliber first)**
>
> Instrument = warn_surface_r2.py (self-built parser, (file, code, line, column) four-tuple
> symmetric diff, project state channel cwd=file parent directory, full path key); command =
> `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>` (337 files × 2
> binaries). **Caliber**: This round before capture already includes all fixtures ⇒ matched set =
> full set 337, no round 1's +8 (matched increment) / +16 (full set) dual caliber problem. Result:
> only_before = 42 W1005 (26 files, all called methods false report elimination = Phase B's −42; per
> file check all in behavior-run exit=0 baseline, method really called and executed—**no one really
> dead code rescued**) / only_after = 3 W1001 (ref-pool-import-use's a.yx:4 check surface ×2 +
> ref-pool-test-pool's ../src/util.yx:2 = Phase A criteria surface). **Snapshot generation (F4)**:
> Phase A time point's after capture (57 residual before Phase B) and final version (15+2=17 after
> Phase B) are **two generations**, must note when referencing; another 1 item exception
> (interface_member_conflict.yx's Dog.name@18) belongs to field access form, not in Phase B's W1005
> removal set, consistent before and after.
>
> **Six, Final gate (closure revision b940300ac)**
>
> lib 2761 passed / integration 288 passed / IR snapshot zero drift (snapshot 7 passed) / corpus
> baseline zero drift (byte count three-way comparable: baseline = b8380e6ab = b940300ac) / clippy
> -D warnings 0 / docs-truth exit=0 / check_tracking exit=0 / generator already run no product
> change. **Gate check arrangement (item 13)**: This batch push's CI / Deploy Docs / Auto Translate
> run conclusions handed to scheduled check link to register (run id + conclusion + consecutive push
> coverage: later run covers previous), not wait in session.
>
> **Seven, Batch tree state**
>
> Both yx49 and yx49-b released (uniformly deleted by gc at closure); no this batch added directory
> outside repo.

## P4 4.9 Reference Pool Correct Implementation (RFC-029f Fix) — Round 3 Closure (4.9.7)

> **4.9 round 3 batch record (2026-10-17 implement → 2026-10-17 closure)**: Covering 4.9.7
> (cross-unit method call resolution registration and receiver attribution, issue `#464`—WBS 4.9's
> last three-level task; after this round closure 4.9 whole secondary task kanban checked). Link =
> producer side t19 → independent verification t20 PASS → review t21 needs_revision (F1: injection
> surface false report) → fix round t24 → independent verification t25 PASS → review t26 PASS
> (incremental, verdict=pass). All commit numbers are post-push true values (judgment method = patch
> blob identical; `git diff <rev>~1 <rev> | git hash-object --stdin` both sides consistent).
>
> **One, Judgment chain and commits (post-push true value)**
>
> | Step | Content                                                                                                                                                                                        | Post-push commit | Judgment method         |
> | ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ----------------------- |
> | 1    | Producer side: cross-unit method call produces D20 resolution entry (`feat(typecheck)`, t19)                                                                                                   | `b8b3cc3f6`      | Patch blob identical    |
> | 2    | Driver side attribution: method reachability per definition unit gives mark, resolution-recorded name group disables pool's name-level exemption (t20 verify PASS / t21 review catch F1)       | `d5c00235a`      | Patch blob identical    |
> | 3    | Injection surface fix round F1: candidate injection per "name ← provider module" **pair-by-pair** filter, eliminate E1107 false report (t24, including F2 expression and F3 nail single point) | `d5dd1a37b`      | Patch blob identical    |
> | 4    | Nail test relocation: `src/frontend/core/typecheck/tests/environment.rs`'s F3 nail moved into declaration surface `src/driver/tests/overload_injection.rs`                                     | `c2f887843`      | Patch blob identical    |
> | 5    | dev top                                                                                                                                                                                        | `e9675c9dd`      | merge commit, see "Two" |
>
> **Two, Hash mapping and dev top two merges (source unconfirmed, not allowed to write member's
> action)**: Round 3's four commits (above table 1–4) all pushed to origin/dev;
> `git worktree list`'s main mount at `e9675c9dd`. dev top has two **non-team-command-surface
> produced** merge commits (`5da05b885` / `e9675c9dd`, reflog shows `pull --tags` + push form),
> source unconfirmed—per item 10④ (reference must be resolvable by anyone) and item 13 (gate handed
> to special check) same closure caliber, here only register fact and reflog evidence, no member
> attribution; also see "Five," lesson ② (push attribution convention must be verifiable).
>
> **Three, D20 contract extension actual form (key form and consumer point)**: Producer side
> packages "candidate / mixed IR name / declaration period parameter name" three parallel tables
> **same order** into `ModuleInfo.method_overloads` (`src/frontend/module/mod.rs:157`) with unit
> crossing, registry merges deterministically per module key ascending
> (`src/frontend/module/registry.rs:150`); typecheck arm injects per this unit's import face pair by
> pair (`src/driver/arms.rs:354` → `visible_method_overloads`, `src/driver/arms.rs:1521-1579`: name
> ← provider module pair by pair, same-name conflict full name skip, own key position not externally
> merged), nail single point `append_overload_group`
> (`src/frontend/core/typecheck/environment.rs:399`: three tables same order append + equal length
> debug assert); consumer side attributes D20 resolution product per **definition unit**
> (`MethodMarks`, `src/driver/helpers.rs:569-573`; `typed_owners` `:591`; `resolved_method_marks`
> `:621`), and makes resolution-recorded name group disable pool's name-level exemption
> (`src/frontend/core/typecheck/passes/dead_code.rs:252-267`, `:825-841`). Key form = entry name
> `{Type}.{method}#k` (`{Type}` is definition side bare name; `#k` declaration order).
>
> **Four, Key scope and false report defense line measured (fix round F1 and five residual
> boundaries)**: F1's red state is pre-fix `d5c00235a`'s injection surface "name set × module set"
> combination brings another module-defined same-name type candidate into same key position ⇒
> `use a.{Temp}` + `use b.{Other}` (a also defines Other, b also defines Temp, method same-name
> same-order) fixture `check` reports 2×E1107, exit 1 (originally successful program compile
> failed); after fix `check`/`run` both exit 0, 4 W1005 as reported both sides **uncalled**
> same-name definition (reverse miss surface also checked). Checked-in fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`; driver-level nail
> `src/driver/tests/overload_injection.rs` 3 + `src/driver/tests/method_unused.rs` round 2's 5 #464
> criteria and 4 #450 old criteria all green; main acceptance fixture
> `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` diagnostic set in fix before and after per
> item consistent (not rolled back). Five residual boundaries and per item measurement (single
> `check` definition file not report / name-level ambiguity E5006 full name skip / boundary (c) per
> t20 measured rewrite / inline alias and `ns.T` not inject candidate surface / overall import
> `use M` and std candidate surface not produce entry) full text see
> [029f](../../rfc/accepted/029f-target-semantics.md) §Cross-unit Method Reachability Attribution
> "Residual Boundaries", not repeated here.
>
> **Five, Process events and lessons (four types continuation)**:
>
> 1. **Contract wording written too narrow directly becomes defect**: One of F1's root causes is
>    contract only writes "only inject module surface that can be named by bare type name", didn't
>    write "attribute per name pair by pair"—implementation per literal took "name set × module set"
>    combination, review only caught E1107 false report. When contract describes behavior boundary
>    must write **combinatorics** dead (per name pair by pair / full combination), can't leave to
>    implementation to understand itself.
> 2. **Convention must be verifiable: "push by team leader" only in dispatch message ⇒ third party
>    push/merge appears can't attribute**. This round dev top has two source-unconfirmed merges (see
>    "Two"), ultimately relied on reflog reverse inference and withdrew one wrong attribution.
>    Verifiable form = member doesn't run `git push`/`fetch`/`pull`, sync uniformly `pull --rebase`,
>    use reflog to verify.
> 3. **Infrastructure unstable, first land patch then change personnel**: This batch total 5
>    unrecoverable failures (3×TRANSPORT, 1×INVALID_REQUEST (thinking protocol), 1×background
>    sub-agent no message exit); disposition convention = produced evidence and patches first land
>    in **workspace** batch directory (under `target/`, not in repo), but **conclusion and numbers
>    must locally land in this record**—workspace path must not be used as document reference source
>    (AGENTS meta rule ①). Then change personnel to continue, same session two consecutive failures
>    don't gamble third.
> 4. **Document commit's real criteria is "locally really build once"**: Round 2 closure's document
>    commit (`042c07860`) made Deploy Docs red on VitePress's Vue template compilation (angle
>    bracket placeholders like `<rev>` etc. not in backticks treated as elements to parse, reported
>    `Element is missing end tag`); markdownlint can't catch this kind, and pre-commit's
>    markdownlint hook in commit only containing docs was judged `no files to check` skip (measured
>    twice). Fix commit `0c837e1b4` with local build exit=0 as evidence. This round that lesson's
>    red→green run: Deploy Docs **39011946793** (red) → **39014696984** (green, see "Six").
>
> **Six, Round 2 gate conclusion supplement registration (item 13, scheduled check link backfill)**:
> Round 2 closure commit `b940300ac`'s push—CI **39011946778** success / Auto Translate
> **39011946796** success / Deploy Docs **39011946793** **failure** (VitePress build red, "Five, 4"
> angle bracket placeholders; already fixed by `0c837e1b4`); after fix commit `0c837e1b4` push three
> green: CI **39014697000** / Auto Translate **39014697044** / Deploy Docs **39014696984**.
> Consecutive push coverage: CI@`0c837e1b4`'s **39014356642** was cancelled by subsequent run with
> concurrency (cancel ≠ failure, later run covers, same caliber as round 2 §Six).
>
> **Seven, Final gate (closure revision `e9675c9dd`, local number; CI conclusion handed to scheduled
> check, see "Eight")**: lib 2770 passed / integration 289 passed / yx_multifile_runner 1 passed /
> IR snapshot 7 passed zero drift / clippy `-D warnings` 0 / docs-truth 0 / generator idempotent
> (rerun `scripts/docs/gen-error-code-docs.py` then `git status` clean); corpus baseline 346 → 347 →
> 348 lines (each +1 behavior record, other lines byte-by-byte unchanged; baseline
> `tests/baselines/corpus-parity.jsonl` on `e9675c9dd` total 348 lines).
>
> **Eight, Gate check arrangement and conclusion (item 13)**: This batch push's post-CI / Auto
> Translate / Deploy Docs conclusions handed to scheduled check link to register (not wait in
> session, also don't write "didn't wait" as "already green"). **Backfill (2026-10-17 scheduled
> check)**: `d5c00235a`'s CI `39023417670` = **failure**—Docs Truth reported "error code document
> lags behind code table/locales", i.e. that commit missed generation product same commit; fixed by
> subsequent `d5dd1a37b` (regenerate generation product): `5da05b885` CI `39027788246` / Deploy Docs
> `39027788268` / Auto Translate `39027788340` all success, covering previous; `e9675c9dd` CI
> `39028827871` / Deploy Docs `39028827896` success; `d0cfc4e62` CI `39033963240` / Deploy Docs
> `39033963278` / Auto Translate `39033963261` success. Three commits' runs each ran to terminal,
> this round no concurrency cancellation. Lesson: **"local generator idempotent" doesn't equal
> "generation product already same commit checked in"**—before closure should check CI conclusion
> per commit, not just look at latest one.
>
> **Nine, Batch tree state (closure gc, status output)**: Before closure `scratch.py status`—t25
> (holder=-) and yx49b (holder=-, previously held by deprecated member verify-r3 stale, team leader
> `release --force` released) two trees both have no holder; `gc` deletes two, frees about 12.7 GB;
> after gc `status` = "no batch tree", `git worktree list` only has main mount. Another: `yx49r3`
> and `yx49r4` are not batch trees (no meta.json), are member's **workspace** directories for
> evidence/probe, not "residual batch tree". No this batch added directory outside repo.
>
> ## P4 4.4 Proof Layer and wasm Wrap-up — Closure (4.4.1 / 4.4.2 / 4.4.3)
>
> **One, Batch composition and rounds (batch name `yx44`)**: Three three-level tasks 4.4.1 / 4.4.2 /
> 4.4.3 and wasm property reachability judgment same batch; batch verification tree built by
> `scripts/dev/scratch.py init` in workspace `target/` (first full build; after closure gc deleted).
> Roles and rounds: implement 4.4.2 = t1 (12 files; local `1fc135437` → post-push
> `24028ffe8`) ｜ implement 4.4.1 phase order rewrite = t2 (only doc/comment, zero behavior change,
> based on `b5e68b3c9`) ｜ implement 4.4.3 = t3 (11 files, including new code W1082; local
> `2d5e610cd` → post-push `52d2276ca`) ｜ wasm reachability judgment = t4 (only judgment, zero
> cleanup, judgment revision `b5e68b3c9`) ｜ 4.4.1 closure and one doc commit = t8 (7 files
> 173+/43-; local `29ebd2145` → post-push `d07993157`) ｜ independent verification = t5 (frozen
> `29ebd2145`) ｜ round 1 review = t6 (revision `1b593e993`) ｜ supplement = t9 (3 files 9+/10-;
> local `1ebddaf95` → post-push `1b593e993`) ｜ fix round = t10 (3 files 23+/23-; local `657c3d5e3`
> → post-push `e184d88b6`) ｜ round 2 review = t11 (`657c3d5e3` = post-push `e184d88b6`, patch-id
> all equal) ｜ closure = t7 (check / record / batch tree gc / doc really build / one doc commit;
> local `c8412565c` → post-push `da724940b`). **Failure and redo**: t6 with `needs_revision`
> terminal state failed (review round per contract must fail and give findings, not accident); t7
> first round interrupted in batch tree gc phase, completed products left in main repo working tree
> uncommitted, second round took over "supplement gap + review"—no redo of completed parts (this
> round added: 4.4 check and evidence annotation, this record, three places pointing to deleted
> batch directory path rewrite, doc really build and closure commit). No cancelled rounds.
>
> **Two, Per task output and measurement numbers (raw commands and output see evidence summary and
> archive; here only record conclusion)**:
>
> - **4.4.1**: `layers/README.md` (including `layers/mod.rs` top comment)'s phase order table
>   rewritten as **actual call order** (predicate → termination → ownership, call point anchored to
>   `checker.rs::check_module_impl`), `equivalence` per real identity description (no production
>   call point); same-source declaration 7 places closed (`01-routing` / `02-stage-contract` /
>   RFC-039 / `06` / typecheck README / layers two files); mechanical reconciliation script on batch
>   tree green baseline **11/11 PASS** (criteria 11 = README side 4: actual call order declaration /
>   two entry anchors / old order declaration disappear; code side 7: 5 line anchors +
>   `check_module_impl` no short circuit + `equivalence` zero production call; reconciliation script
>   is one-time script, recycled with workspace, criteria 11 listed can be rewritten based on it),
>   mutation (c) rollback old declaration → reconciliation 2 FAIL i.e. red.
> - **4.4.2**: Solver acquisition surface unified to `proof/smt/backend.rs`'s process-level shared
>   singleton + unique closure entry `with_shared_solver(consumer, f)`; three consumer points
>   (predicate / termination / ownership) all changed to shared entry, **production link on compile
>   `default_solver()` call points = 0** (only backend selection point sunk to private
>   `new_backend()`). `wasm/src/lib.rs`'s `test_smt` self-check call point is **existing** (earlier
>   than this batch, since `b5e68b3c9`) non-test call point, belongs to "test and independent
>   scenario" surface, doesn't constitute D58 prohibition violation. Independent verification
>   self-built reading (one compile): predicate acq=2 hit=1 miss=1 ｜ termination acq=1 hit=2
>   miss=2 ｜ ownership acq=2 hit=1 miss=1, three points hit/miss sum (4,4) = shared instance
>   cumulative `Some((4,4))`; mutation (a) singleton degenerate → predicate hit=0 i.e. red. **Change
>   word surface disclosure**: That commit includes `z3_backend.rs` 15 lines—`cache_stats` moved
>   from inherent method into `Solver` trait block (return `Some(instance-internal count)`) + 1 line
>   comment change pointer, no semantic coupling. D58 prohibition mechanized: same thread reentry →
>   panic contains D58, probe `recv_timeout(120s)` doesn't hang.
> - **4.4.3**: `body_checker` missing silent degradation converged to
>   `TypeChecker::ownership_inputs` (only criteria point) + new code **W1082** (not overlapping with
>   existing W code: W1080 constraint degradation / W1081 solver missing / W1090 opposite
>   direction); nail test 2/2 green; healthy path corpus 348:348 zero diff; **degradation branch
>   production surface unreachable enumeration holds** (CLI four entries + eval/repl + format +
>   LSP + wasm playground + all `ProgramKind` converge into `check_module_impl`: `init_body_checker`
>   unconditionally before ownership block, function internal 0 return/break/continue,
>   `body_checker` no `.take()`) ⇒ W1082 no user-visible surface. Mutation (b) delete emission →
>   nail test i.e. red. **W1082 belongs to external contract table change (E class),
>   user-approved**; `locales/zh.json` / `locales/en.json` same commit with code, other languages
>   supplemented by i18n bot after push (run see "Nine").
> - **wasm property reachability judgment (t4)**: `orchestrator.rs` current tree **10 = 4 positive
>   arms + 6 negative arms** (caliber = same line `#[cfg(...)]` attribute text, both positive and
>   negative counted; `cfg!` runtime macro 0 hits); 4 wasm positive arms unreachable in playground
>   runtime but **compile time load-bearing** (call point not gated, single arm deletion ⇒ wasm
>   compile fail), 6 `not(wasm32)` negative arms native three-phase table load-bearing ⇒ D58's
>   "delete unreachable" has **no applicable target** in this file, real cleanup form = entire
>   project-mode subtree gated on wasm32 (register P10 S7, three unverified keep original).
>
> **Three, Independent verification (t5, frozen `29ebd2145` = post-push `d07993157` content—two
> revisions per commit patch-id all equal, whole tree difference only in `docs/src/en/**`,
> `docs/src/.i18n-cache.json` etc. bot product surface—`git range-diff` three commits all `=`)**:
> Gate **8/8 all green**—`cargo fmt --check` 0 ｜
> `clippy --all --all-features --all-targets -- -D warnings` 0 ｜ `cargo test` **3128 passed / 0
> failed** (7 test targets total; baseline 3126 + this batch 2) ｜ corpus diff **348:348** per line
> zero difference (sort diff 0 lines) ｜ IR snapshot `UPDATE_SNAPSHOTS=1` rerun then `git status`
> zero drift ｜ `check-obligations.py` 21 = 13 + 8, containment 0 gap ｜ `check-docs-truth.py`
> source 156 / document 156, CLI 21 commands consistent ｜ error code generator rerun before and
> after tree both clean. Build warning only 4 bot language W1082 missing entries (known state) + Z3
> download noise, zero code warnings. **Contract verify caliber correction (must two JSONL paths)**:
> Corpus gate's call caliber =
> `python scripts/ci/check-corpus-parity.py tests/baselines/corpus-parity.jsonl <this generation>.jsonl`—**two
> JSONL paths must be given**; missing second parameter is comparing baseline to baseline (always
> zero diff, criteria idle). This batch executed and verified per two-path caliber (t5's
> `corpus-r44.jsonl` product).
>
> **Four, Mutation evidence (3/3, mechanical reversible)**: All via
> `python scripts/dev/scratch.py trial` (`git apply` → run command → `git apply -R` → per file blob
> compared with HEAD → require `git status` clean):
>
> - (a) Singleton degenerate → shared reading predicate hit=0 miss=2, test FAILED exit 101;
>   `backend.rs` HEAD=`3e2b2d150b24` → mutation `6d00bad4d4aa` → restore = HEAD.
> - (b) Delete W1082 emission → nail test FAILED exit 101; `checker.rs` HEAD=`70c04a41e020` →
>   mutation `db0a623c9b9a` → restore = HEAD.
> - (c) Rollback old phase order declaration → reconciliation 2 FAIL exit 1; `layers/README.md`
>   HEAD=`8b1ae875db32` → mutation `e88f9af6c42d` → restore = HEAD. Three trial restores all
>   `git status` CLEAN. Trial (c) first run failed due to absolute path WSL parse failure exit 127
>   (not criteria red), changed to relative path rerun as authoritative.
>
> **Five, Review rounds and conclusion**: Round 1 (t6, revision `1b593e993`) **verdict =
> needs_revision**—3 findings all doc line number anchor inaccurate (F-01 medium: `02` §"wasm
> status" 4 groups expired line numbers; F-02 low: `02` and `09`'s 4.4.3 delivery anchor
> `checker.rs:1283-1293`; F-03 low: `06`'s comparison statement), four substantive deliveries all
> pass. → Fix round (t10, local `657c3d5e3` = post-push `e184d88b6`, 3 files 23+/23-, with two
> mechanical criteria) → Round 2 (t11, same revision) **verdict = pass, new finding = 0**. **Final
> check basis = round 2's pass**. **Round 3 (closure tip `da724940b`)**: Independent verification
> t12 **verdict = pass** (bound to `da724940b`: four rule texts byte-by-byte, 3.19 / S7 / S8 fields
> and scope ready, count table and judgment chain recalculation, dead reference zero,
> `check-docs-truth` 0, `markdownlint` 0); review t13 **verdict = needs_revision**, 4 findings—F-01
> medium (t10 / round 2 only gives rebase pre-hash, §14 list doesn't match origin/dev reality), F-02
> low (`02:379`'s 09 4.2.8 anchor displaced by this batch itself inserting one line each at
> `09:144`/`:182` to `:274`), F-03 low (`09a` written local drive letter path), F-04 low
> (three-level count row annotation and 157 ≠ table head 156) → fix round t14 (this commit) closed
> one by one. **t10 contract caliber ruling**: Fix surface based on **task record's 5 F-01…F-05**
> (F-04/F-05 are mechanical criteria), dispatch brief not criteria.
>
> **Six, User ruling (2026-10-18) and rule solidification**:
>
> 1. **D58 entry name local rewrite**: `with_shared_solver(f)` →
>    `with_shared_solver(consumer: SolverConsumer, f)`, local fix body, **no revision annotation**
>    (t8 completed in 4.4.1 one commit).
> 2. **wasm judgment registered as P10 S7** (ruling B): with "belonging phase: P10; whether blocks
>    main line: no" + three unverified **keep original**; `06`'s F2 "needs decision" row points to
>    this registration; t4 produced "### F2 judgment" subsection not touched.
> 3. **Crash item registered as P3 3.19** (ruling A): symptom / cause points to
>    `proof/smt/translate.rs`'s sort inference / D59 **A/B class** (crash) / acceptance row (this
>    shape no longer abort—conservative degradation or diagnostic, corpus and IR snapshot zero
>    drift); **characteristic = pre-existing** (t5 two revision comparison, see "Seven" ⑤).
> 4. **Rule solidification one** "correction locally written in body, no new-old comparison left":
>    land `coding-rules.md` (new article), `07-equivalence-oracle.md` §Semantic correction branch
>    criteria item 3, RFC-039 **D59 row** item 3 (same commit registration = WBS row mark error
>    type + local fix affected body + PR required block "error type" one line).
> 5. **Rule solidification two** "contract `inScope` uniformly write path prefix resolvable by path
>    matcher": land `AGENTS.md` team orchestration convention item 6.
> 6. **New code W1082 approved** (external contract table change, E class).
> 7. **Register P10 S8** "document correction form convergence (local rewrite, delete revision
>    annotation)": including 7 audit list scope and acceptance row.
> 8. `09` top count table: secondary **63** (60 + 3.19 / S7 / S8), three-level **157** (154 + 3),
>    with 2026-10-18 annotation; three-level item accounting review see "Fifteen".
>
> **Seven, Process facts to record faithfully**: ① 4.4.2 commit used
> `SKIP=cargo-fix,cargo-clippy-fix` to skip two `--fix` hooks—shared working tree had teammate
> in-progress, `--fix` would casually rewrite other people's files; **equivalent gate already
> manually fully run in final tree** (fmt --check / clippy `-D warnings` both 0). ② `pre-commit`'s
> stash window indeed happened (`Stashing unstaged files…` / `Restored changes from…`), after commit
> `git status` check shows teammate in-progress intact. ③ **4.4.3's inScope lesson**: Prose
> qualification `docs/src/(only error code generation product)` was rejected by `changedPaths` check
> ⇒ `inScope` uniformly write matchable path prefix (already solidified as rule, see "Six" 5). That
> task **actually 11 files**, 7 of which due to declaration writing can't be parsed by path matcher
> weren't listed in `changedPaths`, listed one by one in task result (`codes/w1xxx.rs`, RFC-013 code
> table, `reference/error-code/`'s index and each E section page etc.). ④ **W1082's locale window
> closed**: `locales/zh.json` / `locales/en.json` same commit with code, other languages
> supplemented by i18n bot after push (run see "Nine"). ⑤ **Z3 `Sort mismatch` crash judgment (t5
> criteria ⑦)**: **pre-existing**—same probe, same input on `b5e68b3c9` and frozen revision
> `29ebd2145` stdout **byte-by-byte consistent**
> (`Error: Sort mismatch at argument #1 for function (declare-fun not (Bool) Bool) supplied sort is Int`),
> both exit 1; static attribution consistent (4.4.2 0 line change to `translate.rs`, only
> `cache_stats` migration to `z3_backend.rs`). **Degradation branch production surface unreachable
> enumeration holds** (W1082 no user-visible surface). Probe three lines source code:
> `NonNegative: (x: Int) -> Type = { x >= 0 }` / `AtMost: (n: Int, x: Int) -> Type = { x <= n }` /
> `f: (x: NonNegative(x)) -> AtMost(x, x) = { return x }`. ⑥ **t5 frozen surface and t9's coverage
> difference**: t9 (three places comment/doc supplement, `1b593e993`) commit later than t5's frozen
> revision `29ebd2145`, belongs to zero behavior change, its acceptance covered by t9's
> self-mechanical check + t6 review (t6 on `1b593e993` **reran** t5's key measurement, not retold).
> ⑦ `docs/src/en/**` not manually modified, regenerated by post-push auto-translate workflow (this
> batch that bot landed two commits: `92d490695` i18n locales supplement W1082 other languages,
> `63b6cdf37` auto-translate document). ⑧ **t9 overstep observation three rulings**: ㈠ t9
> on-the-fly corrected `02` same sentence's `wasm/src/lib.rs` line count and gate point line numbers
> (73 / 139·153·170 → 152 / 151·170·187)—**accepted** (same caliber same number essential surface,
> commit message already listed); ㈡ RFC-039 **D35 row** still writes "wasm 27 files' cfg
> branches"—**keep unchanged** (resolution row is historical ruling record, its premise correction
> carried by P10 S7 registration, no rewriting history); ㈢ `docs/src/en/**`'s old entry name
> translation aligned by auto-translate. ⑨ `09` / `09a` originally pointed to deleted batch
> directory two places in closure round changed to **local inline** (evidence summary merged into
> this record's section, probe three lines source code inlined). **2026-10-21 correction**: At that
> time changed point still was workspace path, equal to moving violation down one level; from this
> round this summary's delivery commit, gate numbers and CI run id all inlined, document no longer
> references any workspace path (AGENTS meta rule ① / item 10④).
>
> **Eight, t8 acceptance ④ caliber ruling**: Acceptance original text required
> `git diff | grep '修订注记\|原为\|现改为'` to be empty. Measured **default context caliber
> unreachable**—D58 row neighbor row's D59 / D61 resolution rows themselves contain
> "修订注记 / 原为" text (not this task's change). Ruling: per **new lines / deleted lines / `-U0`
> increment lines** three calibers all empty accepted, and write **subsequent similar acceptance
> only grep new/deleted lines**—criteria should land on this change's lines, not whole diff's
> context.
>
> **Nine, CI gate conclusion (AGENTS item 13①, scheduled check 2026-10-18 17:26 backfill; run id and
> conclusion locally inlined)**: First push (3 commits, tip = `d07993157`): CI **39110908163**
> success ｜ Deploy Docs **39110908155** success ｜ Auto Translate Docs **39110908193**
> success ｜ Auto Translate I18n **39110908144** success; that tip covers `24028ffe8` (4.4.2) /
> `52d2276ca` (4.4.3) / `d07993157` (4.4.1) three commits all content. Second push (1 commit, tip =
> `1b593e993`): CI **39112293391** success ｜ Deploy Docs **39112293062** success ｜ Auto Translate
> Docs **39112292903** success; covers all four commits including t9 supplement. **No concurrency
> cancellation**, latter covers former. Third push (2 commits, tip = `da724940b`): CI
> **39121538555** success ｜ Deploy Docs **39121538562** success ｜ Auto Translate Docs
> **39121538541** success—three runs' headSha all = `da724940b` and **all terminal** (2026-10-18
> third round scheduled check backfill); round 3 independent verification t12 at that time recorded
> as "CI not terminal" observation closed. Unrelated: Nightly **39081392694** failure's headSha =
> `290f9ebc`, not in this batch's commit surface.
>
> **Ten, Three `pull --rebase` (auto-commit other side during push)**: Other side are auto-translate
> and i18n commits; six hashes rewritten and **patch-id all equal** one by one—local `1fc135437` →
> `24028ffe8`, `2d5e610cd` → `52d2276ca`, `29ebd2145` → `d07993157`, `1ebddaf95` → `1b593e993`,
> `657c3d5e3` → `e184d88b6`, `c8412565c` → `da724940b` (after third rebase origin/dev head =
> `da724940b`); `git range-diff` verified **each content unchanged** (three commits all `=`). Member
> didn't execute `git push` / `fetch` / `pull`, sync and push uniformly by team leader. **Evidence
> reminder**: After this reference commit number uniformly use **post-push hash** (above right
> value); `09`'s check row per convention doesn't write commit number.
>
> **Eleven, Uncovered surface (must not write "not tested" as "passed")**: ① doc-tests 0/0 didn't
> carry criteria; ② **wasm32 target real compile not done** (that surface covered by CI's
> `_build-wasm.yml`, this batch's wasm judgment is static argument); ③ mutation only covers 3 places
> named by contract, other gates didn't do reverse corruption verification; ④ corpus diff didn't run
> separately on `b5e68b3c9` (only ran on frozen revision); ⑤ i18n bot supplemented other languages'
> W1082 text behavior not verified (only checked `zh.json` / `en.json`); ⑥ round 1 review and fix
> round reported **same-kind but not in contract findings existing drift** not fixed (list see
> "Twelve"), attribution pending user ruling.
>
> **Twelve, `02` whole article existing drift observation list (user 2026-10-18 agreed to register;
> 2026-10-21 land as S10)**: Caliber = all are **pre-existing drift** (not introduced by this
> batch), **don't constitute this batch's finding**; list found by fix round, round 2 review
> independently sampled and gave mapping on `657c3d5e3` (= post-push `e184d88b6`), recorded item by
> item (list locally inlined as follows, original files recycled with workspace):
>
> 1. `02` §(a) entry table 3a-3d / 5 / 5b: `02:72` quotes `orchestrator.rs:273-399` → current
>    `:155`; `02:73` `:450` → `:250`; `02:74` `:1374` → `:985`; `02:78` `util/diagnostic/mod.rs:565`
>    → `:681`; `lsp/handlers/diagnostics.rs:146`'s `run_diagnostics` no longer at that place;
>    **content level**: `run_diagnostics` and "otherwise manual lex→parse" replaced by 4.6.1 (delete
>    manual sequence).
> 2. `02:1016`: `compile_project :99-237` → current `:107-134` (28 lines Program constructor).
> 3. `02:360-361`: "missing degrades to warning" → current tree `_build-wasm.yml:50-51` is
>    `::error` + `exit 1` (#435); `build.rs:65-72` already changed to `panic!` hard fail;
>    `_build-wasm.yml:35-59` interval needs refetch.
> 4. `02:127` mechanism pointer: mechanism actually in `src/driver/arms.rs::typecheck_orchestrated`
>    (`:288-289` self-stated); conclusion still holds (multi-file arm drops warnings), inaccurate is
>    "result only downstream use" sentence (`type_result` also via `arms.rs:1389` →
>    `helpers.rs:259/:636` feeds `overload_resolutions`); suggested fix = pointer change to
>    `typecheck_orchestrated`, "only downstream use" change to "warnings field no read point".
> 5. `02:128`: `check_project:385-395` / `250-252` needs refetch.
> 6. `02:334` (§S5 (d) investigation paragraph): quoted old code fragment replaced by 4.4.3's
>    `ownership_inputs` criteria point.
> 7. `02` change list table other rows: `predicate.rs:34-36`, `checker.rs:1303-1314` / `:1244`,
>    `backend.rs:67-72`, `wasm/src/lib.rs:30-36` / `:42-51`, `util/diagnostic/mod.rs:621-661`.
>    **This closure doesn't preset S9's WBS item** (user not ruled); after getting ruling then land
>    `09` per registration form. **Subsequent (2026-10-21 land)**: User 2026-10-18 agreed to
>    register, this list as `S10` "02 body and fact alignment (line number anchor + content level
>    expired)" landed `09` (secondary +1 / three-level +1 = S10.1)—**original intended S9 number was
>    occupied by "pre-release audit abstraction closure"** (S9.1 = issue #473 / S9.2 = issue #474),
>    per 09 "number must be unique in whole document" registration convention deferred to S10.
>    **Registration timing deviated from original plan** (originally wrote "land with P5 closure
>    document commit"), per user 2026-10-18 indication this batch land immediately.
>
> **Thirteen, Batch tree recycling (gc and status raw output; tree path in output per repo relative
> path record)**: `python scripts/dev/scratch.py gc` →
> `[scratch] 已删除 yx44-evidence（释放约 0.0 MB）` / `[scratch] gc 完成，共释放约 0.0 MB`;
> `python scripts/dev/scratch.py status` → `yx44-evidence            holder=-` next line
> `<workspace>/yx44/tree`—registry recorded path **doesn't exist**, that name is registry residual
> (`gc`'s `_remove` deletes `meta["path"]`'s parent directory i.e. workspace's `yx44`, residual
> `meta.json` still in scan surface; its content batch=`yx44`, `base_rev=29ebd2145`, `lock=null`,
> containing t5 / t6 / t11 hold history); Closure round renamed that registry file to
> `meta.retired.json` to make it exit scan (**not delete any evidence file, not create any tree**) →
> `python scripts/dev/scratch.py status` = `[scratch] 没有批次树`; rerun `gc` =
> `[scratch] 没有批次树，无需回收`; `git worktree list` only has main mount (`dev`). Batch original
> evidence directory (no `meta.json`, **not batch tree**) at that time kept as archive; **from
> 2026-10-21 workspace directory no longer as evidence source**—closure summary inlined in this
> section, original directory deleted with cleanup (AGENTS meta rule ①). Acceptance expression
> (resolvable): `git status` clean; batch tree after gc `python scripts/dev/scratch.py status` = no
> batch tree; `git worktree list` only has main mount; no this batch added directory outside repo.
>
> **Fourteen, Document real build and commit hygiene**: `npm run docs:build` local **exit 0**
> (AGENTS item 10⑨—markdownlint can't catch VitePress's Vue compile error, and commit only
> containing docs will be judged `no files to check` skip markdownlint); this batch's check /
> annotation / registration / rule modification accumulated into **one** document commit (item 8),
> landed in batch closure; member didn't execute `git push` / `fetch` / `pull`. **This batch final
> two commits both pushed** (closure tip on `git rev-list --count origin/dev..HEAD` = 0, origin/dev
> head = `da724940b`): `e184d88b6` (round 1 findings' document anchor fix, 3 files 23+/23-) +
> `da724940b` (closure document commit, 7 files 96+/10-; including check / record / three place path
> rewrite / registration and rule modification).
>
> **Fifteen, Count table accounting review (round 3 fix F-04; method and result both verifiable)**:
> Judgment object = `09` top "positioning and scope" table's three-level count row. **Method** (two
> independent calibers, per revision use `git show <rev>:<file>` to take byte recalculate): ① **Per
> secondary item sum**—take each secondary kanban row parentheses' "N three-level tasks" (row start
> `- [x]`/`- [ ]` bold title start, row contains "个三级任务", including S7 row inline written
> "1 个三级任务"): current tree 70 secondary items, **sum = 157**. ② **Historical
> self-consistency**—same caliber on `366bc5a69^` get 66, **sum = 151 = current period table head
> 151** (self-consistent); `366bc5a69` that day newly registered `4.9.6` and `0.4.1` two three-level
> (sum 151 → 153), but table head only writes 151 → 152 ⇒ **table head from that commit undercounts
> 1**; subsequent revisions all differ 1 (`92d490695` / `1b593e993` / `d07993157` / `e184d88b6`: sum
> 154 to table head 153; `da724940b`: sum 157 to table head 156). **Result**: True value = **157**,
> so correct table head (`09:13` three-level 156 → 157); annotation 144+2+2+2+2+1+1+3 = 157
> originally correct, not changed; pre-batch this base = **154** (not 153). **Secondary count row
> not touched**—that row's caliber never aligned with kanban item count (`366bc5a69^` is 66 to table
> head 59), this time no verifiable evidence to overturn 53 + 10 = 63, no number changing for
> rounding. Incidental correction (same round): §2's 4.4.1 parenthesis decomposition per
> reconciliation script actual item count changed to "README side 4 + code side 7 = 11", consistent
> with **11/11** and script 11 `ck` calls.

## P3 Debt Round 1 Closure (3.5 / 3.6 / 3.4.6 / 0.5)

> **One, Batch composition and rounds**: This batch closes P3 side remaining debt—**3.5
> Higher-kinded type parameter call path** (3.5.1–3.5.3), **3.6 Function type value call path**
> (3.6.1–3.6.4), **3.4.6 compile path consumption warnings** (D class, user 2026-10-19 ruling "do"),
> and registers together **0.5 test suite layering** (P0, issue #486 slice) and **3.20 dead code
> analysis existing false report** (3.20.1 / 3.20.2). Roles and rounds:
>
> - Implement: 3.5.1–3.5.3 = t1 ｜ 3.6.1 = t2 ｜ 3.4.6 = t6 ｜ 3.6.2 = t7 ｜ 3.6.3 = t8 (main +
>   closing two commits) ｜ 3.6.4 = t9 ｜ 0.5.1 = t10 (#486 slice)
> - Independent verification: round 1 = t3 (frozen revision `81d5e414e`, covering 3.5 / 3.6.1–3.6.3
>   / 3.4.6) ｜ 3.6.4 = t11 (round 2, frozen `8e32e8636`)
> - Review: round 1 = t4 (verdict = **needs_revision**, F1/F2/F3) ｜ fix round = t12 (one
>   commit) ｜ round 2 = t13 (verdict = **pass**)
> - Closure = t5 (check / registration / this record / batch tree gc / doc really build / one
>   document commit)
>
> **Failure and redo**: t4 with needs_revision terminal state failed (review round per contract must
> fail and give findings, not accident) → t12 real fix → t13 review pass. t8 internal experienced
> one attempt interruption (process crash), previous round evidence kept in batch directory,
> continuation attempt supplemented then two commits checked in. No cancelled rounds.
>
> **Two, Per task output and measurement numbers (conclusion caliber; raw commands and output see
> each batch evidence directory)**:
>
> - **3.5.1** (unifier in head position matches constructor variable): Criteria converged to
>   **single implementation** `is_ctor_reference` (`pub(super)` free function, full repo 1
>   definition 3 consumers)—this is t12's real fix for t4's F1 (originally `inference/statements.rs`
>   had another same-name method copy, constitutes prohibition three sense's same-name two
>   implementation). Head rename no longer inline copy: `environment.rs` and `expressions.rs` each
>   first traverse TypeRef / variable table to construct head_map, then uniformly hand to shared
>   implementation `substitute_ctor_heads` (`types/substitute.rs`) to execute—two places head_map
>   construction criteria unchanged, rename only by name acts on head, empty table returns as-is
>   (t12's F2 real fix). Equivalence argument: traversal doesn't change Generic head name.
> - **3.5.2** (explicit type argument accepts type constructor, E1010 disappears): Red state E1010 /
>   E1002, green state exit 0.
> - **3.5.3** (mono product replaces constructor variable): Symbolic type argument criteria narrowed
>   per container generic parameter name; product's constructor variable replaced with real
>   constructor (IR dump evidence: specialized function `Function probe(Box, int64)`).
> - **3.6.1** (local function type value by name call): Criteria single source = `is_static_fn_name`
>   (full repo 1 definition 1 call), aligned with variable evaluation path's name resolution
>   priority (local → global), didn't add second resolution table; IR evidence: module init segment
>   changed from `LoadConst String("f")` + `CallNative` to `LoadGlobal` + `CallDyn`. Regression
>   surface: same-name "function binding + value binding" unreachable (uniformly compile time
>   E2010), non-function value by name call still E1065.
> - **3.6.2** (`use` alias imported function value by name call): Originally reported
>   `E6006 Native function not found: lib.f` (`use lib.{f}` and `use lib.{f as g}` two forms). Fix =
>   in `is_static_fn_name` internal **add one arm** (not copy), add unique implementation
>   `use_alias_value_slot` takes "use alias → qualified name → global slot" two table joint lookup,
>   consumer points 2 (call form criteria + variable evaluation path).
> - **3.6.3** (nested function body references outer frame value): **Criteria changes judgment**—not
>   "make it run right", but that form per RFC-009 §2.3 + SPEC §12.3 should be **compile time
>   rejected** (D59 C class: change silent wrong value to loud; per `09` §P3's D59 row, "silently
>   give wrong answer" belongs to C class—this batch closure brief records as "A/B class", both
>   refer to "old behavior not successful", landing form both are "red first then green + compile
>   time loud rejection"). Original form = **no error, wrong value (void)**: red state stdout
>   `void`, EXIT=0 (silent wrong value archive); green state compile time E1001 `Unknown variable`,
>   EXIT=1, with existing "unknown variable" path **same code same text**. Criteria single source =
>   capture barrier (`enter_capture_barrier`), compile time name (type parameter) not blocked; E1001
>   still only emitted by typecheck, registered unique. **Assignment path covered by E2010** (t8's
>   closing commit: declaration / assignment judgment doesn't apply capture barrier)—otherwise this
>   fix itself would open a new "same-name assignment silent wrong value" channel (c1 form: only
>   barrier degenerates to exit 0 print 10).
> - **3.6.4** (`use lib` overall import then `lib.f(...)`): Goes through FieldAccess callee's
>   namespace branch, originally reported E6006 `lib.f`. Fix = namespace branch resolves qualified
>   name then first judges global slot layout → changes to indirect call; qualified name resolution
>   and slot query reuse `resolve_namespace_data_access` (same source as data access), didn't add
>   resolution table.
> - **3.4.6** (compile path consumption warnings, D class): Before and after change external
>   performance = **multi-file `run` previously always didn't output any W code** (red state
>   archive: multi-file [] vs single-file [(W1001,3),(W1002,5),(W1003,1)]), now four paths
>   (single-file run / multi-file run / single-file check / multi-file check)'s W code set always
>   `{W1003@1, W1001@3, W1002@5}` (non-empty set). Corpus baseline: 348 records **46 changes**, new
>   W code **118** (W1001 34 / W1002 12 / W1003 13 / W1004 42 / W1005 17), **118/118 per item
>   attributed to real existing unused definitions in files (0 not located)**, pure increment (0
>   deleted, 0 exit / stdout / check change). **D59 four items fulfilled**: ① red first then green
>   (red state archive + turn nail test) ② doesn't change "originally successful program" external
>   performance (pure increment, 0 exit / stdout / check change) ③ same commit registration (this
>   record + `09`'s 3.4.6 row) ④ baseline same commit regenerate and manually review diff.
>   **Superset approved**: W1004 / W1005 / W1006 together enter `run` (user 2026-10-19 approved),
>   reason = **one criteria, one channel**—only putting W1001/W1002/W1003 equals creating second
>   filter judgment. **External contract surface change**: `orchestrator::compile_project` return
>   value changed from `Result<ModuleIR, _>` to `Result<(ModuleIR, FileDiagnostics), _>` (in-repo
>   consumers 2 + 2 test direct drive points synchronized adapt).
> - **0.5.1** (test resource profile mark + exclusive lock + fast/full minimum consumption surface):
>   Added `scripts/dev/testsuite.py` (`markers --check` and `cargo test --list` bidirectional
>   reconciliation, `plan` budget binning, `run fast|full` budget insufficient report gap not
>   silently skip, `manual` file never implicit execute) and `src/util/test_support.rs`
>   (cross-process file lock, test body take lock ⇒ any call path mutually exclusive); inline
>   `cost:` mark 3 (2600 file exclusive, peak from 5000 file measured linear extrapolation, **not
>   measured**; 5000 file exclusive, 15.6 GiB / 72.6 s measured; snapshot update manual, peak and
>   duration both extrapolated). Self-verification number: `markers --check` check 3149 `#[test]`, 3
>   mechanical mutations each exit 1, `run fast` batch wave 2793 passed / 47.7 s + exclusive wave 1
>   passed / 14.6 s, budget 1 GiB `run full` list gap 7.11 GiB, two process lock contention measured
>   `held→blocked→held(waited 2.0s)`. **Independent verification belongs to #486 subsequent
>   scheduling** (this batch only has implementer self-verification). **0.5.2 pending scheduling**
>   (pre-commit / CI wiring + 2600 file measured calibration + `test-specification` detailed rules).
>
> **Three, Independent verification (t3, frozen `81d5e414e`; red state baseline `c599de7bb` /
> `1725d0215` / `ef2334de5` all HEAD ancestors, `git merge-base --is-ancestor` verified)**: Probes
> all self-built by verifier (batch tree built by `scripts/dev/scratch.py init` in workspace, probes
> and patches in its `probes/` / `patches/` subdirectories), not retell implementer conclusion; that
> batch tree together with tree-internal evidence already deleted in this batch closure's gc (see
> "Nine"), so this section takes measurement conclusion and numbers **as-is into record** for
> archive. 3.5 red state four probes all report E1003+E1002 / E1010, green state exit 0 and value
> correct (stdout 1 / 7 / 1), with IR value-level evidence (specialized function
> `Function probe(Box, int64)`); 3.6.1 green state output 6 / 10, control "value position call"
> green, "non-function value by name call" compile time E1065; 3.6.2 red state two forms E6006,
> green state exit 0 (probe internal `assert` fail is non-0, exit 0 is value correct); 3.6.3 red
> state stdout `void` / EXIT=0 (silent wrong value verifiable), green state compile time E1001 /
> EXIT=1, while **expression position inline lambda capture this function parameter** allowed
> (EXIT=0, output 6)—allowed surface not accidentally injured. 3.4.6's C2 normalization by verifier
> self-built fixture independently reproduced (four path W set identical, non-empty), baseline diff
> independently recalculated (changed=46, added=0, removed=0, expectation / run.exit / run.stdout /
> check change = 0, new W code 118 same as implementer), attribution sample 10 all hit real
> definition. Gate on frozen tree all green: fmt / clippy (`-D warnings`) / `cargo test --all`
> (**lib 2790 passed + 2 ignored, integration 297 passed**, 0 failed) / corpus diff 348:348
> difference empty / IR snapshot 204 zero drift / obligation ledger / docs-truth. **t11 (round 2,
> frozen `8e32e8636`)** independently reproduced 3.6.4's red→green (red state parent commit
> 16e2a5c88: E6006 `lib.f`, EXIT=1; green state EXIT=0, stdout 27, probe internal 4 assertions all
> passed), criteria single source (`resolve_namespace_data_access` 1 definition 2 consumption, no
> third "qualified name → slot" joint lookup), not regression eight forms all passed, verdict =
> **pass**.
>
> **Four, Review rounds and conclusion**: Round 1 (t4, judging `20abd3ebc`'s parent revision)
> verdict = **needs_revision**, three findings all real defects: F1 = `is_ctor_reference` same name
> two copies (`inference/statements.rs` method copy), F2 = head rename in `environment.rs` and
> `expressions.rs` each inlined one copy, F3 = visibility overstep (should-be-private not private,
> should-be-`pub(super)` not converged). → Fix round t12 (one commit, 4 files +96/−85) → Round 2
> (t13, reviewed revision `20abd3ebc`) verdict = **pass**, new finding = 0: 8 self-built probes'
> value and exit code and pre-fix per item same, 4 IR dumps with round 1 green state **byte-by-byte
> consistent**, corpus 348/348 zero drift, IR snapshot 204 zero drift, `cargo test --all` EXIT=0
> (lib 2794 passed + 2 ignored, integration 297 passed, 0 failed), fmt / clippy / docs-truth all
> exit 0, `check-boundary`'s `pub(crate)` count 167 / frontend-core 26 (= 3.5 pre-batch baseline),
> `check-fanout` same name definition zero hit. **Final check basis = round 2's pass**.
>
> **Five, Mutation evidence (all via `python scripts/dev/scratch.py trial`, apply → run command →
> apply -R → per file blob compare + require `git status` clean)**:
>
> | #   | Target                                        | Post-mutation blob | Capture result                                                                                                                                           |
> | --- | --------------------------------------------- | ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | M1  | 3.6.1 `is_static_fn_name`'s global lookup arm | `022986cbea65…`    | Two probes reproduce E6006, EXIT=1 (after restore blob = HEAD, CLEAN)                                                                                    |
> | M2  | 3.5.2 explicit type argument branch           | `abbcc5ecf116…`    | Probe check reports E1002, EXIT=1 (control function form still green ⇒ attribution precise to 3.5.1)                                                     |
> | M3  | 3.4.6's `MULTI_FILE_STAGES` dead code arm     | `372f7b92755d…`    | run channel W code all gone (C2 red); M3b attribution shows check channel criteria robust, broken is exactly 3.4.6 newly opened compile external channel |
> | M4  | 3.6.3 capture barrier entry                   | `7a7204c60383…`    | Capture kind turn nail test 4 failed, TEST_EXIT=101                                                                                                      |
> | M5  | 3.6.4 value slot → CallDyn block              | `25853376639b…`    | Probe reproduces E6006, EXIT=1 (blob chain `0d837e9d30de…` → `25853376639b…` → `0d837e9d30de…`)                                                          |
> | M8  | 3.5.1 turn nail test                          | —                  | After fix rerun still can fail (E1003, TEST_EXIT=101, mechanical reversible restore CLEAN) ⇒ criteria didn't lose fail-ability                           |
>
> **Six, User ruling and this batch registration**:
>
> 1. **3.4.6 = D class** (externally observable: multi-file `run` new W code), user 2026-10-19
>    ruling "do"; **superset (W1004 / W1005 / W1006) also approved**, reason = one criteria / one
>    channel. Original 2026-10-15 ruling C "per decision B1 held" **precondition** (4.9 pool
>    semantic correction) already closed on 2026-10-17.
> 2. **3.6.3 criteria change judgment**: From "make it run right" to "nested function body
>    references outer frame value → compile time E1001"; assignment path covered by E2010.
> 3. **Two "earlier than this task" silent forms registration unchanged**: ① same-block sibling
>    nested function reference—baseline is E1001 (outer block local family), this batch doesn't
>    expand scope; ② nested function annotation references outer type parameter still prints `void`
>    (t1 registered item). Both are **not** introduced by this batch, all keep original.
> 4. **3.20 "dead code analysis existing false report" registration** (user 2026-10-19 agreed per C
>    disposition then register): 3.20.1 = W1001 doesn't track in-closure reference
>    (`docs/src/reference/stdlib/test.md`'s `t_add` form: referenced in closure yet reports unused);
>    3.20.2 = W1003 treats variant destructuring required import as unused
>    (`docs/src/reference/stdlib/weak.md`'s `use std.option` form: delete then E1002). Two forms on
>    pre-3.4.6 baseline also reproduce, **not introduced by 3.4.6**. **Rollback obligation (written
>    dead)**: After this group fixed must take `weak.md` / `test.md` rewritten to bypass false
>    report examples **restore to natural writing** and **regenerate baseline** (baseline will move
>    once more).
> 5. **0.5 test suite layering** (P0, issue #486): 0.5.1 completed; 0.5.2 (pre-commit / CI wiring +
>    2600 file measured calibration + `test-specification` detailed rules) pending scheduling.
> 6. **#489 ureq 3 migration**: Dependency surface tracking item, **not enter WBS**;
>    `.cargo/audit.toml`'s `RUSTSEC-2026-0381` exemption **with hard removal condition**—ureq
>    migrated to >= 3 then delete that ignore, if still red at that time not allowed to extend
>    deadline, must first fix dependency. 0.26 line no fix version (0.26.12 doesn't exist), ureq 2.x
>    no update version.
> 7. **3.20 number conflict disposition (this round new registration)**: 3.20 number originally
>    occupied by 2026-10-18 registered "pre-release audit (v0.8.4) supplement gap" (3.20.1–3.20.4),
>    while user 2026-10-19 ruling registered "dead code analysis existing false report" needs 3.20.
>    Whole document number uniqueness rule doesn't allow two 3.20, so **migrate pre-release audit
>    group entirely to 3.21** (three-level change 3.21.1–3.21.4, issue anchor #468–#471 unchanged,
>    item content unchanged), 3.20 number gives "dead code analysis existing false report".
>    Migration only changes number, doesn't change any registration's content and attribution;
>    historical commit information not rewritten (migration fact recorded in this record and `09`'s
>    count table annotation).
>
> **Seven, Process facts to record faithfully**:
>
> 1. **Two times inScope written narrow each brought one round of rework**: ① 3.6 series implementer
>    wrote inScope narrow to `ir_gen.rs` predicate layer, didn't cover `types/`'s shared replacement
>    implementation → review round 1's F2 only when landing found head rename in two files each
>    inlined one copy, must open another convergence round; ② 3.5's fix round in `types/` shared
>    implementation and two callers between transporting changes, again hit inScope and actual
>    change surface mismatch (AGENTS.md item 6: inScope must cover mechanical essentials, path
>    prefix can be parsed by matcher). Both reworks are due to "implementer inScope written narrow",
>    **not implementation defect**—but cost is two rounds of rework and one review findings.
> 2. **External sync (2026-10-19 13:08 `pull --rebase --autostash origin dev`, 13:19
>    `pull --tags origin dev`) and V0.8.5 release commit**: Confirmed by user 2026-10-19 as **normal
>    release process, completed, doesn't affect this team**; member commits intact no loss. Evidence
>    (`git reflog --date=iso` / `git reflog show origin/dev --date=iso`): 13:08:43's
>    `pull --rebase --autostash` picked up release chain's "rewrite CHANGELOG retention policy
>    criteria and self-check", 13:19:34's `pull --tags` was fast forward; V0.8.5's release commit
>    (`:twisted_rightwards_arrows: V0.8.5: …` and its `:bookmark:` companion commit, CHANGELOG
>    generation and retention policy two commits) all in `v0.8.5` tag reachable range.
> 3. **Three rebase caused local hash rewrite (referencing old number will miss)**: `b3245aac9` →
>    `56ae469e1` (3.6.3 main), `415f0154b` → `ef2334de5` (3.6.2), `bdbe859e1` → `8e32e8636` (3.6.4).
>    Verifier with "`src/`, `tests/` tree diff empty" proved three are **pure rebase equivalent**
>    (content unchanged, only parent chain changed), this record uniformly uses **currently
>    reachable** hash.
> 4. **F2's unfixable part**: 3.5.1 that commit's commit message wrote "only implementation" is
>    history, **not rewritten**; correction can only be carried by "shared implementation
>    convergence that commit + code comment per fact correction"—this record keeps this sentence, to
>    prevent reader mistakenly thinking no one found that problem at that time.
> 5. **Three observation items (verified, not blocking, but registered in case to prevent being
>    treated as unhandled violation later)**: **O1** Head rename after convergence to shared
>    implementation coverage surface drills more into `MetaType` / `AssocType` / `Literal`, but
>    three coverage probes **all parse fail** (`Pipe` / `Ampersand` / old `Type[...]` syntax;
>    `ast::Type::Union` has no construction point in parser) ⇒ **designed stronger, currently
>    unreachable**, not behavior change; **O2** `check-fanout.py` after fix round newly reports
>    `add_type_param: 2 files`, introducer is 3.6.3's closing commit (`inference/scope.rs`) **not**
>    this batch's fix round, review judged as **heuristic false report** (delegate wrapper + only
>    implementation's division, same form as existing `add_param`); **O3** `09`'s 3.6 row original
>    text still writes "1 three-level task" but actually has 3.6.1–3.6.4, 3.5 three rows missing
>    completion annotation—added by this closure round (see `09`).
> 6. **`src/driver/arms.rs`'s "Decision B1" comment not changed with code same commit (remaining
>    item, this batch not changed)**: 3.4.6 already changed `typecheck_orchestrated`'s FailFast
>    branch to **stash** `result.warnings` (originally directly discard), but that function's above
>    doc comment still writes "`result.warnings` discard—Decision B1", opposite to code fact after
>    3.4.6; same source another comment (`src/driver/tests/program.rs`'s phase table description)
>    **already rewritten same commit with 3.4.6** as "Decision B1 'dead code family warnings don't
>    enter compile path' hereby revoked". This batch `src/**` out of scope, so **didn't change**
>    that comment, registered to next team (belongs to AGENTS.md item 6 "carrier directly named by
>    ruling" kind debt). `02`'s runtime behavior table B1 row already locally corrected to current
>    state (dead code family warnings now via same criteria enter MultiFile `run` path).
> 7. **Member didn't execute `git push` / `fetch` / `pull`**: Sync and push uniformly by team leader
>    (in `git reflog show origin/dev` each commit is "update by push", member side no push record).
>
> **Eight, Uncovered surface (verbatim transcript, must not write "not tested" as "passed")**:
>
> - **t3's 7 items**: ① wasm32 target real compile not done; ② performance impact not measured
>   (3.4.6 added two phases, 3.6.x's indirect call overhead no benchmark number); ③ **W1005 per item
>   attribution not independently reproduced** (verifier's message form regex didn't cover its text
>   form, only checked new W code total 118 consistent); ④ 3.6.3 closing commit (declaration /
>   assignment not applied barrier) not separately mutated, c1 form not independently retested; ⑤
>   t8's 29 probe matrix not fully rerun (only retested three representative forms + M4 turn nail);
>   ⑥ 3.6.4 belongs to round 2 scope, round 1 not independently measured; ⑦ CI gate per convention
>   not waited in session, handed to closure link unified check.
> - **t11's 4 items**: ① qualified name callee with generic argument (`lib.g(Int)(3)`) and named
>   argument call function type value—no in-repo sample, this round not constructed retest; ②
>   `use lib` overall import without item import `p.get_x()` (existing E6006 boundary, earlier than
>   this task) not retest mutation state; ③ on `8e32e8636` not repeated full gate (covered by t9's
>   gate log and t3's all green on `81d5e414e` containing that commit); ④ CI handed to closure link
>   check.
> - **This closure round new registration**: This round's doc change **didn't rerun
>   `cargo test --all`** (change surface is `docs/**` three files, zero code surface); closure
>   round's gate is `npm run docs:build` + `check-docs-truth.py` + batch tree recycling +
>   `git status` check.
>
> **Nine, Batch tree recycling (gc and status raw output)**: `python scripts/dev/scratch.py gc` →
> `[scratch] 已删除 verify-p3-t3（释放约 13220.5 MB）` /
> `[scratch] 已删除 verify-p3-t4（释放约 14428.0 MB）` / `[scratch] gc 完成，共释放约 27648.5 MB`;
> `python scripts/dev/scratch.py status` → `[scratch] 没有批次树`; `git worktree list` → only main
> mount (`dev`). **Also deleted by gc two batch tree internal verification evidence**
> (`verify-p3-t3/` containing independent verification probes, patches, raw logs and conclusion
> text; `verify-p3-t4/` containing review round 2 evidence)—they are **registered batch
> directories**, recycled with tree; this record's sections three, four, five have already taken
> their conclusion, mutation table and gate numbers **as-is into record**, conclusion doesn't fail
> due to file deletion. **Unaffected archive** (not registered batch directory, gc doesn't touch):
> each batch's evidence directory `wbs35` / `wbs35repair2` / `wbs346` / `wbs36` / `wbs362` /
> `wbs363b` / `wbs364` / `wbs486` (in workspace `target/`, **not in repo**; conclusion already
> item-by-item into this record, workspace path not as reference source). Acceptance expression
> (resolvable): `git status` clean; `python scripts/dev/scratch.py status` = no batch tree;
> `git worktree list` only has main mount; no this batch added directory outside repo.
>
> **Ten, Document real build and commit hygiene**: This batch's check / annotation / registration /
> doc correction **accumulated into one** document commit (AGENTS.md item 8), landed in batch
> closure; generator regenerated doc merged into its functional commit. Before commit locally run
> `npm run docs:build` (AGENTS.md item 10⑨—markdownlint can't catch VitePress's Vue compile error,
> and commit only containing docs will be judged `no files to check` skip markdownlint) and
> `python scripts/ci/check-docs-truth.py`. **Commit number taken after push**: This record per
> historical record convention records each round actually happened commit and hash (**note division
> of labor with `09` check row**—`09`'s check / completion registration only writes "task id +
> evidence (gate number, issue number, file:line)" no commit number, because `pull --rebase` will
> rewrite local hash; this record is current round process history record, keeps hash and gives
> rebase before and after comparison, see "Seven" 3). **CI / Deploy Docs conclusion handed to
> "unified check" link**, not waited in session (AGENTS.md item 13)—this batch's CI run id and
> conclusion backfilled by team leader in unified check.
>
> **Eleven, Count table accounting review (this round caliber)**: `09` top's secondary / three-level
> count table follows "**base + increment**" caliber (same as 2026-10-18 registration). This round
> increment = secondary +2 (0.5 / 3.20, where 3.20 is post-migration new occupied number,
> pre-release audit group change 3.21 is net zero), three-level +7 (3.6.2 / 3.6.3 / 3.6.4 / 0.5.1 /
> 0.5.2 / 3.20.1 / 3.20.2). **Remaining pre-existing deviation (this round not corrected, for
> subsequent accounting review)**: Per "per secondary item parentheses' 'N three-level tasks' sum"
> caliber recalculate, this tree gets 162, differs from table head by 7—difference comes from
> 2026-10-18 registration newly added three-level but not synchronized rewrite parent item count
> annotation's 5 places (4.9 writes 7 actually 8, 3.14 writes 1 actually 2, 4.10 writes 3 actually
> 5, 4.4 writes 3 actually 4, S2 writes 1 actually 2) and 1 place reverse deviation (7b writes 6
> actually 5), another 1.2 / 1.3 / 9.1 / S7 four parent items don't carry that annotation (not
> counted in caliber). That difference **not introduced by this batch**, this batch doesn't change
> number for others' items, only registered here.
>
> **Twelve, Remaining pending (this batch registered, not fixed)**: ① `src/driver/arms.rs`'s
> "Decision B1" comment opposite to code fact after 3.4.6 (see "Seven" 6, `src/**` out of scope); ②
> 3.20.1 / 3.20.2 two existing false reports (after fix must restore `weak.md` / `test.md` natural
> writing and regenerate baseline); ③ 0.5.2 (pre-commit / CI wiring + 2600 file measured
> calibration + `test-specification` detailed rules); ④ #489 ureq 3 migration (dependency surface,
> exemption with hard removal condition); ⑤ 3.8 (native shadowing of user function name) and 3.12
> (runtime mode executor configuration propagation) pending ruling, 3.18 precondition pending
> P6—**all not in this team's scope**.

## P3 3.17 Closure (Round 2: 3.17.1 Manual FFI Named Argument + Second Binding Criteria Merge)

> **One, Scope and round relationship**: This section is **round 2**'s closure registration, scope =
> **3.17.1** (issue [#456](https://github.com/ChenXu233/YaoXiang/issues/456)), not including other
> secondary tasks. Round 1 (3.5 / 3.6 / 3.4.6)'s closure see previous section "P3 Debt Round 1
> Closure"; two rounds' relationship written here: **round 1's review round 1 (t4) that
> needs_revision (F1 / F2 / F3) already covered by t12 fix round + t13 review round 2 (verdict =
> pass), not repeated in this section**. This section's own criteria chain = implement **t15**
> (attempt 2 delivered; **previous attempt due to provider timeout, zero product**) → independent
> verification **t16** (verdict = pass) → adversarial review **t17** (verdict = pass, **no unhandled
> findings**, 4 observation items registered).
>
> **Two, Per item output and measurement numbers (3.17.1)**:
>
> - **Parameter name table same source**: `add_native_signature` changed to accept **signature
>   string** and return `Result`—`signature::parse_signature` produces type, unique exit
>   `signature::export_signature_param_info` same injection "parameter name + required minimum"; key
>   and signature table same (same name). In-repo zero caller so signature can be changed; doc
>   explicitly writes "callers who can only get `MonoType` should resolve signature string on their
>   own side then go through this API", and this function **must not** parse parameter name itself.
> - **Red first then green** (red state measured on parent commit `537b46b74`): Manual FFI surface
>   four forms + duplicate naming + legal call total 6, **5 failed, all diagnostics `[]`** (named
>   argument silently discarded); green state same file **12 all passed**—four forms E1014 / E1010 /
>   E1111 / E1111, duplicate naming E1015, legal named call and omittable optional position allowed.
> - **Three call surface same criteria same code**: Manual FFI (typecheck phase) / ordinary user
>   function (IR phase) / stdlib native export (`assert.assert`) three surfaces **per code equal
>   assertion** (E1014 / E1010 / E1111; E1015 see manual surface and user surface). Different
>   observation phase only reflects "who found first"; host native doesn't enter registry, IR phase
>   originally unresolvable (**earlier than this task's embedding surface limit**).
> - **Criteria single source**: `bind_args_to_params` **1 definition**
>   (`inference/arg_binding.rs:35`) + **6 calls** (typecheck 2 + ir_gen 4); second criteria
>   `build_arg_slots` **0 places** (deleted, `ir_gen.rs:516` only remaining comment explaining its
>   history); `ir_gen` only remaining pure transport `slot_order_arg_indices` (`ir_gen.rs:521`, 1
>   definition 2 calls, zero judgment). Line count: `ir_gen` **+58 / −92 (net −34)**; full repo 4
>   files **+448 / −102** (353 lines are new tests).
> - **Not regression**: 8 CLI probes (u1–u4 / d1 / d2 / n1 / n3) before and after mutation **per
>   code consistent**; `ir_gen` blob `0d837e9d…` ↔ `ffe1af8e…` check. Initial merge self-injury
>   fixed: IR side re-judges "missing parameter" mistakenly reports `t.pick2(a = 2)` legal overload
>   as E1010 (named_args unit test caught) ⇒ missing parameter judgment goes to typecheck, IR only
>   replays slot binding; after fix 19 named_args + 12 new unit tests all green. Corpus 348-348 zero
>   drift, IR snapshot zero drift.
>
> **Three, Independent verification (t16, frozen `c3082a160`; red state baseline `537b46b74`, both
> verified by `git merge-base --is-ancestor` as HEAD ancestors; probes all self-built)**:
>
> - **Red→green independent reproduction** (manual FFI surface): Self-built independent Rust unit
>   test `verify_host_ffi.rs` (4: unknown name E1014 / contention E1111 / named then positional
>   E1111 / legal named call allowed), via `scratch.py trial` temporarily hung in `tests/mod.rs`
>   (apply → test → apply -R → CLEAN). Two states each use **respective interface form**'s probe
>   variant (parent commit `add_native_signature(name, MonoType)` old signature / fix state
>   `(name, &str) -> Result` new signature—interface itself is fix surface): parent commit **3
>   failed / 1 passed**, 3 failed diagnostics all `[]` (`got []`), TEST_EXIT=101; fix state **4
>   passed / 0 failed**, TEST_EXIT=0. Red state another time in fix tree undo fix's mutation state
>   reproduce (`M7v2` merge patch: undo fix + undo implementer test + hang self-built old API
>   probe)—3 failed `got []`, TEST_EXIT=101, after restore four files blob consistent with HEAD,
>   `git status` CLEAN, after undo `ir_gen.rs` blob = `0d837e9d30de` (i.e. parent commit original
>   blob). CLI surface (user function + stdlib) two states per code consistent—as expected: these
>   two surfaces 3.16.1 already loud, this commit's fix surface only in manual FFI.
> - **Four forms per item** (CLI self-built probe, `c3082a160` measured): Parameter name error E1014
>   `has no parameter named x (available: a, b)`, missing required position E1010, positional and
>   named contention E1111, named then positional E1111; additional: duplicate naming E1015, stdlib
>   unknown name E1014, stdlib missing required position E1010 (help point name cond), stdlib
>   contention E1111, legal named call EXIT=0 output 3/3, omittable optional position EXIT=0.
> - **Criteria truly deleted not renamed**: `git show c3082a160:src/middle/core/ir_gen.rs` internal
>   `build_arg_slots` **0 hit** (parent commit same file 3 hit, definition at :521);
>   `slot_order_arg_indices` (fix state :521) per line check =
>   `sort_by_key(|&arg_idx| positions[arg_idx])` pure sort, no position/name/omittable judgment
>   residual, doc comment clearly states "no position judgment, no name judgment, no omittable
>   judgment"; deleted 71 line judgment block (parent commit :521-593, six rules: empty table loud /
>   position occupies slot / order illegal E1111 / unknown name E1014 / contention E1111 / enter
>   slot) no longer exists; `bind_args_to_params` 1 definition 6 calls (typecheck 2:
>   `expressions.rs:2854` / `:5653`; ir_gen 4: :7828 / :7926 / :8021 / :8351), consistent with
>   implementer report; parameter name unique exit = `export_signature_param_info` (environment.rs
>   manual / typecheck::mod stdlib / ir_gen lookup), no second self-parse.
> - **Self-injury regression nailed** (`t.pick2(a = 2)`): `git show c3082a160 --name-only` **doesn't
>   contain `named_args.rs`** (4 files: environment.rs / tests/mod.rs / native_param_names.rs /
>   ir_gen.rs) ⇒ nail test not changed; independent rerun that case 1 passed, named_args full set
>   **19 passed / 0 failed**.
> - **Not regression gate** (`c3082a160` tree): `cargo test --all --quiet` (4×4 caliber) EXIT=0—lib
>   **2806** + 2 ignored, integration **297**, 52/44/9/2/1/9 all green 0 failed; `cargo fmt --check`
>   / `clippy -D warnings` both EXIT=0; corpus diff (dual parameter) 348/348 difference empty;
>   `check-snapshot-drift.sh` PASS (204 snapshot zero drift); `check-docs-truth.py` EXIT=0; 3.16.1
>   four forms control (stdlib surface) E1014 / E1010 / E1111 / allowed.
> - **Uncovered surface (t16 five items, verbatim transcript)**: ① `testsuite.py run full` (CI
>   caliber resource profile channel) not rerun—implementer already per ruling fall back (exclusive
>   wave peak 8.11 GiB > budget 7.51 GiB, exit 2 is tool per design reject); this round with 4×4 and
>   2×1 two calibers' `cargo test --all` covered. ② Manual FFI surface (host-FFI) can only via Rust
>   test driver (`add_native_signature` in-repo zero caller, no CLI surface)—"host embedded
>   in-process real FFI call" not tested. ③ Variadic (`...`) named argument form not constructed
>   sample (no in-repo criteria sample). ④ `check-boundary.py` / `check-fanout.py` not rerun
>   (implementer already passed; no cross with behavior criteria). ⑤ CI (GitHub Actions) per
>   convention not waited, handed to closure link check.
>
> **Four, Adversarial review (t17, reviewed revision `c3082a16051cb1bfb8b84bdc62ec6bd29d90ff2d2`,
> parent = `537b46b7488a60b42cdc35110e0a413c80a58169`; all content use `git show <rev>:<file>` take,
> line number refetched on referenced revision)**:
>
> - **① Criteria single source reverse check**: `slot_order_arg_indices` (ir_gen.rs:521-527)
>   **doesn't reorder by name** (sort key is shared criteria given slot number, name only appears in
>   `arg_binding::bind_args_to_params` internal), **no missing parameter backstop** (absent slot
>   doesn't produce argument index at all), **doesn't distinguish omittable position** (IR two
>   places pass full optional, other two places pass true value `i >= required_min`). Full repo
>   "find position by parameter name / find member" grep per item check: **slot calculation only
>   `arg_binding.rs:76` one place**, other 7 hits each belong to other concerns (generic type
>   parameter's constraint interface check / const generic argument replacement / generic type
>   variable binding / generic type parameter replacement / const generic parameter lookup / method
>   definition lookup / struct field lookup).
> - **② Responsibility boundary (missing parameter only loud in typecheck, IR only replays slot
>   binding)**: Typecheck side `bind_named_args` (`expressions.rs:2854`) passes all required ⇒
>   missing any value parameter E1010; native export arm (`:5653`) passes true omittable position
>   `i >= required_min` ⇒ only true required position E1010, `?msg` omission allowed. IR side this
>   commit's two changes (`ir_gen.rs:7926` RFC-004 binding path / `:8021` overload resolution method
>   path) **uniformly pass full optional, only replay slot binding** (comment clearly writes
>   "missing parameter goes to typecheck" and "re-judge missing parameter per declaration table will
>   mistakenly report legal resolution as E1010 (`t.pick2(a = 2)` measured)").
> - **③ API shape**: environment.rs:990-993 clearly writes "this function **must not** parse
>   parameter name itself" and points to unique exit `export_signature_param_info`; :997-999 writes
>   "callers who can only get `MonoType` should resolve signature string on their own side then go
>   through this API".
> - **④ Behavior change surface**: Code four differences (same parameter named twice E1111→E1015
>   code change / unknown name E1014 text change / empty table E1014 text change / positional
>   argument exceeds IR side declaration table: silently discard→pass through), **measured reachable
>   form no diagnostic code / text change** (typecheck first judge, same code), IR product on 5
>   self-built probes **byte-by-byte consistent** (p1 / p5 goes through this commit's changed branch
>   and product unchanged, p2 / p3 / p4 two states same code, p9 unconstructable, p6–p8 source layer
>   unreachable); t15's 8 CLI probes two states per code consistent. Differences all land on
>   **originally failed** programs (program still fails) ⇒ D59 "doesn't change originally successful
>   program external performance" not touched.
> - **⑤ Test quality** (not just look at red/green): 12 new tests
>   (`typecheck/tests/native_param_names.rs`) per item to `test-specification`—naming
>   `test_<what>_<scenario>`, three-section AAA comment, assertions all with custom message, setup
>   repeat ≥3 times already extracted helper, helper internal panic with context; **no "only judge
>   exit code" weak assertion** (first `assert!(diagnostic non-empty)` then
>   `assert_eq!(per code equal)`, both sides empty won't pass hollowly); 3 negative cases (legal
>   named call allowed / omittable position omission allowed / explicit give omittable position
>   allowed) under M1 red state still pass ⇒ they **only prevent false report**, exactly their
>   purpose. **M1** (remove `native_param_names` injection) → `cargo test --lib native_param_names`
>   **9 failed / 3 passed**, TEST_EXIT=101 ⇒ new test **really nails fix**. **M2** (entire segment
>   rollback `ir_gen` merge) → `cargo test --lib` **2806 passed / 0 failed / 2 ignored**, exit 0 ⇒
>   merge surface **no test coverage**. `pick2` nail (existing `named_args.rs:253-283`, this commit
>   not changed) goes through **real full chain** and asserts **runtime value** `Int(20)` ⇒ **really
>   nails overload resolution** (not "no error"), and it's live gatekeeper for "IR must not re-judge
>   missing parameter".
> - **⑥ Limit registration**: Host native only enters `TypeEnvironment`'s two tables, **doesn't
>   enter `ModuleRegistry`** ⇒ IR generation parses namespace member per registry, so host native in
>   IR phase originally unresolvable (E3006), **earlier than this task's embedding surface limit**;
>   that limit with "why manual surface must be observed in type check phase" written in **test file
>   doc comment** and `environment.rs` field doc (not just in report).
> - **Eight gates on reviewed revision all rerun all green**: fmt --check / clippy -D warnings /
>   `cargo test --all` (**3220 passed, 0 failed**; lib 2806 + 2 ignored, integration 297) / corpus
>   diff 348-348 zero drift / IR snapshot 204 zero drift / docs-truth / check-boundary (0 exemption
>   0 violation) / check-fanout (new symbol no same-name multi-definition entry).
> - **Four observation items (verbatim register: verified, judged not constituting finding, no need
>   to dispose, but registered in case to prevent being treated as unhandled violation later)**:
>   - **O1**: Commit message claims "after this merge **full repo only shared criteria one
>     place**"—in terms of **slot binding** holds; but typecheck's **method overload arm** still has
>     one "lookup candidate table by name (E1014) + arity (E1010)" own judgment
>     (`inference/expressions.rs:5578-5600`), it uses **candidate group representative**'s parameter
>     table (`method_overload_param_names`), different from shared criteria's used **resolved
>     candidate** table. Judgment basis: that place **earlier than this commit, this commit didn't
>     touch** (`git show c3082a160 --name-status` only 4 files, no expressions.rs), and belongs to
>     "candidate matching" concern (different table, different semantics), not slot binding.
>     **Wording too broad, registered as observation**.
>   - **O2**: After `add_native_signature` changed to accept signature string, **no**
>     `MonoType → signature string` shared helper ⇒ future callers who can only get MonoType can
>     only each write one serialization. Judgment basis: parameter name / required minimum already
>     locked in doc at unique exit `signature::export_signature_param_info`; currently **zero
>     caller**; doc already written "callers who can only get `MonoType` should resolve signature
>     string on their own side then go through this API" ⇒ residual risk only in "how to construct
>     string" one paragraph, **low**.
>   - **O3**: This commit's **merge part** (ir_gen deduplication) **no any test coverage** (M2:
>     entire segment rollback ir_gen then `cargo test --lib` still 2806 passed all green). Judgment
>     basis: "deduplication" no new behavior to nail, nature; "single source" nature guaranteed by
>     grep + `check-fanout.py` gate. **Optional strengthening**: In `native_param_names.rs` add one
>     **IR layer** case (covering ir_gen:7926 / :8021 path), let merge surface also have guardrail.
>   - **O4 (attribution correction)**: Commit message and report attributed d2's E1010 to "**method
>     arm** arity pre-check first triggers"—measured that E1010's help is generic text "check the
>     argument count of the function call", **not** shared criteria "missing parameter" help
>     ("缺少必需形参：…") ⇒ trigger is **native arm**'s arity pre-check (`native_arity` table).
>     **Code behavior conclusion unchanged** (parent commit existing, this time not changed, already
>     verified), only **attribution wording needs correction**; and "`list.push` duplicate naming is
>     E1010" must qualify as "**arity mismatch + duplicate naming**"—only duplicate naming but arity
>     match (`list.push(list = v, list = 9)`) two states are both **E1015** (duplicate naming before
>     arity judgment).
>
> **Five, Gate and resource channel (one external link)**: This task **first time truly use**
> `python scripts/dev/testsuite.py run full` → **exit 2**—exclusive wave peak 8.11 GiB > local
> budget 7.51 GiB (gap 618 MiB), tool per design **refuses to silently skip** ⇒ fall back
> `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 cargo test --all --quiet` → **exit 0** (3220 passed / 0
> failed). That channel status chain **#486**: `full` in local machine due to peak extrapolation
> being conservative and unavailable, **calibration (2600 file from measured calibration) belongs to
> WBS 0.5.2**.
>
> **Six, Count table: this round not changed**—`3.17.1` already registered on 2026-10-15 and counted
> in table head ("2026-10-15: +2 = 3.17.1 / 3.18.1"), this round only check + annotation,
> **three-level count unchanged** ⇒ `09` top's secondary 67 / three-level 176 **not changed**. This
> round **doesn't add** count deviation (parent item 3.17's "1 three-level task" annotation
> originally correct); previous section "Eleven" recorded 6 places pre-existing deviation (parent
> item count annotation lagging) **unrelated to this round, not introduced by this round**, still
> per that section caliber leave for accounting review.
>
> **Seven, Batch tree recycling (gc and status raw output)**: `python scripts/dev/scratch.py gc` →
> `[scratch] 已删除 verify-p3-t16（释放约 10235.4 MB）` /
> `[scratch] 已删除 verify-p3-t17（释放约 10079.2 MB）` / `[scratch] gc 完成，共释放约 20314.6 MB`;
> `python scripts/dev/scratch.py status` → `[scratch] 没有批次树`; `git worktree list` → only main
> mount (`dev`). **Also deleted by gc two batch tree internal verification evidence**
> (`verify-p3-t16/`'s `EVIDENCE.md`, `verify-p3-t17/`'s `EVIDENCE-t17.md` and probes / patches / raw
> logs)—they are **registered batch directories**, recycled with tree; this section's third, fourth
> sections have already taken their measurement numbers, gate numbers, mutation table and four
> observation items **as-is into record** for archive, conclusion doesn't fail due to file deletion.
> **Unaffected archive** (not registered batch directory, gc doesn't touch): `wbs317` (implementer
> evidence: evidence-3.17.1.md / red-test.txt / trial-before-after.txt + fix-3171.patch /
> attribution table etc.; in workspace `target/`, **not in repo**; conclusion already in this
> section, workspace path not as reference source). Acceptance expression (resolvable): `git status`
> clean; `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only has main
> mount; no this batch added directory outside repo.
>
> **Eight, Document real build and commit hygiene**: This round's check / annotation / registration
> **accumulated into one** document commit (AGENTS.md item 8); before commit locally run
> `npm run docs:build` (AGENTS.md item 10⑨) and `python scripts/ci/check-docs-truth.py`,
> markdownlint zero warning. **Not push** (member not push, see AGENTS.md item 10⑧); **CI / Deploy
> Docs conclusion handed to unified check link** backfill, not waited in session.
>
> **Nine, Remaining items and registration**:
>
> 1. **Round 1 closure registration's remaining item ① closed**: `src/driver/arms.rs`'s "Decision
>    B1" comment opposite to code fact after 3.4.6, by `537b46b74`
>    (`:memo: docs(driver): correct B1 comment opposite to fact (3.4.6 closing omission)`) local
>    rewrite to current state fact (behavior neutral, only comment line; wording aligned with same
>    fact expression in `src/driver/tests/program.rs`).
> 2. **This round new registration**: O1–O4 four observation items (verified, not blocking,
>    registered in case); 0.5.2 (`full` channel's 2600 file measured calibration + pre-commit / CI
>    wiring); #489 ureq 3 migration and `.cargo/audit.toml` exemption's hard removal condition; 3.8
>    / 3.12 pending ruling, 3.18 precondition pending P6—**all not in this team's scope**.
>
> **Ten, CI gate conclusion (AGENTS item 13①, scheduled check 2026-10-21 backfill)**: This batch's
> last commit push tip = `1f5eb8665`
> (`:memo: docs(meta): verify list supplement three CI unique gates, and write "emission point" caliber`)
> → CI **39604109534 success**, Deploy Docs **39603305398 success**; that tip covers `e7a3fd4b1`
> (regenerate error code generation product) and its previous all commits. `e7a3fd4b1` itself that
> round's CI **39603305171 cancelled**—per item 10③ "consecutive push will cancel previous round",
> its conclusion taken by subsequent `1f5eb8665`'s success. Failure chain for traceability:
> `c3082a160` (**39587784184 failure**) and `190a0e7a3` (**39599653111 failure**) both same job
> "Error-code docs up to date (generator idempotent)", root cause = 3.17.1 delete judgment + line
> number translation without rerun generator; fixed by `e7a3fd4b1` regenerate, turned green after
> `1f5eb8665` closure, cumulative red 5 rounds, prompted AGENTS item 10⑤ wording fix (`1f5eb8665` is
> that fix).
>
> Check command and raw output (2026-10-21 13:19Z rerun): `gh run list --workflow ci.yml --limit 4`
> → `39604109534 success` (`1f5eb8665`) / `39603305171 cancelled` (`e7a3fd4b1`) /
> `39599653111 failure` (`190a0e7a3`) / `39587784184 failure` (`c3082a160`);
> `gh run list --workflow docs-deploy.yml --limit 3` → `39603305398` / `39599653238` / `39587784210`
> all success.

## Document Reference Governance and "Unresolvable Reference" Gate (2026-10-21)

> **Trigger**: When cleaning workspace evidence directory found, in `09` / `09a` / `CHANGELOG` /
> branch maintenance guide there are 24 places that take "in-workspace, excluded by `.gitignore`
> directories" (batch tree / probe / evidence directory) as **evidence source**—this kind of
> directory doesn't exist after clone, external contributors can't verify a single number (AGENTS.md
> meta rule ① and item 10④). One of them is 4.4 closure round's fix: change old path to point to
> another local path, equal to moving violation down one layer not eliminating.
>
> **One, Reference local inline (47 places = zh 24 + en 23)**: Conclusion, numbers, CI run id,
> delivery commit locally written into this article and `09`, source changed to in-repo path (record
> section anchor) or revision / run id / issue number. CI run id details and S9 seven group list
> **already inlined**, deleted is only redundant path pointer. en mirror regenerated by
> auto-translate bot based on zh source.
>
> **Two, New mechanical gate** `scripts/ci/check-docs-refs.py` (hung in `ci.yml`'s docs job): Track
> path-like tokens in text files' backticks and markdown links, those hit by `.gitignore` with
> concrete sub-path are red; placeholder form (rule statement) and bare directory name not judged;
> system runtime directories (package manager cache, build product, playground publish directory,
> local agent workbench) go through **item-by-item written reason** exemption table (26; details
> written in gate's own run report—as artifact upload in CI, not in repo). **Criteria can red can
> green**: self-test 9 (4 counterexample must red / 5 positive must green, including cross-batch
> boundary one); end-to-end mutation verification—change back one violation reference → gate red →
> restore → green → that file and pre-mutation byte-by-byte consistent.
>
> **Three, Rule wording supplement**: `AGENTS.md` item 10④ clearly "in-repo path' means git tracked
> in-repo file", item 9 supplement "in-workspace stuff is not evidence source", item 10⑤ local
> verify list supplement this gate (user 2026-10-21 approved).
>
> **Four, Two platform consistency (first version gate's defect and fix)**: First version gate
> **Windows local green, CI (Linux) red**—`git check-ignore` for "directory form path ending with
> `/`" two platform behavior different (Linux reports real rule, Windows reports one **empty
> pattern** = `.gitignore`'s empty line), only recognizing non-empty pattern when Windows side
> missed `docs/public/wasm/` (playground publish directory, RFC-037 reference). Fix: each token
> check three forms (with slash / without slash / without slash + probe sub-item), any form hits
> **non-empty** pattern counts as hit. After fix Windows and Linux dual run **same number**: 1122
> tracked text files / 5533 path-like tokens / 0 violation / 26 exemption / selftest 9/9. Lesson:
> gate script **at least must run once on two OS and checkout form** before daring to hang CI (CI's
> job log needs admin permission, when not available use "Linux real clone reproduce" positioning).
>
> **Five, CI evidence (AGENTS.md item 13①, scheduled check backfill)**: Fix commit `c7a0dd417` → CI
> run **39626722284 success**; previous commit `fff9580f8` → CI run **39623038087 failure** (red in
> newly added "Check doc references are resolvable", i.e. above platform difference, already fixed
> by `c7a0dd417`), same commit Deploy Docs **39623038228 success**, Auto Translate Docs
> **39623038027 success**; earlier commits (`ec2893e0d` / `ff2104b0c` / `ffd43b1d0`)'s CI runs due
> to consecutive push were concurrency **cancelled** (39621547581 / 39620372322 / 39619834870), per
> "later run's conclusion covers previous" take conclusion.
>
> **Five-2, This section in-repo that commit's CI evidence (2026-10-22 scheduled check backfill)**:
> This section in-repo commit `612c65322` → CI run **39798235412 success** (7 jobs all green,
> including newly added "Check doc references are resolvable" and "Error-code docs up to date" two
> steps), Deploy Docs **39798235484 success**; same commit Auto Translate Docs **39798235413
> failure**, failure in last step "Commit translations"—translation itself success (Translate
> documents / Format / Lint all success), but when pushing remote already pushed by this batch's
> code commit and bot's own previous auto-translate commit, `git pull --rebase` + `git push`
> blocked, **result is this article's en mirror not yet contains this section** (zh is only source;
> next push touching `docs/src/**/*.md` will again trigger that workflow supplement translate).
> Parallel 4 bugfix (`eb4124cf0` / `14e9b2704` / `96638ec9f` / `06ac6d687`)'s CI run **39797917813
> cancelled** (consecutive push cancelled by concurrency), per "later run's conclusion covers
> previous" take conclusion.
>
> **Five-3, Gap closure and this batch closure (2026-10-23 scheduled check backfill)**: After
> supplement push bot's Auto Translate Docs run **39904034074 success** (bot commit
> `449360a5a`)—**this article's en mirror already contains this section**, previous round's gap
> closed; same commit CI run **39904034164 success**, Deploy Docs **39904034075 success**. Previous
> round 39798235413's failure is bot's own race: its previous translation commit and this batch's
> code commit simultaneously push remote, `git pull --rebase` + `git push` rejected. **Operation
> lesson**: After push don't immediately push again—leave landing window for bot (this time about 15
> minutes interval succeeded).
>
> **Five-4, One commit brings parallel session change correction and count table review
> (2026-10-23)**: This section closure that commit `8a83485ef`'s **commit message only described
> this article's new section, but that commit actually also brought parallel session's two doc
> changes**—① `09`'s three-level count table head 177 → 182 (added 3.6.5 / 3.6.6 / 4.10.6 / 4.10.7 /
> 4.10.8, and changed 4.10.4 / 4.10.5 description); ② `09a` two `git worktree list` output's local
> path (`E:/git/YaoXiang <sha> [dev]`) cleared to "only main mount (dev)". Cause: team leader when
> committing executed `git add` on `09a` (absorbed other party's unstaged changes into staging
> area), while `09-execution-wbs.md` that commit's change was already in staging area.
>
> **Count table review conclusion: table head 182 not changed**. Per 09a §Fifteen ① caliber (per
> secondary kanban row parentheses' "N three-level tasks" sum), before change `449360a5a` on **sum =
> 170 / table head = 177 (diff +7)**, after change **sum = 177 / table head = 182 (diff +5)**—that
> commit **converges two calibers' difference from 7 to 5**, is correction toward self-consistent
> direction, not newly introduced inconsistency; residual 5 belongs to that row's two calibers
> historically not fully aligned existing drift (§Fifteen already recorded "secondary count row not
> touched…not change number for rounding"). **Therefore this commit doesn't make any count change**.
>
> **Six, Registration caliber**: This section is **doc governance** record, doesn't add WBS kanban
> row, `09` top count table unchanged.

## P4 4.10 Multi-file Semantic Gap Closure (Round 1: 4.10.4 / 4.10.5 + Framed 3.6.5)

> **One, Scope and round relationship**: This section = **4.10** secondary task's first round
> closure, scope = **4.10.4** (issue [#467](https://github.com/ChenXu233/YaoXiang/issues/467),
> rootless multi-file initialization per `use` dependency topology order), **4.10.5** (issue
> [#476](https://github.com/ChenXu233/YaoXiang/issues/476), rootless defining `main` not called exit
> code 1), **framed and merged 3.6.5** (issue
> [#479](https://github.com/ChenXu233/YaoXiang/issues/479), selective import function value after
> top-level rebind silently `void`; **framed from 4.10 team, user 2026-10-21 approved**). This batch
> also contains one **cross-secondary framed** std fix (`src/std/json.yx`'s `\uXXXX` subscript +
> `src/std/tests/json_ops.yx` long-term zero coverage), it doesn't belong to 4.10's task surface,
> only in "Nine" registered and as "test false pass" instance archive.
>
> **Two, Four fixes' landing content and evidence** (commit number takes **post-push** `origin/dev`
> hash; `09`'s check row per convention doesn't write commit number):
>
> - **4.10.4 (#467)**: Root cause = each unit's `init` segment per **source file path lexicographic
>   order** (discovery period's path lexicographic order sort, **not** "discovery order"—`09`
>   original row wrote "discovery order" already corrected this round; empirically renaming
>   dependent file can flip result, so "dependency first" never was invariant). Fix = in Linking arm
>   per `use` dependency compute **Tarjan SCC topology order** (within ring per path lexicographic
>   order as deterministic convention, between SCC per smallest unit index priority out queue) then
>   reorder unit order, `link_module_irs` split into "file table order" and "initialization
>   execution order" two concepts, **zero IR shape change**. Red first then green = change execution
>   order back to identity (fall back to discovery order) then 4 order cases all red, CLI reproduce
>   "entry reads dependency top-level value = `void`, exit 0" and "runtime E6007, exit 1" two forms;
>   after restore byte-by-byte return, 7/7 green. Corpus = added 2 multi-file fixtures, baseline
>   **+2 lines** and original 348 lines byte-by-byte unchanged; **IR snapshot zero drift**.
> - **4.10.5 (#476)**: Run entry changed to return **outcome enum** (completed / defined but not
>   called), CLI only sets exit code **1** based on outcome (2 reserved for CLI / config usage
>   error); two prompt text **verbatim unchanged**, no new second diagnostic; value type `main`
>   already evaluated during init not affected. Nail test 2 in `tests/integration/cli_e2e.rs`.
> - **3.6.5 (#479)**: Value path's alias fallback arm per "**module source + export kind**"
>   split—user module exported function → materialize as closure; std constant / native function →
>   maintain original zero-arg call form; **no new diagnostic code, `src/std` zero change**. Nail
>   test 3 (selective rebind / selective direct call / overall import rebind). Generation product:
>   `docs/src/reference/error-code/E1xxx.md` and `E8xxx.md` each 1 line number translation
>   (1789→1821 / 1976→2008), **generator regenerate, not manually modified**.
> - **std side (cross-secondary framed, user approved)**: `src/std/json.yx`'s `parse_string` in
>   `e == "u"` branch passes **backslash position** to `parse_uchar`, while `hex4`'s contract
>   requires reading 4 hex digits from `'u'` position ⇒ all `\uXXXX` report "invalid \u escape";
>   change to pass "**`'u'` position**" after BMP case adv=6, surrogate pair adv=12 arithmetic
>   unchanged, error position still takes backslash (user-visible position unchanged). Same commit
>   fixed `src/std/tests/json_ops.yx` this **never run test**, and also fixed fixture's own three
>   errors (low surrogate segment extra one hex digit, trailing `\u0039` never pieced into source
>   string, error positioning assertion written as "contains 2" while actual message is "invalid
>   literal at line 1, column 7"). **IR snapshot same commit regenerate**
>   (`src/middle/core/tests/snapshots/src/std/tests/json_ops.snap`), corpus baseline 1 line change.
>
> **Three, Review judgment (batch review round 1, different model family) and three uncovered
> branches' registration**: Review gives two already committed implementations **#467 = pass, #479 =
> pass** (two implementations themselves through independent verification both hold), at same time
> points out **two high level uncovered branches originally only existed in code comment, didn't
> enter WBS**—this round already per review opinion registered as **4.10.6** (import ring no
> warning, still reports misleading E6007) and **4.10.7** (native function as value silently does
> nothing), another registered **4.10.8** (pure yx's std export as value; review marks medium,
> **unverified**) and **3.6.6** (same family criteria convergence, refactor item). Review another
> points out **deferral reason's classification wrong**: deferral reason writes "needs new
> diagnostic code ⇒ D class pending ruling", but D59's branch puts "silent mismatch / silent no
> action" in **C class** (old behavior not successful, can fix along refactor) ⇒ not allowed to use
> unregistered D class label to silently defer. According to this, three already on 2026-10-23 got
> user ruling (issue [#491](https://github.com/ChenXu233/YaoXiang/issues/491) /
> [#492](https://github.com/ChenXu233/YaoXiang/issues/492) /
> [#493](https://github.com/ChenXu233/YaoXiang/issues/493) /
> [#494](https://github.com/ChenXu233/YaoXiang/issues/494)): **4.10.6 = compile time W code warning,
> ring not allowed silent** (warning surface "ring exists" or "ring internal read uninitialized
> value", whether allow `#[allow]` suppression, whether align with existing W10xx family "evidence
> level"—three **pending consideration**, must finalize before landing); **4.10.7 = truly support**
> (materialize native function as value, **no diagnostic code**, terminal state =
> `use std.io.{println}` + `f = println` + `f("hello")` really prints hello); **4.10.8 = C class, no
> code, can directly fix**; **3.6.6 = refactor item** (behavior unchanged).
>
> **Four, Overturned assumption (archive to avoid duplicate work)**: "same file two `use` spellings
> ⇒ dependency edge lost" **doesn't hold**—that form reports existing diagnostic **E5001** (`use`
> target not in unit set), not "silently per discovery order run". Review lists it as overturned
> assumption, so 4.10.4's "no dependency information three cases" each have deterministic
> disposition, no fourth silent channel.
>
> **Five, Task list and terminal state**:
>
> | Task | Subject                                                                            | Terminal state                                                                           |
> | ---- | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
> | t1   | Reproduce and position: two P1's red state forensics + #467 fix feasibility ruling | completed                                                                                |
> | t2   | #467 initialization topology order (implement + nail test + corpus)                | completed (3rd attempt closure)                                                          |
> | t3   | #467 independent verification                                                      | completed, verdict = pass                                                                |
> | t4   | #479 selective import + top-level rebind (implement + nail test)                   | completed (2nd attempt closure)                                                          |
> | t5   | #479 independent verification                                                      | completed, verdict = pass                                                                |
> | t6   | #476 exit code 1                                                                   | **failed** (provider side stream interruption cumulative, 6 attempts)                    |
> | t7   | #476 independent verification                                                      | **cancelled** (cross blocking + framed timing, taken over by team leader then cancelled) |
> | t8   | Batch review (#467 / #479 two implementations)                                     | completed, verdict = pass ×2                                                             |
> | t9   | Batch closure (check + 3.6.5 framed + record + gate + recycle batch tree)          | completed                                                                                |
>
> **Six, Per role attempt accounting and provider side failure disposition**:
>
> | Role         | Undertake                                              | Attempt accounting                                                                                                                                                                                         |
> | ------------ | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | Probe        | t1 (only read forensics), t6 (#476 implement takeover) | t1 one time; t6 **6 attempts not succeeded**—during multiple provider peak stream interruption (`PI_AI_ERROR`, 2064 "server briefly busy, usually recover within 1-5 minutes"), task terminal state failed |
> | Implement A  | t2 first dispatch, t4 takeover, t6 takeover review     | t4 one time (attempt 2); t6's implement finally by its review and supplement, with t4's "clean tree full" plan checked in together                                                                         |
> | Implement B  | t4 first dispatch                                      | attempt 1 due to provider side **400 `invalid_request_error`** (in thinking mode `reasoning_content` must echo) judged failed → reassign                                                                   |
> | Implement C  | t6 period implement takeover                           | consecutive two same **400 `invalid_request_error`** ⇒ per AGENTS item 11 "change personnel" disposition: **remove from team**, its task sent back to shared pool then reassign                            |
> | Implement D  | t2 takeover                                            | attempt 2                                                                                                                                                                                                  |
> | Implement E  | t2 closure, t6 one attempt                             | t2 3rd attempt completed; t6's one attempt also due to same 400 stream interruption failed                                                                                                                 |
> | Verification | t3, t5                                                 | each 1 time, verdict both pass                                                                                                                                                                             |
> | Review       | t8                                                     | 1 time, verdict = pass ×2                                                                                                                                                                                  |
> | Closure      | t9                                                     | 1 time                                                                                                                                                                                                     |
>
> **Provider side stream interruption and change personnel disposition caliber** (this round
> empirically reusable): ① **Stream interruption not defect evidence**—task interrupted by stream
> interruption must not judge failure conclusion based on "zero product", can only judge "not
> completed" and reassign; ② After change personnel **don't rerun rounds with existing conclusions**
> (AGENTS item 12: changing personnel will send back removed member's all tasks to shared pool,
> already ruled rounds shouldn't be rerun by scheduler), this round sent back t6 / t7 directly
> reassign or cancel, no duplicate work; ③ **Zero product during stream interruption** member always
> first look at disk state then decide takeover side (this batch multiple times appeared "implement
> already in working tree, just no one claimed" situation).
>
> **Seven, Repo concurrent write and pre-commit stash window (this round two instances)**:
>
> 1. **Concurrent write**: During batch main working tree was **another concurrent session**
>    written, its staging area already contained `.github/workflows/ci.yml` and
>    `scripts/ci/check-docs-refs.py` etc. **other people's files**. Disposition = **commit authority
>    closed to team leader**: during batch all members **not allowed to execute `git commit`**
>    (`git add` still per explicit path), changes only stay in working tree, commit uniformly by
>    team leader—otherwise one party's commit will roll in others' staged content. This batch's four
>    fixes finally by team leader in four independent commits checked in is because of this.
>    **Closure commit's collision instance** (this round empirically): Closure commit and team
>    leader's concurrent commit collided—team leader that `git commit` read **this round's two
>    staged documents**, rolled this round's content into its own commit (commit message followed
>    team leader's expression, only described "governance section"'s CI backfill). Disposition = use
>    `git commit --amend` change message to **simultaneously cover two places content** (still
>    **one** commit, **one** message), and register this instance here. **Lesson**: Before commit
>    must confirm index only has **own path** (`git status --short` first column all `M ` and path
>    belongs to this task)—concurrent session's staged content will be rolled away by others'
>    `git commit` (this is exactly this section item 1's disposition direction: during batch commit
>    authority closed to team leader).
> 2. **Batch directory deleted by third party**: `scratch.py`'s `rm` / `gc` will **entire batch
>    directory** recursively delete (that directory internal report and probe also disappear). This
>    round empirically: batch directory together with internal positioning report and all probe logs
>    during implementation period were deleted (**attribution unverified**: executor unknown).
>    Disposition = report and probe change to put **outside batch directory**, and key numbers and
>    conclusion **directly write into task output** (not depend on workspace file as source); batch
>    tree rebuilt then measuring side per frozen revision takeover.
> 3. **pre-commit stash window**: This round instance = commit std side that commit (amend
>    supplement IR snapshot) period, pre-commit hook stashed **all unstaged changes**, ran then
>    restore ⇒ parallel read working tree member saw "only staged files" false appearance (this
>    round had member accordingly report "changes lost"). Disposition per AGENTS item 10⑦: ①
>    **Before commit first give unstaged changes patch backup** (this round instance's three files
>    sha256 = `02e89141…` / `3736961f…` / `98eee1d7…`, after restore one by one same); ② During
>    commit period others only per **frozen sha** measure, not read working tree; ③ After each
>    commit complete use `git status` check remaining changes integrity.
>
> **Eight, Local hash rewrite comparison** (this batch's four commits during push experienced
> `git rebase` and `git commit --amend` two kinds of rewrite; `09`'s check row per convention
> doesn't write commit number, this record keeps **post-push** `origin/dev` hash and gives rewrite
> chain):
>
> | Content                             | Pre-push local → post-push `origin/dev`                                                 |
> | ----------------------------------- | --------------------------------------------------------------------------------------- |
> | 4.10.4 (#467)                       | `950a5838e` → (rebase) → `eb4124cf0`                                                    |
> | 3.6.5 (#479)                        | `0ef2e8887` → (rebase + amend) → `14e9b2704`                                            |
> | std side (`\uXXXX` + `json_ops.yx`) | `48bc37af0` → (**amend supplement IR snapshot**) → `c67de7e2f` → (rebase) → `96638ec9f` |
> | 4.10.5 (#476)                       | `47c2d5292` → (rebase) → `06ac6d687`                                                    |
>
> `48bc37af0` → `c67de7e2f` this amend's content = take **regenerated IR snapshot**
> (`src/middle/core/tests/snapshots/src/std/tests/json_ops.snap`) supplement in same commit:
> original commit only contains `src/std/json.yx` / `src/std/tests/json_ops.yx` / corpus baseline
> three files, after amend four files—i.e. "change IR behavior must same commit regenerate baseline"
> red line fulfilled in **same commit**, not leave one "baseline debt". Evidence =
> `git reflog --date=iso` (`commit` → `commit (amend)` → `rebase (pick)` three segments can read one
> by one).
>
> **Nine, Corpus / IR baseline current byte re-verification + this round two new findings**:
>
> - **Current byte re-verification** (before closure on `origin/dev` head rerun):
>   `tests/baselines/corpus-parity.jsonl` = **350 lines**, sha256 =
>   `76bd7a1cc82797fe0c2fb4f3406506ad65e7fe268d15b7d807448f4a30b5118b`; on that byte rerun corpus
>   diff probe, product and baseline **byte-by-byte same** (diff empty) ⇒ current byte zero drift.
>   **Note**: Two verification rounds (t3 / t5)'s judgment bound to **earlier** corpus byte
>   (`41cc0343…`), different from current `76bd7a1c…`—review already pointed this difference, this
>   round per current byte re-verify and register; thereafter reference corpus number must first
>   confirm referenced byte.
> - **New finding ①: `src/std/tests/json_ops.yx` long-term zero coverage**—that file defines `main`
>   but **never called**; script mode top-level statement is program body ⇒ 15 `check_*`'s
>   assertions **never executed once**, exit code always 0, i.e. typical **test false pass**
>   instance. `std.json`'s `\uXXXX` defect therefore long-term unexposed. This round fixed
>   (supplement `main()` call + fix fixture's own three errors + fix `std.json`'s subscript), after
>   fix that file's 15 check's assertions all really run and pass (exit 0). **Lesson**: "has test
>   file" doesn't equal "criteria executed"—under script form "defined `main` but not called" test
>   file is **false green** source, should serve as one independent check item (control: #476's
>   "defined `main` not called" under rootless form exit 0, also makes this kind of file long-term
>   use exit 0 to pad count).
> - **New finding ②: Ring import no W code warning**—4.10.4 made ring internal order
>   **deterministic**, but ring itself still silently tolerated: ring internal unit reads
>   uninitialized top-level value ⇒ misleading E6007 "type mismatch". Already registered as
>   **4.10.6** (issue [#491](https://github.com/ChenXu233/YaoXiang/issues/491); user 2026-10-23
>   ruling A = compile time W code warning, **not allowed silent**; W's warning surface pending
>   consideration).
>
> **Ten, Gate numbers**:
>
> - **Before closure on current `origin/dev` head locally rerun**: `npm run docs:build` → exit 0
>   (doc commit pre **real build**, AGENTS item 10⑨—markdownlint can't catch VitePress's Vue compile
>   error); `python scripts/ci/check-docs-truth.py` → exit 0; `python scripts/ci/check-boundary.py`
>   → exit 0 (violation 0, `pub(crate)` count guard baseline); `python scripts/ci/check-fanout.py` →
>   exit 0; `python scripts/ci/check-docs-refs.py` → exit 0 (1124 tracked text files, 5535 path-like
>   tokens, no unresolvable reference); error code generator rerun then `git diff --quiet` → exit 0
>   (generation product zero drift); corpus diff rerun → same as baseline byte-by-byte (350 lines).
> - **Each fix surface's own gate** (each verification round measured on frozen revision): #467 —
>   fmt 0 / clippy 0 / `cargo test --all` exit 0 (lib **2810 passed** + 2 ignored, integration 297
>   passed, 0 failed) / docs-truth 156/156 pages / boundary 0 / fanout 0 / docs-refs 0 / generator
>   rerun zero drift / **IR snapshot diff empty**; #479 — same caliber exit 0 (lib 2810, integration
>   **300** = 297 + new 3 nail test per name passed); #476 — **clean tree** full exit 0 (9 test
>   target all ok, **3227 passed / 0 failed**, including
>   `tests/yx_runner.rs::test_all_yx_files_pass`).
> - **Gate discipline one instance**: Full `cargo test --all` when "shared working tree with
>   uncommitted changes" once red at `tests/yx_runner.rs::test_all_yx_files_pass`
>   (`src/std/tests/json_ops.yx` exit 1), once recorded as "this batch's defect"; on **clean frozen
>   revision** retest that case passed ⇒ that red point belongs to "tree with #476 patch state"
>   measurement, not fix surface's attribute. **Lesson**: Full gate must measure on **isolated clean
>   tree**, otherwise will misjudge "shared tree state" as "delivery defect".
>
> **Eleven, Batch tree recycling (gc and status raw output)**:
>
> `python scripts/dev/scratch.py gc` → `[scratch] 已删除 wbs410（释放约 13268.2 MB）` /
> `[scratch] gc 完成，共释放约 13268.2 MB`; `python scripts/dev/scratch.py status` →
> `[scratch] 没有批次树`; `git worktree list` → only main mount (`dev`). **Also deleted by gc batch
> tree internal verification evidence** (each verification round's probes, mutation patches, raw
> logs and conclusion text)—they are **registered batch directories**, recycled with tree; this
> section's two, three sections have already taken their conclusion, numbers, mutation table and
> gate numbers **as-is into record**, conclusion doesn't fail due to file deletion. **Unaffected
> archive**: non-registered batch directory under evidence directory (not in repo; conclusion
> already item-by-item into this record, workspace path not as reference source). Acceptance
> expression (resolvable): `git status` clean (after closure doc commit);
> `python scripts/dev/scratch.py status` = no batch tree; `git worktree list` only has main mount;
> no this batch added directory outside repo.
>
> **Twelve, Document real build and commit hygiene**: This round's check / annotation / registration
> **accumulated into one** document commit (AGENTS item 8); before commit locally run
> `npm run docs:build` (AGENTS item 10⑨) and `python scripts/ci/check-docs-truth.py`, markdownlint
> zero warning. **Not push** (member not push, see AGENTS item 10⑧); **CI / Deploy Docs conclusion
> handed to unified check link** backfill, not waited in session (AGENTS item 13). Also: this round
> on-the-way changed two historical record residual **local absolute path** (`git worktree list`
> output's reference) rewrite to expression not containing local path (meta rule ①).
>
> **Thirteen, Remaining items and registration**:
>
> 1. **Pending landing (got user 2026-10-23 ruling, registered in `09`)**: 4.10.6 (issue #491,
>    compile time W code warning, not allowed silent; W's warning surface three pending
>    consideration), 4.10.7 (issue #492, truly support native function as value, no code), 4.10.8
>    (issue #493, C class, no code, can directly fix), 3.6.6 (issue #494, criteria convergence
>    refactor item, behavior unchanged).
> 2. **Known imperfect items**: ① **t6 task record terminal state failed, t7 cancelled** (provider
>    stream interruption + framed timing) ⇒ #476's **independent verification round not separately
>    run**; its verification evidence from "clean tree full exit 0 (3227 passed / 0 failed)" and
>    that round's exit code nail test (`tests/integration/cli_e2e.rs` two). ② Two verification
>    rounds (t3 / t5)'s judgment bound to earlier corpus byte, this round already per current byte
>    re-verify (see "Nine"), but **verification report itself's byte binding still old value**—must
>    explain when referencing. ③ Review **didn't personally rerun** two verification rounds'
>    mutation build (batch tree already released by multiple people, rerun needs exclusive tree +
>    one incremental build), estimate with "product check + blob byte-by-byte check + self-built
>    probe".
> 3. **Uncovered surface** (each verification round self-listed, this round not supplement): ① Ring
>    import's W code warning not implemented (already registered 4.10.6); ② `check` and LSP form not
>    separately probed (with `run` share Linking arm, library level nail test indirect cover); ③
>    Mutation only does "fall back discovery order" one direction—ring internal path order / minimum
>    index tie-break each failure form not do reverse evidence mutation (its behavior covered by
>    nail test assertion); ④ Multi-package / workspace form not in CLI probe; ⑤ #479 side: native
>    function as value, three-segment module path, non-existing export name, `ExportKind` other
>    variants, mutation round not run full, LSP path; ⑥ Review self-built 7 probes, "pure yx's std
>    module function as value" one marks **unverified** (already registered 4.10.8).
> 4. **Commit message scope rejected twice**: This batch two commits' initial scope took from
>    **source directory name** (`ir_gen` / `cli`), twice rejected by
>    `scripts/hooks/check-commit-msg.py` (its scope allow list doesn't have these two words),
>    changed to **subsystem name** `codegen` / `util` then passed (allow list: `frontend` / `middle`
>    / `backends` / `std` / `formatter` / `lsp` / `package` / `util` / `proof` / `driver` /
>    `codegen` / `monomorphize` / `lifetime` etc.). **Lesson**: scope take **subsystem** not source
>    directory name.
> 5. **This batch didn't touch existing debt** (not in this team's scope, transfer as is): 3.8
>    (native shadowing of user function name), 3.12 (runtime mode executor configuration
>    propagation), 3.18 (cross-module type identity, precondition pending P6), 0.5.2 (pre-commit /
>    CI wiring + resource file measured calibration), #489 ureq 3 migration.
