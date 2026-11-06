---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every repository test according to the three gates in Part Seven of
  coding-rules, and provide a clause-gate sampling audit plus an initial retirement-candidate list'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6, “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **three gates (clause / incident history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce an item-by-item decision table that can
be rerun; then perform a stratified sample audit of the clause gate (40 items), measure the
proportion of tests that “cite a clause but cannot support the assertion,” and recompute the
distribution using the **tightened criteria**. **This page and the generator itself are read-only
and do not modify tests**; test retirement is performed in batches (one commit per batch, with the
whole batch revertible via `git revert`), and the results are recorded in the “First Retirement
Batch / Second Retirement Batch” sections below and in `09a`.

Generated artifacts:

| Artifact                                  | Purpose                                                                                                                            |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv`       | Item-by-item decision table (3413 rows + header), in the same directory as this page                                               |
| `scripts/dev/test-gate-census.py`         | Generator (rerunnable, supports `--check` and `--selftest`)                                                                        |
| `scripts/dev/test-gate-census-audit.py`   | Sample audit: reproduces the fixed-seed sample of 40 items (`--sample` / `--check`)                                                |
| `scripts/dev/test-gate-census-audit.json` | Item-by-item decisions and evidence for the 40 items; the generator reads it to classify “audit-confirmed genuine support” as Keep |

## Scope and Definitions

### Scan Surface

Every `#[test]` in all `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

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

| Gate                              | Machine Criterion                                                                                                                                                                                                                                                                                                                                                                                                     | Meaning on a Match                                                      |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Clause Gate (tightened by 0.6-10) | A **verifiable clause** appears within the first 25 lines of the file header: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); for files under `src/**/tests/`, inspect the parent `mod.rs` together with the file                                                                                                                                                          | The test maps to an identifiable clause                                 |
| Incident-History Gate             | The test function body, function doc comment, or **file header** (`//! Regression background: issue #226`) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                          | The test pins down an incident that has actually occurred               |
| Proof Gate                        | **Cannot be determined by machine** (“Would it fail if replaced with a different correct implementation?” requires human judgment). Therefore, anything that fails the first two gates is classified as “Pending Determination,” never automatically as “Neither Passes”; the only exception is a body with **no assertions at all** (including no delegation to `assert*`/`check*` helpers and no `#[should_panic]`) | It can only be classified as “Pending Determination” or “No Assertions” |

Rationale for tightening: the 0.6-9 Clause Gate required only that the file header contain
`§`/`RFC`/`SPEC`, resulting in 2403 tests being classified as “Keep.” The 40-item stratified sample
measured by 0.6-10 found that **half of the citations could not support the assertions** (see
below). After tightening, only section-number forms are accepted, and **having a citation still does
not mean the citation supports the assertion**—every unaudited clause citation is classified only as
“Pending Verification.”

Rationale for the file-header radius: this repository’s convention is to put “Regression background:
issue #N” in the file header rather than repeat it in each test comment. Looking only at each test’s
own comment would miss the incident history for the entire file (measured: 7 tests within the test’s
own radius, 291 within the file-header radius).

### Categories That Are Never Retired

The following tests are always classified as **Keep** and never enter the retirement-candidate pool:

- Corpus tests under `tests/yaoxiang/**` and project-shape tests under `tests/project-corpus/**`
  (behavioral-criteria surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and crossing the process boundary);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition           | Meaning                                                                                                                                                                                                                  |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Keep                  | Only three sources qualify: an Incident-History Gate match, a Proof Gate match (spec-shaped directories / current-state anchor names), or a clause citation **confirmed by the sample audit to provide genuine support** |
| Pending Verification  | Cites a clause (with a section number, or only an entire RFC without a section number) but has not been audited, or the audit found it irrelevant or generic—human verification is required before it can be kept        |
| Retirement Candidate  | Fails both gates and has only weak assertions (or none) in its assertion surface—a candidate is not a verdict; the Proof Gate still requires manual review                                                               |
| Pending Determination | Fails both gates but has strong assertions; the machine reaches no conclusion                                                                                                                                            |

## Distribution After Tightening

| Gate                                                                      | Count    |
| ------------------------------------------------------------------------- | -------- |
| Clause (has a verifiable clause)                                          | 1714     |
| Incident History                                                          | 313      |
| Proof (spec-shaped criteria surface / current-state anchor)               | 447      |
| Pending Determination (fails both gates; Proof Gate awaits manual review) | 924      |
| Neither Passes (no assertions in the body)                                | 15       |
| **Total**                                                                 | **3413** |

| Disposition           | Count    | Compared with 0.6-9                                                                            |
| --------------------- | -------- | ---------------------------------------------------------------------------------------------- |
| Keep                  | **775**  | 2897 (−2122)                                                                                   |
| Pending Verification  | 2122     | 0 (new category)                                                                               |
| Pending Determination | 411      | 412 (−1 after the second retirement batch; unchanged after the generator correction)           |
| Retirement Candidate  | 105      | 136 (21 in the first retirement batch, 11 in the second, and +1 from the generator correction) |
| **Total**             | **3413** | —                                                                                              |

Two definition changes require explanation:

1. **Structure of Keep**: 775 = 447 Proof Gate + 313 Incident-History Gate + 15 confirmed by the
   audit to provide genuine support (of the 20 audit samples with genuine support, the other 5 were
   in files already kept by the Incident-History Gate). Of the original 2897, the 2121 that were
   “kept merely by citing something” are now Pending Verification.
2. **Incident-History Gate 47 → 313**: these are not new incident histories, but **label
   reclassifications**—in 0.6-9, tests that matched both the Clause Gate and Incident-History Gate
   were counted as Clause Gate matches. After tightening, a Clause Gate match no longer
   automatically means Keep, so the Incident-History Gate must catch these tests and their counts
   are folded into that gate. The total number kept is therefore unaffected (those tests were also
   kept in 0.6-9).

File-level criterion: of the 296 test files, **113** have a verifiable section number in the file
header (the old 0.6-9 criterion yielded 221).

## Clause Gate Sample Audit (0.6-10)

### Sampling Criteria (Reproducible)

- **Candidate pool = the 2403 tests classified as “Clause” by the 0.6-9 Clause Gate** (frozen using
  the old criterion in `test-gate-census-audit.py`; the sampling denominator must not change with
  the new criterion, or the conclusions would not be comparable).
- **Strata = directory clusters**: for `src/`, take the first two levels (`src/frontend`,
  `src/middle`, …); group everything else by top-level directory (`tests`, `benches`, `tools`).
- **Allocation**: proportional to the number of tests in each stratum (largest-remainder method),
  with at least one test per stratum.
- **Selection**: within each stratum, sort by (file, line number, test name), then sample randomly
  using a fixed seed.
- **Seed = 20261106** (hard-coded in the script; changing the seed changes the sample, so it must be
  documented and the audit rerun).

Stratified result: `src/frontend` 21 · `tests` 4 · `src/driver` 3 · `src/package` 3 · `src/backends`
2 · `src/formatter` 2 · `src/middle` 2 · `src/util` 2 · `tools` 2 · `src/ast` 1 · `src/lsp` 1 ·
`src/std` 1.

### Item-by-Item Decisions for the 40 Tests

Decision criteria (fixed here to prevent each item from being judged by a different standard):

- **Genuine support**: the citation identifies a specific section/note/decision number, and its
  **topic** is the same as the assertion target;
- **Irrelevant**: the citation identifies a specific clause, but that clause covers **something
  else** (the evidence column states what it actually covers);
- **Generic**: the citation refers only to an entire document/chapter/a list of sections/a phase
  name, with no item that maps to the assertion.

| #   | Test                                                                        | File                       | Decision        | Evidence (What the Cited Clause Actually Covers)                                                                                                                                                                                              |
| --- | --------------------------------------------------------------------------- | -------------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `test_compile_time_value_param_spec_table`                                  | `classify.rs`              | Genuine support | type-system §7.1 “compile-time value parameters” is exactly the assertion target: it verifies the truth table for determining value-position and type-position parameters row by row.                                                         |
| 2   | `test_id_stable_under_subsequent_registrations`                             | `value.rs`                 | Genuine support | RFC-010 “runtime representation and equality” defines the sum-type identity convention: identity is determined only by the name; the assertion verifies that the ID does not change after subsequent registrations.                           |
| 3   | `test_driver_run_check_rootless_multi_file_well_formed_generic_stays_clean` | `check_rootless_mono.rs`   | Genuine support | 02-stage-contract §3 note #14 point ③ discusses consistent monomorphization naming; the test is precisely the pinning test for that note (both the name and the assertion match).                                                             |
| 4   | `test_multifile_benign_import_cycle_message_predicts_no_runtime_failure`    | `import_cycle.rs`          | Genuine support | RFC-013’s W1xxx warning code and user decision A (cycles must not be silent); the assertion verifies that a benign cycle produces exactly one W1007 and predicts no runtime consequences.                                                     |
| 5   | `test_obligation_config_disabled_skip_is_not_owed`                          | `obligations.rs`           | Genuine support | 02-stage-contract §2 “obligation ledger”; the assertion verifies that a ConfigDisabled skip does not create an outstanding obligation.                                                                                                        |
| 6   | `test_format_type_bool`                                                     | `types.rs`                 | Genuine support | formatter specification §9 type annotations; the assertion verifies that the type name is emitted verbatim.                                                                                                                                   |
| 7   | `test_fstring_multiline_single_quotes_inside`                               | `fstring.rs`               | Genuine support | syntax §1.6.3 string literals; the assertion verifies that a single quote inside multiline f-string content does not terminate it.                                                                                                            |
| 8   | `test_float_underscore`                                                     | `literals.rs`              | Genuine support | syntax §1.6 literals (1.6.2 floating-point numbers) covers underscore separators; the assertion verifies that 1_000.5 is lexed as 1000.5.                                                                                                     |
| 9   | `test_empty_unicode_escape_is_error`                                        | `literals.rs`              | Genuine support | syntax §1.6.3 defines a closed set of escapes; the assertion verifies that an empty \u{} escape must produce an error.                                                                                                                        |
| 10  | `test_probe_accepted_const_type_is_registered`                              | `probe.rs`                 | Genuine support | RFC-039 P6 6.4 / WBS 6.6.9 means “constant-parameter determination obeys only injected probes”; the assertion verifies that N is registered when the probe accepts Int.                                                                       |
| 11  | `test_direct_eval_with_bound_variable_proved`                               | `predicate.rs`             | Genuine support | RFC-027 §3-4 (path-condition propagation / compile-time proof pipeline); the assertion verifies that b>0 evaluates directly to Proved when b=5.                                                                                               |
| 12  | `test_unused_type_is_not_an_entry_point`                                    | `dead_code.rs`             | Genuine support | RFC-011 §7 dead-code elimination mechanism; the assertion verifies that an unused type definition is not a reachable root.                                                                                                                    |
| 13  | `test_display_forall_multiple_vars_no_trailing_space`                       | `ast.rs`                   | Genuine support | RFC-027 §8 SMT solver; the assertion verifies the print format for forall with multiple variables.                                                                                                                                            |
| 14  | `test_dep_graph_remove_dependant_clears_edges`                              | `rfc027_phase31_dep_vc.rs` | Genuine support | RFC-027 Phase 3.1’s TypeDepGraph; the assertion verifies that removing a dependent clears its edges and constraints together.                                                                                                                 |
| 15  | `test_condition_bool`                                                       | `conditional.rs`           | Genuine support | RFC-011 §5 conditional types; the assertion verifies the evaluation and determinate state of TypeCondition::Bool.                                                                                                                             |
| 16  | `test_convert_expr_to_const_expr_int_literal`                               | `const_eval.rs`            | Genuine support | RFC-011 §4 compile-time generics; the assertion verifies that an Int literal is converted to ConstExpr::Lit.                                                                                                                                  |
| 17  | `test_definition_on_binding_site_returns_none_precise_only`                 | `definition.rs`            | Genuine support | RFC-039 D55 (precise-only, with no name-based fallback); the assertion verifies that the definition site itself returns None when there is no reference record.                                                                               |
| 18  | `test_type_consistent_arith_passes`                                         | `verify.rs`                | Genuine support | The IR static validator criterion in WBS 2.1 / RFC-039 P2 (the first layer of 07-equivalence-oracle); the assertion verifies that type-consistent arithmetic passes.                                                                          |
| 19  | `test_verify_loose_corpus_green`                                            | `verify.rs`                | Genuine support | The same validator criterion; the assertion verifies that the corpus runs entirely green.                                                                                                                                                     |
| 20  | `test_parse_tag_tolerates_query_and_fragment`                               | `toolchain.rs`             | Genuine support | RFC-037 §installer support; the assertion verifies that the query and fragment are stripped from the release URL.                                                                                                                             |
| 21  | `test_release_unset_register`                                               | `execute.rs`               | Irrelevant      | #299 §2 covers the NewArray fixed-length array instruction (allocation, Void placeholder, and bytecode round-trip); the assertion is that “executing Release on an unset register does not panic.”                                            |
| 22  | `test_fstring_segment_interpolation`                                        | `ast.rs`                   | Irrelevant      | It cites “Language Specification §3 (AST Structure),” but syntax §3 is the statements chapter (3.1 Statement Classification … 3.11 Program Entry and Top-Level Statements); the assertion covers the fields of FStringSegment::Interpolation. |
| 23  | `test_expr_for`                                                             | `ast.rs`                   | Irrelevant      | It cites the same “§3 (AST Structure)” reference; syntax §3 covers statements, while the assertion covers the AST fields of Expr::For.                                                                                                        |
| 24  | `test_add_var_without_mut_stores_immutability`                              | `scope.rs`                 | Irrelevant      | It cites “Language Specification §5.2”: syntax has no Chapter 5, while type-system §5.2 is “Multiple Constraints”; the assertion covers the immutability flag stored by ScopeManager.                                                         |
| 25  | `test_type_evaluator_eval_nested_type`                                      | `evaluator.rs`             | Irrelevant      | It cites syntax §3.11 (Program Entry and Top-Level Statements) and “RFC-027 §362” (which does not exist); the assertion covers the evaluation of a nested type.                                                                               |
| 26  | `test_get_ast_type_universe_level_plain`                                    | `mono.rs`                  | Irrelevant      | It cites “Language Specification §3”; syntax §3 is the statements chapter, while the assertion verifies that the type universe level is 0.                                                                                                    |
| 27  | `test_unify_enum_same`                                                      | `solver.rs`                | Irrelevant      | It cites “Language Specification §3”; the assertion verifies that unification of enums with the same name succeeds, which concerns type unification rather than the statements chapter.                                                       |
| 28  | `test_weak_new_upgrade_some_while_original_alive`                           | `weak.rs`                  | Irrelevant      | It cites RFC-010’s **sum-type identity**, while the assertion covers the lifecycle of a weak reference (#384: while the original Arc is alive, upgrade yields Some).                                                                          |
| 29  | `test_i18n_registry_english_titles`                                         | `codes.rs`                 | Irrelevant      | §3.4’s I3/I4 cover the translations of E1090/E1091; the assertion verifies that E0001’s English title and help text are non-empty.                                                                                                            |
| 30  | `test_format_return_expression`                                             | `expr.rs`                  | Generic         | The citation is a list of §3, §4, §7, §8, §10–§13, §17, and §18, with no item that maps to “how a return expression is rendered.”                                                                                                             |
| 31  | `test_rfc010_generic_syntax`                                                | `lexer_mod.rs`             | Generic         | The citation broadly lists syntax §1.6 and RFC-004/RFC-010; the test describes itself as flexible (allowing any parenthesized form), and there is no verifiable clause.                                                                       |
| 32  | `test_anon_binding_with_fn_annotation_keeps_extracted_signature`            | `types.rs`                 | Generic         | “spec §3, RFC-010, RFC-011” is a broad list, with no entry corresponding to “extracting an annotated signature from an anonymous function binding.”                                                                                           |
| 33  | `test_consumer_unknown_token_returns_empty`                                 | `ownership.rs`             | Generic         | “RFC-009 + RFC-009a” is a whole-document list; the assertion verifies that BrandTree returns an empty consumer set for an unknown token.                                                                                                      |
| 34  | `test_const_expr_neg_int_literal_displays_by_value`                         | `const_data.rs`            | Generic         | RFC-011 §4 covers compile-time generics and does not concern the print form of ConstExpr; the assertion verifies that a negative literal is printed as -5.                                                                                    |
| 35  | `resolve_local_and_method`                                                  | `resolver.rs`              | Generic         | “RFC-029 Refactoring Core” is an entire architectural RFC; the assertion covers the resolver’s qualified names and DefKind.                                                                                                                   |
| 36  | `test_outdated_exec_smoke`                                                  | `outdated.rs`              | Generic         | RFC-014a appears only as background in a comment (the registry has not been published, so a path dependency is used); the assertion is an exec smoke test with no assertions in the body.                                                     |
| 37  | `test_compatible_high_major_range`                                          | `resolver.rs`              | Generic         | “RFC-014 Phase 3” is a phase name, not a clause; the assertion covers version-range compatibility determination.                                                                                                                              |
| 38  | `test_clean_and_size`                                                       | `cache.rs`                 | Generic         | “RFC-014 Phase 3 (Global Cache Design)” is also a phase name; the assertion covers cache clean and dir_size.                                                                                                                                  |
| 39  | `test_strip_ansi_multiple_consecutive_sequences`                            | `ansi.rs`                  | Generic         | §4.2 is the “ANSI Tool Extraction” work item, not a specification clause; the assertion covers the behavior of strip_ansi for consecutive sequences.                                                                                          |
| 40  | `test_verify_sha256_accepts_matching_digest_file_content`                   | `dl.rs`                    | Generic         | It cites “RFC-037: version directory = release package extraction root directory”; the assertion verifies that sha256 validation accepts a matching digest file.                                                                              |

Distribution: **Genuine Support 20 · Irrelevant 9 · Generic 11**.

### Mis-citation Rate and Confidence Interval

- Mis-citations (Irrelevant + Generic) = 20/40 = **50%**;
- Wilson 95% confidence interval (n=40): **[35.2%, 64.8%]**;
- Extrapolated to 2403: approximately **1202** tests have genuine clause support (95% interval **846
  ~ 1557**), and approximately **1201** are mis-cited (interval 846 ~ 1557).

### Impact on the Cleanup Surface

- **“Keep” falls from 2897 to 775** (−2122)—exactly the 2122 tests that were “kept merely by citing
  something” become Pending Verification; this is the effect of tightening, not distribution tuning.
- **Retirement candidates are unrelated to clauses**: the first batch of 26 had nothing to do with
  clauses (not a single assertion in the body), and the audit does not change their classifications;
  21 tests in the first batch have been retired, taking candidates from 136 → 115; 11 tests in the
  second batch have been retired, taking candidates from 115 → 104; after 0.6-13 corrected the
  generator (function-body boundaries are no longer truncated by braces inside string literals), the
  count increases by 1, taking candidates from 104 → 105. The second batch of 11 follows the same
  principle: the bodies **do** contain assertions, but only weak assertions such as `is_some()` /
  `is_none()`, which do not lock down binding power or the parse result. This also shows that the
  “retirement surface” and the “clause mis-citation surface” are separate problems: most mis-cited
  tests have assertions and should receive a valid clause or have their references corrected rather
  than be deleted.
- The 2122 Pending Verification tests represent the subsequent “Clause Gate ticket supplementation”
  workload: add a specific clause that maps to the assertion, or reclassify the test as a retirement
  candidate.

### Limitations (Do Not Cite Beyond Scope)

- The 40 items can provide only a **proportion**; no individual decision may be extrapolated to
  other tests in the same directory;
- The decisions were made by manually reading the code and clause documents; the evidence column
  states “what the cited clause actually covers” for review;
- The sampling denominator is the old-criterion pool from 0.6-9 (2403). The proportion of
  mis-citations among the **1714** tests remaining after tightening was not resampled—tightening
  merely excludes “unverifiable” citations; it does not mean every citation in the new pool
  genuinely supports its assertion (each still requires individual verification).

## Reconciliation with the 0.6-2 Survey

The 0.6-2 survey figures provide the first-version comparison baseline. The reasons for each
difference are listed below:

| Item                           | 0.6-2 Survey                  | This Census                                                                                                                                 | Reason for the Difference                                                                                                                                                                                                                 |
| ------------------------------ | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`      | 3443                          | 3413                                                                                                                                        | Two tests were added after 0.6-2 (the check-criterion regression tests for E3005 and E3022), while 21 no-assertion tests were removed according to the first retirement list and 11 weak-assertion tests according to the second; net −30 |
| Number of test files           | 296                           | 296                                                                                                                                         | Unchanged                                                                                                                                                                                                                                 |
| Clause Gate                    | 227 files (77%)               | 113 files after tightening; 1714 by item                                                                                                    | The criterion was tightened from “the keyword appears” to “there is a section number `§<number>`,” so the file count was nearly halved                                                                                                    |
| Incident-history signal        | 442 tests / 148 files         | 313 within the gate; three radii: 7 within the test itself / 291 within the file header (38 files) / 1218 across the entire file (79 files) | The 148 files in 0.6-2 cover more than the 79 files found by the entire-file radius in this census; its vocabulary and scope were broader. This census strictly uses the four gate-surface signals.                                       |
| Characterization strong signal | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                                                | Characterization is not one of the three gates; mixing it into retirement conclusions would exceed the gate’s authority                                                                                                                   |
| Weak assertions                | 473 (14%)                     | See the “Assertion Summary” column in the TSV                                                                                               | The counting units differ: 0.6-2 counted patterns that assert only `is_ok`/`is_some`; this census uses lexical counting and distinguishes strong, weak, and delegated assertions                                                          |

## Conclusion Summary

1. **The Clause Gate was previously significantly inflated**: under the old criterion, 2403 tests
   were classified as “Clause/Keep,” while the sample audit measured a 50% mis-citation rate (95%
   interval 35%–65%)—“having a citation” and “the citation supports the assertion” are different
   things, and the old criterion treated the former as the latter.
2. **After tightening, approximately 1202 tests genuinely have clause support** (interval 846–1557);
   the rest must either await clause supplementation and verification or be classified under the
   Incident-History Gate / Proof Gate.
3. **The retirement surface is unrelated to clause mis-citations**: the 136 candidates are unrelated
   to clauses (no assertions/weak assertions); 21 tests in the first batch and 11 in the second have
   been retired, and the generator correction adds 1, reducing the count to 105.
4. **The Incident-History Gate is the largest legitimate source of Keep decisions** (313 tests): it
   demonstrates that “pinning down a real incident” is more reliable than “copying a sentence from
   an RFC.”

## First Retirement Batch (Executed, 2026-11-06)

**First batch: 21 of the 26 “no assertion” tests in `src/frontend/core/types/eval/tests/` were
retired, and 5 were kept** (single directory cluster; criterion = all **three gates fail** under
Part Seven of coding-rules: no Clause Gate match, no Incident-History Gate match, and no assertions
in the body). The item-by-item decisions and “same-cell coverage” are provided by the 0.6-11a
decision table (21 deleted / 5 kept); this page does not repeat the reasons and records only the
execution results.

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
- **5 kept** (the decision table could not identify alternative coverage, so the hard requirement is
  to keep them): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls back the whole batch.

## Second Retirement Batch (Executed, 2026-11-06)

**Second batch: 11 of the 18 “weak assertion” tests in `src/frontend/core/parser/pratt/tests/nud.rs`
were retired, and 7 were kept** (single directory cluster = the Pratt prefix parser module;
criterion = all **three gates fail** under Part Seven of coding-rules: no Clause Gate match, no
Incident-History Gate match, and only weak assertions such as `is_some()` / `is_none()` in the body,
which do not lock down binding power or the parse result).

The difference between this batch and the first must be stated clearly: these tests **do contain
assertions** and are not empty tests. The reason for retirement is “all three gates fail + zero
criterion gain,” not “no assertions.” Every test selected for deletion identifies a **unit-level
replacement coverage test in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The replacement
  coverage for each item follows the **same** `parse_expression → parse_prefix → prefix_info` path
  and has stronger assertions (it locks down the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, and `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the item-by-item decision table under “VI. Second Test Retirement
  Batch” in `09a-wbs-history.md` for the reasons. The reasons fall into two categories: the **Proof
  Gate holds** (the first three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are
  the only **unit-level** walkthrough points for the corresponding facts, and the repository
  contains no second unit test that observes them), and the **Incident-History Gate holds**
  (`break_removed` / `continue_removed` explicitly name #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls back the whole batch.

## Generator Correction (0.6-13)

**`body_span` truncates function bodies when it encounters braces inside literals—fixed.**

The old implementation obtained function bodies by balancing braces without distinguishing real code
from braces in string literals, character literals, or comments. Rust source frequently contains
expressions such as `tokenize("{")`, which skewed the balance and truncated the function body: the
body of `test_prefix_info_lbrace` was once truncated to the end of the file, causing the “Assertion
Summary” to capture the remainder of the file (`matchesx11 assertx24 …`) and the test to be binned
as “Pending Determination” rather than “Retirement Candidate”—**the wrong bin**.

Fix: added `mask_noncode`, which first replaces the contents of string literals, character literals,
line comments, block comments (including nested ones), and raw strings (`r"…"` / `r#"…"#` / `br"…"`
/ `cr#"…"#`) with spaces (newlines are preserved, so line numbers and line counts do not change),
then balances the braces. **The mask is used only to find function-body boundaries**; assertion
statistics and the Incident-History Gate still read the original text, and the criterion itself has
not been relaxed. `--selftest` now has four cases (paired and unpaired braces in ordinary strings,
braces in raw strings and block comments, and braces in character literals), each followed by a
sentinel test—the old implementation gets the boundary wrong in all four cases, while the new
implementation gets all four right.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** for every test in
every file (the old implementation balanced by raw braces; the new implementation balances after
masking). Tests with different boundaries are affected. Of the 3413 tests in the repository, the old
implementation extracted the boundaries incorrectly for **11**:

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

### Bin Differences Before and After the Fix: 2

For the other 9 tests, **8 affected only the “Assertion Summary” column** (their bins did not
change), while **1 did not change even the summary**
(`test_fstring_close_brace_escape_preserved_raw`: the old implementation omitted the closing `}`
line, which contains no assertion keyword). The eight summary changes are listed individually in
this section of `09a-wbs-history.md`.

| Test Name                   | File                                          | Old Bin                                       | New Bin                                          | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| --------------------------- | --------------------------------------------- | --------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending Determination / Pending Determination | Pending Determination / **Retirement Candidate** | The actual function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; the file header and function body contain no § section number, issue/regression/defect/incident reference ⇒ both the Clause Gate and Incident-History Gate fail, and the body contains only a weak assertion such as `is_some()` ⇒ Retirement Candidate (a candidate is not a verdict; the Proof Gate still requires manual review)                                                                        |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident History / Keep                       | Clause / **Pending Verification**                | The old implementation truncated the function body to the end of the file and swallowed text containing “regression / defect / incident,” incorrectly classifying it under the Incident-History Gate; the actual function body contains none of these terms ⇒ the Incident-History Gate no longer holds; the file header contains verifiable clauses (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the Clause Gate matches, but it has not been confirmed by the sample audit ⇒ Pending Verification |

The distribution changes accordingly: Clause 1713 → 1714 · Incident History 314 → 313 · Keep 776 →
775 · Pending Verification 2121 → 2122 · Pending Determination 412 → 411 · Retirement Candidate 104
→ 105. **The repository-wide `#[test]` count is unchanged (3413)**—this round changes only the
implementation of the criterion and does not modify any test.

## Rerunning and Self-Checks

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: check whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
python scripts/dev/test-gate-census-audit.py --sample  # Reproduce the 40-item sample (seed 20261106)
python scripts/dev/test-gate-census-audit.py --check   # Check whether the audit data file covers every sample
```

The generator performs only lexical determination; its boundaries are stated explicitly:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name does not have either prefix, it will be
  misclassified as “No Assertions”—the first batch of 26 (21 retired / 5 kept) and the second batch
  of 18 (11 retired / 7 kept) have each been manually reviewed function by function.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator obtains a function body, it
  balances braces; braces in string literals, character literals, and comments skew the balance and
  truncate the body—`test_prefix_info_lbrace` (which contains `tokenize("{")` in its body) was once
  truncated to the end of the file, and the “Assertion Summary” captured the remainder of the file,
  causing it to be binned as “Pending Determination” rather than “Retirement Candidate.” It now
  masks non-code regions (strings / character literals / line comments / block comments / raw
  strings) before balancing them; see “Generator Correction” below.
- **The Proof Gate is not adjudicated**: every test that “fails both gates” is classified only as
  “Pending Determination” or “Retirement Candidate”; the generator never declares that a test
  “should be deleted.”
- **No cross-file deduplication**: completely duplicate tests are not identified in this table
  (0.6-2 measured them at only 1%, so they do not constitute a retirement surface).
- **The audit covers only 40 items**: audit conclusions are used only to classify those 40 items and
  to provide a **proportion** for the 2403-item pool; no individual decision may be extrapolated to
  other tests in the same directory.
