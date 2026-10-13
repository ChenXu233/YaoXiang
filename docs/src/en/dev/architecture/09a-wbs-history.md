# WBS Historical Notes (Landing Records · Audit · Adjudication Trail)

> This document is the **historical notes** for [09-execution-wbs.md](./09-execution-wbs.md)
> (construction task table): landing records, audit findings, adjudication trails, and errata for
> each phase. **Division of labor**: 09 only describes the **plan** (kanban, three-level task table,
> dependencies, risks, acceptance); this document only records **history** (what was done, what was
> found, how it was adjudicated), appended chronologically without rewriting history.

## P2 Establishing the Equivalence Criterion Baseline

> **P2 landing (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus green. **D38
> prediction fulfilled** — the green run empirically detected and fixed three real defects in ir_gen
> (see 06 §G6): the method locals table was truncated to the parameter count (local_count
> underestimated, runtime tolerated via Frame::set_slot), unannotated parameters were dropped by
> filter_map (three occurrences, params metadata misaligned with frame layout), and the match result
> slot relied on implicit Void initialization of the frame (now explicitly pre-initialized). The
> realistic semantics of jump target is the flattened instruction index (not the label-keyed form in
> the 07 text); CFG is built at the instruction level — registered in the verify.rs file header.
> **Commit correspondence**: ir_gen fixes = `a9ea199b`; verify main body = `d5b18390` (⚠ parallel
> stream COMMIT_EDITMSG collision caused its message to be mislabeled as "docs(rfc): RFC-027a…",
> while the content is actually verify.rs + six test files; it was buried in the middle of history
> and could not be amended, hence this record).
>
> **P2 narrowing part landing (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 are all in place, **the P3
> stop-bleeding channel is unlocked as a prerequisite**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; the judgment contract and the single-file corpus share the same
>   `TestFileSpec` (single point on the library side), with binary location / subprocess launch
>   extracted to `tests/common/` and yx_runner migrated in sync (prohibition 3).
> - 2.4.1 / 2.4.3: `tests/integration/proof_obligations.rs`. The trigger source uses the
>   `SumUpTo(3, r)` form nailed down by the rfc027 tests (return position / call site / binding
>   position) — the `Sorted(3)` annotation example in 02 would be substituted and evaluated under
>   the current code, directly Disproved, not showing Unproven silent; the `Sorted(3)` in the 07
>   pseudocode is illustrative only. The red-state empirical data is at the head of the test file.
> - The two red criteria are hung on `#[ignore]` as permanent per the 2.4.5 red-skeleton precedent:
>   the default suite is all green, `-- --ignored` reproduces the red state; **removing the
>   attribute will turn it green when P3 fixes land**.
> - The rationale for the deviation from the 07 (code, file, line) table in comparison normalization
>   (removing file/line, comparing only the error code set) is written in the 2.4.1 test comment.
>
> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> are **moved out of P2**: the `Program` / `Obligations` types they reference do not exist until P4,
> so P2 cannot compile. The former is 4.1.4 in P4 (no double-registration); the latter's `#[ignore]`
> red skeleton is merged into 4.3.1.
>
> **2.3.3 landing (2026-10-07)**: C4 mandatory coverage list checked item by item (07 §143), 10
> categories of semantic corpus layers fully covered and all within the corpus-parity.jsonl diff
> baseline (26 representative files, 26/26 hit, grep empirical) — ref/borrow/move 05-ownership full
> directory (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep, etc.), closure
> capture closures + closure_arg_inference + spawn_capture, currying curry_value_fix +
> curry_tail_expr, spawn 04-concurrency ×13, iterator for spawn_for and 16 other files, sum types
> sum_type_* + pattern_or_guard + match_call_scrutinee, existential mandatory points
> interface_dynamic_dispatch (Vec(Animal) existential type heterogeneous packaging triggers
> existential_coercions) + interface_rebind_dispatch, `?` / Try propagation question_propagation +
> option_try + try_methods + user_try_type, method overloading method_overload (+
> method_overload_ambiguous_err on the diagnostic diff surface), refinement constraints Drop
> sequences refined_* ×5 + 05-ownership-driven ReleasePlan (span key contract also has 2.4.4 red
> criterion permanent). The performance baseline `benches/pipeline.rs` is in the repo (07 §145 third
> smoke test, two of three): first `corpus_compile_frontend` full corpus compilation test 17.6s,
> `cli_cold_start_hello` process cold start 169.7ms; interpreter throughput is already covered by
> the benches/lib.rs hotpath group (interp_fib_recursive_27 / interp_loop_steady_10m), no duplicate
> construction. The regression gate check-perf-regression.sh is delegated to the P9 unified list
> (2.5).

## P3 Correctness Bug Fixes (Minimal Solution)

> **Error type annotation of landed rows** (retrospective annotation, does not alter history): 3.1 =
> B ｜ 3.2 = B (defensive empty arm, criterion turning green is borne by 3.1) ｜ 3.3 = A ｜ 3.4.1 /
> 3.4.2 = A (new W code) ｜ 3.4.3 = equivalent refactor (manual review) ｜ 3.4.6 / 3.4.7 = D
> (user-visible surface, already user-adjudicated) ｜ 3.4.8 ①② = C (main) + A (incidental three
> places) ｜ 3.5 / 3.6 / 3.7 / 3.8 = D (all already user-adjudicated).
>
> **P3 diagnosis (2026-10-12, requested by user)**: Conducting an account triage for the perception
> that "P3 entries keep growing".
>
> **One, first clarify the scale (the perception is amplified by two account defects)**
>
> 1. **Numbering collision**: `3.5.x` is shared by two task families — (a) 2026-10-08's **029g `pub`
>    visibility convergence** (7 tertiary: lexical and parser branches / AST `is_pub` / checker side
>    / dead code exemption convergence / peripheral echo / std and diagnostic text / language
>    reference convergence, all checked); (b) 2026-10-10 registered **3.5 higher-order type
>    parameter call path completion** (3 tertiary). The two share `3.5.x` ⇒ cross-reference
>    ambiguity (**2026-10-12 already re-numbered per user adjudication: 029g family changed to
>    `029g.x`**), and the 029g row was mistakenly counted into P3. **Pending adjudication**:
>    Renumber the 029g family to `029g.x` (or move it to its own phase), and synchronize all other
>    references in the document.
> 2. **Missed check**: The 4 tertiary of `3.7` (3.7.1–3.7.4) are all completed (d922a551 / 38bd4b9e)
>    but not checked ⇒ this entry is corrected in this round.
>
> **Two, P3 actual account (12 secondary rows)**
>
> | Row                                                                        | Status                                                                                                                                                                                     | Disposition recommendation                                                                                                                          |
> | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 3.1 / 3.2 / 3.3                                                            | Completed                                                                                                                                                                                  | —                                                                                                                                                   |
> | 3.4 Silent drop point supplement (8 tertiary)                              | 3.4.1 / 3.4.2 / 3.4.3 / 3.4.7 / 3.4.8 completed; **3.4.6 unlocked** (prerequisite 4.2.1 completed 2026-10-09 be60e01f); 3.4.4 and 3.4.5 **still blocked** (prerequisite 4.3.1 not started) | 3.4.6 **can be done now**; 3.4.4 / 3.4.5 unlocked with 4.3.1                                                                                        |
> | 3.5 Higher-order type parameter call path completion (3 tertiary)          | Not started (declaration side live, call side broken)                                                                                                                                      | Retain in P3; **does not block the refactor main line**, but is an "accepted feature not implemented", with priority higher than pure cleanup items |
> | 3.6 Function type value call path (1 tertiary)                             | Not started (semantics already cut (i))                                                                                                                                                    | **Retain**: correctness defect, clear criterion                                                                                                     |
> | 3.7 init section call name qualification (4 tertiary)                      | **All completed**                                                                                                                                                                          | Check and close in this round                                                                                                                       |
> | 3.8 native shadowing (1 tertiary)                                          | Not started                                                                                                                                                                                | **Stuck on decision** (first determine name resolution priority), not a scheduling issue                                                            |
> | 3.9 / 3.10 / 3.11                                                          | Completed (2026-10-12)                                                                                                                                                                     | —                                                                                                                                                   |
> | 3.12 Executor configuration propagation between runtime modes (1 tertiary) | Not started (Class D, user adjudication B: spin off a separate team)                                                                                                                       | **Recommend moving out of P3 → P8 (execution layer)**, pending adjudication                                                                         |
>
> **Four, the lever of 4.3.1 (main line landmark)**: `4.3.1 obligations.rs + assert_drained()` is
> simultaneously the prerequisite of **3.4.4**, **3.4.5**, and **4.5.1** (cross-layer contract
> PlanId-ization) ⇒ taking it once advances the main line and clears two P3 stranded items. 3.5
> (HKT), 3.6 (function type value call), and 4.6.1 / 4.9.1 all have **no unmet prerequisites** and
> are available anytime; 3.8 is stuck on a semantic decision (user-defined priority vs explicit
> diagnostic).
>
> **Three, mechanism recommendation (anti-recurrence)**: Newly registered secondary / tertiary rows
> **must carry two fields** — "owning phase" (which P it truly belongs to) and "whether it blocks
> the main line"; `registered-pending-scheduling` is no longer the default state, and if not marked
> it is considered unschedulable. This way, bloat is visible and sortable, rather than
> undifferentiated accumulation. **Pending adjudication**: Whether to write this item into the
> registration convention of this document and `src/dev/coding-rules.md`.
>
> **P3 landing (2026-10-06)**: 3.1.1 `proof_calls` narrowed visibility + sole getter (recorded at
> the time as `pub(crate)`; **2026-10-07 verified actual landing is `pub(super)`** — the `#434`
> adjudication referenced by code comments has no record on the docs side, see 3.4.3; fully private
> would require rewriting 11 construction literals, not taken in the stop-bleeding phase; closing up
> when P4 Obligations migration occurs); the execution mechanism is extracted into
> `src/frontend/proof_execution.rs` (prohibition 3 single point — cannot be placed in typecheck/,
> that would create L3→L4 reverse dependency); 3.1.2 completes **four entry points** per 02 evidence
> chain step 5 (the WBS row omitted compile_project `:99`, which must be supplemented and registered
> per 3.1.1 acceptance "criterion turns green"); 3.2.1 empty arm rewritten to bookkeeping
> (proof_calls thrown up) + diagnostic (the `into_result` established conversion path); 3.3.1 SOLVER
> slot made Option, Z3 missing conservatively degrades per `SMTResult::Unknown` (E2031 family
> diagnostics replace process crash). Two bug criteria remove `#[ignore]` to turn normal green,
> supplement the corpus fixture `proof-obligation-honored`; check-fanout inventory migrates
> accordingly (wiring point 19 = inventory). Empirical: criteria 2.4.1 / 2.4.3 turn green, full
> suite lib 2494 / integration 272 / corpus dual layer all green.
>
> **3.4 audit (2026-10-07, post-P3-landing recheck)**: Aiming at the class of defects "failure
> produces no signal" (see definition in 02 §41–52), without changing the code, a re-inventory
> located 8 more occurrences, all registered as 3.4.x in the above table. Method and evidence:
>
> - **Consumer-side inventory** (grep all repository read points field by field for the 16
>   obligation fields): `variant_ctor_calls` **zero consumers** — the only read at `checker.rs:1468`
>   is "feeding it into `TypeCheckResult` itself"; the IR side goes through shape detection at
>   `ir_gen.rs:7374`; `rfc011b.rs:659/688` only asserts the production end, exactly the "only
>   testing the production end, naturally immune to consumer-end defects" form critiqued in 02 §186
>   → 3.4.4. The result-level field `method_overload_ir_names` likewise has zero external consumers;
>   IR uses parallel `overload_resolutions` → 3.4.5.
> - **Stage coverage recheck**: Row 1 of the 02 §91 table (four entry points "no proof_execution")
>   has been fixed by 3.1.2, the current state is 5 places sharing one implementation
>   (`pipeline.rs:189`, `orchestrator.rs:141/340/507/1448`); but rows 2 / 4 / 5 remain unchanged
>   (multi-file `run` still does not output W1003, still does not run the dead code family) → 3.4.6;
>   LSP non-project-internal paths (02 §77 "manual lex→parse") likewise do not run proof → 3.4.7.
> - **Same-shape empty arm scan** (only four consumers of `Unproven` in the entire repo): three
>   refinement branches in checker (covered by 3.1), the ownership arm fixed by 3.2.1 (**source
>   unreachable**), and `environment.rs:422` (**reachable, zero diagnostics, zero bookkeeping, and
>   zero records across the whole repo's docs**) → 3.4.1, the only "all-unknown" item in this audit.
> - **Solver acquisition surface**: The three consumer forms of `default_solver()` differ
>   (`predicate.rs:32` process singleton / `checker.rs:1266` injected per `check_module` /
>   `ownership.rs:621` newly created per back-edge judgment) — 3.3.1 only eliminated the panic; the
>   **silent surface** of the other two belongs to 3.4.2, **singleton-ization and cache hit** belong
>   to P4's 4.4.2 (no double-registration).
> - **Existing ownership not counted into 3.4**: 2.4.4 (`ReleasePlan` span key mismatch, 168 files /
>   373 keys not consumed, Drop silently lost) is still P2 red state permanent, owned by 4.5.1 / D20
>   unchanged; 4.4.1 (layer order declaration opposite to actual, `equivalence` layer not in the
>   pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) are already in P4.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 prerequisites and can be landed in
> place at any time (recommend doing 3.4.1 first — the only "zero-record-across-the-repo" live
> defect, with the smallest change surface); 3.4.4–3.4.8 follow P4 execution; their criteria depend
> on the obligation ledger (4.3.1) or the unified Driver (4.2.x). **3.4 does not establish a new
> criterion system** and uniformly reuses the existing C2 / C3 and P9's 9.2 gate.
>
> **3.4.8 ③④ landing (2026-10-11, 8ecf56f93)**: ③ Add `Linking` at the tail of `CHECK_STAGES`
> (**pure merge**, skipping the entire `is_bin_role` entry check — the E3020 criterion still belongs
> to `RoleClassification` to prevent double reporting; another hard reason: the run path judges
> "whether there is a main" by merged IR, and under the collection semantics "entry's own error → no
> IR" would be mistakenly judged as missing main and hard-abort) → `Monomorphization` (check form);
> `ir_generation_check` retains unit IR (**in memory only**, for pure merge and mono consumption,
> not emitted / not written to disk / not producing snapshots — without retention the aggregation
> request is empty, mono gives false green with `NoObligations`); mono failure in Check goes through
> the per-file collection channel (diagnostics assigned to the entry file + `failed_stages`, not
> setting failure), MultiFile retains `fail_with`; `into_outcome` changed to "first take the
> collection, then judge the failure". ④ Empirically **zero production changes**: the whole-program
> BFS already covers cross-unit mutual recursion (pathological → E3005, well-formed three-layer
> chain executes correct values), delivered four sets of e2e + one mono unit test as living
> criteria.
>
> **Verification chain** (all independent of implementation): t3 (③, 9 self-built fixtures +
> byte-by-byte cross-check with HEAD binary), t4 (④, 10 fixtures + `max_depth` parameterized
> boundaries), t10 (F1 fix, 11 cases + dual binary comparison) all pass; t5 cross-form matrix
> (single/multi/check three forms + corpus 340/340 IDENTICAL + IR snapshot zero drift); t6 review
> PASS; test count 2643 → 2656 (only increase, no decrease).
>
> **F1 (found during verification, D59 A/B class, same-round fix)**: Under standalone multi-file
> check without a project root, the aggregation request name (`lib.f`) and the unit IR definition
> key (`f`) come from different sources → mono falls into silent `continue` of "native has no IR
> definition", check reports 0 error / exit 0. Fix: a reverse lookup jump by **real definition key**
> of each unit on the aggregation side (precise enumeration + verbatim equality + ambiguity
> abandonment, not suffix heuristic; project form not registered, existing path unchanged).
> **Erratum (same day)**: This reverse lookup jump was **deleted** in a subsequent regression fix;
> the root cause was resolved by decoupling the dual-use of `module_key` (adding `script_mode`) at
> the naming caliber — IR definition key = IR call name = aggregation request name =
> `{unit key}.{name}`. **Regression record**: after ③ landing, CI's
> `Check tutorial/guide code examples compile` turned red (258 blocks / 10 failures: 6 same-file
> same-name rebindings + 4 collisions with `<std/list>`, rootless check does not qualify triggering
> the conflict check of `link_module_irs`); fix = ir_gen split `script_mode` (qualified name ⊥
> Script / Bin, E3023 gate changed to look at it) + `ir_generation_check` always passes unit key per
> unit + `.with_script_mode(rootless)` + Script top-level same-name rebinding "latter takes effect"
> (consistent with existing run-path behavior); verification = doc gate 258/0, three-binary
> comparison (HEAD / ③ tree / fix tree) three types of collision fixtures, check form E3005 does not
> regress, E3023 not opened.
>
> **F2 / F3 (accepted)**: mono diagnostic assigned to the entry file item, span can cross units
> (same caliber as run path `path=entry`); Check form holds one ModuleIR per unit (in memory only,
> freed at check end).
>
> **Existing issues left for follow-up (not this-round regression)**: ① The E3005 help text still
> says "internal error; please report this issue", which does not match the resource protection
> semantics (precedent: `e3xxx.rs:47` once moved a similar "user writing error" out of E3005) —
> registered pending scheduling; ② The span rendering label of multi-file E3005 is misaligned
> (already reproduced with HEAD binary, pre-existing); ③ The request collection surface has a **type
> parameter position dependency** (generic calls where the type parameter is not in the value
> parameter position do not emit instantiation requests, mono records `NoObligations`, convergent
> mutual recursion is exactly this form) — same surface as 3.5 (HKT call path), registered to that
> row.

## External Track RFC-029g: Removing `pub` and Auto-Binding (#399

> **P3.5 landing (2026-10-08)**: All seven secondary tasks completed, the entire batch in one commit
> (independently revertible).
>
> - **Empirical vs plan mismatch (registered as-is)**: 029g.1.2 / 029g.1.3 acceptance originally
>   wrote "`pub f = ...` reports a **parse error**" — in fact after deleting `KwPub`, `pub` falls
>   back to an ordinary identifier, and the old form lands on the semantic layer reporting **E1001
>   Unknown variable: 'pub'** (span points to `pub`). Per RFC-039 **D27** "specifically detecting
>   removed syntax is an anti-pattern", **no** migration-specific diagnostic is added; the corpus
>   fixture `pub_declaration_err.yx` nails down E1001, the documentation (`syntax.md`) honestly
>   states this consequence.
> - **Design convergence (beyond RFC literal, requires review)**: After pub removal, the two fields
>   `project_refs` and `cross_file_refs` are semantically fully coincident (the same "in-package
>   reference union", prohibition 1 prohibits parallel representation) → merged into the single
>   `project_refs`; the two switches `set_exempt_pub` / `set_cross_file_refs` are deleted.
>   Behaviorally, the non-pub items of the Internal role change from "file-internal reachability" to
>   "reference pool judgment" (relaxed, in the direction of under-reporting); Lib tightens from "pub
>   absolute exemption" to reference pool judgment; Script top-level bindings that are not
>   referenced start to report W1001 / W1002 / W1004 / W1005.
> - **Dead table cleanup**: `env.exports` + `add_export` / `is_exported` / `is_visible` +
>   `auto_bind_to_type` + the exports writeback of `typecheck/mod.rs` are all deleted (production
>   end feeds the table, zero consumers across the repo).
> - **Test migration** (test count not reduced): add `lexer/tests/rfc029g_lexer.rs` (1); the "pub is
>   recognized" assertion in `parser/statements/tests/bindings.rs` is reversed to "pub is no longer
>   a modifier"; the 7 role semantic cases in `passes/tests/dead_code.rs` are rewritten (reverse 2
>   assertions); the 8 role cases in `tests/integration/cli_e2e.rs` are rewritten (the exemption
>   assertions for `lib` / `internal` are reversed to reporting assertions).
> - **Baseline**: IR snapshot **zero-diff** (`pub` does not participate in IR); the corpus diff
>   baseline adds only 1 entry (`pub_declaration_err.yx` → E1001), command
>   `cargo run --quiet --example corpus_probe` regenerates followed by manual review.
> - **Empirical**: lib 2556 / integration 272 / corpus 322 baseline all green; `cargo fmt` /
>   `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` /
>   `check_tracking.py` all green.

## P4 4.5 Cross-Layer Contract PlanId-ization (D20)

> **4.5 landing record (2026-10-13)**: 4.5.1 (`ReleasePlan` key `Span` → `PlanId`) and 4.5.2
> (`overload_resolutions` key `Span` → `PlanId`) landed in two functional commits, with closing
> (registration / WBS check / gate / push) completed by t7. Prerequisite = 4.3.1 obligation ledger
> (red state archive 173 files / 533 keys).
>
> **One, carrier adjudication (A: `PlanId` travels with AST node)**: Add `PlanId`
> (`src/util/plan_id.rs`, `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX`
> for synthetic nodes outside the parser), issued by the **sole allocator**
> `ParserState::alloc_plan_id`; carriers in two places — `Stmt.plan_id` (statement identity) and
> `Expr::Call.plan_id` (call-site identity), `Expr::call` is the only call constructor in the entire
> repo. The rejected option was the position-coincidence form "both sides derive the same identity
> from span" (the original defect of D20).
>
> **Two, two-level verification and review (members independent)**: Each functional commit has one
> independent verifier and one reviewer, **verifier / reviewer do not reuse implementer evidence**:
> 4.5.1 = t1 implement / t2 independent verification (external tree + 5 mutations) / t3 review;
> 4.5.2 = t4 implement / t5 independent verification (external tree + 5 mutations) / t6 review. The
> review time point and the reviewed revision both fall in immutable history (`git show`
> verifiable). **Provider rename caused t5 / t6 to change members and continue dispatching**: the
> original verifier / reviewer members could not continue due to the provider rename, per the
> orchestration convention, members were changed and continued with the reason written in the record
> (cross-round context preservation takes priority, member change only when compression is
> unavailable); after the change, the conclusion is consistent with the original round (PASS),
> without reusing implementer evidence.
>
> **Three, two defects found and fixed during implementation's self-check (t1, both inScope)**: ①
> The consumption exemption criterion was once hung on `UnitState.ir.is_some()` — the `check` form's
> Linking / Mono will take `unit.ir` away, real non-consumption in check form **silently passes**
> (mutation empirical: D41 red while CLI check exit 0); changed to read the **ledger's own
> completion marker**, and supplemented regression cases. ② Drop sequence drifts with process hash
> seed (partial order `sort_by` + `HashSet` iteration): snapshot criterion ran three consecutive
> times producing 106 / 120 / 114 line-count drifts; changed to **total order sort** (same name
> takes max depth + name ascending), then two consecutive runs had zero drift.
>
> **Four, criteria and evidence numbers**: After 4.5.1, `test_release_plan_spans_consumed` removes
> `#[ignore]` permanent (floor `examined ≥ 173` + no whitelist), 172 IR snapshots regenerated in the
> same commit (programmatic review: added 1083 lines Drop + 86 lines jump index translation, zero
> deletion, no rearrangement), corpus 342 entries diff empty; 4.5.2 adds
> `test_overload_resolutions_consumed` (difference set empty + evidence surface floor `examined ≥ 1`
> / `consumed ≥ 2`; empirical examined=1, consumed entries 2, no resolution skip 210), 9 resolution
> form pinboards, `cargo test --lib` 2712 passed / 0 failed, IR snapshot **zero drift**.
> Mutation-driven: consumption key offset by one ⇒ criterion red + real program
> `error [W1090] ... produced key not consumed` + corpus gate difference; restored to green.
>
> **Five, semantic correction classification basis (D59)**:
> `Type.method(positional arg, name = value)` **silently gave the wrong answer** before the change
> (empirical clean compile, runtime output `1` instead of `20`) ⇒ **Class C** (old behavior not a
> success), fix takes the "loud" option: after the change, the same program emits `error [W1090]` at
> compile time; the final-state semantics of "named arguments participate in resolution" is **Class
> D**, already registered as §3.14 and issue #452 left for a dedicated team. Another: `Drop` is a
> no-op in the interpreter (`backends/interpreter/executor/ops/control.rs` wildcard arm only
> `advance()`) — key migration does not change runtime behavior.
>
> **Six, F-1 / F-2 disposition (found by t5 independent verification)**: **F-1 (medium)** — the
> unbound consumption point (`ir_gen.rs:7766-7790`) is unreachable in the current corpus and test
> surface, and key-controlled correctness has no red state to falsify ⇒ per "known boundary",
> registered in the [02](02-stage-contract.md) §2 revision note fifth paragraph (preservation
> rationale + no new task); **F-2 (low)** — pointer drift ⇒ the `OverloadResolutions` doc comment in
> `types.rs` changed to precise pointers (production `inference/expressions.rs:4550`, consumption
> `ir_gen.rs:7697` / `:7772`), in a separate small commit, not mixed into the doc commit.
>
> **Seven, closing (t7)**: 02 obligation field table two rows changed to `PlanId` key + current
> consumption point line number (`:360` read / `:3103` use / `:1095` and `:1097` settlement marker);
> 07 criterion table and 04-ssa defect class 3 changed to past tense; 09 checks 4.5 / 4.5.1 / 4.5.2;
> CHANGELOG records one line of behavior change; `check-obligations.py` and `check_tracking.py`
> green, CI and Deploy Docs green. **Closing-found bookkeeping deviation**: the pointers
> `ir_gen.rs:357` / `:3102` / `:1096` in t3 / t6 reports differ from the code fact by one to three
> lines (actually `:360` / `:3103` / `:1095`+`:1097`, due to 4.5.2 insertion) — registered per the
> code fact, with a trace left in this note.
