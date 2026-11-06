---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a clause-gate sampling audit and the first list of retirement candidates'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the three gates (clause / incident history / proof) in
[Part Seven of coding-rules](./coding-rules.md), producing a rerunnable item-by-item decision table;
then run a stratified sampling audit of the clause gate (40 items), measure the proportion of tests
that “cite a clause but cannot support the assertion,” and recompute the distribution using the
**tightened criteria**. **This page and the generator itself are read-only and do not modify
tests**; test retirement is carried out in batches (one commit per batch, with the whole batch
revertible via `git revert`), and execution results are recorded in the “First Retirement Batch /
Second Retirement Batch” sections below and in `09a`.

Artifacts:

| Artifact                                  | Purpose                                                                                                                                   |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv`       | Item-by-item decision table (3413 rows + header), in the same directory as this page                                                      |
| `scripts/dev/test-gate-census.py`         | Generator (rerunnable; supports `--check` and `--selftest`)                                                                               |
| `scripts/dev/test-gate-census-audit.py`   | Sampling audit: reproduces the 40-item sample with a fixed seed (`--sample` / `--check`)                                                  |
| `scripts/dev/test-gate-census-audit.json` | Item-by-item decisions and rationale for all 40 items (the generator reads it and classifies “audit-confirmed genuine support” as Retain) |

## Criteria

### Scanning Surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 2939                 |
| `tests/integration`                                                      | 335                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 75                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **3413** (296 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                   | Meaning of a hit                                               |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | The first 25 lines of the file header contain a **verifiable entry**: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); files under `src/**/tests/` are considered together with their parent `mod.rs`                                                                                                                                                                                     | The test maps to a specific, nameable clause                   |
| Incident-history gate             | The test function body, function doc comment, or **file header** (such as `//! Regression context: issue #226`) identifies a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                              | The test pins an incident that has actually occurred           |
| Proof gate                        | **Cannot be determined by the machine** (“if it were replaced with another correct implementation, would it fail?” requires human judgment). Therefore, anything that misses both preceding gates is classified as “Pending adjudication,” not automatically as “neither passes”; the sole exception is a body containing **not a single assertion** (including no delegation to `assert*/check*` helpers and no `#[should_panic]`) | Only “Pending adjudication” or “No assertions” can be produced |

Rationale for tightening: the 0.6-9 clause gate only required the file header to **contain the
tokens `§`/`RFC`/`SPEC`**, so 2403 tests were classified as Retain. The 40-item stratified sample
measured under 0.6-10 found that **half of the citations could not support the assertions**. After
tightening, only the section-number form is recognized, and **having a citation still does not mean
that the citation supports the assertion**—all unaudited clause citations are classified only as
Pending verification.

Rationale for the file-header radius: this repository conventionally places “regression context:
issue #N” in the file header rather than repeating it in individual test comments; looking only at a
test’s own comments would miss the incident history of the entire file (measured: 7 items within the
test’s own radius versus 291 within the file-header radius).

### Categories Never Marked for Retirement

The following tests are **always retained** and do not enter the retirement candidate pool:

- Corpus `tests/yaoxiang/**` and project-shape `tests/project-corpus/**` (behavioral-criteria
  surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and crossing process boundaries);
- Tests whose names contain `unchanged` / `anchor` / `guard` / `preserv` / `baseline` (current-state
  anchors).

### Semantics of Dispositions

| Disposition          | Meaning                                                                                                                                                                                                       |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Retain               | Only three cases: the incident-history gate is hit; the proof gate applies (spec-shaped directories / current-state anchor names); or a clause citation is confirmed as genuine support by the sampling audit |
| Pending verification | A clause is cited (with a section number, or only an entire RFC without one), but it has not been audited / was judged irrelevant or generic—human verification is required before it can be retained         |
| Retirement candidate | Both gates are missed and the assertion surface contains only weak assertions (or no assertions)—a candidate is not a verdict; the proof gate still requires manual review                                    |
| Pending adjudication | Both gates are missed, but strong assertions are present; the machine gives no conclusion                                                                                                                     |

## Distribution After Tightening

| Gate                                                                       | Count    |
| -------------------------------------------------------------------------- | -------- |
| Clause (has a verifiable entry)                                            | 1713     |
| Incident history                                                           | 314      |
| Proof (spec-shaped criterion surface / current-state anchors)              | 447      |
| Pending adjudication (both gates missed; proof gate awaits human judgment) | 924      |
| Neither passes (no assertions anywhere in the body)                        | 15       |
| **Total**                                                                  | **3413** |

| Disposition          | Count    | Compared with 0.6-9                                               |
| -------------------- | -------- | ----------------------------------------------------------------- |
| Retain               | **776**  | 2897 (−2121)                                                      |
| Pending verification | 2121     | 0 (new tier)                                                      |
| Pending adjudication | 412      | 412 (unchanged)                                                   |
| Retirement candidate | 104      | 136 (21 tests retired in the first batch, 11 in the second batch) |
| **Total**            | **3413** | —                                                                 |

Two changes to the criteria must be explained:

1. **Composition of Retain**: 776 = 447 proof-gate cases + 314 incident-history cases + 15 cases
   whose clause citations were confirmed by the audit as genuine support. (Of the audit’s 20
   genuinely supported cases, the files containing the other 5 were already retained by the
   incident-history gate.) The 2121 cases in the original 2897 that were “retained merely for citing
   a clause” are now Pending verification.
2. **Incident-history gate 47 → 314**: this is not newly discovered incident history, but
   **relabeling**. Under 0.6-9, cases that hit both the clause and incident-history gates were
   recorded as clause cases. After tightening, a clause no longer automatically warrants retention,
   so the incident-history gate must catch these cases; their counts are folded into the
   incident-history gate. The total number retained is therefore unaffected (this batch of tests was
   also retained under 0.6-9).

File-level scope: of the 296 test files, **113** have a verifiable section number in the file header
(the old 0.6-9 criterion identified 221).

## Clause-Gate Sampling Audit (0.6-10)

### Sampling Criteria (Reproducible)

- **Candidate pool = the 2403 items classified as “Clause” by the 0.6-9 clause gate** (the old
  criterion is frozen in `test-gate-census-audit.py`; the sampling denominator must not change with
  the new criterion, or the results would not be comparable).
- **Strata = directory clusters**: under `src/`, take the first two levels (`src/frontend`,
  `src/middle`, …); group the rest by top-level directory (`tests`, `benches`, `tools`).
- **Allocation**: proportional to the number of items in each stratum (largest remainder method),
  with at least one item per stratum.
- **Selection**: after sorting within each stratum by (file, line number, test name), take a random
  sample using a fixed seed.
- **Seed = 20261106** (hard-coded in the script; changing the seed changes the sample, so the change
  must be documented and the audit rerun).

Stratified result: `src/frontend` 21 · `tests` 4 · `src/driver` 3 · `src/package` 3 · `src/backends`
2 · `src/formatter` 2 · `src/middle` 2 · `src/util` 2 · `tools` 2 · `src/ast` 1 · `src/lsp` 1 ·
`src/std` 1.

### Item-by-Item Decisions for the 40 Items

Decision criteria (hard-coded here to prevent inconsistent judgments from item to item):

- **Genuine support**: the citation identifies a specific section/note/decision number, and its
  **topic** is the same as that of the assertion target;
- **Irrelevant**: the citation identifies a specific entry, but that entry covers **something else**
  (the rationale column states what it actually covers);
- **Generic**: the citation points only to an entire document/chapter/a list of sections/a phase
  name, and none of them corresponds to the assertion.

| #   | Test                                                                        | File                       | Decision        | Rationale (what the cited clause actually covers)                                                                                                                                                                                               |
| --- | --------------------------------------------------------------------------- | -------------------------- | --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `test_compile_time_value_param_spec_table`                                  | `classify.rs`              | Genuine support | type-system §7.1 “compile-time value parameters” is exactly the assertion target: the truth table for value-position/type-position parameter classification is checked row by row.                                                              |
| 2   | `test_id_stable_under_subsequent_registrations`                             | `value.rs`                 | Genuine support | RFC-010 “runtime representation and equality” defines the sum-type identity convention: identity is determined solely by the name; the assertion verifies that the ID does not change after subsequent registrations.                           |
| 3   | `test_driver_run_check_rootless_multi_file_well_formed_generic_stays_clean` | `check_rootless_mono.rs`   | Genuine support | 02-stage-contract §3, note #14, point ③, states that monomorphized names come from the same source; the test is precisely a pinning test for that note (both the name and the assertion match).                                                 |
| 4   | `test_multifile_benign_import_cycle_message_predicts_no_runtime_failure`    | `import_cycle.rs`          | Genuine support | RFC-013’s W1xxx warning code + user decision A (cycles must not be silent); the assertion checks that a benign cycle produces exactly one W1007 and predicts no runtime consequence.                                                            |
| 5   | `test_obligation_config_disabled_skip_is_not_owed`                          | `obligations.rs`           | Genuine support | 02-stage-contract §2 “obligation ledger”; the assertion checks that a ConfigDisabled skip does not create an outstanding obligation.                                                                                                            |
| 6   | `test_format_type_bool`                                                     | `types.rs`                 | Genuine support | formatter spec §9 type annotations; the assertion checks that the type name is emitted verbatim.                                                                                                                                                |
| 7   | `test_fstring_multiline_single_quotes_inside`                               | `fstring.rs`               | Genuine support | syntax §1.6.3 string literals; the assertion checks that a single quote inside multiline f-string content does not terminate it.                                                                                                                |
| 8   | `test_float_underscore`                                                     | `literals.rs`              | Genuine support | syntax §1.6 literals (1.6.2 floating-point numbers) covers underscore separators; the assertion checks that 1_000.5 is lexed as 1000.5.                                                                                                         |
| 9   | `test_empty_unicode_escape_is_error`                                        | `literals.rs`              | Genuine support | syntax §1.6.3’s closed set of escapes; the assertion checks that an empty \u{} escape must produce an error.                                                                                                                                    |
| 10  | `test_probe_accepted_const_type_is_registered`                              | `probe.rs`                 | Genuine support | RFC-039 P6 6.4 / WBS 6.6.9 is exactly “constant-parameter classification obeys only the injected probe”; the assertion checks that N is registered when the probe recognizes Int.                                                               |
| 11  | `test_direct_eval_with_bound_variable_proved`                               | `predicate.rs`             | Genuine support | RFC-027 §3-4 (path-condition propagation / compile-time proof pipeline); the assertion checks that, with b=5, b>0 evaluates directly to Proved.                                                                                                 |
| 12  | `test_unused_type_is_not_an_entry_point`                                    | `dead_code.rs`             | Genuine support | RFC-011 §7 dead-code elimination mechanism; the assertion checks that an unused type definition is not a reachable root.                                                                                                                        |
| 13  | `test_display_forall_multiple_vars_no_trailing_space`                       | `ast.rs`                   | Genuine support | RFC-027 §8 SMT solver; the assertion checks the printing format for a forall with multiple variables.                                                                                                                                           |
| 14  | `test_dep_graph_remove_dependant_clears_edges`                              | `rfc027_phase31_dep_vc.rs` | Genuine support | RFC-027 Phase 3.1’s TypeDepGraph; the assertion checks that removing a dependent clears its edges and constraints together.                                                                                                                     |
| 15  | `test_condition_bool`                                                       | `conditional.rs`           | Genuine support | RFC-011 §5 conditional types; the assertion checks the evaluation and determinate state of TypeCondition::Bool.                                                                                                                                 |
| 16  | `test_convert_expr_to_const_expr_int_literal`                               | `const_eval.rs`            | Genuine support | RFC-011 §4 compile-time generics; the assertion checks that an Int literal is converted to ConstExpr::Lit.                                                                                                                                      |
| 17  | `test_definition_on_binding_site_returns_none_precise_only`                 | `definition.rs`            | Genuine support | RFC-039 D55 (precise-only; name-based fallback is prohibited); the assertion checks that the definition site itself returns None when it has no reference record.                                                                               |
| 18  | `test_type_consistent_arith_passes`                                         | `verify.rs`                | Genuine support | WBS 2.1 / RFC-039 P2’s IR static-validator criteria (the first layer of 07-equivalence-oracle); the assertion checks that type-consistent arithmetic passes.                                                                                    |
| 19  | `test_verify_loose_corpus_green`                                            | `verify.rs`                | Genuine support | The same validator criterion; the assertion checks that the corpus runs entirely green.                                                                                                                                                         |
| 20  | `test_parse_tag_tolerates_query_and_fragment`                               | `toolchain.rs`             | Genuine support | RFC-037 §installer support; the assertion checks that the query/fragment are stripped from the release URL.                                                                                                                                     |
| 21  | `test_release_unset_register`                                               | `execute.rs`               | Irrelevant      | #299 §2 covers the NewArray fixed-length array instruction (allocation, Void placeholder, bytecode round-trip); the assertion is that “executing Release on an unset register does not panic.”                                                  |
| 22  | `test_fstring_segment_interpolation`                                        | `ast.rs`                   | Irrelevant      | The citation is to “language spec §3 (AST structure),” but syntax §3 is the statements chapter (3.1 statement classification … 3.11 program entry and top-level statements); the assertion concerns the field of FStringSegment::Interpolation. |
| 23  | `test_expr_for`                                                             | `ast.rs`                   | Irrelevant      | The same “§3 (AST structure)” citation; syntax §3 covers statements, while the assertion concerns the AST field of Expr::For.                                                                                                                   |
| 24  | `test_add_var_without_mut_stores_immutability`                              | `scope.rs`                 | Irrelevant      | The citation is to “language spec §5.2”: syntax has no chapter five, while type-system §5.2 is “multiple constraints”; the assertion concerns the immutability flag stored by ScopeManager.                                                     |
| 25  | `test_type_evaluator_eval_nested_type`                                      | `evaluator.rs`             | Irrelevant      | It cites syntax §3.11 (program entry and top-level statements) and “RFC-027 §362” (which does not exist); the assertion concerns evaluation of a nested type.                                                                                   |
| 26  | `test_get_ast_type_universe_level_plain`                                    | `mono.rs`                  | Irrelevant      | It cites “language spec §3”; syntax §3 is the statements chapter; the assertion checks that the type universe level is 0.                                                                                                                       |
| 27  | `test_unify_enum_same`                                                      | `solver.rs`                | Irrelevant      | It cites “language spec §3”; unifying an enum with the same name succeeds, which is type unification and is not covered by the statements chapter.                                                                                              |
| 28  | `test_weak_new_upgrade_some_while_original_alive`                           | `weak.rs`                  | Irrelevant      | It cites RFC-010’s **sum-type identity**, but the assertion concerns the lifetime of a weak reference (#384: while the original Arc is alive, upgrade yields Some).                                                                             |
| 29  | `test_i18n_registry_english_titles`                                         | `codes.rs`                 | Irrelevant      | I3/I4 in §3.4 cover translations for E1090/E1091; the assertion checks that E0001’s en title and help are nonempty.                                                                                                                             |
| 30  | `test_format_return_expression`                                             | `expr.rs`                  | Generic         | The citations are a list of sections—§3, §4, §7, §8, §10–§13, §17, and §18—and none corresponds to “how a return expression is rendered.”                                                                                                       |
| 31  | `test_rfc010_generic_syntax`                                                | `lexer_mod.rs`             | Generic         | The citations are syntax §1.6 + broad lists in RFC-004/RFC-010; the assertion describes itself as flexible (allowing any parenthesization), with no verifiable entry.                                                                           |
| 32  | `test_anon_binding_with_fn_annotation_keeps_extracted_signature`            | `types.rs`                 | Generic         | “spec §3, RFC-010, RFC-011” is a broad list; none corresponds to “extracting the annotated signature of an anonymous function binding.”                                                                                                         |
| 33  | `test_consumer_unknown_token_returns_empty`                                 | `ownership.rs`             | Generic         | “RFC-009 + RFC-009a” lists entire documents; the assertion checks that BrandTree returns an empty consumer set for an unknown token.                                                                                                            |
| 34  | `test_const_expr_neg_int_literal_displays_by_value`                         | `const_data.rs`            | Generic         | RFC-011 §4 covers compile-time generics and does not address the printed form of ConstExpr; the assertion checks that the negative literal is printed as -5.                                                                                    |
| 35  | `resolve_local_and_method`                                                  | `resolver.rs`              | Generic         | “RFC-029 refactoring core” is an entire architecture RFC; the assertion concerns the resolver’s qualified name and DefKind.                                                                                                                     |
| 36  | `test_outdated_exec_smoke`                                                  | `outdated.rs`              | Generic         | RFC-014a is merely background in a comment (the registry is not published, so a path dependency is used); the assertion is an exec smoke test, with no assertions in the body.                                                                  |
| 37  | `test_compatible_high_major_range`                                          | `resolver.rs`              | Generic         | “RFC-014 Phase 3” is a phase name, not an entry; the assertion checks version-range compatibility.                                                                                                                                              |
| 38  | `test_clean_and_size`                                                       | `cache.rs`                 | Generic         | “RFC-014 Phase 3 (global cache design)” is likewise a phase name; the assertion checks cache clean and dir_size.                                                                                                                                |
| 39  | `test_strip_ansi_multiple_consecutive_sequences`                            | `ansi.rs`                  | Generic         | §4.2 is the “ANSI tooling extraction” work item, not a specification clause; the assertion checks strip_ansi’s behavior for consecutive sequences.                                                                                              |
| 40  | `test_verify_sha256_accepts_matching_digest_file_content`                   | `dl.rs`                    | Generic         | It cites “RFC-037: version directory = release package extraction root”; the assertion checks that sha256 verification accepts matching digest file content.                                                                                    |

Distribution: **Genuine support 20 · Irrelevant 9 · Generic 11**.

### Mis-Citation Rate and Confidence Interval

- Mis-citations (irrelevant + generic) = 20/40 = **50%**;
- Wilson 95% confidence interval (n=40): **[35.2%, 64.8%]**;
- Scaled to 2403: approximately **1202** genuinely have clause support (95% interval **846 ~
  1557**), and approximately **1201** are mis-cited (interval 846 ~ 1557).

### Impact on the Cleanup Surface

- **Retain falls from 2897 to 776** (−2121)—precisely the 2121 tests that were “retained merely for
  citing a clause” have moved to Pending verification; the tightening takes effect rather than
  tuning the distribution.
- **Retirement candidates are unrelated to clauses**: the first 26 items were unrelated to clauses
  in the first place (they had not a single assertion in their bodies), and the audit does not
  change their decisions; 21 items in the first batch have been retired, reducing candidates from
  136 to 115; 11 items in the second batch have been retired, reducing candidates from 115 to 104.
  The second batch’s 11 items are similar in kind: they do have assertions in their bodies, but only
  weak assertions such as `is_some()` / `is_none()`, which pin neither binding power nor the parse
  result. This also shows that the “retirement surface” and the “clause mis-citation surface” are
  two separate problems: most mis-cited tests have assertions, so the appropriate action is to add a
  clause or correct the citation, not to delete the test.
- The 2121 Pending verification cases represent the follow-up workload for “clause-gate backfill”:
  give them a specific entry that corresponds to the assertion, or reclassify them as retirement
  candidates.

### Limitations (Do Not Cite Beyond Scope)

- The 40 items can provide only a **proportion**; no individual decision can be extrapolated to
  other tests in the same directory;
- The decisions were made by manually reading the code and clause documents. The rationale column
  already states “what the cited clause actually covers” for review;
- The sampling denominator is the old 0.6-9 criterion pool (2403). The mis-citation rate among the
  **1713 tightened cases** has not been resampled—tightening merely excludes unverifiable citations;
  it does not mean that every citation in the new pool genuinely supports the assertion (they still
  require item-by-item verification).

## Reconciliation with the 0.6-2 Survey

The 0.6-2 survey figures are the first comparative baseline. The differences and their reasons are
listed item by item:

| Item                            | 0.6-2 survey                  | This run                                                                                                                                | Reason for the difference                                                                                                                                                                                                                 |
| ------------------------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 3413                                                                                                                                    | Two tests were added after 0.6-2 (the E3005 and E3022 check-criterion regression tests), then 21 no-assertion tests were deleted according to the first retirement list and 11 weak-assertion tests according to the second list; net −30 |
| Number of test files            | 296                           | 296                                                                                                                                     | Unchanged                                                                                                                                                                                                                                 |
| Clause gate                     | 227 files (77%)               | 113 files after tightening; 1713 items                                                                                                  | The criterion was tightened from “a token appears” to “there is a section number `§<number>`,” nearly halving the file count                                                                                                              |
| Incident-history signals        | 442 items / 148 files         | 314 items within the gate; three radii: 7 in the tests themselves / 291 in file headers (38 files) / 1218 across whole files (79 files) | The 148 files in 0.6-2 are broader than the current whole-file radius of 79 files because its vocabulary/scope was larger; this run strictly follows the four signals on the criteria card                                                |
| Characterization strong signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                                            | Characterization is not one of the three gates; mixing it into retirement conclusions would exceed the permitted scope                                                                                                                    |
| Weak assertions                 | 473 (14%)                     | See the “assertion summary” column in the TSV                                                                                           | The counting unit differs: 0.6-2 counted patterns that assert only is_ok/is_some; this run uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                 |

## Conclusion Summary

1. **The clause gate was previously significantly overstated**: under the old criterion, 2403 items
   were classified as “Clause/Retain,” while the sampled mis-citation rate was 50% (95% interval
   35%–65%)—“a citation” and “a citation that supports the assertion” are two different things, and
   the old criterion treated them as the same.
2. **After tightening, approximately 1202 items genuinely have clause support** (interval 846–1557);
   the rest either need clause-gate backfill or should have been classified through the
   incident-history/proof gates from the outset.
3. **The retirement surface is independent of clause mis-citation**: the 136 candidates have no
   clause relation (no assertions/weak assertions); after 21 items in the first batch and 11 in the
   second were retired, the candidate count fell to 104.
4. **The incident-history gate is the largest legitimate source of Retain decisions** (314 items):
   it demonstrates that “pinning a real incident” is more reliable than copying one line from an
   RFC.

## First Retirement Batch (Executed, 2026-11-06)

**First batch: retire 21 and retain 5 of the 26 “no assertion” tests in
`src/frontend/core/types/eval/tests/`** (a single directory cluster; criteria = **all three gates
fail** under Part Seven of coding-rules: no clause gate, no incident-history gate, and not a single
assertion in the body). The item-by-item decisions and “same-cell coverage” are provided by the
0.6-11a decision table; this page does not repeat the reasons and only records the execution
results.

- **21 retired** (2 in `normalizer.rs` + 19 in `reducer.rs`): `test_normalizer_with_config`,
  `test_normalizer_evaluator`, `test_type_computer_register_alias`,
  `test_type_computer_compute_primitive`, `test_type_computer_set_context`,
  `test_type_computer_compute_type_var`, `test_type_computer_compute_list`,
  `test_type_computer_compute_tuple`, `test_type_computer_compute_fn`,
  `test_reducer_reduce_with_limit`, `test_unifier_different_types`,
  `test_unifier_substitution_after_unify`, `test_unifier_unify_option_same`,
  `test_unifier_unify_result_same`, `test_unifier_unify_dict_same`, `test_unifier_unify_set_same`,
  `test_unifier_unify_range_same`, `test_unifier_unify_arc_same`, `test_unifier_unify_weak_same`,
  `test_unifier_unify_enum_same`, `test_unifier_unify_struct_same`.
- **5 retained** (the decision table cannot identify replacement coverage, so they are retained as
  required): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts the whole batch.

## Second Retirement Batch (Executed, 2026-11-06)

**Second batch: retire 11 and retain 7 of the 18 “weak assertion” tests in
`src/frontend/core/parser/pratt/tests/nud.rs`** (a single directory cluster = the Pratt
prefix-parsing module; criteria = **all three gates fail** under Part Seven of coding-rules: no
clause gate, no incident-history gate, and only weak assertions such as `is_some()` / `is_none()` in
the body, which pin neither binding power nor the parse result).

The difference between this batch and the first must be made explicit: these tests **do have
assertions in their bodies** and are not empty tests. The retirement rationale is “all three gates
fail + zero criterion increment,” not “no assertions”—each test marked for deletion names a
**unit-level replacement coverage test in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The replacement
  coverage for each follows the **same** `parse_expression → parse_prefix → prefix_info` path and
  makes stronger assertions (pinning the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, as well as
  `src/frontend/core/parser/tests/expressions.rs`’s `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr`.
- **7 retained**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the “VI. Second Test Retirement” decision table in `09a-wbs-history.md`
  for the item-by-item rationale. The retention rationales fall into two categories: **the proof
  gate applies** (the first three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are
  the sole unit-level inspection points for the corresponding facts, and no second unit test in the
  repository observes them), and **the incident-history gate applies** (`break_removed` /
  `continue_removed` cite #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts the whole batch.

## Rerunning and Self-Checks

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
python scripts/dev/test-gate-census-audit.py --sample  # Reproduce the 40-item sample (seed 20261106)
python scripts/dev/test-gate-census-audit.py --check   # Whether the audit data file covers all samples
```

The generator performs only lexical determination; its boundaries are stated accurately:

- **Delegated assertions are recognized by name only** (`assert*/check*` helper functions). If a
  test’s assertions are hidden in a helper whose name has neither prefix, it will be misclassified
  as “No assertions”—the bodies of the first 26 items (21 retired / 5 retained) and the second 18
  items (11 retired / 7 retained) have all been manually reviewed item by item.
- **Brace balancing does not skip string literals**: the generator extracts a function body by
  balancing braces and truncates early when it encounters `"{"` in source code. In testing, the body
  of `test_prefix_info_lbrace` (which contains `tokenize("{")`) was truncated to two lines, and its
  “assertion summary” captured the rest of the file (`matchesx11 assertx24 …`), so it was bucketed
  as “Pending adjudication” rather than “Retirement candidate”—**the bucketing is wrong**. This item
  is not in the second batch’s retirement candidate pool (it was not deleted). The generator is
  under `scripts/`, is outside this batch’s inScope, and is left for another card.
- **The proof gate is not adjudicated**: any test that misses both gates is classified only as
  “Pending adjudication” or “Retirement candidate”; the generator never declares that a test “should
  be deleted.”
- **No cross-file deduplication**: completely duplicate tests are not identified in this table
  (0.6-2 measured only 1%, which does not constitute a retirement surface).
- **The audit covers only 40 items**: its conclusions are used only to classify those 40 items and
  to provide a **proportion** for the 2403-item pool; no individual decision may be extrapolated to
  other tests in the same directory.
