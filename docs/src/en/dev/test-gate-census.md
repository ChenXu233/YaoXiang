---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a clause-gate sampling audit plus an initial list of retirement
  candidates'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every repository-wide
`#[test]` according to the three gates (clause / failure-history / proof) in
[Part Seven of coding-rules](./coding-rules.md), producing a reproducible, item-by-item
determination table; then conduct a stratified sample audit (40 cases) of the clause gate to measure
the proportion that “cite a clause but cannot support their assertions,” and recalculate the
distribution using the **tightened criteria**. **This page and the generator are read-only with
respect to tests**—at this stage, no test is deleted or modified.

Generated artifacts:

| Artifact                                  | Purpose                                                                                                                                     |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv`       | Item-by-item determination table (3,445 rows plus header), in the same directory as this page                                               |
| `scripts/dev/test-gate-census.py`         | Generator (rerunnable, supports `--check` and `--selftest`)                                                                                 |
| `scripts/dev/test-gate-census-audit.py`   | Sampling audit: reproduces a fixed-seed sample of 40 cases (`--sample` / `--check`)                                                         |
| `scripts/dev/test-gate-census-audit.json` | Item-by-item determinations and rationales for all 40 cases; the generator reads it to classify “audit-confirmed genuine support” as retain |

## Criteria

### Scan Surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 2971                 |
| `tests/integration`                                                      | 335                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 75                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **3445** (296 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                              | Meaning of a match                                              |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | A **verifiable clause** appears within the first 25 lines of the file: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); files under `src/**/tests/` are examined together with their parent `mod.rs`                                                                                                                                                                                 | The test maps to a clause that can be named explicitly          |
| Failure-history gate              | The test body, function doc comment, or **file header** (`//! Regression context: issue #226`, for example) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                  | The test pins an incident that has actually occurred            |
| Proof gate                        | **Cannot be decided mechanically** (“Would it turn red if replaced with a correct alternative implementation?” requires human judgment). Therefore, anything that misses the first two gates is classified as “pending determination,” not automatically as “neither gate passes”; the sole exception is a body with **no assertions at all** (including no delegation to `assert*`/`check*` helpers and no `#[should_panic]`) | Only “pending determination” or “no assertions” can be assigned |

Rationale for tightening: the 0.6-9 clause gate only required the file header to contain the tokens
`§`/`RFC`/`SPEC`, so 2403 cases were classified as “retain.” The 40-case stratified sample in 0.6-10
found that **half of the citations could not support the assertions**. After tightening, only the
section-number form is recognized, and **having a citation still does not mean that the citation
supports the assertion**—all unaudited clause citations therefore reach only “pending verification.”

Rationale for the file-header radius: this repository conventionally places text such as “Regression
context: issue #N” in the file header and does not repeat it in individual test comments;
considering only each test’s own comments would miss the history of entire files (measured results:
7 cases within the test’s own radius, 291 cases within the file-header radius).

### Categories That Are Never Retired

The following tests are **always retained** and never become retirement candidates:

- Corpora under `tests/yaoxiang/**` and project-shape corpora under `tests/project-corpus/**`
  (behavioral-criterion surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and crossing a process boundary);
- Status-quo anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition           | Meaning                                                                                                                                                                                                   |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Retain                | Only three reasons: a failure-history gate match, a proof-gate match (spec-shaped directory / status-quo anchor name), or a clause citation whose genuine support was confirmed by the sampling audit     |
| Pending verification  | Cites a clause (with a section number, or an entire RFC without a section number) but is unaudited, or the audit judged it irrelevant or generic—human verification is required before it can be retained |
| Retirement candidate  | Misses both gates and has only weak assertions (or no assertions) in its assertion surface—the candidate is not a verdict, and the proof gate still requires manual review                                |
| Pending determination | Misses both gates but has strong assertions; the machine reaches no conclusion                                                                                                                            |

## Distribution After Tightening

| Gate                                                                         | Count    |
| ---------------------------------------------------------------------------- | -------- |
| Clause (has a verifiable clause)                                             | 1713     |
| Failure history                                                              | 314      |
| Proof (spec-shaped criterion surface / status-quo anchor)                    | 447      |
| Pending determination (misses both gates; proof gate awaits manual judgment) | 935      |
| Neither gate passes (no assertions in the body)                              | 36       |
| **Total**                                                                    | **3445** |

| Disposition           | Count    | Compared with 0.6-9 |
| --------------------- | -------- | ------------------- |
| Retain                | **776**  | 2897 (−2121)        |
| Pending verification  | 2121     | 0 (new tier)        |
| Pending determination | 412      | 412 (unchanged)     |
| Retirement candidate  | 136      | 136 (unchanged)     |
| **Total**             | **3445** | —                   |

Two changes to the criteria require explanation:

1. **Structure of “retain”**: 776 = 447 proof-gate cases + 314 failure-history-gate cases + 15 cases
   whose genuine support was confirmed by the audit. Of the audit’s 20 genuinely supported cases,
   the other 5 were in files already retained by the failure-history gate. Of the original 2897
   retained cases, the 2121 that had been retained “on the strength of a single citation” are now
   pending verification.
2. **Failure-history gate 47 → 314**: this is not an increase in failure history, but
   **relabeling**. Under 0.6-9, cases that matched both the clause and failure-history gates were
   recorded as clause matches. After tightening, a clause match no longer automatically means
   retain, so these cases must be caught by the failure-history gate, and their counts are folded
   into that tier. The total number retained is therefore unaffected (these tests were also retained
   under 0.6-9).

File-level criteria: of the 296 test files, **113** have a verifiable section number in the file
header (the old 0.6-9 criterion found 221).

## Clause-Gate Sampling Audit (0.6-10)

### Sampling Criteria (Reproducible)

- **Candidate pool = the 2403 cases classified as “clause” by the 0.6-9 clause gate** (frozen using
  the old criterion in `test-gate-census-audit.py`; the sample denominator must not change with the
  new criterion, or the results would not be comparable).
- **Strata = directory clusters**: take the first two levels under `src/` (`src/frontend`,
  `src/middle`, etc.); group everything else by top-level directory (`tests`, `benches`, `tools`).
- **Allocation**: proportional to the number of cases in each stratum (largest remainder method),
  with at least one case per stratum.
- **Extraction**: sort within each stratum by (file, line number, test name), then sample randomly
  using a fixed seed.
- **Seed = 20261106** (hard-coded in the script; changing the seed changes the sample and requires
  the documentation to say so and the audit to be rerun).

Stratified results: `src/frontend` 21 · `tests` 4 · `src/driver` 3 · `src/package` 3 ·
`src/backends` 2 · `src/formatter` 2 · `src/middle` 2 · `src/util` 2 · `tools` 2 · `src/ast` 1 ·
`src/lsp` 1 · `src/std` 1.

### Item-by-Item Determination of the 40 Cases

Determination criteria (fixed here to avoid case-by-case inconsistency):

- **Genuine support**: the citation identifies a specific section, note, or determination number,
  and its **topic** is the same as the assertion target;
- **Irrelevant**: the citation identifies a specific clause, but that clause covers **something
  else** (the rationale states what it actually covers);
- **Generic**: the citation points only to an entire document, chapter, list of sections, or phase
  name, with no clause matching the assertion.

| #   | Test                                                                        | File                       | Determination   | Rationale (what the cited clause actually covers)                                                                                                                                                                                            |
| --- | --------------------------------------------------------------------------- | -------------------------- | --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `test_compile_time_value_param_spec_table`                                  | `classify.rs`              | Genuine support | type-system §7.1 “compile-time value parameters” is exactly the assertion target: the truth table for distinguishing value-position from type-position parameters is verified row by row.                                                    |
| 2   | `test_id_stable_under_subsequent_registrations`                             | `value.rs`                 | Genuine support | RFC-010 “runtime representation and equality” defines sum-type identity as being determined only by the name; the test asserts that the ID does not change after subsequent registrations.                                                   |
| 3   | `test_driver_run_check_rootless_multi_file_well_formed_generic_stays_clean` | `check_rootless_mono.rs`   | Genuine support | 02-stage-contract §3 note #14 item ③ discusses using the same source for monomorphization names; the test is the pinning test for that note, with both the name and assertions matching.                                                     |
| 4   | `test_multifile_benign_import_cycle_message_predicts_no_runtime_failure`    | `import_cycle.rs`          | Genuine support | RFC-013’s W1xxx warning code and user ruling A (cycles must not be silent); the assertions require exactly one W1007 for a benign cycle and predict no runtime consequences.                                                                 |
| 5   | `test_obligation_config_disabled_skip_is_not_owed`                          | `obligations.rs`           | Genuine support | 02-stage-contract §2 “obligation ledger”; the test asserts that a ConfigDisabled skip does not create an outstanding obligation.                                                                                                             |
| 6   | `test_format_type_bool`                                                     | `types.rs`                 | Genuine support | formatter spec §9 “type annotations”; the test asserts that the type name is emitted verbatim.                                                                                                                                               |
| 7   | `test_fstring_multiline_single_quotes_inside`                               | `fstring.rs`               | Genuine support | syntax §1.6.3 “string literals”; the test asserts that a single quote inside multiline f-string content does not terminate it.                                                                                                               |
| 8   | `test_float_underscore`                                                     | `literals.rs`              | Genuine support | syntax §1.6 “literals” (1.6.2 floating-point numbers) covers underscore separators; the test asserts that 1_000.5 is lexed as 1000.5.                                                                                                        |
| 9   | `test_empty_unicode_escape_is_error`                                        | `literals.rs`              | Genuine support | syntax §1.6.3’s closed set of escapes; the test asserts that an empty \u{} escape must produce an error.                                                                                                                                     |
| 10  | `test_probe_accepted_const_type_is_registered`                              | `probe.rs`                 | Genuine support | RFC-039 P6 6.4 / WBS 6.6.9 specifically state that “constant-parameter determination follows only injected probes”; the test asserts that N is registered when the probe accepts Int.                                                        |
| 11  | `test_direct_eval_with_bound_variable_proved`                               | `predicate.rs`             | Genuine support | RFC-027 §3-4 (path-condition propagation / compile-time proof pipeline); the test asserts that b>0 evaluates directly to Proved when b=5.                                                                                                    |
| 12  | `test_unused_type_is_not_an_entry_point`                                    | `dead_code.rs`             | Genuine support | RFC-011 §7 dead-code elimination mechanism; the test asserts that an unused type definition is not a reachable root.                                                                                                                         |
| 13  | `test_display_forall_multiple_vars_no_trailing_space`                       | `ast.rs`                   | Genuine support | RFC-027 §8 SMT solver; the test asserts the printing format for forall with multiple variables.                                                                                                                                              |
| 14  | `test_dep_graph_remove_dependant_clears_edges`                              | `rfc027_phase31_dep_vc.rs` | Genuine support | RFC-027 Phase 3.1’s TypeDepGraph; the test asserts that removing a dependent clears its edges and constraints along with it.                                                                                                                 |
| 15  | `test_condition_bool`                                                       | `conditional.rs`           | Genuine support | RFC-011 §5 conditional types; the test asserts the evaluation and determinacy of TypeCondition::Bool.                                                                                                                                        |
| 16  | `test_convert_expr_to_const_expr_int_literal`                               | `const_eval.rs`            | Genuine support | RFC-011 §4 compile-time generics; the test asserts that an Int literal is converted to ConstExpr::Lit.                                                                                                                                       |
| 17  | `test_definition_on_binding_site_returns_none_precise_only`                 | `definition.rs`            | Genuine support | RFC-039 D55 (precise-only, with no name-based fallback); the test asserts that the definition site returns None when it has no reference records of its own.                                                                                 |
| 18  | `test_type_consistent_arith_passes`                                         | `verify.rs`                | Genuine support | The IR static-validator criterion in WBS 2.1 / RFC-039 P2 (the first layer of 07-equivalence-oracle); the test asserts that type-consistent arithmetic passes.                                                                               |
| 19  | `test_verify_loose_corpus_green`                                            | `verify.rs`                | Genuine support | The same validator criterion; the test asserts that the corpus passes entirely green.                                                                                                                                                        |
| 20  | `test_parse_tag_tolerates_query_and_fragment`                               | `toolchain.rs`             | Genuine support | RFC-037 §installer support; the test asserts that the query and fragment are stripped from the release URL.                                                                                                                                  |
| 21  | `test_release_unset_register`                                               | `execute.rs`               | Irrelevant      | #299 §2 covers the NewArray fixed-length array instruction (allocation, Void placeholder, and bytecode round-trip); the assertion concerns “executing Release on an unset register does not panic.”                                          |
| 22  | `test_fstring_segment_interpolation`                                        | `ast.rs`                   | Irrelevant      | The test cites “Language Spec §3 (AST Structure),” but syntax §3 is the statements chapter (3.1 Statement Classification … 3.11 Program Entry and Top-Level Statements); the assertion concerns the fields of FStringSegment::Interpolation. |
| 23  | `test_expr_for`                                                             | `ast.rs`                   | Irrelevant      | The same “§3 (AST Structure)” citation; syntax §3 covers statements, while the assertion concerns the AST fields of Expr::For.                                                                                                               |
| 24  | `test_add_var_without_mut_stores_immutability`                              | `scope.rs`                 | Irrelevant      | The test cites “Language Spec §5.2”: syntax has no Chapter Five, while type-system §5.2 is “Multiple Constraints”; the assertion concerns the immutability flag stored by ScopeManager.                                                      |
| 25  | `test_type_evaluator_eval_nested_type`                                      | `evaluator.rs`             | Irrelevant      | The test cites syntax §3.11 (Program Entry and Top-Level Statements) and “RFC-027 §362” (which does not exist); the assertion concerns evaluation of a nested type.                                                                          |
| 26  | `test_get_ast_type_universe_level_plain`                                    | `mono.rs`                  | Irrelevant      | The test cites “Language Spec §3,” but syntax §3 is the statements chapter; the assertion concerns a type-universe level of 0.                                                                                                               |
| 27  | `test_unify_enum_same`                                                      | `solver.rs`                | Irrelevant      | The test cites “Language Spec §3”; the assertion concerns successful unification of enums with the same name, which belongs to type unification rather than the statements chapter.                                                          |
| 28  | `test_weak_new_upgrade_some_while_original_alive`                           | `weak.rs`                  | Irrelevant      | The test cites RFC-010’s **sum-type identity**, but the assertion concerns the lifetime of a weak reference (#384: upgrade returns Some while the original Arc is alive).                                                                    |
| 29  | `test_i18n_registry_english_titles`                                         | `codes.rs`                 | Irrelevant      | §3.4’s I3/I4 cover translations for E1090/E1091; the assertion concerns a nonempty English title and help text for E0001.                                                                                                                    |
| 30  | `test_format_return_expression`                                             | `expr.rs`                  | Generic         | The citations are a list of sections—§3, §4, §7, §8, §10–§13, §17, and §18—with no clause matching “how a return expression is rendered.”                                                                                                    |
| 31  | `test_rfc010_generic_syntax`                                                | `lexer_mod.rs`             | Generic         | The citations are syntax §1.6 plus broad references to RFC-004/RFC-010; the test describes itself as flexible (allowing any parenthesized form), and there is no verifiable clause.                                                          |
| 32  | `test_anon_binding_with_fn_annotation_keeps_extracted_signature`            | `types.rs`                 | Generic         | “spec §3, RFC-010, RFC-011” is a broad list, and no clause corresponds to “extracting an annotated signature from an anonymous function binding.”                                                                                            |
| 33  | `test_consumer_unknown_token_returns_empty`                                 | `ownership.rs`             | Generic         | “RFC-009 + RFC-009a” cites entire documents; the assertion concerns BrandTree returning an empty consumer set for an unknown token.                                                                                                          |
| 34  | `test_const_expr_neg_int_literal_displays_by_value`                         | `const_data.rs`            | Generic         | RFC-011 §4 covers compile-time generics, not the display form of ConstExpr; the assertion concerns a negative literal being printed as -5.                                                                                                   |
| 35  | `resolve_local_and_method`                                                  | `resolver.rs`              | Generic         | “RFC-029 Refactoring Core” is an entire architecture RFC; the assertions concern the resolver’s qualified names and DefKind.                                                                                                                 |
| 36  | `test_outdated_exec_smoke`                                                  | `outdated.rs`              | Generic         | RFC-014a appears only as background in a comment (the registry was unpublished, so a path dependency was used); the assertion is an exec smoke test, and the body contains no assertions.                                                    |
| 37  | `test_compatible_high_major_range`                                          | `resolver.rs`              | Generic         | “RFC-014 Phase 3” is a phase name rather than a clause; the assertion concerns compatibility determination for a version range.                                                                                                              |
| 38  | `test_clean_and_size`                                                       | `cache.rs`                 | Generic         | “RFC-014 Phase 3 (Global Cache Design)” is likewise a phase name; the assertions concern cache clean and dir_size.                                                                                                                           |
| 39  | `test_strip_ansi_multiple_consecutive_sequences`                            | `ansi.rs`                  | Generic         | §4.2 is the “ANSI Tool Extraction” work item, not a specification clause; the assertion concerns the behavior of strip_ansi for consecutive sequences.                                                                                       |
| 40  | `test_verify_sha256_accepts_matching_digest_file_content`                   | `dl.rs`                    | Generic         | “RFC-037: version directory = release-package extraction root directory” is cited; the assertion concerns sha256 verification accepting a matching digest file.                                                                              |

Distribution: **genuine support 20 · irrelevant 9 · generic 11**.

### Mis-Citation Rate and Confidence Interval

- Mis-citations (irrelevant + generic) = 20/40 = **50%**;
- Wilson 95% confidence interval (n=40): **[35.2%, 64.8%]**;
- Extrapolated to 2403: approximately **1202** cases genuinely have clause support (95% interval
  **846–1557**), and approximately **1201** are mis-cited (interval 846–1557).

### Impact on the Cleanup Surface

- **“Retain” drops from 2897 to 776** (−2121)—exactly the 2121 cases that were retained “on the
  strength of a citation” now become pending verification; this is the effect of tightening, not
  distribution tuning.
- The **136 retirement candidates remain unchanged**: the first 26 were already unrelated to clauses
  (not a single assertion in their bodies), and the audit does not change their determinations. This
  also shows that the “retirement surface” and the “clause mis-citation surface” are separate
  problems: most mis-cited tests have assertions, so the appropriate action is to add a clause or
  correct the citation rather than delete the test.
- The 2121 pending-verification cases represent the subsequent “backfill the clause gate” workload:
  add a specific clause that matches the assertions, or reclassify the case as a retirement
  candidate.

### Limitations (Do Not Cite Beyond Their Scope)

- The 40 cases can establish only a **proportion**; an individual determination cannot be
  extrapolated to other tests in the same directory;
- The determinations come from manually reading code and clause documents. The rationale column
  already states “what the cited clause actually covers” for review;
- The sample denominator is the old-criterion pool from 0.6-9 (2403). How many of the **1713** cases
  remaining after tightening are mis-cited was not resampled—tightening excludes “unverifiable”
  citations, but that does not mean every citation in the new pool genuinely supports its assertion
  (they still require case-by-case verification).

## Reconciliation with the 0.6-2 Survey

The 0.6-2 survey numbers are the first comparison baseline. The reasons for each difference are
listed below:

| Item                            | 0.6-2 survey                  | This run                                                                                                        | Reason for the difference                                                                                                                                                                           |
| ------------------------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 3445                                                                                                            | Two cases were added after 0.6-2 (regression tests for the `check` criteria of E3005 and E3022); root package lib 2969 → 2971                                                                       |
| Number of test files            | 296                           | 296                                                                                                             | Unchanged                                                                                                                                                                                           |
| Clause gate                     | 227 files (77%)               | 113 files after tightening; 1713 by item                                                                        | The criterion changed from “the tokens appear” to “a section number §<number> is present,” so the file count was nearly halved                                                                      |
| Failure-history signals         | 442 cases / 148 files         | 314 cases within the gate; three radii: test’s own 7 / file header 291 (38 files) / entire file 1218 (79 files) | The 148 files in the 0.6-2 survey cover a broader range than the current entire-file radius of 79 files, because its vocabulary and scope are broader; this run strictly uses the four gate signals |
| Characterization strong signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                    | Characterization is not one of the three gates; mixing it into a retirement conclusion would exceed the gate’s authority                                                                            |
| Weak assertions                 | 473 (14%)                     | See the “Assertion Summary” column in the TSV                                                                   | The counting units differ: 0.6-2 counted patterns that assert only is_ok/is_some; this run uses lexical counting and distinguishes strong, weak, and delegated assertions                           |

## Conclusion Summary

1. **The clause gate was previously significantly overstated**: under the old criterion, 2403 cases
   were classified as “clause/retain,” while the sampling audit found a 50% mis-citation rate (95%
   interval 35%–65%)—“having a citation” and “having a citation that supports the assertion” are
   different things, and the old criterion treated the former as the latter.
2. **Approximately 1202 cases genuinely have clause support after tightening** (interval 846–1557);
   the rest must either await verification and supplementation or should originally have been judged
   by the failure-history/proof gates.
3. **The retirement surface is unaffected**: the 136 candidates are unrelated to clauses (no
   assertions/weak assertions), and the first 26 remain unchanged.
4. **The failure-history gate is the largest legitimate source of retentions** (314 cases): it
   demonstrates that “pinning an actual incident” is more reliable than “copying a sentence from an
   RFC.”

## Initial Retirement List (Listed Only, Not Deleted)

**Batch: the 26 “no assertion” tests under `src/frontend/core/types/eval/tests/`** (a single
directory cluster whose cases share the same rationale: no assertions in their bodies). Selection
rationale: this is the repository’s only “whole-directory no-assertion cluster,” making it the
strongest deletion candidate—when a test asserts nothing, deleting it loses no criteria. The same
directory contains another 327 tests (source line counts containing assertions: `conditional.rs`
127, `const_eval.rs` 158, `operations.rs` 72, `normalizer.rs` 60, `reducer.rs` 60) that cover the
real behavior of the same APIs.

| #   | Test name                                       | File          | What it asserts                                                                                                  | Why deleting it loses no criteria                                                                                                              |
| --- | ----------------------------------------------- | ------------- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `test_normalizer_with_config`                   | normalizer.rs | No assertions: constructs `TypeNormalizer`, then discards the result of `normalize(Int(32))`                     | There are no assertions to lose; the normalized result is pinned by `test_normalizer_normalize_primitives` in the same file using `assert_eq!` |
| 2   | `test_normalizer_evaluator`                     | normalizer.rs | No assertions: only obtains the `evaluator()` handle without calling it                                          | There are no assertions to lose; handle access is incidentally covered by the reduction cases with assertions in the same file                 |
| 3   | `test_normalizer_context`                       | normalizer.rs | No assertions: only obtains the `context()` handle                                                               | Same as above                                                                                                                                  |
| 4   | `test_type_computer_new_and_with_config`        | reducer.rs    | No assertions: constructs with `new()` / `with_config()`, then discards the result                               | There are no assertions to lose; construction is a prerequisite step in the remaining computation cases, which contain assertions              |
| 5   | `test_type_computer_register_alias`             | reducer.rs    | No assertions: registers an alias, then discards the result of `compute()`                                       | There are no assertions to lose; the alias-reduction result is pinned by `test_reducer_delta_reduces_alias` using `assert_eq!`                 |
| 6   | `test_type_computer_compute_primitive`          | reducer.rs    | No assertions: discards the result of `compute(Int(32))`                                                         | There are no assertions to lose; primitive-type computation is covered by cases with assertions such as `test_reducer_stuck_on_primitives`     |
| 7   | `test_type_computer_compute_type_ref`           | reducer.rs    | No assertions: registers an alias, then discards the result of `compute(TypeRef)`                                | There are no assertions to lose; same as #5                                                                                                    |
| 8   | `test_type_computer_set_context`                | reducer.rs    | No assertions: only obtains `context()` (the comment says “just verify can be created”)                          | There are no assertions to lose                                                                                                                |
| 9   | `test_type_computer_compute_type_var`           | reducer.rs    | No assertions: discards the result of `compute(TypeVar)`                                                         | There are no assertions to lose; type-variable reduction is covered by `test_reducer_reduce_type_var`                                          |
| 10  | `test_type_computer_compute_list`               | reducer.rs    | No assertions: discards the result of `compute(List<Int>)`                                                       | There are no assertions to lose; list reduction is covered by `test_reducer_reduce_list_type`                                                  |
| 11  | `test_type_computer_compute_tuple`              | reducer.rs    | No assertions: discards the result of `compute((Int, String))`                                                   | There are no assertions to lose; tuple reduction is covered by `test_reducer_reduce_tuple_type`                                                |
| 12  | `test_type_computer_compute_fn`                 | reducer.rs    | No assertions: discards the result of `compute(Fn)`                                                              | There are no assertions to lose; function-type reduction is covered by `test_reducer_reduce_function_type`                                     |
| 13  | `test_reducer_reduce_with_limit`                | reducer.rs    | No assertions: discards the result of `reduce_with_limit()`                                                      | There are no assertions to lose; step-limit semantics are pinned by `test_reducer_step_count` (`assert_eq!`)                                   |
| 14  | `test_reducer_reduce_type_ref_with_alias_chain` | reducer.rs    | No assertions: discards the result of reducing an A→B→Int alias chain                                            | There are no assertions to lose; single-hop alias reduction is pinned by `test_reducer_delta_reduces_alias`                                    |
| 15  | `test_reducer_reduce_type_ref_self_referential` | reducer.rs    | No assertions: discards the result of reducing a self-referential alias                                          | There are no assertions to lose; a self-reference remaining stuck is covered by the `assert_eq!` in `test_reducer_unknown_alias_stays_stuck`   |
| 16  | `test_unifier_different_types`                  | reducer.rs    | No assertions: calls `unify(Int, String)`, then matches three branches, all of which are empty                   | There are no assertions to lose; different types not unifying is covered by `test_unifier_unify_different_concrete`                            |
| 17  | `test_unifier_substitution_after_unify`         | reducer.rs    | No assertions: obtains and discards the substitution table after unification                                     | There are no assertions to lose; the substitution table is covered by cases with assertions such as `test_unifier_same_types`                  |
| 18  | `test_unifier_unify_option_same`                | reducer.rs    | No assertions: discards the result of `unify(Option<Int>, Option<Int>)` (the comment says “may fail or succeed”) | There are no assertions to lose; this form has no defined result by design, so deleting it loses no criteria                                   |
| 19  | `test_unifier_unify_result_same`                | reducer.rs    | No assertions: discards the result of `unify(Result<Int>, Result<Int>)`                                          | There are no assertions to lose; this form has no defined result by design, so deleting it loses no criteria                                   |
| 20  | `test_unifier_unify_dict_same`                  | reducer.rs    | No assertions: discards the result of `unify(Dict<String, Int>, Dict<String, Int>)`                              | There are no assertions to lose; this form has no defined result by design, so deleting it loses no criteria                                   |
| 21  | `test_unifier_unify_set_same`                   | reducer.rs    | No assertions: discards the result of `unify(Set<Bool>, Set<Bool>)`                                              | There are no assertions to lose; container-type unification is covered by `test_unifier_unify_composite_types`                                 |
| 22  | `test_unifier_unify_range_same`                 | reducer.rs    | No assertions (Range)                                                                                            | Same as above                                                                                                                                  |
| 23  | `test_unifier_unify_arc_same`                   | reducer.rs    | No assertions (Arc)                                                                                              | Same as above                                                                                                                                  |
| 24  | `test_unifier_unify_weak_same`                  | reducer.rs    | No assertions (Weak)                                                                                             | Same as above                                                                                                                                  |
| 25  | `test_unifier_unify_enum_same`                  | reducer.rs    | No assertions (Enum)                                                                                             | Same as above                                                                                                                                  |
| 26  | `test_unifier_unify_struct_same`                | reducer.rs    | No assertions (Struct)                                                                                           | Same as above                                                                                                                                  |

**Preconditions for deletion (left as gates for the execution batch; not performed by this work
item)**: ① cases with assertions in the same directory must still cover the same API (otherwise,
first add assertions to those cases); ② deletion must be validated in the same batch as
`cargo test --lib` / `run fast`; ③ update this page and `test-gate-census.tsv` in the same commit
(`--check` will fail if they are not); ④ register the change under
[Part Seven of coding-rules](./coding-rules.md).

## Rerun and Self-Check

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: check whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Run minimal positive/negative self-tests (does not scan the repository)
python scripts/dev/test-gate-census-audit.py --sample  # Reproduce the 40-case sample (seed 20261106)
python scripts/dev/test-gate-census-audit.py --check   # Check whether the audit data file covers every sample
```

The generator performs lexical classification only, and states its boundaries honestly:

- **Delegated assertions are recognized by name only** (`assert*`/`check*` helper functions). If a
  test’s assertions are hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as “no assertions”—the first 26 cases were manually reviewed case by case, and the
  rationales on this page are based on that manual review.
- **The proof gate is not decided**: any test that misses both the first two gates is classified
  only as “pending determination” or “retirement candidate”; the generator never declares that it
  “should be deleted.”
- **No cross-file deduplication**: completely duplicate tests are not identified in this table
  (0.6-2 measured them at only 1%, so they do not constitute a retirement surface).
- **The audit covers only 40 cases**: its conclusions are used only to classify those 40 cases and
  provide a proportion for the 2403-case pool; an individual determination must not be extrapolated
  to other tests in the same directory.
