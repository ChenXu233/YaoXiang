---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every repository test according to the three gates in Part Seven of
  coding-rules, and provide a sampled clause-gate audit plus the first retirement-candidate list'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6, “Major Cleanup”: mechanically classify every repository-wide
`#[test]` according to the [Part Seven of coding-rules](./coding-rules.md) **Three Gates (Clause /
Incident History / Proof)**, and produce a rerunnable, item-by-item determination table. Then
conduct a stratified sampled audit (40 items) of the clause gate, measure the proportion of tests
that “cite a clause but cannot support the assertion,” and recalculate the distribution using the
**tightened criteria**. **This page and the generator itself are read-only with respect to tests**;
test retirement is performed in batches (one commit per batch, with each batch revertible as a whole
using `git revert`), and execution results are recorded in the three sections below, “First
Retirement Batch / Second Retirement Batch / Third Retirement Batch,” and in `09a`.

Generated artifacts:

| Artifact                                  | Purpose                                                                                                                                   |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv`       | Item-by-item determination table (3411 rows plus header), in the same directory as this page                                              |
| `scripts/dev/test-gate-census.py`         | Generator (rerunnable, supports `--check` and `--selftest`)                                                                               |
| `scripts/dev/test-gate-census-audit.py`   | Sampled audit: reproduces the 40-item sample with a fixed seed (`--sample` / `--check`)                                                   |
| `scripts/dev/test-gate-census-audit.json` | Item-by-item determinations and rationale for the 40 items (the generator reads it to classify “audit-confirmed genuine support” as keep) |

## Definitions

### Scanning surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root-package lib (`src/`, including each module’s `tests/` subdirectory) | 2939                 |
| `tests/integration`                                                      | 335                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 75                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **3411** (296 files) |

### Machine criteria for the three gates

| Gate                               | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Meaning of a hit                                               |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| Clause gate (tightened for 0.6-10) | A **verifiable clause** appears within the first 25 lines of the file header: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); files under `src/**/tests/` are examined together with their parent `mod.rs`                                                                                                                                                                                                                                | The test maps to an identifiable clause                        |
| Incident-history gate              | The test body, function doc comment, or **file header** (such as `//! Regression context: issue #226`) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                                             | The test pins an incident that has actually occurred           |
| Proof gate                         | **Cannot be determined by machine** (“If it were replaced with another correct implementation, would it fail?” requires human judgment). Therefore, anything that fails the first two gates is classified as “pending adjudication” and is never automatically classified as “neither passes.” The sole exception is a test with **no assertion at all** in its body (including no delegation to `assert*` / `check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only “pending adjudication” or “no assertions” can be assigned |

Rationale for the tightening: the 0.6-9 clause gate required only that the file header contain the
characters `§` / `RFC` / `SPEC`, resulting in 2403 tests being classified as “keep.” The 40-item
stratified sample in 0.6-10 found that **half of the citations could not support the assertions**
(see the section below). After tightening, only section-number forms are recognized, and **having a
citation still does not mean the citation supports the assertion**—every unaudited clause citation
is classified only as “pending verification.”

Rationale for the file-header radius: this repository’s convention is to place text such as
“Regression context: issue #N” in the file header rather than repeating it in each test comment.
Looking only at the test’s own comment would miss the incident history of the entire file (measured:
7 hits at the test’s own radius, versus 291 at the file-header radius).

### Categories that are never retired

The following tests are **always “keep”** and never enter the retirement-candidate pool:

- Corpus `tests/yaoxiang/**` and project-shape `tests/project-corpus/**` (behavior-criterion
  surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and cross a process boundary);
- Tests whose names contain `unchanged` / `anchor` / `guard` / `preserv` / `baseline` (current-state
  anchors).

### Disposition semantics

| Disposition          | Meaning                                                                                                                                                                                                                   |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Only three cases: an incident-history gate hit, a proof gate hit (spec-shaped directory / current-state anchor name), or a **clause citation whose genuine support was confirmed by the sampled audit**                   |
| Pending verification | Cites a clause (with a section number, or citing only an entire RFC without a section number) but has not been audited, or the audit judged it irrelevant or generic—human verification is required before it can be kept |
| Retirement candidate | Fails both gates and has only weak assertions (or no assertions) in its assertion surface—a candidate is not a verdict; the proof gate still requires human review                                                        |
| Pending adjudication | Fails both gates but has strong assertions; the machine reaches no conclusion                                                                                                                                             |

## Distribution after tightening

| Gate                                                               | Count    |
| ------------------------------------------------------------------ | -------- |
| Clause (with a verifiable clause)                                  | 1714     |
| Incident history                                                   | 313      |
| Proof (spec-shaped criterion surface / current-state anchor)       | 447      |
| Pending adjudication (fails both gates; proof awaits human review) | 926      |
| Neither passes (no assertions in the body)                         | 11       |
| **Total**                                                          | **3411** |

| Disposition          | Count    | Compared with 0.6-9                                                                                                                       |
| -------------------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | **775**  | 2897 (−2122)                                                                                                                              |
| Pending verification | 2122     | 0 (new category)                                                                                                                          |
| Pending adjudication | 414      | 412 (−1 after the second retirement batch, +3 from the 0.6-15 criterion correction)                                                       |
| Retirement candidate | 100      | 136 (first retirement batch: 21; second retirement batch: 11; generator correction: +1; third batch: −2; 0.6-15 criterion correction: −3) |
| **Total**            | **3411** | —                                                                                                                                         |

Two changes in the definitions require explanation:

1. **The structure of “keep”**: 775 = 447 proof-gate hits + 313 incident-history hits + 15 cases
   confirmed by the audit to have genuine support. Of the audit’s 20 genuinely supported cases, the
   files containing the other 5 were already kept by the incident-history gate. Of the original
   2897, the 2121 tests kept “on the strength of a citation alone” are now pending verification.
2. **Incident-history gate 47 → 313**: these are not new incidents, but **label reclassification**.
   In 0.6-9, tests that hit both the clause and incident-history gates were recorded as clause hits.
   After tightening, a clause hit no longer automatically results in “keep,” so these tests must be
   caught by the incident-history gate and their counts folded into that category. The total number
   kept is therefore unaffected (these tests were also kept in 0.6-9).

File-level criterion: of the 296 test files, **113** have a verifiable section number in their file
headers (the old 0.6-9 criterion identified 221).

## Sampled audit of the clause gate (0.6-10)

### Sampling methodology (reproducible)

- **Candidate pool = the 2403 tests classified as “clause” by the 0.6-9 clause gate** (frozen using
  the old criterion in `test-gate-census-audit.py`; the sampling denominator must not change with
  the new criterion, or the results will not be comparable).
- **Strata = directory clusters**: for `src/`, take the first two levels (`src/frontend`,
  `src/middle`, …); everything else is grouped by top-level directory (`tests`, `benches`, `tools`).
- **Allocation**: proportional to the number of items in each stratum (largest-remainder method),
  with at least one item per stratum.
- **Selection**: after sorting by (file, line number, test name), randomly sample within each
  stratum using a fixed seed.
- **Seed = 20261106** (hard-coded in the script; changing the seed changes the sample, so the change
  must be documented and the audit rerun).

Stratified result: `src/frontend` 21 · `tests` 4 · `src/driver` 3 · `src/package` 3 · `src/backends`
2 · `src/formatter` 2 · `src/middle` 2 · `src/util` 2 · `tools` 2 · `src/ast` 1 · `src/lsp` 1 ·
`src/std` 1.

### Item-by-item determinations for the 40 sampled tests

Determination criteria (fixed here to avoid inconsistent judgments from item to item):

- **Genuine support**: the citation identifies a specific section, note, or determination number,
  and its **subject** is the same as the assertion’s subject;
- **Irrelevant**: the citation identifies a specific clause, but that clause concerns **something
  else** (the rationale column states what it actually covers);
- **Generic**: the citation refers only to an entire document, chapter, list of sections, or phase
  name, without any citation matching the assertion.

| #   | Test                                                                        | File                       | Determination   | Rationale (what the cited clause actually covers)                                                                                                                                                                                                         |
| --- | --------------------------------------------------------------------------- | -------------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `test_compile_time_value_param_spec_table`                                  | `classify.rs`              | Genuine support | type-system §7.1, “Compile-time value parameters,” is exactly the assertion subject: the truth table for value-position/type-position parameter determination is checked row by row.                                                                      |
| 2   | `test_id_stable_under_subsequent_registrations`                             | `value.rs`                 | Genuine support | RFC-010, “Runtime representation and equality,” defines sum-type identity as being determined only by the name; the assertion verifies that the ID does not change after subsequent registrations.                                                        |
| 3   | `test_driver_run_check_rootless_multi_file_well_formed_generic_stays_clean` | `check_rootless_mono.rs`   | Genuine support | Point ③ of Note #14 in 02-stage-contract §3 covers monomorphization naming provenance; this test is the pinning test for that note, with both the name and assertion matching.                                                                            |
| 4   | `test_multifile_benign_import_cycle_message_predicts_no_runtime_failure`    | `import_cycle.rs`          | Genuine support | RFC-013’s W1xxx warning code and User Ruling A (“cycles must not be silent”); the assertion verifies that a benign cycle produces exactly one W1007 and predicts no runtime consequence.                                                                  |
| 5   | `test_obligation_config_disabled_skip_is_not_owed`                          | `obligations.rs`           | Genuine support | 02-stage-contract §2, “Obligation ledger”; the assertion verifies that skipping due to ConfigDisabled does not create an outstanding obligation.                                                                                                          |
| 6   | `test_format_type_bool`                                                     | `types.rs`                 | Genuine support | formatter specification §9, type annotations; the assertion verifies that the type name is emitted verbatim.                                                                                                                                              |
| 7   | `test_fstring_multiline_single_quotes_inside`                               | `fstring.rs`               | Genuine support | syntax §1.6.3, string literals; the assertion verifies that a single quote inside multiline f-string content does not terminate it.                                                                                                                       |
| 8   | `test_float_underscore`                                                     | `literals.rs`              | Genuine support | syntax §1.6, literals (1.6.2 floating point), covers underscore separators; the assertion verifies that 1_000.5 is lexed as 1000.5.                                                                                                                       |
| 9   | `test_empty_unicode_escape_is_error`                                        | `literals.rs`              | Genuine support | syntax §1.6.3’s closed set of escapes; the assertion verifies that an empty `\u{}` escape must produce an error.                                                                                                                                          |
| 10  | `test_probe_accepted_const_type_is_registered`                              | `probe.rs`                 | Genuine support | RFC-039 P6 6.4 / WBS 6.6.9 is precisely “constant-parameter determination listens only to injected probes”; the assertion verifies that N is registered when the probe accepts Int.                                                                       |
| 11  | `test_direct_eval_with_bound_variable_proved`                               | `predicate.rs`             | Genuine support | RFC-027 §3–4 (path-condition propagation / compile-time proof pipeline); the assertion verifies that, with b=5, b>0 evaluates directly to Proved.                                                                                                         |
| 12  | `test_unused_type_is_not_an_entry_point`                                    | `dead_code.rs`             | Genuine support | RFC-011 §7, dead-code elimination mechanism; the assertion verifies that an unused type definition is not a reachable root.                                                                                                                               |
| 13  | `test_display_forall_multiple_vars_no_trailing_space`                       | `ast.rs`                   | Genuine support | RFC-027 §8, SMT solver; the assertion verifies the printing format for `forall` with multiple variables.                                                                                                                                                  |
| 14  | `test_dep_graph_remove_dependant_clears_edges`                              | `rfc027_phase31_dep_vc.rs` | Genuine support | RFC-027 Phase 3.1’s TypeDepGraph; the assertion verifies that removing a dependent clears its edges and constraints together.                                                                                                                             |
| 15  | `test_condition_bool`                                                       | `conditional.rs`           | Genuine support | RFC-011 §5, conditional types; the assertion verifies the evaluation and determinate state of `TypeCondition::Bool`.                                                                                                                                      |
| 16  | `test_convert_expr_to_const_expr_int_literal`                               | `const_eval.rs`            | Genuine support | RFC-011 §4, compile-time generics; the assertion verifies that an Int literal converts to `ConstExpr::Lit`.                                                                                                                                               |
| 17  | `test_definition_on_binding_site_returns_none_precise_only`                 | `definition.rs`            | Genuine support | RFC-039 D55 (precise-only; no name-based fallback); the assertion verifies that the definition site returns None when it has no reference record of its own.                                                                                              |
| 18  | `test_type_consistent_arith_passes`                                         | `verify.rs`                | Genuine support | The IR static-validator criterion in WBS 2.1 / RFC-039 P2 (the first layer of 07-equivalence-oracle); the assertion verifies that type-consistent arithmetic passes.                                                                                      |
| 19  | `test_verify_loose_corpus_green`                                            | `verify.rs`                | Genuine support | The same validator criterion; the assertion verifies that the entire corpus runs green.                                                                                                                                                                   |
| 20  | `test_parse_tag_tolerates_query_and_fragment`                               | `toolchain.rs`             | Genuine support | RFC-037 § installer support; the assertion verifies that the query and fragment are stripped from the release URL.                                                                                                                                        |
| 21  | `test_release_unset_register`                                               | `execute.rs`               | Irrelevant      | #299 §2 covers the NewArray fixed-length array instruction (allocation, Void placeholder, bytecode round trip); the assertion instead verifies that “executing Release on an unset register does not panic.”                                              |
| 22  | `test_fstring_segment_interpolation`                                        | `ast.rs`                   | Irrelevant      | It cites “Language Specification §3 (AST Structure),” but syntax §3 is the statements chapter (3.1 Statement Classification … 3.11 Program Entry and Top-Level Statements); the assertion instead concerns the fields of `FStringSegment::Interpolation`. |
| 23  | `test_expr_for`                                                             | `ast.rs`                   | Irrelevant      | It cites the same “§3 (AST Structure)” reference; syntax §3 covers statements, whereas the assertion concerns the AST fields of `Expr::For`.                                                                                                              |
| 24  | `test_add_var_without_mut_stores_immutability`                              | `scope.rs`                 | Irrelevant      | It cites “Language Specification §5.2”: syntax has no fifth chapter, while type-system §5.2 is “Multiple Constraints”; the assertion instead concerns the immutability flag stored by ScopeManager.                                                       |
| 25  | `test_type_evaluator_eval_nested_type`                                      | `evaluator.rs`             | Irrelevant      | It cites syntax §3.11 (Program Entry and Top-Level Statements) and the nonexistent “RFC-027 §362”; the assertion instead concerns evaluation of a nested type.                                                                                            |
| 26  | `test_get_ast_type_universe_level_plain`                                    | `mono.rs`                  | Irrelevant      | It cites “Language Specification §3,” but syntax §3 is the statements chapter; the assertion instead verifies that the type-universe level is 0.                                                                                                          |
| 27  | `test_unify_enum_same`                                                      | `solver.rs`                | Irrelevant      | It cites “Language Specification §3”; the assertion verifies successful unification of enums with the same name, which belongs to type unification rather than the statements chapter.                                                                    |
| 28  | `test_weak_new_upgrade_some_while_original_alive`                           | `weak.rs`                  | Irrelevant      | It cites RFC-010’s **sum-type identity**, while the assertion concerns the lifetime of a weak reference (#384: upgrading to Some while the original Arc is alive).                                                                                        |
| 29  | `test_i18n_registry_english_titles`                                         | `codes.rs`                 | Irrelevant      | I3/I4 in §3.4 cover the translations of E1090/E1091; the assertion instead verifies that E0001’s English title and help text are nonempty.                                                                                                                |
| 30  | `test_format_return_expression`                                             | `expr.rs`                  | Generic         | The citation is a list of sections—§3, §4, §7, §8, §10–§13, §17, and §18—and none corresponds to “how a return expression is rendered.”                                                                                                                   |
| 31  | `test_rfc010_generic_syntax`                                                | `lexer_mod.rs`             | Generic         | The citation broadly lists syntax §1.6 + RFC-004/RFC-010; the assertion describes itself as flexible (allowing any parenthesized form), and there is no verifiable clause.                                                                                |
| 32  | `test_anon_binding_with_fn_annotation_keeps_extracted_signature`            | `types.rs`                 | Generic         | The broad list “spec §3, RFC-010, RFC-011” contains nothing corresponding to “extracting an annotated signature for an anonymous function binding.”                                                                                                       |
| 33  | `test_consumer_unknown_token_returns_empty`                                 | `ownership.rs`             | Generic         | The citation lists the entire “RFC-009 + RFC-009a”; the assertion verifies that BrandTree returns an empty consumer set for an unknown token.                                                                                                             |
| 34  | `test_const_expr_neg_int_literal_displays_by_value`                         | `const_data.rs`            | Generic         | RFC-011 §4 covers compile-time generics and does not address the printing form of ConstExpr; the assertion instead verifies that a negative literal prints as -5.                                                                                         |
| 35  | `resolve_local_and_method`                                                  | `resolver.rs`              | Generic         | “RFC-029 Refactoring Core” is an entire architectural RFC; the assertion instead verifies the resolver’s qualified name and DefKind.                                                                                                                      |
| 36  | `test_outdated_exec_smoke`                                                  | `outdated.rs`              | Generic         | RFC-014a appears only as background in a comment (the registry is unpublished, so a path dependency is used); the assertion is an exec smoke test with no assertions in its body.                                                                         |
| 37  | `test_compatible_high_major_range`                                          | `resolver.rs`              | Generic         | “RFC-014 Phase 3” is a phase name rather than a clause; the assertion verifies compatibility determination for a version interval.                                                                                                                        |
| 38  | `test_clean_and_size`                                                       | `cache.rs`                 | Generic         | “RFC-014 Phase 3 (Global Cache Design)” is likewise a phase name; the assertion verifies cache clean and dir_size.                                                                                                                                        |
| 39  | `test_strip_ansi_multiple_consecutive_sequences`                            | `ansi.rs`                  | Generic         | §4.2 is the “ANSI Utility Extraction” work item, not a specification clause; the assertion instead verifies strip_ansi’s consecutive-sequence behavior.                                                                                                   |
| 40  | `test_verify_sha256_accepts_matching_digest_file_content`                   | `dl.rs`                    | Generic         | It cites “RFC-037: Version catalog = release package unpacked root directory”; the assertion instead verifies that sha256 verification accepts matching digest-file content.                                                                              |

Distribution: **genuine support 20 · irrelevant 9 · generic 11**.

### Mis-citation rate and confidence interval

- Mis-citations (irrelevant + generic) = 20/40 = **50%**;
- Wilson 95% confidence interval (n=40): **[35.2%, 64.8%]**;
- Extrapolated to 2403: approximately **1202** tests genuinely have clause support (95% interval
  **846–1557**), while approximately **1201** are mis-citations (interval 846–1557).

### Impact on the cleanup surface

- **“Keep” falls from 2897 to 775** (−2122)—exactly the 2122 tests kept “on the strength of a
  citation alone” become pending verification; the tightening takes effect rather than tuning the
  distribution.
- **Retirement candidates are unrelated to clauses**: the first batch of 26 was already unrelated to
  clauses (not one assertion in the body), and the audit does not change their determinations. After
  the first batch of 21 was retired, candidates went from 136 → 115; after the second batch of 11
  was retired, 115 → 104. After the 0.6-13 generator fix (function-body boundaries are no longer
  truncated by braces inside string literals), the count increased by one, 104 → 105. After the
  third batch of 2 was retired, 105 → 103. After the 0.6-15 criterion fix (helper names containing
  digits / the `match` + `panic!` form are recognized as assertions), 3 tests moved out of “neither
  passes,” and candidates went from 103 → 100. The second batch of 11 is analogous: the tests **do**
  contain assertions, but only weak assertions such as `is_some()` / `is_none()`, which do not lock
  down binding power or the parsing result. This also shows that the “retirement surface” and the
  “clause mis-citation surface” are two separate problems: most mis-cited tests do contain
  assertions, and the appropriate action is to add a clause or correct the citation rather than
  delete the test.
- The 2122 pending-verification tests represent the workload for the subsequent “clause-gate ticket
  completion” effort: add a specific clause that matches each assertion, or reclassify the test as a
  retirement candidate.

### Limitations (do not extrapolate beyond these bounds)

- The 40 items can provide only a **proportion**; no individual determination may be extrapolated to
  other tests in the same directory;
- Determinations were made by manually reading the code and clause documents, and the rationale
  column states “what the cited clause actually covers” for review;
- The sampling denominator is the old 0.6-9 criterion pool (2403). The mis-citation rate within the
  **tightened pool of 1714** tests has not been resampled—tightening merely excludes “unverifiable”
  citations; it does not mean every citation in the new pool genuinely supports its assertion (each
  still requires item-by-item verification).

## Reconciliation with the 0.6-2 survey

The 0.6-2 survey figures are the first-version comparison baseline. The reasons for each difference
are itemized below:

| Item                            | 0.6-2 survey                  | Current                                                                                                        | Reason for the difference                                                                                                                                                                                                                                           |
| ------------------------------- | ----------------------------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 3411                                                                                                           | Two were added after 0.6-2 (check-criterion regression tests for E3005 and E3022), then 21 no-assertion tests were deleted according to the first retirement list, 11 weak-assertion tests according to the second list, and 2 according to the third list; net −32 |
| Number of test files            | 296                           | 296                                                                                                            | Unchanged                                                                                                                                                                                                                                                           |
| Clause gate                     | 227 files (77%)               | After tightening: 113 files; 1714 items                                                                        | The criterion was tightened from “contains the relevant characters” to “contains a section number `§<number>`,” nearly halving the file count                                                                                                                       |
| Incident-history signals        | 442 items / 148 files         | 313 items within the gate; three radii: test’s own 7 / file header 291 (38 files) / whole file 1218 (79 files) | The 148 files in 0.6-2 exceeded the 79 files found here at the whole-file radius because its vocabulary and scope were broader; the current survey strictly follows the four card-surface signals                                                                   |
| Characterization strong signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                   | Characterization is not one of the three gates, so mixing it into retirement conclusions would exceed the mandate                                                                                                                                                   |
| Weak assertions                 | 473 (14%)                     | See the “Assertion Summary” column in the TSV                                                                  | The counting units differ: 0.6-2 counted patterns that assert only `is_ok` / `is_some`; the current survey uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                           |

## Conclusion summary

1. **The clause gate was previously substantially inflated**: under the old criterion, 2403 tests
   were classified as “clause/keep,” while the sampled audit measured a 50% mis-citation rate (95%
   interval 35%–65%)—“citation” and “citation supporting the assertion” are different things, and
   the old definition treated the former as the latter.
2. **After tightening, approximately 1202 tests genuinely have clause support** (interval 846–1557);
   the rest must either be verified by adding clauses or judged through the incident-history/proof
   gates.
3. **The retirement surface is unrelated to clause mis-citations**: the 136 candidates are unrelated
   to clauses (no assertions / weak assertions). The first batch retired 21 tests and the second
   retired 11; a generator correction added one and the third batch removed two, reducing the
   candidates to 103.
4. **The incident-history gate is the largest legitimate source of “keep”** (313 items): it
   demonstrates that “pinning an actual incident” is more reliable than “copying a sentence from an
   RFC.”

## First retirement batch (executed, 2026-11-06)

**First batch: of the 26 “no assertion” tests in `src/frontend/core/types/eval/tests/`, 21 were
retired and 5 kept** (a single directory cluster; criterion = all **three gates fail** according to
Part Seven of coding-rules: no clause-gate hit, no incident-history-gate hit, and no assertion of
any kind in the body). The 0.6-11a determination table provides the item-by-item determinations and
“same-cell coverage” (21 deleted / 5 kept); this page records only the execution result and does not
repeat the rationale.

- **21 retired** (`normalizer.rs`: 2; `reducer.rs`: 19): `test_normalizer_with_config`,
  `test_normalizer_evaluator`, `test_type_computer_register_alias`,
  `test_type_computer_compute_primitive`, `test_type_computer_set_context`,
  `test_type_computer_compute_type_var`, `test_type_computer_compute_list`,
  `test_type_computer_compute_tuple`, `test_type_computer_compute_fn`,
  `test_reducer_reduce_with_limit`, `test_unifier_different_types`,
  `test_unifier_substitution_after_unify`, `test_unifier_unify_option_same`,
  `test_unifier_unify_result_same`, `test_unifier_unify_dict_same`, `test_unifier_unify_set_same`,
  `test_unifier_unify_range_same`, `test_unifier_unify_arc_same`, `test_unifier_unify_weak_same`,
  `test_unifier_unify_enum_same`, `test_unifier_unify_struct_same`.
- **5 kept** (the determination table could not identify replacement coverage, so they must be kept
  under the hard requirement): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Second retirement batch (executed, 2026-11-06)

**Second batch: of the 18 “weak assertion” tests in `src/frontend/core/parser/pratt/tests/nud.rs`,
11 were retired and 7 kept** (a single directory cluster = the pratt prefix-parsing module;
criterion = all **three gates fail** according to Part Seven of coding-rules: no clause-gate hit, no
incident-history-gate hit, and the body contains only weak assertions such as `is_some()` /
`is_none()`—it does not lock down binding power or the parsing result).

The difference from the first batch must be stated clearly: these tests **do contain assertions**
and are not empty tests. The retirement rationale is “all three gates fail + zero criterion
increment,” not “no assertions.” For every test selected for deletion, a **unit-level replacement
test in the same module** was identified.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Each replacement
  follows the **same** `parse_expression → parse_prefix → prefix_info` path and makes stronger
  assertions (locking down AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, as well as `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the “VI. Second Test Retirement Batch” determination table in
  `09a-wbs-history.md` for the item-by-item rationale. The rationale for keeping them falls into two
  categories: **the proof gate holds** (the first three, together with `test_prefix_info_at` and
  `test_prefix_info_eof`, are the sole **unit-level** observation points for their corresponding
  facts, and the repository has no second unit test that observes them), and **the incident-history
  gate holds** (`break_removed` / `continue_removed` cite #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Generator correction (0.6-13)

**`body_span` truncates function bodies when it encounters braces inside literals — fixed.**

The old implementation balanced braces to obtain the function body without distinguishing actual
code from braces inside string literals, character literals, or comments. Forms such as
`tokenize("{")` are common in Rust source, so the balance was skewed and the function body was
truncated: `test_prefix_info_lbrace` was truncated at the end of the file, and the “Assertion
Summary” captured the rest of the file (`matchesx11 assertx24 …`), causing it to be classified as
“pending adjudication” rather than “retirement candidate”—a **misclassification**.

Fix: add `mask_noncode` to first replace the contents of string literals, character literals, line
comments, block comments (including nested ones), and raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (newlines are preserved unchanged, so line numbers and line counts remain the
same), and then balance the braces. **The mask is used only to find function-body boundaries**;
assertion statistics and the incident-history gate still read the original source, and the criteria
themselves have not been relaxed. `--selftest` now includes 4 cases (paired and unpaired braces in
ordinary strings, braces in raw strings and block comments, and braces in character literals), each
followed by a sentinel test—the old implementation selected the wrong boundary in all 4 cases, while
the new implementation selects the correct boundary in every case.

### Tests affected by the same bug: 11

Counting method: compare the **function-body boundaries before and after the fix** for each test in
each file (the old implementation balanced braces literally, while the new implementation balances
them after masking). Any test whose boundaries differ was affected. Of the repository-wide 3413
tests, the old implementation selected the wrong boundary for **11**:

| File                                                   | Count |
| ------------------------------------------------------ | ----- |
| `src/frontend/core/lexer/tests/fstring.rs`             | 3     |
| `src/formatter/handlers/tests/expr.rs`                 | 2     |
| `src/driver/tests/manifest_error.rs`                   | 1     |
| `src/frontend/core/lexer/tests/literals.rs`            | 1     |
| `src/frontend/core/parser/pratt/tests/nud.rs`          | 1     |
| `src/frontend/core/parser/statements/tests/imports.rs` | 1     |
| `src/frontend/core/parser/statements/tests/types.rs`   | 1     |
| `src/frontend/core/parser/tests/expressions.rs`        | 1     |

### Classification differences before and after the fix: 2

For the other 9, **8 affected only the “Assertion Summary” column** (classification did not change),
and **1 did not change even the summary** (`test_fstring_close_brace_escape_preserved_raw`: the old
implementation omitted the closing `}` line, which contains no assertion keywords). The eight
summary changes are listed individually in this section of `09a-wbs-history.md`.

| Test name                   | File                                          | Old classification                          | New classification                              | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| --------------------------- | --------------------------------------------- | ------------------------------------------- | ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending adjudication / Pending adjudication | Pending adjudication / **Retirement candidate** | The actual function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; the file header and function body contain neither a § section number nor issue/regression/defect/incident ⇒ neither the clause gate nor the incident-history gate holds, and the body contains only a weak assertion such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires human review)                                                                       |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident history / Keep                     | Clause / **Pending verification**               | The old implementation truncated the function body at the end of the file and swallowed text containing “regression / defect / incident,” falsely triggering the incident-history gate; the actual function body contains none of these terms ⇒ the incident-history gate no longer holds. The file header has verifiable clauses (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate hits, but because genuine support was not confirmed by the sampled audit, the test is pending verification |

The distribution changes accordingly: Clause 1713 → 1714 · Incident history 314 → 313 · Keep 776 →
775 · Pending verification 2121 → 2122 · Pending adjudication 412 → 411 · Retirement candidate 104
→ 105. **The repository-wide `#[test]` count remains unchanged (3413)**—this round changes only the
implementation of the criteria, not any tests.

## Third retirement batch (executed, 2026-11-07)

**Third batch: of the 5 retirement candidates in `src/middle/passes/codegen/tests/`, 2 were retired
and 3 kept after item-by-item review.** (A single directory cluster = the `middle::passes::codegen`
module under test; criterion = all **three gates fail** according to Part Seven of coding-rules.)

This batch follows the **keep criterion** established by the previous batch: a unit-level
observation point may be replaced only by another **unit test**; corpus and integration tests
operate at the process-boundary layer and cannot replace a unit-level observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—its function body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (apart from the
    function name after comments are removed): both consist of `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that “the `execute_module` entry
    path returns non-Void,” but the body never observes the return value from `execute_module`
    (`run_fixture` discards it with `.expect(...)`)—the claimed intent is not implemented, and the
    body is a pure duplicate ⇒ zero criterion increment. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file strictly covers it:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` is identical in form to the inline `Symbol` in this
    test). “A name that was never inserted cannot be found” and “it cannot be found after being
    popped” follow the same path in `SymbolScopeManager::lookup` (searching each layer with
    `scope.get(name)`, returning `None` if not found) ⇒ this test has zero increment. Replacement
    coverage = `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that does
    **not** use serialization to connect directly to execution (the serialized variant is a separate
    path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit-level observation point for the
    `id > 255` rejection branch of `OperandResolver::to_reg` (the bytecode layer has its own
    independent overflow checks and does not cover this function; no second test in the repository
    makes `to_reg` go out of range). `is_err()` is a discriminating assertion (removing the guard
    would make it fail), not merely proof that nothing panicked.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

### A census blind spot found by this batch (**fixed in 0.6-15**; see “Generator Criterion Corrections” below)

The census’s “delegated assertion” recognition pattern, `\b(assert|check)[A-Za-z_]*\s*\(`, cannot
match helper names containing digits: `[A-Za-z_]*` does not include digits, so calls such as
`assert_slot_is_42(&interp)` are not recognized as assertions. Consequently, 2 of the 5 candidates
in this batch were incorrectly classified as “no assertions of any kind in the body.”

After reviewing all 15 repository-wide “neither passes” items item by item (broadening helper names
to include digits and additionally recognizing the `match` + `panic!` form), **4 did in fact contain
assertions**—3 used the numeric helper names above (`assert_slot_is_42`), and 1 was
`test_weak_from_arc` in `src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form;
test-specification rule 6.1 requires `assert!(matches!(...))`, but it does assert). The remaining 11
truly contained not a single assertion.

**Applying the machine’s “neither passes” classification directly would have deleted these 4 tests
despite their criteria**—they were retained only after this batch’s item-by-item human review; the
corresponding generator-side fix is described in “Generator Criterion Corrections (0.6-15)” below.

## Generator criterion corrections (0.6-15)

**Two criterion blind spots have been fixed: helper names containing digits, and the `match` +
`panic!` form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411);
it changes only how the generator recognizes whether a test body contains an assertion.

### What was fixed

1. **Digits are now allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so a test containing such a call was
   treated as having “not a single assertion in its body.”
2. **Added recognition of the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the
   assertion is written in an arm, so no `assert!` macro is visible lexically (test-specification
   rule 6.1 requires `assert!(matches!(...))`, but existing code contains this form). Recognition
   method: scan the expression after the `match` keyword up to the first `{`, determine the block
   boundary by **brace balancing**, and treat it as an assertion only if `panic!(` occurs within the
   block. **The determination is performed on masked lines**—braces inside strings and comments do
   not participate in balancing, and `panic!` inside a comment is not treated as an assertion.

### Classification differences: 3 items (gate and disposition changed)

| Test name                                           | File                                           | Old classification                    | New classification                          | Rationale                                                                                                                                                                                                                                                    |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither passes / Retirement candidate | Pending adjudication / Pending adjudication | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending adjudication / Pending adjudication | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains digits and did not match the old pattern; it is now recognized as an assertion                                                                                           |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending adjudication / Pending adjudication | Same as above (the serialization round-trip test)                                                                                                                                                                                                            |

A further **77 items changed only the “Assertion Summary” column** (gate and disposition unchanged):
most gained `match_panicxN` in the summary (the body contains the `match` + `panic!` form), and 4
gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3; `src/package/tests/cache.rs`: 1—helper
names containing digits).

**The relaxation changes only the “neither passes” category**: that category is the machine’s
**high-confidence verdict** (not a single assertion in the body), and its criterion is a hard
requirement. “Retirement candidate” is only a pool awaiting human review; the size of that pool is
not a verdict. All other classifications, including the weak-assertion pool, therefore continue to
follow the original criteria. Result: Gate—Neither passes 14 → 11 · Pending adjudication 923 → 926;
Disposition—Retirement candidate 103 → 100 · Pending adjudication 411 → 414.

### Item-by-item review of the remaining 11 “neither passes” items

After reading all 11 repository-wide “neither passes” items in the original source, **all 11 truly
contain not a single assertion** (including no `assert*` / `check*` helper, no `match` + `panic!`
form, and no `#[should_panic]`):

| Test name                                       | File                                             | What the body actually does                                                                     | Review conclusion                                        |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | Five `.expect(...)` calls, merely guaranteeing that it does not crash                           | Truly no assertions (proves only that it does not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                  | Truly no assertions                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them              | Truly no assertions                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Truly no assertions                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Truly no assertions                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential alias `reduce` and discards the result                              | Truly no assertions                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                        | Truly no assertions (proves only that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Truly no assertions (proves only that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Truly no assertions (proves only that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                            | Truly no assertions                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Truly no assertions                                      |

Five of these (`test_normalizer_context` and the four items in reducer.rs) were kept by the 0.6-11a
determination table because it **could not identify replacement coverage**; the conclusions of this
review are consistent with that table.

### Reverse sample: did the relaxation go too far?

Five items were randomly sampled with a fixed seed (20261107) from the 88 items newly recognized as
assertions, and their original source was read to confirm that they really do assert:

| Sampled item                                                | File                                                 | Assertion in the original source                                                                                                                     |
| ----------------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching arm, with `panic!("Expected AssocType")` in the non-matching arm                                              |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm, with `panic!("expected InvalidPackage, got …")` in the non-matching arm                  |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | An `assert_eq!` comparing the constraint type in the matching arm, with `panic!("不是 Dynamic 变体…")` in the non-matching arm                       |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm, with `panic!("Expected TokensDelta variant")` in the non-matching arm                        |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm verify the error code and message, with `panic!("不是 Disproved…")` in the non-matching arm |

All 5 assert concrete values or concrete variants (replacing the implementation with another correct
path would make them fail)—**the relaxation did not go too far**. Reverse controls were also added
to `--selftest`: no `panic!` inside a `match`, and `panic!` appearing only in a comment; neither may
be recognized as an assertion.

### The criterion has teeth (developmental proof)

When the generator is reverted to the old implementation (`HELPER_RE` without digits and `match` +
`panic!` recognition disabled), all newly added `--selftest` cases **fail**, with exit code 1; the
new implementation passes them all.

### Rollback

The generator and census table are in the same commit, so `git revert <sha>` restores them as a
whole.

## Repository-wide redundancy scan (0.6-16)

User ruling: **Do not proceed batch by batch**—calculate the complete repository-wide redundancy
surface in one pass, produce a complete “keeper → deleted item” comparison, and then delete in one
operation. **“Mutation proof that deleting a test will make it fail” must not be used as the basis
for deletion** (for a test already covered by a stronger case, mutation proves only that “it will
fail,” not that “it is irreplaceable”). The only criterion is **dominance**.

### Definitions and methodology

- **Per-item artifacts are not checked in** (User Ruling 2026-11-07: the 10.75 MB boundary-tier
  comparison table and its generation script were deleted together). This section retains the
  **definitions, funnel, and conclusions**—they are reusable knowledge. The per-item table was a
  one-time analysis artifact and can be recalculated using this section’s methodology when needed.
- **Call set** = the **product symbols** reached by a test: symbols called directly in the test
  body + symbols propagated through **helpers in the same file** (≥1 level), minus the file’s own
  scaffolding helper names (pass-through only; these are not counted as elements) and
  standard-library pipeline methods (len / clone / iter / …). Only symbols defined within the
  repository are counted.
- **Assertion strength**: 4 = assert_eq! / assert_ne! > 3 = assert! / matches! (or the match +
  panic! form) > 2 = is_ok() / is_some() / is_none() / is_err() > 1 = #[should_panic] > 0 =
  everything else.
- **Dominance** (same module = within the same file; call sets intersect): A dominates B ⇔ calls(B)
  is a proper subset of calls(A), strength(A) > strength(B), and the input-literal set of B is
  covered by the input-literal set of A. ⇒ **Strict tier** (eligible for retirement); everything
  else (equal call sets / equal strength / neither contains the other / reverse containment /
  different inputs / no comparable string inputs) ⇒ **boundary tier, listed but not deleted**.
- **“Cross-module does not count as dominance” takes “same file” literally**: tests in different
  modules have independent observation surfaces.

### A narrowing relative to the card surface: the input dimension (narrowing only, no relaxation)

If the criterion were only “proper call-set subset + strictly stronger strength,” the machine would
classify two tests that verify different properties as dominated. In the measured example,
`test_infix_call_pos_args` (which verifies call arguments) would dominate `test_infix_add` (which
verifies the `+` operator): the former additionally calls len / is_empty, which are unrelated to the
property under test; deleting `test_infix_add` would lose the unit-level observation surface for the
`+` operator. Therefore, one additional **input dimension** is added: the string literals supplied
to the dominated test must be covered by those supplied to the dominating test. If there are no
comparable string inputs, the test is not classified as dominated, in line with the “if it cannot be
identified, keep it” criterion.

### Distribution

- Repository-wide 3409 tests (296 modules); strength distribution 4=1397 · 3=1758 · 2=1 · 1=0 ·
  0=253.
- **Funnel** (of the 68574 pairs with the same module and intersecting call sets, converging one
  layer at a time): Intersection 68574 → + proper call-set subset 1426 → + strictly stronger
  strength 1426 → + covered inputs **12** ⇒ strict tier: 12 pairs / 9 dominated tests (recalculating
  after deleting those 2 tests yields this figure).
- **Boundary tier: 68574 pairs**, with the following rationale distribution: equal call sets 27668 ·
  equal strength 21066 · reverse call-set containment 7882 · weaker strength 5775 · neither call set
  contains the other 4771 · different inputs 1117 · no comparable string inputs 295. The per-item
  boundary-tier comparison table was not checked in, for the same reason as above; it can be
  recalculated using this section’s methodology when needed.

### Intersection with the “zero assertion” line

- **1** test in the strict tier is also classified by the census as “neither passes (no assertions
  in the body)” (`test_reducer_reduce_type_ref_with_alias_chain`)—this is precisely the test that
  the 0.6-11a determination table **could not identify replacement coverage for** and therefore
  kept. The two lines disagree about it; the 0.6-11a human determination takes precedence (**do not
  delete**).
- Among the listed tests in the boundary tier, 95 pairs are classified as “neither passes.”

### Item-by-item determinations (strict tier: 11 items, each read in the original source)

| Listed test                                             | File                                                  | Dominating test                                                                      | Decision   | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------ | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_from_ast_type_fn`                                 | src/frontend/core/types/tests/mono.rs                 | `test_from_ast_type_fn_drops_untyped_param_as_loss`                                  | **Delete** | The dominating test likewise converts `ast::Type::Fn` into `MonoType::Fn`, and additionally locks down the parameters when it hits the Fn arm (`params == vec![MonoType::Int(32)]`), panicking otherwise. The conversion arm (`ast::Type::Fn` → `MonoType::Fn`) is structural and does not vary with the number of parameters—if the listed test were false (could not convert to the Fn variant), the dominating test would necessarily fail ⇒ dominance holds |
| `test_is_installed`                                     | src/package/vendor/tests/mod.rs                       | `test_clean`                                                                         | **Delete** | The implementation of `is_installed` is `self.dep_path(name, version).exists()` (an existence determination). The dominating test asserts both outcomes of is_installed: `foo` (retained) is true, while `bar` / `baz` (removed by clean) are false. The listed test’s two assertions (false before creating the directory, true afterward) are each implied by these: if either were false, the dominating test would necessarily fail ⇒ dominance holds       |
| `test_expr_ref`                                         | src/frontend/core/parser/tests/ast.rs                 | `test_expr_callable_parts_lambda_returns_params_and_body`                            | Keep       | The dominating test only verifies the `callable_parts()` decomposition of a lambda and never constructs an `Expr::Ref`; the listed test verifies the observation surface for “the Ref variant is manually constructed,” and the dominating test could fail while the listed test remained green ⇒ dominance does not hold                                                                                                                                       |
| `test_expr_try`                                         | src/frontend/core/parser/tests/ast.rs                 | `test_expr_callable_parts_lambda_returns_params_and_body`                            | Keep       | Same: the dominating test never touches `Expr::Try`                                                                                                                                                                                                                                                                                                                                                                                                             |
| `test_skip_failure`                                     | src/frontend/core/parser/pratt/tests/parser_state.rs  | `test_skip_success`                                                                  | Keep       | The two assertions are **opposites**: the dominating test asserts that skip returns true for `;`, while the listed test asserts that skip returns false for a non-`;` value. If skip always returned true, the dominating test would remain green while the listed test failed ⇒ dominance does not hold                                                                                                                                                        |
| `test_measure_disproof_silent_without_well_foundedness` | src/frontend/core/sema/layers/tests/termination.rs    | `test_measure_disproof_emits_e4022_when_well_founded`                                | Keep       | The two tests verify **opposite branches** (well-founded emits E4022 / non-well-founded is silent), and their input surfaces differ (the non-well-founded input never reaches the dominating test’s branch) ⇒ dominance does not hold                                                                                                                                                                                                                           |
| `test_associated_def_type_param_into_type`              | src/frontend/core/types/eval/tests/dependent_types.rs | `test_associated_def_direct_substitute`, `test_substitute`                           | Keep       | The listed test verifies that `TypeParam.into_type()` returns TypeRef; both dominating tests first substitute and then call `into_type()`, obtaining the substituted Int(32). If `into_type()` were inaccurate for TypeParam, the dominating tests would remain green ⇒ dominance does not hold                                                                                                                                                                 |
| `test_reducer_reduce_type_ref_with_alias_chain`         | src/frontend/core/types/eval/tests/reducer.rs         | `test_reducer_register_alias`                                                        | Keep       | The dominating test only verifies alias registration, while the listed test verifies the “A→B→Int(32) chained reduce” path (the 0.6-11a determination table already determined that no replacement coverage could be identified and kept it) ⇒ dominance does not hold                                                                                                                                                                                          |
| `test_from_ast_type_meta_type`                          | src/frontend/core/types/tests/mono.rs                 | `test_from_ast_type_fn_drops_untyped_param_as_loss`, `test_from_ast_type_primitives` | Keep       | Neither dominating test converts `ast::Type::MetaType` (one converts Fn, and the other converts primitive type names) ⇒ dominance does not hold                                                                                                                                                                                                                                                                                                                 |
| `test_from_ast_type_meta_type_nested`                   | src/frontend/core/types/tests/mono.rs                 | `test_calculate_meta_type_level_args`, `test_get_ast_type_universe_level_nested`     | Keep       | The dominating tests only calculate the **level** of a meta type and never convert it into `MonoType::MetaType` (the listed test verifies exactly that conversion) ⇒ dominance does not hold                                                                                                                                                                                                                                                                    |
| `test_empty_code_is_none`                               | src/lsp/handlers/tests/diagnostics.rs                 | `test_no_span_uses_default_range`                                                    | Keep       | The two assertions concern different subjects: the listed test verifies that the diagnostic **code** is empty, while the dominating test verifies that the **range** uses the default value ⇒ dominance does not hold                                                                                                                                                                                                                                           |

### Conclusion

The strict-tier machine criterion identifies 11 items. After reading the original source item by
item, **2 were deleted and 9 kept**; this round **retires the 2 tests in one operation** (none of
the 9 kept tests was changed, and the census criteria were not modified). **Only these 2 dominance
relationships can hold in this repository**—for the other 9, the mechanical relationship arises
because the “dominating test piggybacks on extra setup / a stronger assertion form,” not because it
covers the listed test’s observation surface. Without the input dimension, the mechanical criterion
would yield 1426 pairs, many of which are false dominance of this kind (see the funnel below).

## Rerun and self-check

```bash
python scripts/dev/test-gate-census.py            # Recalculate and overwrite the TSV (two runs produce byte-identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: verify that the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-check (does not scan the repository)
python scripts/dev/test-gate-census-audit.py --sample  # Reproduce the 40-item sample (seed 20261106)
python scripts/dev/test-gate-census-audit.py --check   # Verify that the audit data file covers every sampled item
```

The generator performs only lexical determination; its boundaries are stated explicitly:

- **Delegated assertions are recognized only by name** (`assert*` / `check*` helper functions). If a
  test’s assertion is hidden in a helper whose name lacks these prefixes, it is misclassified as “no
  assertions”—the first batch of 26 (21 retired / 5 kept) and second batch of 18 (11 retired / 7
  kept) were each manually reviewed at the function-body level.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator obtains a function body, it
  balances braces; braces inside string literals, character literals, or comments skew the balance
  and truncate the body. `test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was
  previously truncated at the end of the file, so the “Assertion Summary” captured the rest of the
  file and classified it as “pending adjudication” rather than “retirement candidate.” The generator
  now masks non-code regions (strings / character literals / line comments / block comments / raw
  strings) before balancing braces; see “Generator Correction” below.
- **“Delegated assertion” recognition (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not include digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions, and tests containing such calls
  were misclassified as having “not a single assertion in the body.” At the time, 4 of the
  repository-wide 15 “neither passes” items actually contained assertions (3 numeric helpers + 1
  `match` + `panic!` form). The name portion has now been broadened to `[A-Za-z0-9_]*`, and
  recognition of the `match` + `panic!` form has been added; see “Generator Criterion Corrections”
  below.
- **The proof gate is not decided**: any test that “fails both gates” is classified only as “pending
  adjudication” or “retirement candidate”; the generator never declares that it “should be deleted.”
- **No cross-file deduplication**: completely duplicate tests are not identified in this table
  (0.6-2 measured this at only 1%, so it does not constitute a retirement surface).
- **The audit covers only 40 items**: audit conclusions are used only to classify those 40 items and
  to estimate a **proportion** for the 2403-item pool; no individual determination may be
  extrapolated to other tests in the same directory.
