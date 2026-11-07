---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a clause-gate sampling audit and the first retirement candidate list'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)

The prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **three gates (clause / incident history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a rerunnable item-by-item decision
table. Then perform a stratified sampling audit (40 items) of the clause gate, measure the
proportion of tests that “cite a clause but cannot support their assertions,” and recalculate the
distribution using the **tightened criteria**. **This page and the generator itself only read tests
and do not modify them.** Test retirement is performed in batches (one commit per batch, with the
whole batch revertible via `git revert`), and execution results are recorded in the “First
Retirement Batch / Second Retirement Batch / Third Retirement Batch” sections below and in `09a`.

Generated artifacts:

| Artifact                                  | Purpose                                                                                                                                    |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `docs/src/dev/test-gate-census.tsv`       | Item-by-item decision table (3411 rows plus the header), in the same directory as this page                                                |
| `scripts/dev/test-gate-census.py`         | Generator (rerunnable, with `--check` and `--selftest`)                                                                                    |
| `scripts/dev/test-gate-census-audit.py`   | Sampling audit: reproduces the 40-item sample with a fixed seed (`--sample` / `--check`)                                                   |
| `scripts/dev/test-gate-census-audit.json` | Item-by-item decisions and rationales for the 40 samples (the generator reads it to classify “the audit confirms genuine support” as keep) |

## Definitions

### Scan scope

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

### Machine criteria for the three gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                           | Meaning of a hit                                           |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | A **verifiable item** appears in the first 25 lines of the file header: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); for files under `src/**/tests/`, the parent `mod.rs` is inspected as well                                                                                                                                                                                                                | The test can be mapped to an identifiable clause           |
| Incident-history gate             | The test body, function doc comment, or **file header** (`//! Regression context: issue #226`, for example) identifies a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                          | The test pins an incident that actually occurred           |
| Proof gate                        | **The machine cannot determine this** (“Would it turn red if replaced with another correct implementation?” requires human judgment). Therefore, anything that fails the first two gates is classified as “pending judgment,” never automatically as “neither passes.” The sole exception is a test with **no assertions at all in its body** (including no delegation to `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only “pending judgment” or “no assertions” can be assigned |

Rationale for tightening: The 0.6-9 clause gate only required the file header to **contain
`§`/`RFC`/`SPEC`**, so 2403 tests were classified as “keep.” The 40-item stratified sample in 0.6-10
found that **half of the citations could not support the assertions** (see the next section). After
tightening, only section-number forms are accepted, and **having a citation still does not mean that
the citation supports the assertion**—an unaudited clause citation can only reach “pending
verification.”

Rationale for the file-header radius: This repository conventionally writes “Regression context:
issue #N” in the file header and does not repeat it in individual test comments. Looking only at
each test’s own comments would miss the history of an entire file (measured: 7 tests in the test’s
own radius versus 291 in the file-header radius).

### Categories never judged for retirement

The following tests are **always “keep”** and never enter the retirement-candidate pool:

- The `tests/yaoxiang/**` corpus and `tests/project-corpus/**` project shapes (behavioral criterion
  surfaces);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and across process boundaries);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition semantics

| Disposition          | Meaning                                                                                                                                                                                                           |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Only three cases: the incident-history gate is hit; the proof gate applies (spec-shaped directory / current-state-anchor name); or **the sampling audit confirms genuine support** for a clause citation          |
| Pending verification | A clause is cited (with a section number, or only an entire RFC without a section number), but it has not been audited / the audit judged it irrelevant or too broad—a human must verify it before it can be kept |
| Retirement candidate | Both machine-adjudicable gates fail and the assertion surface contains only weak assertions (or none)—a candidate is not a verdict; the proof gate still requires human review                                    |
| Pending judgment     | Both machine-adjudicable gates fail but strong assertions are present, so the machine assigns no conclusion                                                                                                       |

## Distribution (after tightening)

| Gate                                                                              | Count    |
| --------------------------------------------------------------------------------- | -------- |
| Clause (with a verifiable item)                                                   | 1713     |
| Incident history                                                                  | 313      |
| Proof (spec-shaped criterion surface / current-state anchor)                      | 447      |
| Pending judgment (both machine-adjudicable gates fail; proof awaits human review) | 924      |
| Neither passes (no assertions in the body)                                        | 11       |
| **Total**                                                                         | **3408** |

| Disposition          | Count    | Compared with 0.6-9                                                                                                                                                                                      |
| -------------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | **775**  | 2897 (−2122)                                                                                                                                                                                             |
| Pending verification | 2121     | 0 (new tier)                                                                                                                                                                                             |
| Pending judgment     | 414      | 412 (−1 after the second retirement batch, +3 from the 0.6-15 criterion correction)                                                                                                                      |
| Retirement candidate | 98       | 136 (−21 from the first batch, −11 from the second, +1 from the generator correction, −2 from the third, −3 from the 0.6-15 criterion correction, −1 from the redundancy scan, −1 from the fourth batch) |
| **Total**            | **3408** | —                                                                                                                                                                                                        |

Two changes to the criteria need to be explained:

1. **Structure of “keep”**: 775 = 447 proof-gate tests + 313 incident-history-gate tests + 15 tests
   whose genuine support was confirmed by the audit (of the audit’s 20 genuinely supported tests,
   the other 5 were in files already kept by the incident-history gate). The 2121 tests in the
   original 2897 that were “kept based on a single citation” are now pending verification.
2. **Incident-history gate 47 → 313**: This does not represent newly discovered incident histories;
   it is a **relocation of labels**. In 0.6-9, a test that hit both the clause and incident-history
   gates was recorded as a clause test. After tightening, a clause hit no longer automatically means
   “keep,” so the incident-history gate must catch these tests, and its count includes them. The
   total number kept is therefore unaffected (those tests were also kept in 0.6-9).

File-level scope: Of 296 test files, **113** have a **verifiable section number** in the file header
(the old 0.6-9 criterion yielded 221).

## Clause-Gate Sampling Audit (0.6-10)

### Sampling criteria (reproducible)

- **Candidate pool = the 2403 tests classified as “clause” by the 0.6-9 clause gate** (frozen under
  the old criterion in `test-gate-census-audit.py`; the sampling denominator must not change with
  the new criterion, or the conclusions would not be comparable).
- **Strata = directory clusters**: use the first two levels under `src/` (`src/frontend`,
  `src/middle`, etc.), and group everything else by top-level directory (`tests`, `benches`,
  `tools`).
- **Allocation**: proportional to the number of tests in each stratum (largest-remainder method),
  with at least one test per stratum.
- **Extraction**: sort by (file, line number, test name) within each stratum, then take a random
  sample with a fixed seed.
- **Seed = 20261106** (hard-coded in the script; changing the seed changes the sample, so the change
  must be documented and the audit rerun).

Stratified result: `src/frontend` 21 · `tests` 4 · `src/driver` 3 · `src/package` 3 · `src/backends`
2 · `src/formatter` 2 · `src/middle` 2 · `src/util` 2 · `tools` 2 · `src/ast` 1 · `src/lsp` 1 ·
`src/std` 1.

### Item-by-item decisions for the 40 samples

Decision criteria (fixed here to prevent inconsistent judgments):

- **Genuine support**: The citation identifies a specific section / note / decision number, and its
  **topic** is the same as the assertion target;
- **Irrelevant**: The citation identifies a specific item, but that item concerns **something else**
  (the rationale states what it actually covers);
- **Broad**: The citation refers only to an entire document / chapter / list of sections / phase
  name, and no item corresponds to the assertion.

| #   | Test                                                                        | File                       | Decision        | Rationale (what the cited clause actually covers)                                                                                                                                                                                  |
| --- | --------------------------------------------------------------------------- | -------------------------- | --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `test_compile_time_value_param_spec_table`                                  | `classify.rs`              | Genuine support | type-system §7.1 “compile-time value parameters” is exactly the assertion target: the truth table for distinguishing value-position from type-position parameters is verified row by row.                                          |
| 2   | `test_id_stable_under_subsequent_registrations`                             | `value.rs`                 | Genuine support | RFC-010 “runtime representation and equality” defines sum-type identity as being determined only by the name; the assertion verifies that an ID does not change after subsequent registrations.                                    |
| 3   | `test_driver_run_check_rootless_multi_file_well_formed_generic_stays_clean` | `check_rootless_mono.rs`   | Genuine support | 02-stage-contract §3, note #14, point ③ discusses common monomorphization naming; the test is the pinning test for that note (both the name and the assertion match).                                                              |
| 4   | `test_multifile_benign_import_cycle_message_predicts_no_runtime_failure`    | `import_cycle.rs`          | Genuine support | RFC-013’s W1xxx warning code + user adjudication A (cycles must not go unreported); the assertion verifies that a benign cycle produces exactly one W1007 and predicts no runtime consequences.                                    |
| 5   | `test_obligation_config_disabled_skip_is_not_owed`                          | `obligations.rs`           | Genuine support | 02-stage-contract §2 “obligation ledger”; the assertion verifies that a ConfigDisabled skip does not create an outstanding obligation.                                                                                             |
| 6   | `test_format_type_bool`                                                     | `types.rs`                 | Genuine support | formatter spec §9, type annotations; the assertion verifies that the type name is emitted verbatim.                                                                                                                                |
| 7   | `test_fstring_multiline_single_quotes_inside`                               | `fstring.rs`               | Genuine support | syntax §1.6.3, string literals; the assertion verifies that a single quote inside multiline f-string content does not terminate it.                                                                                                |
| 8   | `test_float_underscore`                                                     | `literals.rs`              | Genuine support | syntax §1.6, literals (1.6.2 floating-point numbers), covers underscore separators; the assertion verifies that 1_000.5 is tokenized as 1000.5.                                                                                    |
| 9   | `test_empty_unicode_escape_is_error`                                        | `literals.rs`              | Genuine support | syntax §1.6.3, the closed set of escapes; the assertion verifies that an empty \u{} escape must produce an error.                                                                                                                  |
| 10  | `test_probe_accepted_const_type_is_registered`                              | `probe.rs`                 | Genuine support | RFC-039 P6 6.4 / WBS 6.6.9 states that “constant-parameter determination obeys only the injected probe”; the assertion verifies that N is registered when the probe accepts Int.                                                   |
| 11  | `test_direct_eval_with_bound_variable_proved`                               | `predicate.rs`             | Genuine support | RFC-027 §3-4 (path-condition propagation / compile-time proof pipeline); the assertion verifies that, when b=5, b>0 evaluates directly to Proved.                                                                                  |
| 12  | `test_unused_type_is_not_an_entry_point`                                    | `dead_code.rs`             | Genuine support | RFC-011 §7, dead-code elimination mechanism; the assertion verifies that an unused type definition is not a reachable root.                                                                                                        |
| 13  | `test_display_forall_multiple_vars_no_trailing_space`                       | `ast.rs`                   | Genuine support | RFC-027 §8, SMT solver; the assertion verifies the printing format for a forall with multiple variables.                                                                                                                           |
| 14  | `test_dep_graph_remove_dependant_clears_edges`                              | `rfc027_phase31_dep_vc.rs` | Genuine support | RFC-027 Phase 3.1’s TypeDepGraph; the assertion verifies that removing a dependent clears its edges and constraints together.                                                                                                      |
| 15  | `test_condition_bool`                                                       | `conditional.rs`           | Genuine support | RFC-011 §5, conditional types; the assertion verifies the evaluation and determinate state of TypeCondition::Bool.                                                                                                                 |
| 16  | `test_convert_expr_to_const_expr_int_literal`                               | `const_eval.rs`            | Genuine support | RFC-011 §4, compile-time generics; the assertion verifies that an Int literal is converted to ConstExpr::Lit.                                                                                                                      |
| 17  | `test_definition_on_binding_site_returns_none_precise_only`                 | `definition.rs`            | Genuine support | RFC-039 D55 (precise-only, prohibiting name-based fallback); the assertion verifies that returning None when the definition site itself has no reference record.                                                                   |
| 18  | `test_type_consistent_arith_passes`                                         | `verify.rs`                | Genuine support | WBS 2.1 / RFC-039 P2’s criterion for the IR static validator (the first layer of 07-equivalence-oracle); the assertion verifies that type-consistent arithmetic passes.                                                            |
| 19  | `test_verify_loose_corpus_green`                                            | `verify.rs`                | Genuine support | The same validator criterion; the assertion verifies that the corpus passes in its entirety.                                                                                                                                       |
| 20  | `test_parse_tag_tolerates_query_and_fragment`                               | `toolchain.rs`             | Genuine support | RFC-037 § installer support; the assertion verifies that the query/fragment are stripped from the release URL.                                                                                                                     |
| 21  | `test_release_unset_register`                                               | `execute.rs`               | Irrelevant      | #299 §2 discusses the NewArray fixed-length array instruction (allocation, Void placeholder, bytecode round-trip); the assertion is that “executing Release on an unset register does not panic.”                                  |
| 22  | `test_fstring_segment_interpolation`                                        | `ast.rs`                   | Irrelevant      | It cites “language spec §3 (AST structure),” but syntax §3 is the statements chapter (3.1 statement classification … 3.11 program entry and top-level statements); the assertion concerns the FStringSegment::Interpolation field. |
| 23  | `test_expr_for`                                                             | `ast.rs`                   | Irrelevant      | It cites the same “§3 (AST structure)” reference; syntax §3 discusses statements, while the assertion concerns the AST fields of Expr::For.                                                                                        |
| 24  | `test_add_var_without_mut_stores_immutability`                              | `scope.rs`                 | Irrelevant      | It cites “language spec §5.2”: syntax has no fifth chapter, and type-system §5.2 is “multiple constraints”; the assertion concerns the immutability flag stored by ScopeManager.                                                   |
| 25  | `test_type_evaluator_eval_nested_type`                                      | `evaluator.rs`             | Irrelevant      | It cites syntax §3.11 (program entry and top-level statements) and “RFC-027 §362” (which does not exist); the assertion concerns the evaluation of nested types.                                                                   |
| 26  | `test_get_ast_type_universe_level_plain`                                    | `mono.rs`                  | Irrelevant      | It cites “language spec §3,” but syntax §3 is the statements chapter; the assertion verifies that the type-universe level is 0.                                                                                                    |
| 27  | `test_unify_enum_same`                                                      | `solver.rs`                | Irrelevant      | It cites “language spec §3”; the assertion verifies that unifying same-name enums succeeds, which belongs to type unification rather than the statements chapter.                                                                  |
| 28  | `test_weak_new_upgrade_some_while_original_alive`                           | `weak.rs`                  | Irrelevant      | It cites RFC-010’s **sum-type identity**, but the assertion concerns the lifetime of a weak reference (#384: while the original Arc is alive, upgrade yields Some).                                                                |
| 29  | `test_i18n_registry_english_titles`                                         | `codes.rs`                 | Irrelevant      | §3.4’s I3/I4 discuss translations of E1090/E1091; the assertion verifies that E0001’s English title and help text are nonempty.                                                                                                    |
| 30  | `test_format_return_expression`                                             | `expr.rs`                  | Broad           | The citation is a list of sections §3, §4, §7, §8, §10–§13, §17, and §18, none of which corresponds to “how a return expression is rendered.”                                                                                      |
| 31  | `test_rfc010_generic_syntax`                                                | `lexer_mod.rs`             | Broad           | The citation consists of syntax §1.6 plus the generic references RFC-004/RFC-010; the assertion describes itself as flexible (allowing arbitrary parenthesization), with no verifiable item.                                       |
| 32  | `test_anon_binding_with_fn_annotation_keeps_extracted_signature`            | `types.rs`                 | Broad           | “spec §3, RFC-010, RFC-011” is a generic list, and no item corresponds to “signature extraction for an annotated anonymous-function binding.”                                                                                      |
| 33  | `test_consumer_unknown_token_returns_empty`                                 | `ownership.rs`             | Broad           | “RFC-009 + RFC-009a” lists entire documents; the assertion verifies that BrandTree returns an empty consumer set for an unknown token.                                                                                             |
| 34  | `test_const_expr_neg_int_literal_displays_by_value`                         | `const_data.rs`            | Broad           | RFC-011 §4 discusses compile-time generics and does not concern the printing form of ConstExpr; the assertion verifies that a negative literal prints as -5.                                                                       |
| 35  | `resolve_local_and_method`                                                  | `resolver.rs`              | Broad           | “RFC-029 Refactoring Core” is an entire architecture RFC; the assertion concerns the resolver’s qualified names and DefKind.                                                                                                       |
| 36  | `test_outdated_exec_smoke`                                                  | `outdated.rs`              | Broad           | RFC-014a is only background in a comment (the registry has not been published, so a path dependency is used); the assertion is an exec smoke test, with no assertions in the body.                                                 |
| 37  | `test_compatible_high_major_range`                                          | `resolver.rs`              | Broad           | “RFC-014 Phase 3” is a phase name, not an item; the assertion verifies version-range compatibility determination.                                                                                                                  |
| 38  | `test_clean_and_size`                                                       | `cache.rs`                 | Broad           | “RFC-014 Phase 3 (global cache design)” is likewise a phase name; the assertion verifies cache clean and dir_size.                                                                                                                 |
| 39  | `test_strip_ansi_multiple_consecutive_sequences`                            | `ansi.rs`                  | Broad           | §4.2 is the “ANSI utility extraction” work item, not a specification clause; the assertion concerns strip_ansi’s behavior for consecutive sequences.                                                                               |
| 40  | `test_verify_sha256_accepts_matching_digest_file_content`                   | `dl.rs`                    | Broad           | It cites “RFC-037: version directory = release-package extraction root directory”; the assertion verifies that sha256 verification accepts a matching digest file.                                                                 |

Distribution: **20 genuine support · 9 irrelevant · 11 broad**.

### Mis-citation rate and confidence interval

- Mis-citations (irrelevant + broad) = 20/40 = **50%**;
- Wilson 95% confidence interval (n=40): **[35.2%, 64.8%]**;
- Extrapolated to 2403: approximately **1202 tests** genuinely have clause support (95% interval
  **846–1557**), and approximately **1201 tests** are mis-cited (interval 846–1557).

### Impact on the cleanup surface

- **“Keep” fell from 2897 to 775** (−2122)—exactly the 2122 tests “kept based on a citation alone”
  moved to pending verification. The tightening took effect; this is not distribution tuning.
- **Retirement candidates are unrelated to clauses**: the first batch of 26 was already unrelated to
  clauses (not one assertion in the body), and the audit does not change their decisions. The first
  batch retired 21 tests, reducing candidates from 136 → 115; the second batch retired 11, reducing
  them from 115 → 104; after the 0.6-13 generator fix (function-body boundaries are no longer
  truncated by braces inside literals), +1, candidates 104 → 105; the third batch retired 2,
  candidates 105 → 103; after the 0.6-15 criterion fix (helper names containing digits / the
  `match` + `panic!` form are recognized as assertions), 3 tests moved out of “neither passes,”
  candidates 103 → 100. The second batch of 11 is analogous: the bodies **do** contain assertions,
  but only weak assertions such as `is_some()` / `is_none()`, which do not pin binding strength or
  the parse result. This also shows that the “retirement surface” and the “mis-cited-clause surface”
  are separate problems: most mis-cited tests have assertions and should receive a clause or have
  their references corrected rather than be deleted.
- The 2122 pending-verification tests represent the future workload for “backfilling clause-gate
  tickets”: either add a specific item that corresponds to the assertion or reclassify the test as a
  retirement candidate.

### Limitations (do not cite beyond scope)

- The 40 samples can provide only a **proportion**; no individual decision may be extrapolated to
  other tests in the same directory;
- The decisions were made by manually reading the code and the clause documents. The rationale
  column states “what the cited clause actually covers” for review;
- The sampling denominator is the old 0.6-9 criterion pool (2403). The number of mis-citations among
  the **1714 tests remaining after tightening** was not resampled—tightening only excludes
  “unverifiable” citations; it does not mean that every citation in the new pool genuinely supports
  its assertion (each still requires individual verification).

## Reconciliation with the 0.6-2 Survey

The 0.6-2 survey figures form the first comparison baseline. The reasons for each difference are
listed below:

| Item                            | 0.6-2 Survey                  | This Census                                                                                                 | Reason for the Difference                                                                                                                                                                                                                                                   |
| ------------------------------- | ----------------------------- | ----------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 3411                                                                                                        | Two tests were added after 0.6-2 (the check-criterion regression tests for E3005 and E3022), then 21 no-assertion tests were deleted according to the first batch, 11 weak-assertion tests according to the second batch, and 2 tests according to the third batch; net −32 |
| Number of test files            | 296                           | 296                                                                                                         | Unchanged                                                                                                                                                                                                                                                                   |
| Clause gate                     | 227 files (77%)               | 113 files after tightening; 1714 by test                                                                    | The criterion was tightened from “the word appears” to “there is a section number `§<number>`,” so the file count was nearly halved                                                                                                                                         |
| Incident-history signal         | 442 tests / 148 files         | 313 tests in the gate; three radii: test itself 7 / file header 291 (38 files) / whole file 1218 (79 files) | The 148 files in 0.6-2 were broader than the 79 files in this census’s whole-file radius and used a broader vocabulary and scope; this census strictly uses the four defined signals.                                                                                       |
| Characterization strong signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                | Characterization is not one of the three gates; mixing it into the retirement conclusion would exceed the census’s authority                                                                                                                                                |
| Weak assertions                 | 473 (14%)                     | See the TSV “Assertion Summary” column                                                                      | The counting units differ: 0.6-2 counted the pattern “asserts only is_ok/is_some,” whereas this census uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                                       |

## Conclusion Summary

1. **The clause gate was previously significantly overstated**: under the old criterion, 2403 tests
   were classified as “clause/keep,” while the sample measured a 50% mis-citation rate (95% interval
   35%–65%). “Having a citation” and “having a citation that supports the assertion” are different
   things; the old criterion treated the former as the latter.
2. **Approximately 1202 tests genuinely have clause support after tightening** (846–1557); the rest
   must either await a backfilled clause or be judged through the incident-history/proof gates.
3. **The retirement surface is unrelated to clause mis-citations**: the 136 candidates were
   unrelated to clauses (no assertions / weak assertions). The first batch retired 21 tests, the
   second retired 11, the generator correction added 1, the third batch removed 2, the 0.6-15
   criterion correction removed 3, the redundancy scan removed 1, and the fourth batch removed 1,
   reducing the candidates to 98.
4. **The incident-history gate is the largest legitimate source of “keep”** (313 tests): it
   demonstrates that “pinning a real incident” is more reliable than “copying an RFC sentence.”

## First Retirement Batch (Executed 2026-11-06)

**First batch: of the 26 “no assertion” tests in `src/frontend/core/types/eval/tests/`, 21 were
retired and 5 were kept** (a single directory cluster; criterion = **all three gates fail**
according to Part Seven of coding-rules: no clause-gate hit, no incident-history-gate hit, and no
assertions anywhere in the body). The item-by-item decisions and “same-cell coverage” are provided
by the 0.6-11a decision table (21 deleted / 5 kept); this page does not repeat the rationales and
records only the execution results.

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
- **5 tests kept** (the decision table could not identify replacement coverage, so they were kept
  under the hard requirement): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls back the whole batch.

## Second Retirement Batch (Executed 2026-11-06)

**Second batch: of the 18 “weak assertion” tests in `src/frontend/core/parser/pratt/tests/nud.rs`,
11 were retired and 7 were kept** (a single directory cluster = the pratt prefix-parsing module;
criterion = **all three gates fail** according to Part Seven of coding-rules: no clause-gate hit, no
incident-history-gate hit, and the body contains only weak assertions such as `is_some()` /
`is_none()`—neither pinning binding strength nor the parse result).

The difference between this batch and the first needs to be made explicit: these tests **do contain
assertions** and are not empty tests. The reason for retirement is “all three gates fail + zero
criterion increment,” not “no assertions.” Every test selected for deletion identifies a
**unit-level replacement-coverage test in the same module**.

- **11 tests retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Every replacement
  follows the **same** `parse_expression → parse_prefix → prefix_info` path and has a stronger
  assertion (pinning the AST shape rather than merely checking `is_some`): `test_parse_int_literal`
  / `test_parse_float_literal` / `test_parse_string_literal` / `test_parse_bool_true` /
  `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` / `test_parse_return_expr`
  in the same file, plus `src/frontend/core/parser/tests/expressions.rs`’s `test_grouped_expr` /
  `test_tuple` / `test_list_literal` / `test_list_empty` / `test_ref_expr`.
- **7 tests kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the item-by-item rationales in the “VI. Second Test Retirement Batch”
  decision table in `09a-wbs-history.md`. There are two reasons for keeping them: **the proof gate
  applies** (the first three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are the
  sole unit-level observation points for their corresponding facts, and the repository has no second
  unit test that observes them), or **the incident-history gate applies** (`break_removed` /
  `continue_removed` explicitly name #314).
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls back the whole batch.

## Generator Correction (0.6-13)

**`body_span` truncates function bodies at braces inside literals — fixed.**

The old implementation located a function body by balancing braces without distinguishing real code
from braces in string / character literals or comments. Forms such as `tokenize("{")` are common in
Rust source, so the balance was thrown off and the function body was truncated: the body of
`test_prefix_info_lbrace` was once truncated at the end of the file, causing the “assertion summary”
to capture the remainder of the file (`matchesx11 assertx24 …`). It was consequently binned as
“pending judgment” instead of “retirement candidate”—**the wrong bin**.

Fix: add `mask_noncode` to replace the contents of string literals / character literals / line
comments / block comments (which may be nested) / raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (preserving newlines exactly, so line numbers and line counts do not change),
and only then balance the braces. **The mask is used only to locate function-body boundaries**;
assertion counting and the incident-history gate still read the original text, and the criteria
themselves have not been loosened. `--selftest` now includes four cases (paired / unpaired braces in
ordinary strings, braces in raw strings and block comments, and braces in character literals), each
followed by a sentinel test—the old implementation found the wrong boundary in all four cases, while
the new implementation finds all four correctly.

### Tests affected by the same bug: 11

Counting method: compare the **function-body boundaries before and after the fix** test by test,
file by file (old implementation: balance braces directly; new implementation: balance after
masking). Tests with different boundaries are affected. Of 3413 tests in the repository, the old
implementation found the wrong boundary for **11**:

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

### Bin differences before and after the fix: 2

For the other nine, **eight only changed the “assertion summary” column** (their bin was unchanged),
and **one did not change even the summary** (`test_fstring_close_brace_escape_preserved_raw`: the
old implementation omitted the closing `}` line, which contained no assertion terms). The eight
summary changes are listed individually in the corresponding section of `09a-wbs-history.md`.

| Test Name                   | File                                          | Old Bin                             | New Bin                                     | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --------------------------- | --------------------------------------------- | ----------------------------------- | ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending judgment / Pending judgment | Pending judgment / **Retirement candidate** | The real function body contains only one weak assertion, `assert!(state.prefix_info().is_some())`; the file header and function body contain neither a § section number nor issue/regression/defect/incident terms ⇒ both the clause gate and incident-history gate fail, and the body contains only a weak assertion such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires human review)                                         |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident history / Keep             | Clause / **Pending verification**           | The old implementation truncated the body at the end of the file and swallowed text containing “regression / defect / incident,” falsely triggering the incident-history gate; the real function body contains no such terms ⇒ the incident-history gate no longer applies; the file header contains verifiable items (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate is hit, but the sampling audit did not confirm genuine support ⇒ pending verification |

The distribution consequently changes: clause 1713 → 1714 · incident history 314 → 313 · keep 776 →
775 · pending verification 2121 → 2122 · pending judgment 412 → 411 · retirement candidate 104
→ 105. **The repository-wide `#[test]` count is unchanged (3413)**—this round changed only the
implementation of the criteria, not any test.

## Third Retirement Batch (Executed 2026-11-07)

**Third batch: of the 5 retirement candidates in `src/middle/passes/codegen/tests/`, 2 were retired
and 3 were kept.** (a single directory cluster = the `middle::passes::codegen` module under test;
criterion = **all three gates fail** according to Part Seven of coding-rules.)

This batch continues the **retention criterion** established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus / integration tests belong
to the process-boundary layer and cannot replace a unit-level observation point.

- **2 tests retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—its function body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (apart from the
    function name): both are `run_fixture()` + `assert_slot_is_42(&interp)`. Its doc comment claims
    to verify that “the `execute_module` entry path returns non-Void,” but the body never observes
    the return value of `execute_module` (`run_fixture` discards it via `.expect(...)`)— the claimed
    intent is not implemented, and the function body is a pure duplicate ⇒ zero criterion increment.
    Replacement coverage = `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file strictly covers it:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` has the same shape as the inline `Symbol` in this
    test). “A name that was never inserted cannot be found” and “a name cannot be found after it is
    popped” follow the same path in `SymbolScopeManager::lookup` (walk each `scope.get(name)` layer,
    returning `None` when not found) ⇒ this test adds zero coverage. Replacement coverage =
    `test_scope_nesting`.
- **3 tests kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that does
    “not use serialization and directly executes” (the serialization variant is a different path),
    and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization round trip (`write_to` → `read_from` → execute).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit-level observation point for the
    `id > 255` rejection branch of `OperandResolver::to_reg` (the bytecode layer has independent
    overflow checks and does not cover this function; no second test in the repository makes
    `to_reg` go out of range). `is_err()` is a discriminating assertion (removing the guard would
    turn the test red), not merely “proof that it did not panic.”
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls back the whole batch.

### One census blind spot found in this batch (**fixed in 0.6-15**, see “Generator Criterion Correction” below)

The census’s “delegated assertion” recognition pattern `\b(assert|check)[A-Za-z_]*\s*\(` **does not
match helpers whose names contain digits**: `[A-Za-z_]*` does not include digits, so a call such as
`assert_slot_is_42(&interp)` **is not recognized as an assertion**— 2 of the 5 candidates in this
batch were consequently mis-binned as “no assertions anywhere in the body.”

After reviewing all 15 tests classified as “neither passes” item by item (broadening helper names to
include digits and additionally recognizing the `match` + `panic!` form), **4 tests do in fact
contain assertions**—3 use the numeric helpers above (`assert_slot_is_42`), and 1 is
`test_weak_from_arc` in `src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form;
test-specification rule 6.1 requires `assert!(matches!(...))`, but the test does in fact assert).
The remaining 11 tests genuinely contain no assertions.

**Acting directly on the machine’s “neither passes” classification would have deleted these 4 tests
even though they contain criteria.** The item-by-item human review in this batch prevented that; the
generator-side fix is described in “Generator Criterion Correction (0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots have been fixed: helper names containing digits, and the `match` +
`panic!` form.** This round **does not delete any test** (the repository-wide `#[test]` count
remains 3411); it only changes how the generator recognizes “whether the body contains assertions.”

### What was fixed

1. **The name portion of delegated-assertion recognition now permits digits**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern did not
   match helpers such as `assert_slot_is_42(&interp)`, so tests containing such calls were treated
   as having “no assertions anywhere in the body.”
2. **Added the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the assertion is written
   in one of the arms, so no `assert!` macro is visible lexically (test-specification rule 6.1
   requires `assert!(matches!(...))`, but existing code contains this form). Detection method: the
   `match` keyword and the first `{` (while scanning the expression) open a block, whose boundary is
   determined by **brace balancing**; the form counts only if `panic!(` appears inside it.
   **Detection operates on masked lines**— braces in strings and comments do not participate in
   balancing, and `panic!` in a comment is not treated as an assertion.

### Bin differences: 3 tests (both gate and disposition changed)

| Test Name                                           | File                                           | Old Bin                               | New Bin                             | Rationale                                                                                                                                                                                                                                                    |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither passes / Retirement candidate | Pending judgment / Pending judgment | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending judgment / Pending judgment | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern did not match; it is now recognized as an assertion                                                                                       |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending judgment / Pending judgment | Same as above (the serialization-round-trip test)                                                                                                                                                                                                            |

A further **77 tests changed only the “assertion summary” column** (the gate and disposition were
unchanged): most gained `match_panicxN` in the summary (the body contains the `match` + `panic!`
form), and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs` has 3, and
`src/package/tests/cache.rs` has 1—helper names containing digits).

**The relaxation changes only the “neither passes” tier**: that tier is the machine’s **confident
verdict** (no assertions anywhere in the body), and its criterion is a hard requirement; “retirement
candidate” is only a pool awaiting human review, so the size of the pool is not a verdict. All other
bins, including the weak-assertion pool, continue to use the previous criteria. Result: gate—neither
passes 14 → 11 · pending judgment 923 → 926; disposition—retirement candidate 103 → 100 · pending
judgment 411 → 414.

### Item-by-item review of the 11 remaining “neither passes” tests

After reading the original source of all 11 tests classified as “neither passes,” **all 11 genuinely
contain no assertions** (including no `assert*` / `check*` helpers, no `match` + `panic!` form, and
no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                     | Review Conclusion                                           |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | Five `.expect(...)` calls that merely guarantee it does not crash                               | Genuinely no assertions (proves only that it did not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains a `context()` handle and discards it                                                    | Genuinely no assertions                                     |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer`s, obtains their contexts, and discards them                       | Genuinely no assertions                                     |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Genuinely no assertions                                     |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Genuinely no assertions                                     |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                              | Genuinely no assertions                                     |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` with invalid content and discards the result                      | Genuinely no assertions (proves only that it did not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Genuinely no assertions (proves only that it did not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Genuinely no assertions (proves only that it did not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                            | Genuinely no assertions                                     |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertions                                     |

Five of these (`test_normalizer_context` and the four in reducer.rs) were kept because the 0.6-11a
decision table **could not identify replacement coverage**; this review reaches the same conclusion.

### Reverse spot-check: Did the relaxation go too far?

Five tests were selected with a fixed seed (20261107) from the 88 items “newly recognized as
assertions,” and their source was read to confirm that they really assert:

| Spot-checked Item                                           | File                                                 | Assertion in the Source                                                                                                                              |
| ----------------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | The matching arm contains two `assert_eq!`; the non-matching arm uses `panic!("Expected AssocType")`                                                 |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | The matching arm contains `assert!(msg.contains("src/evil.yx"))`; the non-matching arm uses `panic!("expected InvalidPackage, got …")`               |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | The matching arm uses `assert_eq!` to compare constraint types; the non-matching arm uses `panic!("不是 Dynamic 变体…")`                             |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | The matching arm contains `assert!(!delta.edits.is_empty())`; the non-matching arm uses `panic!("Expected TokensDelta variant")`                     |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | The matching arm contains three `assert_eq!` / `assert!` checks of the error code and message; the non-matching arm uses `panic!("不是 Disproved…")` |

All five assert a concrete value or variant (replacing the implementation with another correct path
would turn them red)—**the relaxation did not go too far**. The reverse controls were also added to
`--selftest`: a `match` with no `panic!`, and a `panic!` that appears only in a comment; **neither
may** be recognized as an assertion.

### The criteria have teeth (developmental proof)

Revert the generator to the old implementation (`HELPER_RE` without digits + `match` + `panic!`
recognition disabled), and the newly added `--selftest` cases all turn **red**, with exit code 1;
the new implementation passes them all.

### Rollback

The generator and census table are in the same commit, so `git revert <sha>` restores the entire
state.

## Fourth Retirement Batch (Executed 2026-11-07)

**Fourth batch: of the 10 “retirement candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend’s test directories), 1 was retired and 9 were
kept** (a single directory cluster, with all three files read before making any decisions: 4 in
`tests/bytecode_load.rs` · 2 in `tests/weak.rs` · 4 in `executor/tests/debug.rs`). Criterion = the
**three gates** according to Part Seven of coding-rules; retention criterion = “a unit-level
observation point can only be replaced by another unit test; corpus/integration tests belong to the
process-boundary layer”; **deletion requires identifying a specific replacement-coverage test; if no
replacement can be identified or the test cannot be understood, retain it**.

- **1 test retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, followed by one step
  and retrieval of `capture_stack`), and its assertion is **strictly stronger**: it asserts that the
  stack is nonempty and that `stack[0].function_name == "main"` (pinning the top-of-stack position),
  whereas the listed test only asserts `stack.iter().any(|f| f.function_name == "main")` (any
  position is sufficient) —the former being true implies the latter ⇒ zero criterion increment.
- **9 tests kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each test, each is the
  **sole unit-level observation point** for a property (the load magic-number/version branches are
  distinct, the YXBC routing layer, the Some/None sides of Weak, stepping with no frame, step_out
  popping the stack, and running to completion with an empty stack), and no replacement unit test
  can be identified ⇒ retain under the hard requirement. See Section Eleven of
  `docs/src/dev/architecture/09a-wbs-history.md` for the item-by-item rationales (the decision table
  is kept in the transcript rather than on this page).
- **Numbers**: Repository-wide `#[test]` 3409 → 3408; one deletion = `git diff` counts one
  `-#[test]` line and zero `+#[test]` lines; the dominating test
  `test_capture_stack_during_execution` passes when run alone. Census: retirement candidate 99 → 98
  · pending judgment 925 → 924 · clause 1713 · incident history 313 · proof 447 · neither passes 11
  · pending verification 2121 · keep 775.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls back the whole batch.

## Rerun and Self-Check

```bash
python scripts/dev/test-gate-census.py            # Recalculate and overwrite the TSV (two runs are byte-for-byte identical)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: check whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
python scripts/dev/test-gate-census-audit.py --sample  # Reproduce the 40-item sample (seed 20261106)
python scripts/dev/test-gate-census-audit.py --check   # Check whether the audit data file covers every sample
```

The generator performs lexical determination only, and states its boundaries honestly:

- **Delegated assertions are identified by name only** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name does not have one of these prefixes, it is
  misclassified as “no assertions”—the first batch of 26 (21 retired / 5 kept) and the second batch
  of 18 (11 retired / 7 kept) were all manually reviewed function by function.
- **Function-body boundaries (fixed in 0.6-13)**: The generator balances braces when locating a
  function body, but braces in string / character literals and comments skew the balance and
  truncate the body—`test_prefix_info_lbrace` (which contains `tokenize("{")`) was once truncated at
  the end of the file, causing the “assertion summary” to capture the rest of the file and the test
  to be binned as “pending judgment” instead of “retirement candidate.” The generator now masks
  non-code regions (strings / character literals / line comments / block comments / raw strings)
  before balancing the braces; see “Generator Correction” below.
- **Delegated-assertion recognition (fixed in 0.6-15)**: The old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` does not include digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions, and tests containing such calls
  were misclassified as having “no assertions anywhere in the body.” At the time, 4 of the
  repository’s 15 “neither passes” tests actually contained assertions (3 numeric helpers + 1
  `match` + `panic!` form). The name portion is now `[A-Za-z0-9_]*`, and the `match` + `panic!` form
  is recognized; see “Generator Criterion Correction” below.
- **The proof gate is not adjudicated**: Any test that fails both machine-adjudicable gates is
  classified only as “pending judgment” or “retirement candidate”; the generator never announces
  that it “should be deleted.”
- **No cross-file deduplication**: Exactly duplicated tests are not identified in this table (0.6-2
  measured only 1%, which does not constitute a retirement surface).
- **The audit covers only 40 tests**: Its conclusions classify only those 40 tests and provide a
  **proportion** for the 2403-test pool; no individual decision may be extrapolated to other tests
  in the same directory.
