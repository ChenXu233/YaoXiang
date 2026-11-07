---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Great Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a clause-gate sampling audit and an initial retirement candidate list'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Great Cleanup)

Prerequisite material for WBS 0.6 “Great Cleanup”: mechanically classify every repository-wide
`#[test]` according to the **three gates (clause / history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a rerunnable item-by-item decision
table; then perform a stratified sampling audit of the clause gate (40 items), measure the
proportion of tests that “cite a clause but cannot support their assertions,” and recompute the
distribution using the **tightened criteria**. **This page and the generator itself are read-only
and do not modify tests**; test retirement is performed in batches (one commit per batch, fully
revertible with `git revert`), and execution results are recorded in the batch sections below and in
`09a`.

Artifacts:

| Artifact                                  | Purpose                                                                                                                             |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv`       | Item-by-item decision table (2055 rows plus the header), in the same directory as this page                                         |
| `scripts/dev/test-gate-census.py`         | Generator (rerunnable, supports `--check` and `--selftest`)                                                                         |
| `scripts/dev/test-gate-census-audit.py`   | Sampling audit: reproduces the 40-item sample with a fixed seed (`--sample` / `--check`)                                            |
| `scripts/dev/test-gate-census-audit.json` | Per-item decisions and rationale for all 40 items (the generator reads it and classifies “audit-confirmed genuine support” as Keep) |

## Criteria

### Scan Surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 1581                 |
| `tests/integration`                                                      | 335                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 75                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2055** (285 files) |

### Machine Criteria for the Three Gates

| Gate                               | Machine Criteria                                                                                                                                                                                                                                                                                                                                                                                                                                       | Meaning of a Match                                        |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------- |
| Clause gate (tightened for 0.6-10) | A **verifiable entry** appears within the first 25 lines of the file header: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); files under `src/**/tests/` are examined together with their parent `mod.rs`                                                                                                                                                                                                   | The test maps to an identifiable clause                   |
| History gate                       | The test function body, function doc comment, or **file header** (such as `//! Regression context: issue #226`) identifies a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                 | The test pins down an incident that has actually occurred |
| Proof gate                         | **The machine cannot decide this** (“If it were replaced with another correct implementation, would it fail?” requires human judgment). Therefore, anything that fails the first two gates is classified as Undetermined and is never automatically classified as “fails both”; the sole exception is a test with **no assertions at all** (including no delegated `assert*` / `check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only Undetermined or No assertions can be assigned        |

Rationale for tightening: the 0.6-9 clause gate only required the file header to contain `§` / `RFC`
/ `SPEC`, resulting in 2403 tests being classified as Keep. The 40-item stratified sample measured
in 0.6-10 found that **half of the citations could not support the assertions** (see the next
section). After tightening, only section-number forms are recognized, and **having a citation still
does not mean that the citation supports the assertion**—every unaudited clause citation is
classified only as Needs Review.

Rationale for the file-header radius: this repository’s convention is to put “regression context:
issue #N” in the file header rather than repeat it in every test comment; looking only at a test’s
own comments would miss the history of the entire file (measured: 7 tests within the test’s own
radius, 291 within the file-header radius).

### Categories That Must Never Be Retired

The following tests are **always classified as Keep** and never enter the retirement candidate pool:

- Corpus tests under `tests/yaoxiang/**` and project-shape tests under `tests/project-corpus/**`
  (behavioral-criteria surface);
- Tests under `tests/integration/**`, plus `tests/formatter_integration.rs` and
  `tests/yx_runner*.rs` (spec-shaped and crossing a process boundary);
- Status anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` / `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                                               |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Only three possible sources: a history-gate match, proof-gate status (spec-shaped directories / status-anchor names), or a **clause citation confirmed by the sampling audit to provide genuine support**             |
| Needs Review         | Cites a clause (with a section number, or only an entire RFC without a section number) but has not been audited, or the audit found the citation irrelevant or generic—human review is required before it can be kept |
| Retirement candidate | Fails both gates and its assertion surface contains only weak assertions (or no assertions)—a candidate is not a verdict; the proof gate still requires manual review                                                 |
| Undetermined         | Fails both gates but contains strong assertions; the machine reaches no conclusion                                                                                                                                    |

## Distribution After Tightening

| Gate                                                             | Count    |
| ---------------------------------------------------------------- | -------- |
| Clause (has a verifiable entry)                                  | 811      |
| History                                                          | 313      |
| Proof (spec-shaped criteria surface / status anchors)            | 424      |
| Undetermined (fails both gates; proof gate awaits manual review) | 496      |
| Fails both gates (no assertions in the body)                     | 11       |
| **Total**                                                        | **2055** |

| Disposition          | Count    | Compared with 0.6-9                                                                                                                                                                                                              |
| -------------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | **746**  | 2897 (−2151)                                                                                                                                                                                                                     |
| Needs Review         | 1045     | 0 (new tier)                                                                                                                                                                                                                     |
| Undetermined         | 203      | 412 (−1 after the second retirement batch, +3 from the 0.6-15 criteria correction, −211 from the fifth retirement batch)                                                                                                         |
| Retirement candidate | 61       | 136 (−21 from the first retirement batch, −11 from the second, +1 from the generator correction, −2 from the third, −3 from the 0.6-15 criteria correction, −1 from the redundancy scan, −1 from the fourth, −37 from the fifth) |
| **Total**            | **2055** | —                                                                                                                                                                                                                                |

Two changes to the criteria require explanation:

1. **Structure of Keep**: 746 = 424 proof-gate tests (390 on the spec-shaped criteria surface + 34
   status anchors) + 313 history-gate tests + 9 citations confirmed by the audit to provide genuine
   support. Of the original 2897 Keep tests, the 2121 that were kept “merely for having a citation”
   are now classified as Needs Review.
2. **History gate 47 → 313**: these are not new history cases, but **label reclassification**—in
   0.6-9, tests that matched both the clause and history gates were recorded under the clause gate.
   Once the tightened clause gate no longer grants an automatic Keep, the history gate must catch
   these tests, so its count includes them. The total number kept is therefore unaffected (these
   tests were also classified as Keep in 0.6-9).

File-level criteria: of the 285 test files, **136** have a verifiable section number in the file
header (the old 0.6-9 criterion found 221).

## Clause-Gate Sampling Audit (0.6-10)

### Sampling Criteria (Reproducible)

- **Candidate pool = the 2403 tests classified as Clause by the 0.6-9 clause gate** (frozen using
  the old criterion in `test-gate-census-audit.py`; the sampling denominator must not change with
  the new criterion, or the results would not be comparable).
- **Strata = directory clusters**: take the first two levels under `src/` (`src/frontend`,
  `src/middle`, etc.); group everything else by top-level directory (`tests`, `benches`, `tools`).
- **Allocation**: proportional to the number of tests in each stratum (largest-remainder method),
  with at least one test per stratum.
- **Selection**: within each stratum, sort by (file, line number, test name), then sample randomly
  with a fixed seed.
- **Seed = 20261106** (hard-coded in the script; changing the seed changes the sample, requiring the
  documentation to be updated and the audit rerun).

Stratified result: `src/frontend` 21 · `tests` 4 · `src/driver` 3 · `src/package` 3 · `src/backends`
2 · `src/formatter` 2 · `src/middle` 2 · `src/util` 2 · `tools` 2 · `src/ast` 1 · `src/lsp` 1 ·
`src/std` 1.

### Item-by-Item Decisions for the 40 Tests

Decision criteria (fixed here to prevent inconsistent judgments):

- **Genuine support**: the citation identifies a specific section, note, or decision number, and its
  **topic** is the same as the assertion target;
- **Irrelevant**: the citation identifies a specific entry, but that entry concerns **something
  else** (the rationale states what it actually covers);
- **Generic**: the citation refers only to an entire document, an entire chapter, a list of
  chapters, or a stage name, with no entry that matches the assertion.

| #   | Test                                                                        | File                       | Decision        | Rationale (What the cited clause actually covers)                                                                                                                                                                                           |
| --- | --------------------------------------------------------------------------- | -------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `test_compile_time_value_param_spec_table`                                  | `classify.rs`              | Genuine support | type-system §7.1 “compile-time value parameters” is exactly the assertion target: the truth table for value-position/type-position parameter determination is checked row by row.                                                           |
| 2   | `test_id_stable_under_subsequent_registrations`                             | `value.rs`                 | Genuine support | RFC-010’s “runtime representation and equality” sum-type identity convention means that identity is determined only by the name; the assertion verifies that the ID does not change after subsequent registration.                          |
| 3   | `test_driver_run_check_rootless_multi_file_well_formed_generic_stays_clean` | `check_rootless_mono.rs`   | Genuine support | 02-stage-contract §3 note #14 point ③ discusses same-origin monomorphization naming; this test is the pin test for that note (both the name and assertion match).                                                                           |
| 4   | `test_multifile_benign_import_cycle_message_predicts_no_runtime_failure`    | `import_cycle.rs`          | Genuine support | RFC-013’s W1xxx warning code + user adjudication A (cycles must not be silent); the assertion verifies that a benign cycle produces exactly one W1007 and predicts no runtime consequences.                                                 |
| 5   | `test_obligation_config_disabled_skip_is_not_owed`                          | `obligations.rs`           | Genuine support | 02-stage-contract §2 “obligation ledger”; the assertion verifies that skipping via ConfigDisabled does not create an outstanding obligation.                                                                                                |
| 6   | `test_format_type_bool`                                                     | `types.rs`                 | Genuine support | formatter specification §9 type annotations; the assertion verifies that the type name is emitted unchanged.                                                                                                                                |
| 7   | `test_fstring_multiline_single_quotes_inside`                               | `fstring.rs`               | Genuine support | syntax §1.6.3 string literals; the assertion verifies that a single quote inside multiline f-string content does not terminate it.                                                                                                          |
| 8   | `test_float_underscore`                                                     | `literals.rs`              | Genuine support | syntax §1.6 literals (1.6.2 floating-point numbers) covers underscore separators; the assertion verifies that 1_000.5 is lexed as 1000.5.                                                                                                   |
| 9   | `test_empty_unicode_escape_is_error`                                        | `literals.rs`              | Genuine support | syntax §1.6.3 defines a closed set of escapes; the assertion verifies that an empty \u{} escape must produce an error.                                                                                                                      |
| 10  | `test_probe_accepted_const_type_is_registered`                              | `probe.rs`                 | Genuine support | RFC-039 P6 6.4 / WBS 6.6.9 means “constant-parameter determination obeys only the injected probe”; the assertion verifies that N is registered when the probe accepts Int.                                                                  |
| 11  | `test_direct_eval_with_bound_variable_proved`                               | `predicate.rs`             | Genuine support | RFC-027 §3-4 (path-condition propagation / compile-time proof pipeline); the assertion verifies that b>0 is directly evaluated as Proved when b=5.                                                                                          |
| 12  | `test_unused_type_is_not_an_entry_point`                                    | `dead_code.rs`             | Genuine support | RFC-011 §7 dead-code elimination mechanism; the assertion verifies that an unused type definition is not a reachable root.                                                                                                                  |
| 13  | `test_display_forall_multiple_vars_no_trailing_space`                       | `ast.rs`                   | Genuine support | RFC-027 §8 SMT solver; the assertion verifies the printing format for forall with multiple variables.                                                                                                                                       |
| 14  | `test_dep_graph_remove_dependant_clears_edges`                              | `rfc027_phase31_dep_vc.rs` | Genuine support | RFC-027 Phase 3.1’s TypeDepGraph; the assertion verifies that removing a dependent clears its edges and constraints together.                                                                                                               |
| 15  | `test_condition_bool`                                                       | `conditional.rs`           | Genuine support | RFC-011 §5 conditional types; the assertion verifies the evaluation and determinate state of TypeCondition::Bool.                                                                                                                           |
| 16  | `test_convert_expr_to_const_expr_int_literal`                               | `const_eval.rs`            | Genuine support | RFC-011 §4 compile-time generics; the assertion verifies conversion of an Int literal into ConstExpr::Lit.                                                                                                                                  |
| 17  | `test_definition_on_binding_site_returns_none_precise_only`                 | `definition.rs`            | Genuine support | RFC-039 D55 (precise-only; name-based fallback is forbidden); the assertion verifies that the definition site returns None when it has no reference record of its own.                                                                      |
| 18  | `test_type_consistent_arith_passes`                                         | `verify.rs`                | Genuine support | WBS 2.1 / RFC-039 P2 IR static-validator criteria (the first layer of 07-equivalence-oracle); the assertion verifies that type-consistent arithmetic passes.                                                                                |
| 19  | `test_verify_loose_corpus_green`                                            | `verify.rs`                | Genuine support | The same validator criterion; the assertion verifies that the corpus passes completely.                                                                                                                                                     |
| 20  | `test_parse_tag_tolerates_query_and_fragment`                               | `toolchain.rs`             | Genuine support | RFC-037 § installer support; the assertion verifies that the query/fragment are stripped from the release URL.                                                                                                                              |
| 21  | `test_release_unset_register`                                               | `execute.rs`               | Irrelevant      | #299 §2 covers the NewArray fixed-length array instruction (allocation, Void placeholder, bytecode round trip); the assertion instead verifies that executing Release on an unset register does not panic.                                  |
| 22  | `test_fstring_segment_interpolation`                                        | `ast.rs`                   | Irrelevant      | It cites “Language Specification §3 (AST Structure),” but syntax §3 is the statements chapter (3.1 Statement Categories … 3.11 Program Entry and Top-Level Statements); the assertion concerns the fields of FStringSegment::Interpolation. |
| 23  | `test_expr_for`                                                             | `ast.rs`                   | Irrelevant      | It cites the same “§3 (AST Structure)” reference; syntax §3 covers statements, while the assertion concerns the AST fields of Expr::For.                                                                                                    |
| 24  | `test_add_var_without_mut_stores_immutability`                              | `scope.rs`                 | Irrelevant      | It cites “Language Specification §5.2”: syntax has no chapter 5, while type-system §5.2 is “multiple constraints”; the assertion concerns the immutable flag stored by ScopeManager.                                                        |
| 25  | `test_type_evaluator_eval_nested_type`                                      | `evaluator.rs`             | Irrelevant      | It cites syntax §3.11 (Program Entry and Top-Level Statements) and “RFC-027 §362” (which does not exist); the assertion concerns evaluation of a nested type.                                                                               |
| 26  | `test_get_ast_type_universe_level_plain`                                    | `mono.rs`                  | Irrelevant      | It cites “Language Specification §3,” but syntax §3 is the statements chapter; the assertion concerns a type-universe level of 0.                                                                                                           |
| 27  | `test_unify_enum_same`                                                      | `solver.rs`                | Irrelevant      | It cites “Language Specification §3”; successful unification of same-named enums belongs to type unification, not the statements chapter.                                                                                                   |
| 28  | `test_weak_new_upgrade_some_while_original_alive`                           | `weak.rs`                  | Irrelevant      | It cites RFC-010’s **sum-type identity**, but the assertion concerns the lifecycle of a weak reference (#384: upgrade returns Some while the original Arc is alive).                                                                        |
| 29  | `test_i18n_registry_english_titles`                                         | `codes.rs`                 | Irrelevant      | §3.4’s I3/I4 cover the translations of E1090/E1091; the assertion instead concerns a nonempty English title and help text for E0001.                                                                                                        |
| 30  | `test_format_return_expression`                                             | `expr.rs`                  | Generic         | The citations are a list of §3, §4, §7, §8, §10–§13, §17, and §18, with no entry matching “how a return expression is rendered.”                                                                                                            |
| 31  | `test_rfc010_generic_syntax`                                                | `lexer_mod.rs`             | Generic         | The citations are syntax §1.6 plus broad lists of RFC-004/RFC-010; the assertion describes itself as flexible (allowing arbitrary parenthesization) and has no verifiable entry.                                                            |
| 32  | `test_anon_binding_with_fn_annotation_keeps_extracted_signature`            | `types.rs`                 | Generic         | “spec §3, RFC-010, RFC-011” is a broad list; none corresponds to “extracting the annotated signature of an anonymous function binding.”                                                                                                     |
| 33  | `test_consumer_unknown_token_returns_empty`                                 | `ownership.rs`             | Generic         | “RFC-009 + RFC-009a” lists entire documents; the assertion verifies that BrandTree returns an empty consumer set for an unknown token.                                                                                                      |
| 34  | `test_const_expr_neg_int_literal_displays_by_value`                         | `const_data.rs`            | Generic         | RFC-011 §4 covers compile-time generics and does not concern the printing form of ConstExpr; the assertion verifies that a negative literal is printed as -5.                                                                               |
| 35  | `resolve_local_and_method`                                                  | `resolver.rs`              | Generic         | “RFC-029 Refactoring Core” is an entire architecture RFC; the assertion concerns a resolver’s qualified name and DefKind.                                                                                                                   |
| 36  | `test_outdated_exec_smoke`                                                  | `outdated.rs`              | Generic         | RFC-014a is only background in a comment (the registry was unpublished, so a path dependency is used); the assertion is an exec smoke test with no assertions in the body.                                                                  |
| 37  | `test_compatible_high_major_range`                                          | `resolver.rs`              | Generic         | “RFC-014 Phase 3” is a stage name, not an entry; the assertion verifies version-range compatibility.                                                                                                                                        |
| 38  | `test_clean_and_size`                                                       | `cache.rs`                 | Generic         | “RFC-014 Phase 3 (global cache design)” is likewise a stage name; the assertion verifies cache clean and dir_size.                                                                                                                          |
| 39  | `test_strip_ansi_multiple_consecutive_sequences`                            | `ansi.rs`                  | Generic         | §4.2 is the “ANSI utility extraction” work item, not a specification clause; the assertion concerns consecutive-sequence behavior in strip_ansi.                                                                                            |
| 40  | `test_verify_sha256_accepts_matching_digest_file_content`                   | `dl.rs`                    | Generic         | It cites “RFC-037: version directory = release-package extraction root directory”; the assertion verifies that sha256 validation accepts matching digest-file content.                                                                      |

Distribution: **20 genuine support · 9 irrelevant · 11 generic**.

### Mis-citation Rate and Confidence Interval

- Mis-citations (irrelevant + generic) = 20/40 = **50%**;
- Wilson 95% confidence interval (n=40): **[35.2%, 64.8%]**;
- Extrapolated to 2403: approximately **1202 tests** genuinely have clause support (95% interval
  **846–1557**), with approximately **1201 mis-citations** (interval 846–1557).

### Impact on the Cleanup Surface

- **Keep falls from 2897 to 775** (−2122)—exactly the 2122 tests that were kept “for having a
  citation” now fall into Needs Review; this is the effect of tightening the criteria, not
  distribution tuning.
- **Retirement candidates are unrelated to clauses**: the initial 26 candidates were already
  unrelated to clauses (they contained no assertions at all), and the audit does not change their
  classifications; 21 tests from the first batch were retired, reducing candidates from 136 to 115;
  11 tests from the second batch were retired, reducing candidates from 115 to 104; after the 0.6-13
  generator fix (function-body boundaries are no longer truncated by braces in literals), the count
  increases by 1, from 104 to 105; two tests from the third batch were retired, reducing candidates
  from 105 to 103; after the 0.6-15 criteria fix (helper names containing digits / the `match` +
  `panic!` form are recognized as assertions), three tests move out of “fails both gates,” reducing
  candidates from 103 to 100. The second batch is analogous: these tests **do** contain assertions,
  but only weak assertions such as `is_some()` / `is_none()`, which do not pin down binding power or
  the parse result. This also demonstrates that the “retirement surface” and the “mis-citation
  surface” are separate problems: most mis-cited tests contain assertions, and the appropriate
  action is to add a clause or correct the citation, not to delete the test.
- The 2122 Needs Review tests represent the workload for the subsequent “clause-gate ticket
  completion”: add a specific entry that matches each assertion, or reclassify the test as a
  retirement candidate.

### Limitations (Do Not Extrapolate Beyond the Sample)

- The 40 tests can establish only a **proportion**; no individual decision may be extrapolated to
  other tests in the same directory;
- Decisions were derived by manually reading code and clause documents; the rationale column states
  “what the cited clause actually covers” for review;
- The sampling denominator is the old 0.6-9 criterion pool (2403). The mis-citation rate within the
  **1714 tests remaining after tightening** was not resampled—tightening only excludes
  “unverifiable” citations and does not mean that every citation in the new pool genuinely supports
  its assertion (those still require item-by-item review).

## Reconciliation with the 0.6-2 Survey

The 0.6-2 survey numbers are the first-version comparison baseline. The reasons for each difference
are listed below:

| Item                            | 0.6-2 Survey                  | This Census                                                                                            | Reason for the Difference                                                                                                                                                                                                                                                                       |
| ------------------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                   | Two tests were added after 0.6-2 (E3005 and E3022 check-criteria regression tests), followed by deletion of 21 no-assertion tests from the first batch, 11 weak-assertion tests from the second batch, 2 from the third batch, 1 from the fourth batch, and 1353 from the fifth batch; net −388 |
| Number of test files            | 296                           | 285                                                                                                    | Eleven entire files were retired in the fifth batch                                                                                                                                                                                                                                             |
| Clause gate                     | 227 files (77%)               | 136 files after tightening; 811 by item                                                                | The criterion was tightened from “the token appears” to “a section number `§<number>` is present,” nearly halving the file count; the fifth batch retired 902 tests                                                                                                                             |
| History signals                 | 442 tests / 148 files         | 313 in the gate; three radii: test itself 7 / file header 291 (38 files) / entire file 1218 (79 files) | The 148 files in 0.6-2 are broader than the 79 files found here at the entire-file radius because its vocabulary/range was larger; this census strictly uses the four card-surface signals                                                                                                      |
| Strong characterization signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                           | Characterization is not one of the three gates; mixing it into a retirement conclusion would exceed the mandate                                                                                                                                                                                 |
| Weak assertions                 | 473 (14%)                     | See the TSV “Assertion Summary” column                                                                 | The counting units differ: 0.6-2 counted patterns that only asserted is_ok/is_some, whereas this census uses lexical counting and distinguishes strong / weak / delegated assertions                                                                                                            |

## Conclusion Summary

1. **The clause gate was previously significantly overstated**: under the old criterion, 2403 tests
   were classified as Clause/Keep, while the sampled audit found a 50% mis-citation rate (95%
   interval 35%–65%)—“having a citation” and “the citation supporting the assertion” are different
   things, and the old criteria conflated them.
2. **After tightening, approximately 1202 tests genuinely have clause support** (interval 846–1557);
   the remainder must either receive a reviewed clause or be classified through the history
   gate/proof gate.
3. **The retirement surface is unrelated to clause mis-citations**: the 136 candidates have no
   assertions or only weak assertions; 21 tests from the first batch and 11 from the second have
   been retired, the generator correction added 1, the third batch removed 2, the 0.6-15 criteria
   correction removed 3, the redundancy scan removed 1, the fourth batch removed 1, and the fifth
   batch removed 1353 tests under the mechanical rule “retain only the strongest test for the same
   symbol and result category,” reducing the candidates to 61. Fifth-batch retirement distribution
   by gate: Clause 902 · Undetermined 428 · Proof 23 · History 0 · Fails both gates 0—**not a single
   History-gate test or zero-assertion test was retired**.
4. **The history gate is the largest legitimate source of Keeps** (313 tests): it demonstrates that
   “pinning down a real incident” is more reliable than “copying a sentence from an RFC.”

## First Retirement Batch (Executed, 2026-11-06)

**First batch: of 26 “no assertion” tests under `src/frontend/core/types/eval/tests/`, 21 were
retired and 5 were retained** (single directory cluster; criterion = all **three gates fail**
according to Part Seven of coding-rules: no clause gate, no history gate, and no assertions in the
body). The item-by-item decisions and “same-slot coverage” are provided by the 0.6-11a decision
table (21 deletions / 5 retentions); this page does not repeat the rationale and records only the
execution result.

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
- **5 retained** (the decision table could not identify replacement coverage, so the hard
  requirement is to retain them): `test_normalizer_context`,
  `test_type_computer_new_and_with_config`, `test_type_computer_compute_type_ref`,
  `test_reducer_reduce_type_ref_with_alias_chain`, `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), and `git revert <sha>` rolls it back as a whole.

## Second Retirement Batch (Executed, 2026-11-06)

**Second batch: of 18 “weak assertion” tests in `src/frontend/core/parser/pratt/tests/nud.rs`, 11
were retired and 7 were retained** (single directory cluster = the Pratt prefix-parsing module;
criterion = all **three gates fail** according to Part Seven of coding-rules: no clause gate, no
history gate, and the body contains only weak assertions such as `is_some()` / `is_none()`, which do
not pin down binding power or the parse result).

The difference from the first batch must be made explicit: these tests **do contain assertions** and
are not empty tests. The reason for retirement is “all three gates fail + zero criterion increment,”
not “no assertions”—each test selected for deletion identifies the name of a **unit-level
replacement test in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The replacement
  coverage for every test follows the **same** `parse_expression → parse_prefix → prefix_info` path
  and makes stronger assertions (pinning down the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, plus `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 retained**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the decision table in “VI. Second Test Retirement Batch” in
  `09a-wbs-history.md` for the item-by-item rationale. The retention rationales fall into two
  categories: **the proof gate applies** (the first three, along with `test_prefix_info_at` and
  `test_prefix_info_eof`, are the sole unit-level observation points for their respective facts, and
  the repository contains no second unit test that observes them), or **the history gate applies**
  (`break_removed` / `continue_removed` identify #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), and `git revert <sha>` rolls it back as a whole.

## Generator Correction (0.6-13)

**`body_span` truncated function bodies at braces inside literals — fixed.**

The old implementation balanced braces without distinguishing real code from braces in
string/character literals or comments. Such expressions as `tokenize("{")` are common in Rust
source, so the balance was thrown off and the function body was truncated: the body of
`test_prefix_info_lbrace` was previously truncated to the end of the file, causing the “Assertion
Summary” to include the remainder of the file (`matchesx11 assertx24 …`) and the test to be placed
in “Undetermined” rather than “retirement candidate”—a **misclassification**.

Fix: add `mask_noncode` to replace the contents of string literals / character literals / line
comments / block comments (which may be nested) / raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (preserving newlines exactly, with line numbers and line counts unchanged),
then balance the braces. **The mask is used only to locate function-body boundaries**; assertion
counting and the history gate still read the original source, and the criteria themselves have not
been relaxed. `--selftest` now includes four cases (paired/unpaired braces in ordinary strings,
braces in raw strings and block comments, and braces in character literals), each followed by a
sentinel test—the old implementation found the wrong boundary in all four cases, while the new
implementation finds all of them correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** for every test in
every file (old implementation balanced by literal contents vs. new implementation balancing after
masking); tests with different boundaries are affected. Of the 3413 tests in the entire repository,
the old implementation found the wrong boundary for **11**:

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

### Binning Differences Before and After the Fix: 2

For the other 9, **8 only affected the “Assertion Summary” column** (the bin did not change), and
**1 did not change even the summary** (`test_fstring_close_brace_escape_preserved_raw`: the old
implementation omitted the closing `}` line, which contained no assertion token). The eight summary
changes are listed individually in the corresponding section of `09a-wbs-history.md`.

| Test Name                   | File                                          | Old Bin                     | New Bin                                 | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| --------------------------- | --------------------------------------------- | --------------------------- | --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Undetermined / Undetermined | Undetermined / **Retirement candidate** | The real function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; the file header and function body contain no § section number, issue/regression/defect/incident term, etc. ⇒ neither the clause gate nor the history gate applies, and the body contains only weak assertions such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires manual review)                                                    |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | History / Keep              | Clause / **Needs Review**               | The old implementation truncated the function body to the end of the file, swallowing text containing “regression / defect / incident” and incorrectly classifying it under the history gate; the real function body contains no such terms ⇒ the history gate no longer applies; the file header contains a verifiable entry (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate matches, but because it was not confirmed by the sampling audit, it is Needs Review |

The distribution changes accordingly: Clause 1713 → 1714 · History 314 → 313 · Keep 776 → 775 ·
Needs Review 2121 → 2122 · Undetermined 412 → 411 · Retirement candidate 104 → 105. **The
repository-wide `#[test]` count remains unchanged (3413)**—this round changes only the
implementation of the criteria, not any test.

## Third Retirement Batch (Executed, 2026-11-07)

**Third batch: of 5 retirement candidates under `src/middle/passes/codegen/tests/`, 2 were retired
and 3 were retained.** (single directory cluster = the `middle::passes::codegen` module under test;
criterion = all **three gates fail** according to Part Seven of coding-rules.)

This batch continues the **retention criteria** established by the previous batch: a unit-level
observation point may only be replaced by another **unit test**; corpus/integration tests operate at
the process-boundary layer and cannot replace a unit observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is **verbatim
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (apart from
    comments and the function name): both are `run_fixture()` + `assert_slot_is_42(&interp)`. Its
    doc comment claims to verify that the `execute_module` entry path returns non-Void, but the body
    never observes the return value of `execute_module` (`run_fixture` discards it with
    `.expect(...)`)— the claimed intent was not implemented, and the function body is a pure
    duplicate ⇒ zero criterion increment. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file provides strictly
    stronger coverage: positive lookup (`lookup("a").is_some()`), negative lookup
    (`lookup("b").is_none()`), `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` is identical to the inline `Symbol` in this test).
    “A name that was never inserted cannot be found” and “it cannot be found after being popped”
    follow the same path in `SymbolScopeManager::lookup` (walk each `scope.get(name)` until `None`
    is returned) ⇒ this test adds zero increment. Replacement coverage = `test_scope_nesting`.
- **3 retained**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that does
    **not** go directly from serialization to execution (the serialization variant is a different
    path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit-level observation point for the
    rejection branch where `OperandResolver::to_reg` receives `id > 255` (the bytecode layer has
    independent overflow checks that do not cover this function; no other test in the repository
    causes `to_reg` to go out of range). `is_err()` is a discriminating assertion (removing the
    guard would make the test fail), not merely a “prove that it does not panic” check.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), and `git revert <sha>` rolls it back as a whole.

### Census Blind Spot Found by This Batch (**Fixed in 0.6-15**, See “Generator Criteria Correction” Below)

The census’s “delegated assertion” recognition pattern `\b(assert|check)[A-Za-z_]*\s*\(` **cannot
match helper names containing digits**: `[A-Za-z_]*` does not include digits, so a call such as
`assert_slot_is_42(&interp)` is **not recognized as an assertion**— two of the five candidates in
this batch were consequently misclassified as “no assertions in the body.”

After reviewing all 15 repository-wide “fails both gates” tests item by item (by allowing digits in
helper names and additionally recognizing the `match` + `panic!` form), **4 did in fact contain
assertions**—3 used the numeric helpers above (`assert_slot_is_42`), and 1 was `test_weak_from_arc`
in `src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule
6.1 requires `assert!(matches!(...))`, but it is nevertheless an assertion). The other 11 genuinely
contained no assertions at all.

**Taking these 4 “fails both gates” results directly from the machine would have deleted tests that
contain criteria**—the item-by-item human review in this batch prevented that; the generator-side
fix is described in “Generator Criteria Correction (0.6-15)” below.

## Generator Criteria Correction (0.6-15)

**Two criteria blind spots have been fixed: helper names containing digits, and the `match` +
`panic!` form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411);
it changes only how the generator detects whether a body contains an assertion.

### What Was Fixed

1. **Digits are now allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing them were
   treated as “no assertions in the body.”
2. **Added recognition of the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the
   assertion is written in an arm, so `assert!` is not visible lexically (test-specification rule
   6.1 requires `assert!(matches!(...))`, but existing code contains this form). Recognition method:
   scan the expression after the `match` keyword up to the first `{`, open a block, determine the
   block boundary by **balancing braces**, and count it only if `panic!(` appears inside the block.
   **The decision is made on masked lines**— braces in strings and comments do not participate in
   balancing, and `panic!` in comments is not recognized as an assertion.

### Binning Differences: 3 (Gate and Disposition Changed)

| Test Name                                           | File                                           | Old Bin                                 | New Bin                     | Rationale                                                                                                                                                                                                                                                    |
| --------------------------------------------------- | ---------------------------------------------- | --------------------------------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Fails both gates / Retirement candidate | Undetermined / Undetermined | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Fails both gates / Retirement candidate | Undetermined / Undetermined | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, so the old pattern could not match it; it is now recognized as an assertion                                                                                     |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Fails both gates / Retirement candidate | Undetermined / Undetermined | The same applies (the serialization-round-trip test)                                                                                                                                                                                                         |

Another **77 tests changed only the “Assertion Summary” column** (the gate and disposition did not
change): most summaries gained `match_panicxN` (the body contains a `match` + `panic!` form), and 4
gained `helperxN` (3 in `src/driver/tests/manifest_error.rs` and 1 in
`src/package/tests/cache.rs`—helper names containing digits).

**The relaxation changes only the “fails both gates” tier**: that tier is the machine’s **confident
verdict** (no assertions in the body), and its criterion is a hard requirement; “retirement
candidate” is only a pool awaiting manual review, so its size is not a verdict. Therefore, all other
bins, including the weak-assertion pool, continue to use the original criteria. Result: Gate Fails
both gates 14 → 11 · Undetermined 923 → 926; Disposition Retirement candidate 103 → 100 ·
Undetermined 411 → 414.

### Current 11 “Fails Both Gates” Tests: Item-by-Item Review

After all 11 repository-wide “fails both gates” tests were read and reviewed in their original form,
**all 11 genuinely contain no assertions** (including no `assert*` / `check*` helpers, no `match` +
`panic!` form, and no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                     | Review Conclusion                                           |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | Five `.expect(...)` calls that only ensure it does not crash                                    | Genuinely no assertion (proves only that it does not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                  | Genuinely no assertion                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them              | Genuinely no assertion                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Genuinely no assertion                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Genuinely no assertion                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                              | Genuinely no assertion                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                        | Genuinely no assertion (proves only that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Genuinely no assertion (proves only that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Genuinely no assertion (proves only that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                            | Genuinely no assertion                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertion                                      |

Five of these (`test_normalizer_context` and the four in reducer.rs) were retained because the
0.6-11a decision table **could not identify replacement coverage**; the conclusion of this review
agrees with that decision.

### Reverse Sampling: Did the Relaxation Go Too Far?

Five tests were selected at random with a fixed seed (20261107) from the 88 items newly recognized
as assertions and read in their original form to confirm that they genuinely assert:

| Sampled Item                                                | File                                                 | Assertion in the Original Source                                                                                                                       |
| ----------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching arm of a `match`, with `panic!("Expected AssocType")` in the non-matching arm                                   |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm, with `panic!("expected InvalidPackage, got …")` in the non-matching arm                    |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | An `assert_eq!` comparing constraint types in the matching arm, with `panic!("不是 Dynamic 变体…")` in the non-matching arm                            |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm, with `panic!("Expected TokensDelta variant")` in the non-matching arm                          |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm validate the error code and message, with `panic!("不是 Disproved…")` in the non-matching arm |

All five assert a concrete value or concrete variant (replacing the implementation with another
correct path would make them fail)—**the relaxation did not go too far**. The reverse controls were
also added to `--selftest`: a `match` without `panic!`, and a `panic!` appearing only in a comment;
neither may be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

After reverting the generator to the old implementation (`HELPER_RE` without digits + recognition of
the `match` + `panic!` form disabled), all newly added `--selftest` cases **failed**, with exit code
1; the new implementation passes all of them.

### Rollback

The generator and census table are in the same commit, and `git revert <sha>` restores both as a
whole.

## Fourth Retirement Batch (Executed, 2026-11-07)

**Fourth batch: of 10 “retirement candidates” in the directory cluster `src/backends/interpreter/`
(the interpreter-backend test directories), 1 was retired and 9 were retained** (single directory
cluster, with all three files read before making decisions: 4 in `tests/bytecode_load.rs` · 2 in
`tests/weak.rs` · 4 in `executor/tests/debug.rs`). Criterion = the **three gates** according to Part
Seven of coding-rules; retention criteria = “a unit-level observation point may only be replaced by
another unit test; corpus/integration tests operate at the process-boundary layer”; **deletion
requires identifying a specific replacement-coverage test by name; if it cannot be identified or
cannot be understood, retain the test**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, followed by one step
  and then `capture_stack`), and makes **strictly stronger** assertions: it verifies that the stack
  is nonempty and that `stack[0].function_name == "main"` (pinning down the top-of-stack position),
  whereas the listed test only verifies `stack.iter().any(|f| f.function_name == "main")` (any
  position is sufficient) —the former being true necessarily implies the latter ⇒ zero criterion
  increment.
- **9 retained**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each one, they are each
  the **sole unit-level observation point** for a particular property (the distinct
  magic-number/version branches of loading, the YXBC routing layer, both Some/None sides of Weak,
  stepping with no frames, popping the frame on step_out, and running until the stack is empty), and
  no unit test capable of replacing them could be identified ⇒ retained as required. See Section 11
  of `docs/src/dev/architecture/09a-wbs-history.md` for the item-by-item rationale (the decision
  table is recorded there and is not duplicated here).
- **Numbers**: repository-wide `#[test]` 3409 → 3408; 1 deletion = `git diff` counts `-#[test]` 1
  line, `+#[test]` 0 lines; the dominant test `test_capture_stack_during_execution` passes when run
  individually. Census: retirement candidate 99 → 98 · Undetermined 925 → 924 · Clause 1713 ·
  History 313 · Proof 447 · Fails both gates 11 · Needs Review 2121 · Keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), and `git revert <sha>` rolls it back as a whole.

## Fifth Retirement Batch (Executed, 2026-11-07)

**Fifth batch: 1353 unit tests were retired in bulk under the mechanical rule “for the same symbol
and result category, retain only the strongest test”** (only unit tests under `src/**/tests/**` were
changed; not a single test in the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (mechanically executed, without item-by-item justification)**: for the same symbol under
  test + the same assertion-result category, retain only the test with the strongest assertion and
  delete the others.
  - Symbol under test = a repository-internal symbol called in the test body (including one level of
    propagation through helper functions in the same file); when multiple symbols are reached,
    classify by the **primary symbol** (the first repository-internal call). Symbols are qualified
    by repository path and are not merged across modules by bare name.
  - Result category = concrete value / a particular error code (E code) / a particular variant match
    / ok only / err only / should_panic only.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when tests are equally strong, retain the
    first one to appear in the file.
  - When one test asserts multiple categories (making the primary category indeterminable), **retain
    it** rather than guess.
- **Three exceptions (not one was missed)**: ① names or comments identify `issue #` / regression /
  defect / incident (history gate); ② a symbol has only one test (deleting it would leave the symbol
  without a unit-level observation point); ③ `#[should_panic]` is the symbol’s only should_panic
  case. **Zero of the 313 History-gate tests and 11 “fails both gates” tests were retired.**
- **Numbers**: repository-wide `#[test]` 3408 → 2055; 1353 deletions = `git diff` counts `-#[test]`
  1353 lines, `+#[test]` 0 lines. Census (by gate): Clause 1713 → 811 · History 313 → 313 · Proof
  447 → 424 · Undetermined 924 → 496 · Fails both gates 11 → 11. Census (by disposition): Keep 775 →
  746 · Needs Review 2121 → 1045 · Undetermined 414 → 203 · Retirement candidate 98 → 61. Retirement
  count by gate: Clause 902 · Undetermined 428 · Proof 23.
- **Cleanup accompanying the deletions**: helper functions used only by deleted tests, imports that
  became invalid, and struct fields that became unused were removed as well
  (`cargo clippy --all --all-features --all-targets -- -D warnings` reported 0 warnings).
- **Two out-of-scope deletions requiring a record**: 29 tests classified as Keep by the census were
  nevertheless deleted by this batch’s rule—23 status anchors (names containing
  unchanged/anchor/guard/preserve/baseline, many of which were false positives caused by **domain
  terms** such as “guard” or “preserves,” for example `test_while_guard_scope_balanced`) and 6
  citations confirmed by the audit to provide genuine clause support. This batch followed the closed
  list of three exceptions on the card and did not provide additional protection for these two
  categories; to restore them, use the Keep/Delete correspondence table on this page to add them
  back.
- **Sampling-audit pool (must be redrawn)**: the 40-item sample was drawn with a fixed seed from the
  test pool, and after the fifth retirement batch, 38 original samples no longer exist, so
  `python scripts/dev/test-gate-census-audit.py --check` reports “38 missing, 38 extra”— a new
  sample must be drawn from the new pool and each item judged again (this batch did not make those
  judgments, to avoid classifying assertions that had not been read as “genuine support”).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), and `git revert <sha>` rolls it back as a whole.

## Rerun and Self-Checks

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
python scripts/dev/test-gate-census-audit.py --sample  # Reproduce the 40-item sample (seed 20261106)
python scripts/dev/test-gate-census-audit.py --check   # Whether the audit data file covers the entire sample
```

The generator performs only lexical classification, and its boundaries are stated explicitly:

- **Delegated assertions are recognized only by name** (`assert*` / `check*` helper functions). If a
  test’s assertion is hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as “no assertions”—the initial 26 tests (21 retired / 5 retained) and the subsequent
  18 tests (11 retired / 7 retained) were all manually reviewed at the function-body level.
- **Function-body boundaries (fixed in 0.6-13)**: when obtaining a function body, the generator
  balances braces, so braces in string/character literals and comments can throw the balance off and
  truncate the body—`test_prefix_info_lbrace` (which contains `tokenize("{")`) was previously
  truncated to the end of the file, causing the “Assertion Summary” to include the remainder of the
  file and the test to be placed in “Undetermined” rather than “retirement candidate.” The generator
  now masks non-code regions (strings / character literals / line comments / block comments / raw
  strings) before balancing the braces; see “Generator Correction” below.
- **“Delegated assertion” recognition (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not include digits in `[A-Za-z_]*`, so a call such as
  `assert_slot_is_42(&interp)` was not recognized as an assertion—tests containing such calls would
  be misclassified as “no assertions in the body.” At the time, 4 of the 15 repository-wide “fails
  both gates” tests actually contained assertions (3 numeric helpers + 1 `match` + `panic!` form).
  The name portion has now been relaxed to `[A-Za-z0-9_]*`, and recognition of the `match` +
  `panic!` form has been added; see “Generator Criteria Correction” below.
- **The proof gate is not decided**: every test that “fails both gates” is classified only as
  “Undetermined” or “retirement candidate”; the generator never declares that it “should be
  deleted.”
- **No cross-file deduplication**: exactly duplicated tests are not identified in this table (0.6-2
  measured only 1%, which does not constitute a retirement surface).
- **The audit covers only 40 tests**: its conclusions are used only to characterize these 40 tests
  and to provide a **proportion** for the 2403-test pool; no individual decision may be extrapolated
  to other tests in the same directory.
