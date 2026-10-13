# WBS History Notes (Implementation Record · Audit · Adjudication Trail)

> This document is the **historical notes** for [09-execution-wbs.md](./09-execution-wbs.md)
> (construction task list): implementation records, audit findings, adjudication trails, and errata
> for each phase. **Division of labor**: 09 only describes the **plan** (kanban, three-level task
> list, dependencies, risks, acceptance); this document only records **history** (what was done,
> what was found, how it was adjudicated), appended chronologically, without rewriting history.

## P2 Establish Equivalence Criterion Baseline

> **P2 implementation (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green
> run. **D38 prediction fulfilled**—the green run process measured and fixed three real defects in
> ir_gen (see 06 §G6 for details): method locals table truncated to parameter count (`local_count`
> underestimated, runtime relying on `Frame::set_slot` fallback), unannotated parameters dropped by
> `filter_map` (three places, `params` metadata misaligned with frame layout), match result slot
> depending on frame's implicit Void initialization (now explicitly pre-initialized). The actual
> semantics of jump `target` is flattened instruction index (not label keying as in the 07 text),
> CFG built at instruction level—recorded in verify.rs file header. **Commit correspondence**:
> ir_gen fix = `a9ea199b`; verify main body = `d5b18390` (⚠ parallel flow COMMIT_EDITMSG collision
> caused its message to be mislabeled as "docs(rfc): RFC-027a…", content is actually verify.rs + six
> test files, buried in history unamendable, hereby recorded).
>
> **P2 narrowed-scope implementation (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3
> bleeding-stop channel prerequisite unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; judgment contract uses the same `TestFileSpec` as single-file
>   corpus (single source of truth on library side), binary location/subprocess launch extracted to
>   `tests/common/` and yx_runner migrated simultaneously (prohibition three).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. The trigger source adopts the
>   `SumUpTo(3, r)` form nailed by rfc027 tests (return position/call point/binding position)—the
>   `Sorted(3)` annotation example in 02 would be substituted and evaluated in current code leading
>   directly to Disproved, not presented as silent Unproven, the 07 pseudocode's `Sorted(3)` is
>   illustrative only. Red-state measured data is in the test file header.
> - The two red criteria, per the 2.4.5 red skeleton precedent, are tagged with `#[ignore]` as
>   permanent fixtures: default suite is all green, `-- --ignored` reproduces red state; **removed
>   when P3 fix lands to turn green**.
> - The reason for the intentional deviation from the 07 (code, file, line) table in comparison
>   normalization (excluding file/line, comparing only error code sets) is written in the 2.4.1 test
>   comments.
>
> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **moved out of P2**: the `Program` / `Obligations` types they reference only exist in P4, P2
> cannot compile. The former is P4's 4.1.4 (no double registration), the latter's `#[ignore]` red
> skeleton is merged into 4.3.1.
>
> **2.3.3 implementation (2026-10-07)**: C4 mandatory coverage list checked item by item (07 §143),
> all 10 semantic corpus layers fully covered and all within corpus-parity.jsonl diff baseline (26
> representative files 26/26 hit, grep verified)—ref/borrow/move 05-ownership full directory
> (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep, etc.), closure capture
> closures + closure_arg_inference + spawn_capture, currying curry_value_fix + curry_tail_expr,
> spawn 04-concurrency ×13, iterator for spawn_for etc. 16 files, sum types sum_type__+
> pattern_or_guard + match_call_scrutinee, existential coercion site interface_dynamic_dispatch
> (Vec(Animal) existential heterogeneous packaging triggers existential_coercions) +
> interface_rebind_dispatch, `?`/Try propagation question_propagation + option_try + try_methods +
> user_try_type, method overload method_overload (+ method_overload_ambiguous_err in diagnostic
> diff), refinement constraints Drop sequence refined__ ×5 + 05-ownership driven ReleasePlan (span
> key contract has additional 2.4.4 red criterion permanent fixture). Performance baseline
> `benches/pipeline.rs` checked in (07 §145 three smokes of two): `corpus_compile_frontend` full
> corpus compilation first test 17.6s, `cli_cold_start_hello` process cold start 169.7ms;
> interpretation throughput is already covered by benches/lib.rs hotpath group
> (interp_fib_recursive_27 / interp_loop_steady_10m), no duplicate construction. Regression gate
> check-perf-regression.sh belongs to P9 unified list (2.5).

## P3 Fix Correctness Vulnerabilities (Minimal Solution)

> **Error type annotation for already-implemented lines** (retrospective annotation, does not modify
> history): 3.1 = B ｜ 3.2 = B (defensive empty arm, criterion turning green handled by 3.1) ｜ 3.3
> = A ｜ 3.4.1 / 3.4.2 = A (new W codes) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 /
> 3.4.7 = D (user-visible surface, already user-adjudicated) ｜ 3.4.8 ①② = C (main) + A (incidental
> three places) ｜ 3.5 / 3.6 / 3.7 / 3.8 = D (all user-adjudicated).

> **P3 diagnosis (2026-10-12, user request)**: An accounting triage for the perception that "P3
> entries keep growing".
>
> **1. First, clarify the scale (the perception is amplified by two accounting defects)**
>
> 1. **Numbering collision**: `3.5.x` is shared by two task families—(a) **029g `pub` visibility
>    convergence** from 2026-10-08 (7 third-level: lexical and parser branch / AST `is_pub` /
>    checker side / dead code exemption convergence / peripheral echo / std and diagnostic text /
>    language reference convergence, all checked); (b) **3.5 higher-order type parameter call path
>    completion** registered on 2026-10-10 (3 third-level). Both share `3.5.x` ⇒ cross-reference
>    ambiguity (**renumbered per user adjudication on 2026-10-12: 029g family changed to
>    `029g.x`**), and 029g's lines were mistakenly counted into P3. **Pending adjudication**: rename
>    the 029g family to `029g.x` (or move to its belonging phase), and synchronize other references
>    in the document.
> 2. **Missing checkmarks**: 3.7's 4 third-level (3.7.1–3.7.4) are all completed (d922a551 /
>    38bd4b9e) but not checked ⇒ this entry corrected this time.
>
> **2. P3 actual accounting (12 second-level lines)**
>
> | Line                                                                 | Status                                                                                                                                                                                | Disposition suggestion                                                                                                          |
> | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                      | Completed                                                                                                                                                                             | —                                                                                                                               |
> | 3.4 Silent drop point additions (8 third-level)                      | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 completed; **3.4.6 unlocked** (prerequisite 4.2.1 completed on 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (prerequisite 4.3.1 not started) | 3.4.6 **doable now**; 3.4.4/3.4.5 unlocked with 4.3.1                                                                           |
> | 3.5 Higher-order type parameter call path completion (3 third-level) | Not started (declaration side live, call side broken)                                                                                                                                 | Keep P3; **does not block refactor main line**, but is an "accepted feature not implemented", priority higher than pure cleanup |
> | 3.6 Function type value call path (1 third-level)                    | Not started (semantics already cut to (i))                                                                                                                                            | **Keep**: correctness defect, clear criterion                                                                                   |
> | 3.7 init segment call name qualification (4 third-level)             | **All completed**                                                                                                                                                                     | Check and close this time                                                                                                       |
> | 3.8 native shadowing (1 third-level)                                 | Not started                                                                                                                                                                           | **Stuck on decision** (first determine name resolution priority), not a scheduling issue                                        |
> | 3.9 / 3.10 / 3.11                                                    | Completed (2026-10-12)                                                                                                                                                                | —                                                                                                                               |
> | 3.12 Runtime-mode executor config propagation (1 third-level)        | Not started (Class D, user adjudicated Option B: separate team)                                                                                                                       | **Suggest move out of P3 → P8 (execution layer)**, pending adjudication                                                         |
>
> **4. 4.3.1's leverage (main line marker)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the prerequisite of **3.4.4**, **3.4.5** and **4.5.1** (cross-layer contract
> PlanId-ization) ⇒ doing it once advances the main line and clears two P3 stuck items. 3.5 (HKT),
> 3.6 (function type value call) and 4.6.1/4.9.1 all have **no unmet prerequisites**, all
> immediately doable; 3.8 is stuck on semantic decision (user-defined first vs. explicit
> diagnostic).
>
> **3. Mechanism suggestions (prevent recurrence)**: Newly registered second/third-level lines
> **must carry two fields**—"belonging phase" (which P it really belongs to) and "whether it blocks
> the main line"; `registered pending scheduling` no longer as default state, unmarked is treated as
> unschedulable. This way expansion is visible and sortable, rather than undifferentiated
> accumulation. **Pending adjudication**: whether to write this item into this document's
> registration convention and `src/dev/coding-rules.md`.

> **P3 implementation (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + single getter (at
> the time recorded as `pub(crate)`; **verified on 2026-10-07 the actual landing is
> `pub(super)`**—the `#434` adjudication referenced in code comments has no docs-side registration,
> see 3.4.3; fully private requires rewriting 11 constructor literals, bleeding-stop phase does not
> take this; closed off during P4 Obligations migration); execution mechanism extracted to
> `src/frontend/proof_execution.rs` (prohibition three single source—cannot be placed in typecheck/,
> that would create L3→L4 reverse dependency); 3.1.2 according to 02 evidence chain step 5
> supplemented **four entries** (WBS line number omitted compile_project `:99`, must be supplemented
> and registered per 3.1.1 acceptance "criterion turns green"); 3.2.1 empty arm changed to
> bookkeeping (proof_calls throws up) + diagnostic (`into_result` established conversion path);
> 3.3.1 SOLVER slot Option-ized, Z3 missing conservatively degrades to `SMTResult::Unknown` (E2031
> family diagnostic replaces process crash). Two vulnerability criteria removed `#[ignore]` to turn
> normal green, supplementary corpus fixture `proof-obligation-honored`; check-fanout inventory
> migrated accordingly (wiring point 19=inventory). Measured: criteria 2.4.1/2.4.3 turn green, full
> suite lib 2494 / integration 272 / corpus double-layer all green.

> **3.4 audit (2026-10-07, post-P3-implementation review)**: For the category of defects "failure
> produces no signal" (definition in 02 §41-52), on unchanged code re-inventoried and located 8 more
> places, all registered as 3.4.x in the table above. Method and evidence:
>
> - **Consumer-side inventory** (16 obligation fields grep all repo read points one by one):
>   `variant_ctor_calls` **zero consumers**—the only read `checker.rs:1468` is "self-injected into
>   `TypeCheckResult`", IR side uses shape detection `ir_gen.rs:7374`; `rfc011b.rs:659/688` only
>   asserts production side, exactly the "only tests production side, naturally immune to
>   consumer-side defects" pattern criticized in 02 §186 → 3.4.4. `method_overload_ir_names`
>   result-level field similarly zero external consumers, IR uses parallel `overload_resolutions` →
>   3.4.5.
> - **Stage coverage review**: 02 §91 table row 1 (four entries "no proof_execution") has been fixed
>   by 3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 unchanged (multi-file `run` still does not
>   output W1003, does not run dead code family) → 3.4.6; LSP non-project-internal path (02 §77
>   "manual lex→parse") similarly does not run proof → 3.4.7.
> - **Same-shape empty arm scan** (`Unproven` only four consumers repo-wide): checker three
>   refinement branches (3.1 covered), 3.2.1 fixed ownership arm (**source unreachable**), and
>   `environment.rs:422` (**reachable, zero diagnostics, zero bookkeeping, and zero registration in
>   entire repo docs**) → 3.4.1, the only "all-unknown" item in this audit.
> - **Solver acquisition side**: `default_solver()` three consumer forms differ (`predicate.rs:32`
>   process singleton / `checker.rs:1266` injected per `check_module` / `ownership.rs:621` newly
>   built per back-edge judgment)—3.3.1 only eliminated one panic; the other two's **silent
>   surface** goes to 3.4.2, **singleton-ization and cache hit** goes to P4's 4.4.2 (no double
>   registration).
> - **Not counted in 3.4, existing attribution**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys unconsumed, Drop silently lost) still P2 red-state permanent fixture, attribution
>   4.5.1/D20 unchanged; 4.4.1 (layer order declaration reversed from actual, `equivalence` layer
>   not in pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) already in P4
>   registry.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 prerequisite, can be landed in place at
> any time (suggest doing 3.4.1 first—only "zero registration repo-wide" living vulnerability,
> smallest change surface); 3.4.4–3.4.8 follow P4 execution, their criteria depend on obligation
> ledger (4.3.1) or unified Driver (4.2.x). **3.4 does not establish a new criterion system**,
> uniformly reuses existing C2/C3 and P9's 9.2 gate.

> **3.4.8 ③④ implementation (2026-10-11, 8ecf56f93)**: ③ Add `Linking` at the tail of `CHECK_STAGES`
> (**pure merge**, entire block skips `is_bin_role` entry validation—E3020 criterion still
> attributed to `RoleClassification` for double reporting; another hard reason: run path judges "has
> main" by merged IR, in collection semantics "entry itself errored → no IR" would be misjudged as
> missing main and hard-aborted) → `Monomorphization` (checks shape); `ir_generation_check` retains
> unit IR (**memory only**, for pure merge and mono consumption, no external emit/no disk write/no
> snapshot produced—if not retained then aggregation request is empty, mono takes `NoObligations`
> false green); mono failure in Check takes per-file collection channel (diagnostic attributed to
> entry file + `failed_stages`, not set as failure), MultiFile keeps `fail_with`; `into_outcome`
> changed to "first take collection then judge fault". ④ Empirically **zero production change**:
> full-program BFS already covers cross-unit mutual recursion (pathological → E3005, well-formed
> three-layer chain executes correct value), four e2e groups + one mono unit test delivered as
> living criterion.
>
> **Verification chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross-check with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundary), t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-shape matrix
> (single/multi/check three shapes + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increase, no decrease).
>
> **F1 (discovered during verification period, D59 A/B class, fixed in same round)**: In rootless
> standalone multi-file check, aggregation request name (`lib.f`) and unit IR definition key (`f`)
> come from different sources → mono drops to "native no IR definition" silent `continue`, check
> reports 0 error/exit 0. Fix: aggregation side added reverse lookup by each unit's **real
> definition key** (exact enumeration + literal equality + ambiguity abandonment, not suffix
> heuristic; project shape not registered, existing path unchanged). **Erratum (same day)**: this
> reverse lookup was **deleted** in subsequent regression fix; root cause now solved by `module_key`
> dual-duty decoupling (added `script_mode`) from naming caliber—IR definition key = IR call name =
> aggregation request name = `{unit_key}.{name}`. **Regression record**: After ③ landed, CI's
> `Check tutorial/guide code examples compile` went red (258 blocks / 10 failures: 6 items same-file
> same-name rebinding + 4 items colliding with `<std/list>`, rootless check without qualification
> triggers `link_module_irs` conflict validation); fix = ir_gen split `script_mode` (qualified name
> ⊥ Script/Bin, E3023 gate changed to look at it) + `ir_generation_check` per-unit always pass unit
> key + `.with_script_mode(rootless)` + Script top-level same-name rebinding "latter wins"
> (consistent with run path existing behavior); verification = docs gate 258/0, three-binary
> cross-check (HEAD/③ tree/fix tree) three collision fixtures, check shape E3005 no regression,
> E3023 not opened.
>
> **F2/F3 (accepted)**: mono diagnostic attributed to entry file entry, span can cross units (same
> caliber as run path `path=entry`); Check shape holds one ModuleIR per unit (memory only, released
> when check ends).
>
> **Pre-existing issues for follow-up (not this round's regression)**: ① E3005's help text still
> writes "internal error; please report this issue", inconsistent with resource protection semantics
> (precedent: `e3xxx.rs:47` once moved similar "user-written error" out of E3005)—registered pending
> scheduling; ② multi-file E3005's span render label misalignment (reproduced with HEAD binary,
> pre-existing); ③ request collection side has **type parameter position dependency** (generic calls
> where type parameter is not in value parameter position do not emit instantiation request, mono
> records `NoObligations`, convergent mutual recursion takes this form)—same surface as 3.5 (HKT
> call path), registered to that line.

## External Track RFC-029g: Delete `pub` and Auto-Binding (#399)

> **P3.5 implementation (2026-10-08)**: All seven second-level tasks completed, the entire batch in
> one commit (independently revertable).
>
> - **Measurement differs from plan (registered as-is)**: 029g.1.2/029g.1.3 acceptance originally
>   wrote "`pub f = ...` reports **parse error**"—in fact after deleting `KwPub`, `pub` falls back
>   to ordinary identifier, old writing hits semantic layer reporting **E1001 Unknown variable:
>   'pub'** (span points to `pub`). Per RFC-039 **D27** "specifically detecting removed syntax is
>   anti-pattern", **no new** migration-specific diagnostic; corpus fixture `pub_declaration_err.yx`
>   nails down E1001, documentation (`syntax.md`) honestly states this consequence.
> - **Design convergence (exceeds RFC literal, needs review)**: After pub deletion, `project_refs`
>   and `cross_file_refs` two fields have completely overlapping semantics (same "in-package
>   reference union", prohibition one prohibits parallel representation) → merged into single
>   `project_refs`, `set_exempt_pub`/`set_cross_file_refs` two switches deleted. Behaviorally,
>   Internal role non-pub items change from "in-file reachability" to "reference pool judgment"
>   (relaxed, false-negative direction); Lib tightened from "pub absolute exemption" to reference
>   pool judgment; Script unreferenced top-level binding now reports W1001/W1002/W1004/W1005.
> - **Dead table cleanup**: `env.exports` + `add_export`/`is_exported`/`is_visible` +
>   `auto_bind_to_type` + `typecheck/mod.rs`'s exports write-back all deleted (production-side
>   injection, zero repo-wide consumers).
> - **Test migration** (test count does not decrease): added `lexer/tests/rfc029g_lexer.rs` (1);
>   `parser/statements/tests/bindings.rs`'s "pub is recognized" assertion reversed to "pub is no
>   longer a modifier"; `passes/tests/dead_code.rs` 7 role semantic cases rewritten (2 assertions
>   reversed); `tests/integration/cli_e2e.rs` 8 role cases rewritten (`lib`/`internal` exemption
>   assertions reversed to report assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` does not participate in IR); corpus diff baseline
>   only adds 1 entry (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerate then manual review.
> - **Measured**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.
