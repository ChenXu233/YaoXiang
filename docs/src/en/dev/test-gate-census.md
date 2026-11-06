---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository using the three gates from Part Seven of
  coding-rules, and provide a sampled audit of the clause gate and an initial list of retirement
  candidates'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 "Major Cleanup": mechanically classify every `#[test]` in the
repository according to the
[three gates (clause / incident history / proof) in Part Seven of coding-rules](./coding-rules.md),
producing a reproducible, item-by-item decision table; then perform a stratified sampled audit of
the clause gate (40 items) to measure the proportion of tests that "cite a clause but cannot support
their assertions," and recompute the distribution using **tightened criteria**. **This page and the
generator are read-only with respect to tests**—this stage neither deletes nor modifies any test.

Outputs:

| Output                                    | Purpose                                                                                                                                 |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv`       | Item-by-item decision table (3424 rows plus the header), in the same directory as this page                                             |
| `scripts/dev/test-gate-census.py`         | Generator (rerunnable, supports `--check` and `--selftest`)                                                                             |
| `scripts/dev/test-gate-census-audit.py`   | Sample audit: reproduces 40 samples with a fixed seed (`--sample` / `--check`)                                                          |
| `scripts/dev/test-gate-census-audit.json` | Item-by-item decisions and rationales for the 40 samples (the generator reads it to classify "audit-confirmed genuine support" as Keep) |

## Criteria

### Scan Scope

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Scope                                                                    | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module's `tests/` subdirectory) | 2950                 |
| `tests/integration`                                                      | 335                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 75                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **3424** (296 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                        | Meaning on a hit                                                   |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------ |
| Clause gate (tightened in 0.6-10) | The first 25 lines of the file contain a **verifiable item**: section-number form `§<数字>` (including `RFC-<数字> §<数字>` and `SPEC §<数字>`); files under `src/**/tests/` are examined together with their parent `mod.rs`                                                                                                                                                                                            | The test can be tied to a specific, nameable clause                |
| Incident-history gate             | The test function body, function doc comment, or **file header** (such as `//! 回归背景：issue #226`) names an actual failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                               | The test pins down an incident that actually occurred              |
| Proof gate                        | **Cannot be determined by machine** ("If we replaced it with another correct implementation, would it fail?" requires human judgment). Therefore, anything that fails both preceding gates is classified as "pending judgment," never automatically as "neither passes"; the only exception is a body with **not a single assertion** (including no delegation to an `assert*`/`check*` helper and no `#[should_panic]`) | It can only be classified as "pending judgment" or "no assertions" |

Rationale for tightening: In 0.6-9, the clause gate only required the file header to **contain
`§`/`RFC`/`SPEC`**, resulting in 2403 tests being classified as Keep. The 40-item stratified sample
in 0.6-10 found that **half of the citations could not support the assertions** (see below). After
tightening, only section-number forms count, and **having a citation still does not mean the
citation can support the assertion**—every unaudited clause citation is classified only as "pending
verification."

Rationale for the file-header radius: This repository's convention is to put "regression context:
issue #N" in the file header rather than repeat it in each test comment. Looking only at a test's
own comments misses the history of the entire file (measured: 7 cases within the test's own radius
versus 291 within the file-header radius).

### Categories That Are Never Retired

The following tests are **always classified as Keep** and never become retirement candidates:

- Corpus tests under `tests/yaoxiang/**` and project-shape tests under `tests/project-corpus/**`
  (behavioral criterion surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and crossing the process boundary);
- Status-quo anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                                       |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Only three sources: an incident-history gate hit, the proof gate (spec-shaped criterion surface / status-quo anchor names), or a clause citation **confirmed by the sample audit to provide genuine support** |
| Pending verification | Cites a clause (with a section number, or only an entire RFC without a section number) but is unaudited or was audited as irrelevant or generic—it must be manually verified before it can be kept            |
| Retirement candidate | Both gates fail and the assertion surface contains only weak assertions (or no assertions)—a candidate is not a verdict, and the proof gate still requires human review                                       |
| Pending judgment     | Both gates fail, but strong assertions are present; the machine gives no verdict                                                                                                                              |

## Distribution (After Tightening)

| Gate                                                               | Count    |
| ------------------------------------------------------------------ | -------- |
| Clause (with a verifiable item)                                    | 1713     |
| Incident history                                                   | 314      |
| Proof (spec-shaped criterion surface / status-quo anchor)          | 447      |
| Pending judgment (both gates fail; proof gate awaits human review) | 935      |
| Neither passes (no assertions in the body)                         | 15       |
| **Total**                                                          | **3424** |

| Disposition          | Count    | Compared with 0.6-9                 |
| -------------------- | -------- | ----------------------------------- |
| Keep                 | **776**  | 2897 (−2121)                        |
| Pending verification | 2121     | 0 (new tier)                        |
| Pending judgment     | 412      | 412 (unchanged)                     |
| Retirement candidate | 115      | 136 (21 in the first batch retired) |
| **Total**            | **3424** | —                                   |

Two changes in criteria require explanation:

1. **Structure of Keep**: 776 = 447 from the proof gate + 314 from the incident-history gate + 15
   confirmed by the audit to provide genuine support (of the audit's 20 genuinely supported tests,
   the other 5 were already classified as Keep through the incident-history gate). The 2121 tests
   among the original 2897 that were "kept on the strength of a single citation" are now pending
   verification.
2. **Incident-history gate 47 → 314**: These are not newly discovered incidents; this is a
   **relabeling**. In 0.6-9, cases where "both clause and incident history matched" were recorded as
   clause matches. After tightening, a clause match no longer automatically results in Keep, so the
   incident-history gate must catch these cases, and their counts are folded into the
   incident-history total. The overall Keep count is therefore unaffected (these tests were also
   classified as Keep in 0.6-9).

File scope: of the 296 test files, **113** have a verifiable section number in their file header
(the old 0.6-9 criterion identified 221).

## Clause-Gate Sample Audit (0.6-10)

### Sampling Methodology (Reproducible)

- **Candidate pool = the 2403 tests classified as "clause" by the 0.6-9 clause gate** (frozen using
  the old criterion in `test-gate-census-audit.py`; the sampling denominator must not change with
  the new criterion, or the conclusions would not be comparable).
- **Stratification = directory clusters**: use the first two levels under `src/` (`src/frontend`,
  `src/middle`, etc.); group the rest by top-level directory (`tests`, `benches`, `tools`).
- **Allocation**: proportional to the number of tests in each stratum (largest remainder method),
  with at least one test per stratum.
- **Extraction**: sort each stratum by (file, line number, test name), then sample randomly with a
  fixed seed.
- **Seed = 20261106** (hard-coded in the script; changing the seed changes the sample, so the change
  must be documented and the audit rerun).

Stratified results: `src/frontend` 21 · `tests` 4 · `src/driver` 3 · `src/package` 3 ·
`src/backends` 2 · `src/formatter` 2 · `src/middle` 2 · `src/util` 2 · `tools` 2 · `src/ast` 1 ·
`src/lsp` 1 · `src/std` 1.

### Item-by-Item Decisions for the 40 Tests

Decision criteria (fixed here to keep every item judged consistently):

- **Genuine support**: The citation identifies a specific section, note, or decision number, and its
  **topic** is the same as the assertion target;
- **Irrelevant**: The citation identifies a specific item, but that item covers **something else**
  (the rationale states what it actually covers);
- **Generic**: The citation points only to an entire document, an entire chapter, a list of
  sections, or a stage name, with no item that matches the assertion.

| #   | Test                                                                        | File                       | Decision        | Rationale (what the cited clause actually covers)                                                                                                                                                                                           |
| --- | --------------------------------------------------------------------------- | -------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `test_compile_time_value_param_spec_table`                                  | `classify.rs`              | Genuine support | type-system §7.1 "Compile-Time Value Parameters" is exactly the assertion target: the truth table for value-position/type-position parameter decisions is verified row by row.                                                              |
| 2   | `test_id_stable_under_subsequent_registrations`                             | `value.rs`                 | Genuine support | RFC-010's "Runtime Representation and Equality" defines the identity convention for sum types: identity is determined only by the name; the test asserts that the ID does not change after subsequent registrations.                        |
| 3   | `test_driver_run_check_rootless_multi_file_well_formed_generic_stays_clean` | `check_rootless_mono.rs`   | Genuine support | 02-stage-contract §3 note #14, point ③ discusses monomorphization naming provenance; the test is the pinning test for that note (both its name and assertions match).                                                                       |
| 4   | `test_multifile_benign_import_cycle_message_predicts_no_runtime_failure`    | `import_cycle.rs`          | Genuine support | RFC-013's W1xxx warning code and user ruling A (a cycle must not be silent); the test asserts that a benign cycle produces exactly one W1007 and predicts no runtime consequence.                                                           |
| 5   | `test_obligation_config_disabled_skip_is_not_owed`                          | `obligations.rs`           | Genuine support | 02-stage-contract §2, "Obligation Ledger"; the test asserts that skipping ConfigDisabled does not create an owed obligation.                                                                                                                |
| 6   | `test_format_type_bool`                                                     | `types.rs`                 | Genuine support | formatter specification §9, Type Annotations; the test asserts that the type name is emitted unchanged.                                                                                                                                     |
| 7   | `test_fstring_multiline_single_quotes_inside`                               | `fstring.rs`               | Genuine support | syntax §1.6.3, String Literals; the test asserts that a single quote inside multiline f-string content does not terminate it.                                                                                                               |
| 8   | `test_float_underscore`                                                     | `literals.rs`              | Genuine support | syntax §1.6, Literals (1.6.2 Floating Point), covers underscore separators; the test asserts that 1_000.5 is lexed as 1000.5.                                                                                                               |
| 9   | `test_empty_unicode_escape_is_error`                                        | `literals.rs`              | Genuine support | syntax §1.6.3's closed set of escapes; the test asserts that an empty \u{} escape must produce an error.                                                                                                                                    |
| 10  | `test_probe_accepted_const_type_is_registered`                              | `probe.rs`                 | Genuine support | RFC-039 P6 6.4 / WBS 6.6.9 is precisely "constant-parameter decisions obey only the injected probe"; the test asserts that N is registered when the probe accepts Int.                                                                      |
| 11  | `test_direct_eval_with_bound_variable_proved`                               | `predicate.rs`             | Genuine support | RFC-027 §3-4 (path-condition propagation / compile-time proof pipeline); the test asserts that, with b=5, b>0 evaluates directly to Proved.                                                                                                 |
| 12  | `test_unused_type_is_not_an_entry_point`                                    | `dead_code.rs`             | Genuine support | RFC-011 §7, Dead-Code Elimination Mechanism; the test asserts that an unused type definition is not a reachable root.                                                                                                                       |
| 13  | `test_display_forall_multiple_vars_no_trailing_space`                       | `ast.rs`                   | Genuine support | RFC-027 §8, SMT Solver; the test asserts the printing format for forall with multiple variables.                                                                                                                                            |
| 14  | `test_dep_graph_remove_dependant_clears_edges`                              | `rfc027_phase31_dep_vc.rs` | Genuine support | RFC-027 Phase 3.1's TypeDepGraph; the test asserts that removing a dependent clears its edges and constraints along with it.                                                                                                                |
| 15  | `test_condition_bool`                                                       | `conditional.rs`           | Genuine support | RFC-011 §5, Conditional Type; the test asserts the evaluation and determinate state of TypeCondition::Bool.                                                                                                                                 |
| 16  | `test_convert_expr_to_const_expr_int_literal`                               | `const_eval.rs`            | Genuine support | RFC-011 §4, Compile-Time Generics; the test asserts that an Int literal is converted to ConstExpr::Lit.                                                                                                                                     |
| 17  | `test_definition_on_binding_site_returns_none_precise_only`                 | `definition.rs`            | Genuine support | RFC-039 D55 (precise-only; forbids name-based fallback); the test asserts that a definition site with no reference record returns None.                                                                                                     |
| 18  | `test_type_consistent_arith_passes`                                         | `verify.rs`                | Genuine support | The IR static validator criterion in WBS 2.1 / RFC-039 P2 (the first layer of 07-equivalence-oracle); the test asserts that type-consistent arithmetic passes.                                                                              |
| 19  | `test_verify_loose_corpus_green`                                            | `verify.rs`                | Genuine support | The same validator criterion; the test asserts that the corpus passes entirely.                                                                                                                                                             |
| 20  | `test_parse_tag_tolerates_query_and_fragment`                               | `toolchain.rs`             | Genuine support | RFC-037 § Installer Support; the test asserts that the query and fragment are stripped from the release URL.                                                                                                                                |
| 21  | `test_release_unset_register`                                               | `execute.rs`               | Irrelevant      | #299 §2 covers the NewArray fixed-length array instruction (allocation, Void placeholder, bytecode round-trip); the assertion is that "executing Release on an unset register does not panic."                                              |
| 22  | `test_fstring_segment_interpolation`                                        | `ast.rs`                   | Irrelevant      | It cites "Language Specification §3 (AST Structure)," but syntax §3 is the statements chapter (3.1 Statement Categories … 3.11 Program Entry and Top-Level Statements); the assertion concerns the fields of FStringSegment::Interpolation. |
| 23  | `test_expr_for`                                                             | `ast.rs`                   | Irrelevant      | It cites the same "§3 (AST Structure)"; syntax §3 covers statements, while the assertion concerns the AST fields of Expr::For.                                                                                                              |
| 24  | `test_add_var_without_mut_stores_immutability`                              | `scope.rs`                 | Irrelevant      | It cites "Language Specification §5.2": syntax has no Chapter Five, while type-system §5.2 is "Multiple Constraint"; the assertion concerns the immutable flag stored by ScopeManager.                                                      |
| 25  | `test_type_evaluator_eval_nested_type`                                      | `evaluator.rs`             | Irrelevant      | It cites syntax §3.11 (Program Entry and Top-Level Statements) and "RFC-027 §362" (which does not exist); the assertion concerns the evaluation of nested types.                                                                            |
| 26  | `test_get_ast_type_universe_level_plain`                                    | `mono.rs`                  | Irrelevant      | It cites "Language Specification §3"; syntax §3 is the statements chapter, while the assertion concerns the type universe's level being 0.                                                                                                  |
| 27  | `test_unify_enum_same`                                                      | `solver.rs`                | Irrelevant      | It cites "Language Specification §3"; the assertion that unification of same-named enums succeeds belongs to type unification, not the statements chapter.                                                                                  |
| 28  | `test_weak_new_upgrade_some_while_original_alive`                           | `weak.rs`                  | Irrelevant      | It cites RFC-010's **sum-type identity**, while the assertion concerns the lifetime of a weak reference (#384: upgrade is Some while the original Arc remains alive).                                                                       |
| 29  | `test_i18n_registry_english_titles`                                         | `codes.rs`                 | Irrelevant      | §3.4's I3/I4 cover the translations of E1090/E1091; the assertion concerns E0001's English title and help being non-empty.                                                                                                                  |
| 30  | `test_format_return_expression`                                             | `expr.rs`                  | Generic         | The citation is a list of §3, §4, §7, §8, §10–§13, §17, and §18, with no item matching "how the return expression is rendered."                                                                                                             |
| 31  | `test_rfc010_generic_syntax`                                                | `lexer_mod.rs`             | Generic         | The citation is syntax §1.6 plus a broad list of RFC-004/RFC-010; the test describes itself as flexible (allowing arbitrary parenthesization), with no verifiable item.                                                                     |
| 32  | `test_anon_binding_with_fn_annotation_keeps_extracted_signature`            | `types.rs`                 | Generic         | "spec §3, RFC-010, RFC-011" is a broad list, with no item corresponding to "extracting the annotated signature of an anonymous function binding."                                                                                           |
| 33  | `test_consumer_unknown_token_returns_empty`                                 | `ownership.rs`             | Generic         | "RFC-009 + RFC-009a" lists entire documents; the assertion is that BrandTree returns an empty consumer set for an unknown token.                                                                                                            |
| 34  | `test_const_expr_neg_int_literal_displays_by_value`                         | `const_data.rs`            | Generic         | RFC-011 §4 covers compile-time generics and does not concern ConstExpr's rendering; the assertion concerns rendering a negative literal as -5.                                                                                              |
| 35  | `resolve_local_and_method`                                                  | `resolver.rs`              | Generic         | "RFC-029 Refactoring Core" is an entire architecture RFC; the assertion concerns the resolver's qualified names and DefKind.                                                                                                                |
| 36  | `test_outdated_exec_smoke`                                                  | `outdated.rs`              | Generic         | RFC-014a appears only as background in a comment (the registry was unpublished, so a path dependency was used); the assertion is an exec smoke test, with no assertions in the body.                                                        |
| 37  | `test_compatible_high_major_range`                                          | `resolver.rs`              | Generic         | "RFC-014 Phase 3" is a stage name, not an item; the assertion concerns the compatibility decision for a version range.                                                                                                                      |
| 38  | `test_clean_and_size`                                                       | `cache.rs`                 | Generic         | "RFC-014 Phase 3 (Global Cache Design)" is likewise a stage name; the assertion concerns cache clean and dir_size.                                                                                                                          |
| 39  | `test_strip_ansi_multiple_consecutive_sequences`                            | `ansi.rs`                  | Generic         | §4.2 is the "ANSI Utility Extraction" work item, not a specification clause; the test asserts strip_ansi's behavior for consecutive sequences.                                                                                              |
| 40  | `test_verify_sha256_accepts_matching_digest_file_content`                   | `dl.rs`                    | Generic         | It cites "RFC-037: Version Catalog = Distribution Package Extraction Root"; the assertion is that sha256 verification accepts a matching digest file.                                                                                       |

Distribution: **Genuine support 20 · Irrelevant 9 · Generic 11**.

### Mis-Citation Rate and Confidence Interval

- Mis-citations (irrelevant + generic) = 20/40 = **50%**;
- Wilson 95% confidence interval (n=40): **[35.2%, 64.8%]**;
- Extrapolated to 2403: approximately **1202** tests genuinely have clause support (95% interval
  **846 ~ 1557**), and approximately **1201** are mis-cited (interval 846 ~ 1557).

### Impact on the Cleanup Surface

- **Keep falls from 2897 to 776** (−2121)—exactly the 2121 tests that were "kept on the strength of
  a citation" are now pending verification; the tightened criterion takes effect rather than merely
  tuning the distribution.
- **Retirement candidates are unrelated to clauses**: the first 26 were already unrelated to clauses
  (not a single assertion in their bodies), and the audit does not change their decisions; the first
  batch of 21 has been retired, reducing candidates from 136 to 115. This also shows that the
  "retirement surface" and the "clause mis-citation surface" are two different problems: most
  mis-cited tests have assertions, so they need a supplied clause or corrected citation rather than
  deletion.
- The 2121 pending-verification tests represent the workload for subsequent "clause-gate
  remediation": supply a specific item that matches the assertion, or reclassify the test as a
  retirement candidate.

### Limitations (Must Not Be Cited Beyond Their Scope)

- The 40 items can provide only a **proportion**; no individual decision may be extrapolated to
  other tests in the same directory;
- The decisions result from manually reading the code and the clause documents; the rationale column
  states "what the cited clause actually covers" for review;
- The sampling denominator is the old 0.6-9 criterion pool (2403). The number of mis-citations among
  the **1713 tightened tests** was not resampled— tightening merely excludes unverifiable citations;
  it does not mean that every citation in the new pool can genuinely support its assertion (they
  still require item-by-item verification).

## Reconciliation with the 0.6-2 Survey

The 0.6-2 survey figures provide the first comparison baseline. The reasons for each difference are
listed below:

| Item                                      | 0.6-2 Survey                  | This Census                                                                                              | Reason for the Difference                                                                                                                                                                              |
| ----------------------------------------- | ----------------------------- | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| All `#[test]` functions in the repository | 3443                          | 3424                                                                                                     | Two tests were added after 0.6-2 (check-criteria regression tests for E3005 and E3022), and 21 no-assertion tests were removed according to the first retirement list; net −19                         |
| Number of test files                      | 296                           | 296                                                                                                      | Unchanged                                                                                                                                                                                              |
| Clause gate                               | 227 files (77%)               | Tightened to 113 files; 1713 by item                                                                     | The criterion was tightened from "the token appears" to "there is a section number of the `§<数字>` form," nearly halving the file count                                                               |
| Incident-history signals                  | 442 tests / 148 files         | 314 within the gate; three radii: 7 test-local / 291 file-header (38 files) / 1218 whole-file (79 files) | The 148 files in 0.6-2 represent a broader scope than the 79 files found here with the whole-file radius, due to its larger vocabulary and scope; this census strictly applies the four listed signals |
| Characterization-based strong signals     | 1176 (34%) / strong 631 (18%) | Not measured                                                                                             | Characterization is not one of the three gates; incorporating it into retirement conclusions would exceed their scope                                                                                  |
| Weak assertions                           | 473 (14%)                     | See the TSV's "Assertion Summary" column                                                                 | The counting units differ: 0.6-2 counted patterns that assert only `is_ok`/`is_some`; this census uses lexical counting and distinguishes strong, weak, and delegated assertions                       |

## Conclusion Summary

1. **The clause gate was previously significantly overstated**: under the old criterion, 2403 tests
   were classified as "clause/Keep," while the sample measured a 50% mis-citation rate (95% interval
   35%–65%)—"having a citation" and "having a citation that supports the assertion" are different
   things, and the old criterion conflated them.
2. **After tightening, approximately 1202 tests genuinely have clause support** (interval 846–1557);
   the remainder either need a citation while pending verification or should have been classified
   through the incident-history or proof gate in the first place.
3. **The retirement surface is unrelated to the clause mis-citation surface**: the 136 candidates
   are clause-independent (no assertions/weak assertions); the first batch of 21 has been retired,
   reducing the candidates to 115.
4. **The incident-history gate is the largest legitimate source of Keep** (314 tests): this
   demonstrates that "pinning down an actual incident" is more dependable than copying a line from
   an RFC.

## First Retirement Batch (Executed, 2026-11-06)

**First batch: 21 of the 26 "no assertions" tests under `src/frontend/core/types/eval/tests/` were
retired and 5 were kept** (a single directory cluster; the criterion was that all **three gates in
Part Seven of coding-rules** fail: no clause, no incident history, and not a single assertion in the
body). The item-by-item decisions and "same-cell coverage" are provided by the 0.6-11a decision
table (delete 21 / keep 5). This page does not restate the rationales; it records only the execution
result.

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
- **5 tests kept** (the decision table could not identify replacement coverage, so they were kept as
  required): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is a single commit (this page and `test-gate-census.tsv` are in the
  same commit); `git revert <sha>` rolls back the entire batch.

## Rerun and Self-Test

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
python scripts/dev/test-gate-census-audit.py --sample  # Reproduce the 40-item sample (seed 20261106)
python scripts/dev/test-gate-census-audit.py --check   # Whether the audit data file covers every sample
```

The generator performs only lexical classification, and states its boundaries accurately:

- **Delegated assertions are identified only by name** (`assert*`/`check*` helper functions). If a
  test's assertion is hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as "no assertions"—all 26 tests in the first batch (21 retired / 5 kept) had their
  function bodies manually reviewed one by one.
- **The proof gate is not adjudicated**: any test that fails both gates is classified only as
  "pending judgment" or "retirement candidate"; the generator never declares that a test "should be
  deleted."
- **No cross-file deduplication**: exactly duplicated tests are not identified in this table (0.6-2
  measured them at only 1%, so they do not constitute a retirement surface).
- **The audit covers only 40 tests**: its conclusions classify only these 40 tests and provide a
  **proportion** for the 2403-test pool; no individual decision may be extrapolated to other tests
  in the same directory.
