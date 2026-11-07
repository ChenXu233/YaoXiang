---
title: 'Census of the Three Test Gates (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a stratified sample audit of the clause gate together with the initial
  retirement candidate list'
---

# Census of the Three Test Gates (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **three gates (clause / incident history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a rerunnable per-test determination
table; then perform a stratified sample audit of the clause gate (40 tests), measure the proportion
of tests that “cite a clause but cannot support the assertion,” and recompute the distribution using
the **tightened criteria**. **This page and the generator itself are read-only and do not modify
tests**; test retirement is carried out in batches (one commit per batch, fully revertible with
`git revert`), and the execution results are recorded in the three sections below— “First Retirement
/ Second Retirement / Third Retirement”—and in `09a`.

Generated artifacts:

| Artifact                                  | Purpose                                                                                                                                                       |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv`       | Per-test determination table (3411 rows plus the header), located in the same directory as this page                                                          |
| `scripts/dev/test-gate-census.py`         | Generator (rerunnable, supports `--check` and `--selftest`)                                                                                                   |
| `scripts/dev/test-gate-census-audit.py`   | Sample audit: reproduces the 40-test sample with a fixed seed (`--sample` / `--check`)                                                                        |
| `scripts/dev/test-gate-census-audit.json` | Per-test determinations and evidence for the 40 tests (the generator reads this file and classifies “confirmed by the audit as genuinely supporting” as Keep) |

## Criteria

### Scanning Scope

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 2939                 |
| `tests/integration`                                                      | 335                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 75                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **3411** (296 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine Criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Meaning of a Match                                              |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | A **verifiable entry** appears within the first 25 lines of the file header: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); for files under `src/**/tests/`, inspect the parent `mod.rs` together with the file                                                                                                                                                                                                           | The test maps to a specific, citable clause                     |
| Incident-history gate             | The test function body, function doc comment, or **file header** (such as `//! 回归背景：issue #226`) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                               | The test pins down an incident that has actually occurred       |
| Proof gate                        | **Cannot be determined mechanically** (“If it were replaced by another correct implementation, would it fail?” requires human judgment). Therefore, every test that fails the first two gates is classified as “pending determination,” not automatically as “neither passes”; the sole exception is a test with **no assertions at all** in its body (including no delegation to an `assert*`/`check*` helper, no `match` + `panic!` form, and no `#[should_panic]`) | Only “pending determination” or “no assertions” can be assigned |

Reason for tightening: the clause gate in 0.6-9 required only the **presence of `§`/`RFC`/`SPEC`**
in the file header, so 2403 tests were classified as “Keep.” The 40-test stratified sample in 0.6-10
found that **half of the citations could not support the assertions** (see below). After tightening,
only the section-number form is accepted, and **the presence of a citation still does not mean that
the citation supports the assertion**—every unaudited clause citation is classified only as “needs
review.”

Reason for the file-header radius: this repository conventionally places text such as “regression
background: issue #N” in the file header and does not repeat it in individual test comments;
examining only each test’s own comments would miss the incident history of entire files (measured: 7
tests under the test’s own radius, versus 291 under the file-header radius).

### Categories That Are Never Retired

The following tests are **always classified as “Keep”** and do not enter the retirement candidate
pool:

- Corpus `tests/yaoxiang/**` and project-shape `tests/project-corpus/**` (behavior-criterion
  surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and crossing the process boundary);
- Tests whose names contain `unchanged` / `anchor` / `guard` / `preserv` / `baseline`, which anchor
  the status quo.

### Disposition Semantics

| Disposition           | Meaning                                                                                                                                                                                                                    |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                  | Only three sources qualify: a history-gate match, the proof gate (a spec-shaped directory / a status-quo anchor name), or a **clause citation confirmed by the sample audit to provide genuine support** for the assertion |
| Needs review          | Cites a clause (with a section number, or only an entire RFC without a section number) but has not been audited / was judged irrelevant or generic by the audit; a human must review it before it can be kept              |
| Retirement candidate  | Fails both gates and has only weak assertions (or no assertions) in its assertion surface—a candidate is not a verdict; the proof gate still requires human review                                                         |
| Pending determination | Fails both gates but has strong assertions; the machine reaches no conclusion                                                                                                                                              |

## Distribution After Tightening

| Gate                                                                  | Count    |
| --------------------------------------------------------------------- | -------- |
| Clause (with a verifiable entry)                                      | 1714     |
| Incident history                                                      | 313      |
| Proof (spec-shaped criterion surface / status-quo anchors)            | 447      |
| Pending determination (fails both gates; proof awaits human judgment) | 926      |
| Neither passes (no assertions in the body)                            | 11       |
| **Total**                                                             | **3411** |

| Disposition           | Count    | Compared with 0.6-9                                                                                                                                      |
| --------------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                  | **775**  | 2897 (−2122)                                                                                                                                             |
| Needs review          | 2122     | 0 (new category)                                                                                                                                         |
| Pending determination | 414      | 412 (−1 after the second retirement batch, +3 from the 0.6-15 criterion correction)                                                                      |
| Retirement candidate  | 100      | 136 (21 in the first retirement batch, 11 in the second, +1 from the generator correction, −2 in the third, and −3 from the 0.6-15 criterion correction) |
| **Total**             | **3411** | —                                                                                                                                                        |

Two changes in criteria require explanation:

1. **Structure of “Keep”**: 775 = 447 from the proof gate + 313 from the incident-history gate + 15
   confirmed by the audit to provide genuine support (5 of the audit’s 20 genuinely supported tests
   were already in files retained by the incident-history gate). The 2121 original “Keep” decisions
   based on nothing more than a citation have now moved to “Needs review.”
2. **Incident-history gate 47 → 313**: this is not an increase in incident history; it is
   **reassignment of labels**. In 0.6-9, tests that matched both the clause and incident-history
   gates were recorded under the clause gate. Once tightening removed the automatic “Keep” decision
   for the clause gate, these tests had to be captured by the incident-history gate, so their counts
   were folded into that category. The total number of “Keep” decisions is therefore unchanged
   (those tests were also classified as “Keep” in 0.6-9).

File-level criteria: of the 296 test files, **113** have a verifiable section number in the file
header (the old 0.6-9 criterion identified 221 files).

## Clause-Gate Sample Audit (0.6-10)

### Sampling Criteria (Reproducible)

- **Candidate pool = the 2403 tests classified as “Clause” by the 0.6-9 clause gate** (frozen using
  the old criteria in `test-gate-census-audit.py`; the sampling denominator must not change with the
  new criteria, or the results will not be comparable).
- **Strata = directory clusters**: take the first two levels under `src/` (`src/frontend`,
  `src/middle`, …), and group everything else by top-level directory (`tests`, `benches`, `tools`).
- **Allocation**: proportional to the number of tests in each stratum (largest remainder method),
  with at least one test per stratum.
- **Sampling**: sort within each stratum by (file, line number, test name), then sample with a fixed
  random seed.
- **Seed = 20261106** (hard-coded in the script; changing the seed changes the sample, so the change
  must be documented in this page and the audit rerun).

Stratified result: `src/frontend` 21 · `tests` 4 · `src/driver` 3 · `src/package` 3 · `src/backends`
2 · `src/formatter` 2 · `src/middle` 2 · `src/util` 2 · `tools` 2 · `src/ast` 1 · `src/lsp` 1 ·
`src/std` 1.

### Per-Test Determinations for the 40 Tests

Decision criteria (fixed here to avoid inconsistent decisions from one test to another):

- **Genuine support**: the citation identifies a specific section/note/decision number, and its
  **subject** is the same as the assertion target;
- **Irrelevant**: the citation identifies a specific entry, but that entry concerns a **different
  subject** (the evidence column states what it actually covers);
- **Generic**: the citation refers only to an entire document/chapter/a list of sections/a phase
  name, without a single entry that maps to the assertion.

| #   | Test                                                                        | File                       | Determination   | Evidence (what the cited clause actually covers)                                                                                                                                                                                                |
| --- | --------------------------------------------------------------------------- | -------------------------- | --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `test_compile_time_value_param_spec_table`                                  | `classify.rs`              | Genuine support | type-system §7.1 “Compile-Time Value Parameters” is exactly the assertion target: the truth table for distinguishing value-position from type-position parameters is checked row by row.                                                        |
| 2   | `test_id_stable_under_subsequent_registrations`                             | `value.rs`                 | Genuine support | RFC-010 “Runtime Representation and Equality” defines the identity convention for sum types: identity is determined only by the name; the assertion verifies that the ID does not change after subsequent registrations.                        |
| 3   | `test_driver_run_check_rootless_multi_file_well_formed_generic_stays_clean` | `check_rootless_mono.rs`   | Genuine support | Note 14, item ③ of 02-stage-contract §3 discusses monomorphization-name homology; this test is the pinning test for that note, with both the name and assertion matching.                                                                       |
| 4   | `test_multifile_benign_import_cycle_message_predicts_no_runtime_failure`    | `import_cycle.rs`          | Genuine support | RFC-013’s W1xxx warning code + user adjudication A (a cycle must not be silent); the assertion verifies that a benign cycle produces exactly one W1007 and predicts no runtime consequence.                                                     |
| 5   | `test_obligation_config_disabled_skip_is_not_owed`                          | `obligations.rs`           | Genuine support | 02-stage-contract §2 “Obligation Ledger”; the assertion verifies that a ConfigDisabled skip does not create an outstanding obligation.                                                                                                          |
| 6   | `test_format_type_bool`                                                     | `types.rs`                 | Genuine support | formatter specification §9 type annotations; the assertion verifies that the type name is emitted unchanged.                                                                                                                                    |
| 7   | `test_fstring_multiline_single_quotes_inside`                               | `fstring.rs`               | Genuine support | syntax §1.6.3 string literals; the assertion verifies that a single quote inside multiline f-string content does not terminate it.                                                                                                              |
| 8   | `test_float_underscore`                                                     | `literals.rs`              | Genuine support | syntax §1.6 literals (1.6.2 floating-point numbers) cover underscore separators; the assertion verifies that 1_000.5 is lexed as 1000.5.                                                                                                        |
| 9   | `test_empty_unicode_escape_is_error`                                        | `literals.rs`              | Genuine support | syntax §1.6.3 defines a closed set of escapes; the assertion verifies that an empty \u{} escape must produce an error.                                                                                                                          |
| 10  | `test_probe_accepted_const_type_is_registered`                              | `probe.rs`                 | Genuine support | RFC-039 P6 6.4 / WBS 6.6.9 states that “constant-parameter decisions obey only an injected probe”; the assertion verifies that N is registered when the probe accepts Int.                                                                      |
| 11  | `test_direct_eval_with_bound_variable_proved`                               | `predicate.rs`             | Genuine support | RFC-027 §3-4 (path-condition propagation / compile-time proof pipeline); the assertion verifies that b>0 directly evaluates to Proved when b=5.                                                                                                 |
| 12  | `test_unused_type_is_not_an_entry_point`                                    | `dead_code.rs`             | Genuine support | RFC-011 §7 dead-code elimination mechanism; the assertion verifies that an unused type definition is not a reachable root.                                                                                                                      |
| 13  | `test_display_forall_multiple_vars_no_trailing_space`                       | `ast.rs`                   | Genuine support | RFC-027 §8 SMT solver; the assertion verifies the printing format for forall with multiple variables.                                                                                                                                           |
| 14  | `test_dep_graph_remove_dependant_clears_edges`                              | `rfc027_phase31_dep_vc.rs` | Genuine support | RFC-027 Phase 3.1’s TypeDepGraph; the assertion verifies that removing a dependent also clears its edges and constraints.                                                                                                                       |
| 15  | `test_condition_bool`                                                       | `conditional.rs`           | Genuine support | RFC-011 §5 conditional types; the assertion verifies the evaluation and determinacy of TypeCondition::Bool.                                                                                                                                     |
| 16  | `test_convert_expr_to_const_expr_int_literal`                               | `const_eval.rs`            | Genuine support | RFC-011 §4 compile-time generics; the assertion verifies conversion of an Int literal into ConstExpr::Lit.                                                                                                                                      |
| 17  | `test_definition_on_binding_site_returns_none_precise_only`                 | `definition.rs`            | Genuine support | RFC-039 D55 (precise-only; no name-based fallback); the assertion verifies that the definition site returns None when it has no reference record of its own.                                                                                    |
| 18  | `test_type_consistent_arith_passes`                                         | `verify.rs`                | Genuine support | IR static-validator criteria in WBS 2.1 / RFC-039 P2 (the first layer of 07-equivalence-oracle); the assertion verifies that type-consistent arithmetic passes.                                                                                 |
| 19  | `test_verify_loose_corpus_green`                                            | `verify.rs`                | Genuine support | The same validator criterion; the assertion verifies that the corpus runs entirely green.                                                                                                                                                       |
| 20  | `test_parse_tag_tolerates_query_and_fragment`                               | `toolchain.rs`             | Genuine support | RFC-037 §installer support; the assertion verifies that query/fragment components are stripped from the release URL.                                                                                                                            |
| 21  | `test_release_unset_register`                                               | `execute.rs`               | Irrelevant      | #299 §2 covers the NewArray fixed-length array instruction (allocation, Void placeholder, and bytecode round-trip); the assertion instead verifies that “executing Release with an unset register does not panic.”                              |
| 22  | `test_fstring_segment_interpolation`                                        | `ast.rs`                   | Irrelevant      | It cites “Language Specification §3 (AST Structure),” but syntax §3 is the statements chapter (3.1 Statement Classification … 3.11 Program Entry and Top-Level Statements); the assertion concerns the fields of FStringSegment::Interpolation. |
| 23  | `test_expr_for`                                                             | `ast.rs`                   | Irrelevant      | It cites the same “§3 (AST Structure)” reference; syntax §3 covers statements, while the assertion concerns the AST fields of Expr::For.                                                                                                        |
| 24  | `test_add_var_without_mut_stores_immutability`                              | `scope.rs`                 | Irrelevant      | It cites “Language Specification §5.2”: syntax has no Chapter 5, while type-system §5.2 is “Multiple Constraints”; the assertion instead concerns the immutability flag stored by ScopeManager.                                                 |
| 25  | `test_type_evaluator_eval_nested_type`                                      | `evaluator.rs`             | Irrelevant      | It cites syntax §3.11 (Program Entry and Top-Level Statements) and “RFC-027 §362” (which does not exist); the assertion concerns the evaluation of nested types.                                                                                |
| 26  | `test_get_ast_type_universe_level_plain`                                    | `mono.rs`                  | Irrelevant      | It cites “Language Specification §3,” but syntax §3 is the statements chapter; the assertion verifies that the type-universe level is 0.                                                                                                        |
| 27  | `test_unify_enum_same`                                                      | `solver.rs`                | Irrelevant      | It cites “Language Specification §3”; unifying enums with the same name is type unification, not part of the statements chapter.                                                                                                                |
| 28  | `test_weak_new_upgrade_some_while_original_alive`                           | `weak.rs`                  | Irrelevant      | It cites RFC-010’s **sum-type identity**, but the assertion concerns the lifecycle of a weak reference (#384: upgrade returns Some while the original Arc is alive).                                                                            |
| 29  | `test_i18n_registry_english_titles`                                         | `codes.rs`                 | Irrelevant      | §3.4’s I3/I4 cover translations for E1090/E1091; the assertion instead verifies that E0001 has a nonempty English title and help text.                                                                                                          |
| 30  | `test_format_return_expression`                                             | `expr.rs`                  | Generic         | The citation is a list of sections—§3, §4, §7, §8, §10–§13, §17, and §18—with no entry that maps to “how a return expression is rendered.”                                                                                                      |
| 31  | `test_rfc010_generic_syntax`                                                | `lexer_mod.rs`             | Generic         | The citation consists of syntax §1.6 plus generic references to RFC-004/RFC-010; the assertion describes itself as flexible (allowing arbitrary parenthesization), with no verifiable entry.                                                    |
| 32  | `test_anon_binding_with_fn_annotation_keeps_extracted_signature`            | `types.rs`                 | Generic         | “spec §3, RFC-010, RFC-011” is a generic list, with no entry corresponding to “extracting an annotated signature from an anonymous function binding.”                                                                                           |
| 33  | `test_consumer_unknown_token_returns_empty`                                 | `ownership.rs`             | Generic         | “RFC-009 + RFC-009a” cites entire documents; the assertion verifies that BrandTree returns an empty consumer set for an unknown token.                                                                                                          |
| 34  | `test_const_expr_neg_int_literal_displays_by_value`                         | `const_data.rs`            | Generic         | RFC-011 §4 covers compile-time generics and does not concern the printing form of ConstExpr; the assertion verifies that a negative literal is printed as -5.                                                                                   |
| 35  | `resolve_local_and_method`                                                  | `resolver.rs`              | Generic         | “RFC-029 Refactoring Core” is an entire architectural RFC; the assertion concerns the resolver’s qualified name and DefKind.                                                                                                                    |
| 36  | `test_outdated_exec_smoke`                                                  | `outdated.rs`              | Generic         | RFC-014a is only background in a comment (the registry was unpublished, so a path dependency is used); the assertion is an exec smoke test with no assertions in the body.                                                                      |
| 37  | `test_compatible_high_major_range`                                          | `resolver.rs`              | Generic         | “RFC-014 Phase 3” is a phase name, not an entry; the assertion verifies compatibility decisions for version ranges.                                                                                                                             |
| 38  | `test_clean_and_size`                                                       | `cache.rs`                 | Generic         | “RFC-014 Phase 3 (Global Cache Design)” is likewise a phase name; the assertion verifies cache clean and dir_size.                                                                                                                              |
| 39  | `test_strip_ansi_multiple_consecutive_sequences`                            | `ansi.rs`                  | Generic         | §4.2 is the “ANSI Utility Extraction” work item, not a specification clause; the assertion verifies the behavior of strip_ansi for consecutive sequences.                                                                                       |
| 40  | `test_verify_sha256_accepts_matching_digest_file_content`                   | `dl.rs`                    | Generic         | It cites “RFC-037: Version Directory = Extracted Release Package Root”; the assertion verifies that sha256 verification accepts a matching digest file.                                                                                         |

Distribution: **20 genuine support · 9 irrelevant · 11 generic**.

### Incorrect-Citation Rate and Confidence Interval

- Incorrect citations (irrelevant + generic) = 20/40 = **50%**;
- Wilson 95% confidence interval (n=40): **[35.2%, 64.8%]**;
- Extrapolated to 2403: approximately **1202 tests** genuinely have clause support (95% interval
  **846–1557**), while approximately **1201 tests** have incorrect citations (interval 846–1557).

### Impact on the Cleanup Surface

- **“Keep” falls from 2897 to 775** (−2122)—exactly the 2122 tests previously kept “on the strength
  of a citation” have moved to “Needs review”; this is the effect of tightening, not distribution
  tuning.
- **Retirement candidates are unrelated to clauses**: the first batch of 26 was already unrelated to
  clauses (not a single assertion in the body), so the audit does not change their determinations.
  The first batch retired 21 tests, reducing candidates from 136 → 115; the second batch retired 11
  tests, reducing candidates from 115 → 104; after 0.6-13 fixed the generator (function-body
  boundaries were no longer truncated by braces inside literals), the count rose by 1, from 104 →
  105; the third batch retired 2 tests, reducing candidates from 105 → 103; after 0.6-15 fixed the
  criteria (helper names containing digits / the `match` + `panic!` form were recognized as
  assertions), 3 tests moved out of “Neither passes,” reducing candidates from 103 → 100. The same
  applies to the 11 tests in the second batch: they **do** contain assertions, but only weak
  assertions such as `is_some()` / `is_none()`, which do not constrain binding power or the parsing
  result. This also shows that the “retirement surface” and the “incorrect-citation surface” are
  separate issues: most tests with incorrect citations have assertions; the appropriate action is to
  supply a clause or correct the citation, not to delete the test.
- The 2122 tests classified as “Needs review” represent the workload for backfilling the clause
  gate: supply a specific entry that maps to the assertion, or reclassify the test as a retirement
  candidate.

### Limitations (Do Not Cite Beyond These Bounds)

- A sample of 40 tests can provide only a **proportion**; it cannot extrapolate an individual test’s
  determination to other tests in the same directory;
- The determinations were produced by manually reading the code and the clause documents. The
  evidence column records “what the cited clause actually covers” for review;
- The sampling denominator is the old-criteria pool from 0.6-9 (2403). The incorrect-citation rate
  among the **1714 tests remaining after tightening** was not resampled—tightening merely excludes
  “unverifiable” citations; it does not mean every citation in the new pool genuinely supports its
  assertion (each still requires individual review).

## Reconciliation with the 0.6-2 Survey

The figures from the 0.6-2 survey constitute the first comparison baseline. The reasons for each
difference are listed below:

| Item                            | 0.6-2 Survey                  | This Census                                                                                                          | Reason for the Difference                                                                                                                                                                                                                                                  |
| ------------------------------- | ----------------------------- | -------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 3411                                                                                                                 | 2 tests were added after 0.6-2 (the check-criterion regression tests for E3005 and E3022), while 21 no-assertion tests were removed according to the first batch, 11 weak-assertion tests according to the second batch, and 2 tests according to the third batch; net −32 |
| Number of test files            | 296                           | 296                                                                                                                  | Unchanged                                                                                                                                                                                                                                                                  |
| Clause gate                     | 227 files (77%)               | 113 files after tightening; 1714 tests                                                                               | The criterion was tightened from “the keyword appears” to “there is a section number `§<number>`,” so the file count was nearly halved                                                                                                                                     |
| Incident-history signals        | 442 tests / 148 files         | 313 tests within the gate; three radii: the test itself 7 / file header 291 (38 files) / entire file 1218 (79 files) | The 148 files in the 0.6-2 survey were broader than the 79 files under this census’s entire-file radius; its vocabulary and scope were larger. This census strictly uses the four signals on the gate card.                                                                |
| Characterization strong signals | 1176 (34%) / strong 631 (18%) | Not tested                                                                                                           | Characterization is not one of the three gates; mixing it into retirement conclusions would exceed the gate’s authority                                                                                                                                                    |
| Weak assertions                 | 473 (14%)                     | See the “Assertion Summary” column in the TSV                                                                        | The counting unit differs: 0.6-2 counted patterns that asserted only is_ok/is_some; this census uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                                             |

## Conclusion Summary

1. **The clause gate was previously significantly overstated**: under the old criterion, 2403 tests
   were classified as “Clause/Keep”; the sampled audit found an incorrect-citation rate of 50% (95%
   interval 35%–65%)—“citing a clause” and “the citation supports the assertion” are different
   things, and the old criterion treated the former as the latter.
2. **Approximately 1202 tests genuinely have clause support after tightening** (interval 846–1557);
   the rest must either receive the missing citations and be reviewed or were already properly
   covered by the incident-history gate / proof gate.
3. **The retirement surface is unrelated to incorrect clause citations**: the 136 candidates are
   unrelated to clauses (no assertions / weak assertions); 21 were retired in the first batch and 11
   in the second, with +1 from the generator correction and −2 from the third batch, reducing the
   candidate count to 103.
4. **The incident-history gate is the largest legitimate source of “Keep” decisions** (313 tests):
   it demonstrates that “pinning down one real incident” is more reliable than “copying a sentence
   from an RFC.”

## First Retirement Batch (Executed, 2026-11-06)

**The first batch retired 21 of the 26 “no-assertion” tests in `src/frontend/core/types/eval/tests/`
and retained 5** (a single directory cluster; criterion = all **three gates fail** according to Part
Seven of coding-rules: no clause gate, no incident-history gate, and no assertions at all in the
body). The per-test determinations and “same-cell coverage” are provided by the 0.6-11a
determination table (21 deletions / 5 retentions). This page does not repeat the reasons and records
only the execution result.

- **21 tests retired** (2 in `normalizer.rs` + 19 in `reducer.rs`): `test_normalizer_with_config`,
  `test_normalizer_evaluator`, `test_type_computer_register_alias`,
  `test_type_computer_compute_primitive`, `test_type_computer_set_context`,
  `test_type_computer_compute_type_var`, `test_type_computer_compute_list`,
  `test_type_computer_compute_tuple`, `test_type_computer_compute_fn`,
  `test_reducer_reduce_with_limit`, `test_unifier_different_types`,
  `test_unifier_substitution_after_unify`, `test_unifier_unify_option_same`,
  `test_unifier_unify_result_same`, `test_unifier_unify_dict_same`, `test_unifier_unify_set_same`,
  `test_unifier_unify_range_same`, `test_unifier_unify_arc_same`, `test_unifier_unify_weak_same`,
  `test_unifier_unify_enum_same`, `test_unifier_unify_struct_same`.
- **5 tests retained** (the determination table could not identify alternative coverage, so they
  were retained as required): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the entire batch.

## Second Retirement Batch (Executed, 2026-11-06)

**The second batch retired 11 of the 18 “weak-assertion” tests in
`src/frontend/core/parser/pratt/tests/nud.rs` and retained 7** (a single directory cluster = the
Pratt prefix-parsing module; criterion = all **three gates fail** according to Part Seven of
coding-rules: no clause gate, no incident-history gate, and the body contains only weak assertions
such as `is_some()` / `is_none()`—neither binding power nor the parsing result is constrained).

The difference from the first batch must be stated clearly: these tests **do contain assertions**
and are not empty tests. The reason for retirement is “all three gates fail + zero criterion
increment,” not “no assertions”—every test selected for deletion names a **unit-level alternative
coverage test in the same module**.

- **11 tests retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The alternative
  coverage for every test follows the **same** `parse_expression → parse_prefix → prefix_info` path
  and makes stronger assertions (constraining the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, and `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 tests retained**: `test_prefix_info_match`, `test_prefix_info_unsafe`,
  `test_prefix_info_spawn`, `test_prefix_info_at`, `test_prefix_info_break_removed`,
  `test_prefix_info_continue_removed`, `test_prefix_info_eof`—see the “VI. Second Test Retirement
  Batch” determination table in `09a-wbs-history.md` for the per-test reasons. The retention reasons
  fall into two categories: **the proof gate holds** (the first three, together with
  `test_prefix_info_at` / `test_prefix_info_eof`, are the sole unit-level probes for the
  corresponding facts, and the repository contains no second unit test that observes them), and
  **the incident-history gate holds** (`break_removed` / `continue_removed` name #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the entire batch.

## Generator Correction (0.6-13)

**`body_span` truncates function bodies at braces inside literals—this has been fixed.**

The old implementation balanced braces to obtain the function body without distinguishing real code
from braces in string/character literals or comments. Such expressions as `tokenize("{")` are common
in Rust source, so the balance was thrown off and the function body was truncated: the body of
`test_prefix_info_lbrace` was once truncated to the end of the file, causing the “Assertion Summary”
to capture the rest of the file (`matchesx11 assertx24 …`) and the test to be classified as “Pending
Determination” rather than “Retirement Candidate”—a **classification error**.

Fix: add `mask_noncode` to first replace the contents of string literals, character literals, line
comments, block comments (which may be nested), and raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (preserving newlines exactly, so line numbers and line counts do not change),
and only then balance the braces. **The mask is used solely to locate function-body boundaries**;
assertion counting and the incident-history gate still read the original source, and the criteria
themselves have not been relaxed. `--selftest` now includes 4 cases (paired/unpaired braces in
ordinary strings, and braces in raw strings and block comments), each followed by a sentinel
test—the old implementation found the wrong boundary in all 4 cases, while the new implementation
finds all of them correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** file by file and
test by test (the old implementation balanced braces without masking; the new implementation
balances them after masking). Any test with different boundaries is affected. Of the 3413 tests in
the repository, the old implementation found the wrong boundary for **11**:

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

### Classification Differences Before and After the Fix: 2

For the other 9 tests, **8 affected only the “Assertion Summary” column** (their classification did
not change), and **1 did not change even its summary**
(`test_fstring_close_brace_escape_preserved_raw`: the old implementation omitted the closing `}`
line, and that line contains no assertion keywords). The 8 summary changes are listed individually
in this section of `09a-wbs-history.md`.

| Test Name                   | File                                          | Old Classification                            | New Classification                               | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| --------------------------- | --------------------------------------------- | --------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending determination / Pending determination | Pending determination / **Retirement candidate** | The real function body contains only one weak assertion, `assert!(state.prefix_info().is_some())`; the file header and body contain no § section number, issue/regression/defect/incident marker, so neither the clause gate nor the incident-history gate passes. The body contains only a weak assertion such as `is_some()` ⇒ Retirement candidate (a candidate is not a verdict; the proof gate still requires human review)                               |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident history / Keep                       | Clause / **Needs review**                        | The old implementation truncated the body to the end of the file and swallowed text containing “regression / defect / incident,” incorrectly matching the incident-history gate. The real body contains no such wording ⇒ the incident-history gate no longer passes; the file header contains verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate matches, but without confirmation by the sample audit ⇒ Needs review |

The distribution changes accordingly: Clause 1713 → 1714 · Incident History 314 → 313 · Keep 776 →
775 · Needs Review 2121 → 2122 · Pending Determination 412 → 411 · Retirement Candidate 104 → 105.
**The repository-wide `#[test]` count remains unchanged (3413)**—this round changes only the
implementation of the criteria and does not modify any test.

## Third Retirement Batch (Executed, 2026-11-07)

**The third batch individually determined the 5 retirement candidates in
`src/middle/passes/codegen/tests/`, retiring 2 and retaining 3.** (a single directory cluster = the
`middle::passes::codegen` module under test; criterion = all **three gates fail** according to Part
Seven of coding-rules.)

This batch follows the retention criterion established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus / integration tests operate
across a process boundary and therefore cannot replace a unit-level observation point.

- **2 tests retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—its function body is **verbatim
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (apart from
    comments, only the function name differs): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that “the `execute_module` entry
    path returns non-Void,” but the body never observes the return value from `execute_module`
    (`run_fixture` discards it with `.expect(...)`)— the claimed intent was not implemented, and the
    function body is a pure duplicate ⇒ zero criterion increment. Alternative coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file strictly covers it:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)`, identical in shape to the inline `Symbol` in this
    test). “A name that was never inserted cannot be found” and “a name cannot be found after being
    popped” follow the same path in `SymbolScopeManager::lookup` (walk each `scope.get(name)`; if
    not found, return `None`) ⇒ this test adds zero coverage. Alternative coverage =
    `test_scope_nesting`.
- **3 tests retained**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that
    “executes directly without serialization” (the serialization variant is a different path), and
    `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization round-trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit-level observation point for the
    `id > 255` rejection branch of `OperandResolver::to_reg` (the bytecode layer has independent
    overflow checks that do not cover this function; no second test in the repository drives
    `to_reg` out of range). `is_err()` is a discriminating assertion (removing the guard would make
    the test fail), not merely proof that the code did not panic.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the entire batch.

### A Census Blind Spot Found in This Batch (**Fixed in 0.6-15**, see “Generator Criterion Correction” below)

The census’s recognition of “delegated assertions” uses `\b(assert|check)[A-Za-z_]*\s*\(`, which
**does not match helpers with digits in their names**: because `[A-Za-z_]*` excludes digits, a call
such as `assert_slot_is_42(&interp)` was not recognized as an assertion— so 2 of the 5 candidates in
this batch were incorrectly classified as “no assertions in the body.”

After all 15 repository-wide “Neither passes” cases were individually reviewed (by allowing digits
in helper names and additionally recognizing the `match` + `panic!` form), **4 did in fact contain
assertions**—3 used the numeric helper above (`assert_slot_is_42`), and 1 was `test_weak_from_arc`
in `src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule
6.1 requires `assert!(matches!(...))`, but the existing code does contain an assertion). The other
11 truly contain no assertions.

**Taking these 4 directly from the machine’s “Neither passes” category would have deleted tests that
do contain criteria**—manual review in this batch prevented that; the generator-side fix is
described in “Generator Criterion Correction (0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots have been fixed: helper names containing digits, and the `match` +
`panic!` form.** This round **does not delete any test** (the repository-wide `#[test]` count
remains 3411); it changes only how the generator determines whether a test body “contains
assertions.”

### What Was Fixed

1. **Digits are now allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern did not
   match helper names such as `assert_slot_is_42(&interp)`, so tests containing such calls were
   treated as “not a single assertion in the body.”
2. **Added recognition of the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the
   assertion is written in an arm, so the `assert!` macro is not visible lexically
   (test-specification rule 6.1 requires `assert!(matches!(...))`, but this form exists in the
   current code). Detection method: after the `match` keyword, scan the expression up to the first
   `{` to identify the block; determine the block boundary by **balancing braces**; the form counts
   only if `panic!(` occurs inside the block. **The determination is performed on masked lines**—
   braces in strings and comments do not participate in balancing, and `panic!` in comments is not
   mistaken for an assertion.

### Classification Differences: 3 (the gates and dispositions changed)

| Test Name                                           | File                                           | Old Classification                    | New Classification                            | Evidence                                                                                                                                                                                                                                                                 |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither passes / Retirement candidate | Pending determination / Pending determination | The body contains `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—an assertion of the variant whose form was not previously recognized; it is now recognized as an assertion, and the machine does not determine its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending determination / Pending determination | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit and did not match the old pattern; it is now recognized as an assertion                                                                                                      |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending determination / Pending determination | Same as above (the serialization round-trip test)                                                                                                                                                                                                                        |

Another **77 tests changed only the “Assertion Summary” column** (their gates and dispositions did
not change): most gained `match_panicxN` in the summary because their bodies contain the `match` +
`panic!` form, and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3,
`src/package/tests/cache.rs`: 1—helper names containing digits).

**The relaxation changes only the “Neither passes” category**: that category is the machine’s
**confident verdict** (not a single assertion in the body), and its criterion is a hard requirement;
a “Retirement candidate” is only a pool awaiting human review, so the size of that pool is not a
verdict. All other classifications, including the weak-assertion pool, follow the original criteria.
Result: Gate Neither passes 14 → 11 · Pending determination 923 → 926; Disposition Retirement
candidate 103 → 100 · Pending determination 411 → 414.

### Existing 11 “Neither Passes” Cases: Individual Review Conclusions

After individually reading the source of all 11 repository-wide “Neither passes” cases, **all 11
truly contain no assertions** (including no `assert*` / `check*` helpers, no `match` + `panic!`
form, and no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                   | Review Conclusion                                        |
| ----------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | Five `.expect(...)` calls, guaranteeing only that it does not crash                           | Truly no assertions (proves only that it does not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                | Truly no assertions                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them            | Truly no assertions                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                           | Truly no assertions                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                     | Truly no assertions                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                            | Truly no assertions                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                      | Truly no assertions (proves only that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                              | Truly no assertions (proves only that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                        | Truly no assertions (proves only that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs the lsp / repl and discards them (inside the wasm32 cfg)                           | Truly no assertions                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Truly no assertions                                      |

Of these, 5 (`test_normalizer_context` and the 4 tests in reducer.rs) were retained in the 0.6-11a
determination table because **no alternative coverage could be identified**; the conclusions of this
review are consistent with that table.

### Reverse Spot Check: Did the Relaxation Go Too Far?

From the 88 entries “newly recognized as assertions,” 5 were sampled with a fixed seed (20261107)
and their source was read to confirm that they really contain assertions:

| Spot-Checked Entry                                          | File                                                 | Assertion in the Source                                                                                                                        |
| ----------------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching arm; `panic!("Expected AssocType")` if it does not match                                                |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm; `panic!("expected InvalidPackage, got …")` if it does not match                    |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | `assert_eq!` compares the constraint type in the matching arm; `panic!("不是 Dynamic 变体…")` if it does not match                             |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm; `panic!("Expected TokensDelta variant")` if it does not match                          |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm validate the error code and message; `panic!("不是 Disproved…")` if it does not match |

All 5 assert concrete values or concrete variants (replacing the implementation with another correct
path would make them fail)—**the relaxation did not go too far**. Reverse controls were also added
to `--selftest`: a `match` without `panic!`, and `panic!` appearing only in a comment, must **not**
be recognized as assertions.

### The Criterion Has Teeth (Developmental Proof)

After reverting the generator to the old implementation (`HELPER_RE` without digits + `match` +
`panic!` recognition disabled), all newly added `--selftest` cases **fail**, with exit code 1; the
new implementation passes all of them.

### Rollback

The generator and the census table are in the same commit; `git revert <sha>` restores the entire
state.

## Rerun and Self-Check

```bash
python scripts/dev/test-gate-census.py            # Recalculate and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
python scripts/dev/test-gate-census-audit.py --sample  # Reproduce the 40-test sample (seed 20261106)
python scripts/dev/test-gate-census-audit.py --check   # Whether the audit data file covers all samples
```

The generator performs lexical determination only; its boundaries are stated honestly:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as “no assertions”—the first batch of 26 tests (21 retired / 5 retained) and the
  second batch of 18 tests (11 retired / 7 retained) were all individually reviewed by reading the
  function body.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator obtains a function body, it
  balances braces; braces in string/character literals and comments throw off the balance and
  truncate the function body—`test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was
  once truncated to the end of the file, causing the “Assertion Summary” to capture the rest of the
  file and the test to be classified as “Pending Determination” rather than “Retirement Candidate.”
  It now masks non-code regions (string/character literals, line comments, block comments, and raw
  strings) before balancing, as described in “Generator Correction” below.
- **Recognition of “delegated assertions” (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not allow digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions—tests containing such calls were
  misclassified as “not a single assertion in the body.” At the time, 4 of the repository-wide 15
  “Neither passes” tests actually contained assertions (3 numeric helpers + 1 `match` + `panic!`
  form). The name portion has now been relaxed to `[A-Za-z0-9_]*`, and recognition of the `match` +
  `panic!` form has been added, as described in “Generator Criterion Correction” below.
- **The proof gate is not decided mechanically**: every test that “fails both gates” is classified
  only as “Pending Determination” or “Retirement Candidate”; the generator does not declare that it
  “should be deleted.”
- **No cross-file deduplication**: completely duplicate tests are not identified in this table
  (0.6-2 measured the rate at only 1%, which does not constitute a retirement surface).
- **The audit covers only 40 tests**: audit conclusions are used only to characterize those 40 tests
  and to provide a **proportion** for the pool of 2403 tests; an individual test’s determination
  must not be extrapolated to other tests in the same directory.
