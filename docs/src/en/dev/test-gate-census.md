---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement-candidate list and retirement records for each batch'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **three gates (clause / incident history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), producing a repeatable item-by-item decision table.
A sample audit of the clause gate (40 items) measured the proportion of cases where “a clause was
cited but could not support the assertions” and, on that basis, **tightened the criteria**; this
audit was abolished and removed in 0.6-19. **This page and the generator itself are read-only and do
not modify tests**; test retirement is performed in batches (one commit per batch, with the whole
batch revertible via `git revert`), and execution results are recorded in the batch retirement
subsections and commit messages below.

Generated artifacts:

| Artifact                            | Purpose                                                                              |
| ----------------------------------- | ------------------------------------------------------------------------------------ |
| `docs/src/dev/test-gate-census.tsv` | Item-by-item decision table (2682 rows + header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                          |

## Methodology

### Scan Surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 2078                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2682** (335 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine Criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                | Meaning of a Match                                             |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | The first 25 lines of the file header contain a **verifiable clause entry**: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); for files under `src/**/tests/`, inspect the parent `mod.rs` as well                                                                                                                                                                                                                     | The test maps to a clause that can be named explicitly         |
| Incident-history gate             | The test function body, function doc comment, or **file header** (such as `//! Regression background: issue #226`) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                             | The test pins an incident that has actually occurred           |
| Proof gate                        | **Cannot be determined by machine** (“If replaced with another correct implementation, would it fail?” requires human judgment). Therefore, every test that fails both preceding gates falls under “Pending adjudication” and is never automatically classified as “Neither gate passes”; the sole exception is a body with **not a single assertion** (including no `assert*`/`check*` helper delegation, no `match` + `panic!` form, and no `#[should_panic]`) | Only “Pending adjudication” or “No assertions” can be assigned |

Rationale for tightening: The 0.6-9 clause gate only required `§`/`RFC`/`SPEC` to appear in the file
header, so 2403 tests were classified as “Retain.” The measured result of the 40-item stratified
sample in 0.6-10 was that **half of the citations could not support the assertions** (mis-citation
rate: 50%, 95% interval: 35%–65%)—this measurement was the **direct reason for abandoning the rule
of retaining a test based on a single citation**. After tightening, only the section-number form is
accepted, and **a citation still does not mean that the citation supports the assertion**—all clause
citations are classified at most as “Pending verification.” (This sample audit was abolished and
removed in 0.6-19: after the test pool shrank, 38 items in the original sample no longer existed,
and `--check` consistently reported “missing 38, extra 38”; the conclusion is retained, but the
audit artifacts are no longer maintained.)

Rationale for the file-header radius: This repository’s convention is to write “Regression
background: issue #N” in the file header rather than repeat it in each test’s comments; looking only
at a test’s own comments would miss the history of the entire file (measured: test-own radius: 7
items; file-header radius: 291 items).

### Categories That Are Never Retired

The following tests are **always “Retain”** and do not enter the retirement-candidate pool:

- Corpus tests under `tests/yaoxiang/**` and project-shape tests under `tests/project-corpus/**`
  (behavioral-criteria surface);
- Tests under `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs`
  (spec-shaped and crossing a process boundary);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                                       |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Retain               | Only two reasons: the incident-history gate matched, or the proof gate applies (spec-shaped criterion surface / current-state anchor name)                                                                    |
| Pending verification | A clause is cited (with a section number, or only as an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a human before the test can be retained |
| Retirement candidate | Both gates fail, and the assertion surface contains only weak assertions (or none)—a candidate is not a verdict; the proof gate still requires manual review                                                  |
| Pending adjudication | Both gates fail, but strong assertions are present; the machine gives no conclusion                                                                                                                           |

## Distribution (Regenerated 2026-11-26; Rechecked During the 4.9.9 Narrowing Round; Batch 8.7 Rechecked 2026-11-27)

| Gate                                                                                    | Count    |
| --------------------------------------------------------------------------------------- | -------- |
| Clause (has a verifiable entry)                                                         | 1017     |
| Incident history                                                                        | 559      |
| Proof (spec-shaped criterion surface / current-state anchor)                            | 571      |
| Pending adjudication (neither of the first two gates passes; proof awaits human review) | 529      |
| Neither gate passes (no assertions in the body)                                         | 6        |
| **Total**                                                                               | **2682** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Retain               | **1130** | 737 (+393)                        |
| Pending verification | 1271     | 1054 (+217)                       |
| Pending adjudication | 223      | 203 (+20)                         |
| Retirement candidate | 58       | 61 (−3)                           |
| **Total**            | **2682** | 2055 (+627)                       |

Structure of “Retain”: 1130 = Proof gate 571 (spec-shaped criterion surface 518 + current-state
anchors 53) + incident-history gate 559. At file granularity: among 335 test files, the **clause
gate** covers **122 files**; a verifiable section number can be detected in the first 25 lines of
the file header (`src/**/tests*/`, including the parent `mod.rs`) in **166 files**.

Regeneration reconciliation: The previously committed table contained 2680 items, while the current
tree contains 2682; the current tree contains 2 items absent from the old table (the two wiring
self-checks in `parser/tests/wiring.rs`), the old table contains 0 items absent from the current
tree, and 0 same-key rows were recomputed. Per-key records are stored in the TSV.

### Two Criterion Changes at the 0.6-19 Tightening Point

1. **Structure of “Retain”**: 737 = Proof gate 424 (spec-shaped criterion surface 390 +
   current-state anchors 34) + incident-history gate 313. Of the original 2897 tests, the 2121
   classified as “Retain based on a single citation” now fall under Pending verification (after the
   sample audit was abolished in 0.6-19, the 9 items previously classified as “Retain because the
   audit confirmed genuine support” also moved to Pending verification—clause citations no longer
   have an “audit-confirmed” tier).
2. **Incident-history gate 47 → 313**: These are not new incidents, but rather **label
   reassignment**—in 0.6-9, tests that matched both the clause and incident-history gates were
   recorded under the clause gate. After tightening, a clause no longer automatically resulted in
   retention, so these tests had to be captured by the incident-history gate, and their count was
   folded into that gate. The total number retained was therefore unaffected (these tests were also
   retained in 0.6-9).

The file-level criterion at the time: among 285 test files, **136 files** had a verifiable section
number in their file headers (the old 0.6-9 criterion found 221).

## Reconciliation Between the 0.6-19 Snapshot and the 0.6-2 Survey

The 0.6-2 survey figures are the baseline for the first comparison. The reasons for each difference
are listed below:

| Item                              | 0.6-2 Survey                  | This Time                                                                                                 | Reason for the Difference                                                                                                                                                                                                                                                                    |
| --------------------------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`         | 3443                          | 2055                                                                                                      | 2 tests were added after 0.6-2 (check-criterion regression tests for E3005 and E3022), followed by the deletion of 21 no-assertion tests in the first batch, 11 weak-assertion tests in the second batch, 2 in the third batch, 1 in the fourth batch, and 1353 in the fifth batch; net −388 |
| Number of test files              | 296                           | 285                                                                                                       | 11 entire files were removed in the fifth batch                                                                                                                                                                                                                                              |
| Clause gate                       | 227 files (77%)               | 136 files after tightening; 808 items                                                                     | The criterion was tightened from “the token appears” to “there is a section number `§<number>`”, nearly halving the file count; the fifth batch retired 902 items, and after the sample audit was abolished in 0.6-19, another 9 moved from “Retain” to Pending verification                 |
| Incident-history signals          | 442 items / 148 files         | 313 items in the gate; three radii: test-own 7 / file header 291 (38 files) / entire file 1218 (79 files) | The 148 files in the 0.6-2 survey covered a wider scope than the current whole-file radius of 79 files and used a broader vocabulary and range; this time, the four signals on the gate card are followed strictly                                                                           |
| Characterization / strong signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                              | Characterization is not one of the three gates; mixing it into retirement conclusions would exceed the gate’s authority                                                                                                                                                                      |
| Weak assertions                   | 473 (14%)                     | See the “Assertion Summary” column in the TSV                                                             | The counting unit differs: 0.6-2 counted patterns that asserted only `is_ok`/`is_some`; this time, lexical counting is used and strong, weak, and delegated assertions are distinguished                                                                                                     |

## 0.6-19 Historical Conclusion Summary

1. **The clause gate had previously been significantly inflated**: Under the old criterion, 2403
   items were classified as “Clause/Retain”. The measured sample mis-citation rate was 50% (95%
   interval: 35%–65%)—“having a citation” and “having a citation that supports the assertion” are
   two different things; the old criterion treated the former as the latter. (This sample audit was
   abolished and removed in 0.6-19. Its conclusion is retained; see “Abolition of the Sample Audit”
   below for the reason.)
2. **Approximately 1202 items genuinely have clause support after tightening** (range: 846–1557);
   the remainder must either obtain Pending verification or were already meant to be judged by the
   incident-history or proof gate.
3. **The retirement surface is unrelated to the clause mis-citation surface**: The 136 candidates
   had no clauses (no assertions/weak assertions); the first batch retired 21, the second batch
   retired 11, the generator correction added 1, the third batch removed 2, the 0.6-15 criterion
   correction removed 3, the redundancy scan removed 1, the fourth batch removed 1, and the fifth
   batch removed 1353 by mechanically applying the rule “for the same symbol and result category,
   retain only the strongest item,” reducing the candidates to 61. The fifth batch’s retirement
   count by gate was: Clause 902 · Pending adjudication 428 · Proof 23 · Incident history 0 ·
   Neither gate passes 0—**not a single item on the incident-history gate or the zero-assertion line
   was lost**.
4. **The incident-history gate is the largest legitimate source of retained tests** (313 items): it
   demonstrates that “pinning an actual incident” is more reliable than “copying a line from an
   RFC.”

## First Retirement Batch (Executed 2026-11-06)

**First batch: of 26 “no-assertion” tests under `src/frontend/core/types/eval/tests/`, 21 were
retired and 5 retained** (single directory cluster; criterion = **all three gates fail** under Part
Seven of coding-rules: no clause, no incident history, and no assertions in the body). The
item-by-item decisions and “same-cell coverage” are provided by the 0.6-11a decision table (21
deleted / 5 retained); their individual rationales are not repeated here, only the execution result
is recorded.

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
- **5 retained** (the decision table could not identify replacement coverage, so they were retained
  under the hard requirement): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Second Retirement Batch (Executed 2026-11-06)

**Second batch: of 18 “weak-assertion” tests in `src/frontend/core/parser/pratt/tests/nud.rs`, 11
were retired and 7 retained** (single directory cluster = the Pratt prefix-parsing module; criterion
= **all three gates fail** under Part Seven of coding-rules: no clause, no incident history, and the
body contains only weak assertions such as `is_some()` / `is_none()`—it does not pin associativity
or the parse result).

The difference from the first batch must be stated explicitly: these tests **contain assertions**
and are not empty. The reason for retirement is “all three gates fail + zero criterion delta,” not
“no assertions”—every test selected for deletion identifies a **unit-level replacement coverage test
name in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The replacement
  coverage for every item follows the **same** `parse_expression → parse_prefix → prefix_info` path
  and makes stronger assertions (pinning the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, plus `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 retained**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—the individual rationales are in the commit message (Task 0.6, second
  test-retirement batch). There are two categories of retention reason: **the proof gate holds**
  (the first three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are the sole
  **unit-level** walkthrough points for their corresponding facts, and no second unit test in the
  repository observes them), and **the incident-history gate holds** (`break_removed` /
  `continue_removed` name #314).
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Generator Correction (0.6-13)

**`body_span` encounters braces inside literals and truncates the function body — fixed.**

The old implementation obtained the function body by balancing braces and did not distinguish real
code from braces in string/character literals or comments. Forms such as `tokenize("{")` are common
in Rust source, so the balance was skewed and the function body was truncated: the body of
`test_prefix_info_lbrace` was once truncated to the end of the file, causing the “Assertion Summary”
to pick up the remainder of the file (`matchesx11 assertx24 …`) and the item to be placed in the
“Pending adjudication” bucket rather than the “Retirement candidate” bucket—**the bucketing was
wrong**.

Fix: add `mask_noncode` to replace the contents of string literals / character literals / line
comments / block comments (which may be nested) / raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces first (newlines are preserved as-is, and line numbers and line counts do not
change), then balance the braces. **The mask is used only to locate function-body boundaries**;
assertion statistics and the incident-history gate still read the original source, and the criteria
themselves have not been loosened. `--selftest` gained 4 cases (paired / unpaired braces in an
ordinary string, braces in raw strings and block comments, and braces in a character literal), each
followed by a sentinel test—the old implementation found the wrong boundary in all four cases, while
the new implementation finds them all correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** for each test in
each file (the old implementation balanced raw braces, whereas the new implementation balances
braces after masking); any test with differing boundaries is affected. Of 3413 tests in the
repository, the boundaries of **11 tests** were extracted incorrectly by the old implementation:

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

### Before/After Bucket Differences: 2

For the other 9 tests, **8 affected only the “Assertion Summary” column** (their buckets were
unchanged), while **1 did not even change its summary**
(`test_fstring_close_brace_escape_preserved_raw`: the old implementation omitted the closing `}`
line, which contained no assertion keywords). The 8 summary changes are listed individually in the
commit message.

| Test Name                   | File                                          | Old Bucket                                  | New Bucket                                      | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| --------------------------- | --------------------------------------------- | ------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending adjudication / Pending adjudication | Pending adjudication / **Retirement candidate** | The actual function body contains only one weak assertion, `assert!(state.prefix_info().is_some())`; the file header and function body contain no `§` section number and no issue/regression/defect/incident ⇒ both the clause gate and incident-history gate fail; the body contains only a weak assertion such as `is_some()` ⇒ Retirement candidate (a candidate is not a verdict; the proof gate still requires manual review)                                                               |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident history / Retain                   | Clause / **Pending verification**               | The old implementation truncated the body to the end of the file and swallowed text containing “regression / defect / incident,” incorrectly classifying it under the incident-history gate; the actual function body contains none of these terms ⇒ the incident-history gate no longer holds; the file header has verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate matches, but it has not been confirmed by the sample audit ⇒ Pending verification |

The distribution changed accordingly: Clause 1713 → 1714 · Incident history 314 → 313 · Retain 776 →
775 · Pending verification 2121 → 2122 · Pending adjudication 412 → 411 · Retirement candidate 104
→ 105. **The repository-wide `#[test]` count is unchanged (3413)**—this round changed only the
implementation of the criteria, not any test.

## Third Retirement Batch (Executed 2026-11-07)

**Third batch: 5 retirement candidates under `src/middle/passes/codegen/tests/` were evaluated
individually; 2 were retired and 3 retained.** (single directory cluster = the
`middle::passes::codegen` module under test; criterion = **all three gates fail** under Part Seven
of coding-rules.)

This batch continues the **retention criterion** established in the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus and integration tests
operate across a process boundary and cannot replace a unit-level observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is
    **character-for-character identical** to `test_global_slot_store_then_load_roundtrip` in the
    same file (only the function name differs after comments are removed): both consist of
    `run_fixture()` + `assert_slot_is_42(&interp)`. Its doc comment claims to verify that the
    `execute_module` entry path returns non-Void, but the body never observes the return value of
    `execute_module` (`run_fixture` discards it with `.expect(...)`)— the claimed intent was not
    implemented, and the body is a pure duplicate ⇒ zero criterion delta. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file provides strict
    coverage: positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`),
    the `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` has the same shape as the inline `Symbol` here). “A
    name that was never inserted cannot be found” and “a name cannot be found after being popped”
    follow the same path in `SymbolScopeManager::lookup` (searching each layer via `scope.get(name)`
    and returning `None` when not found) ⇒ this test adds zero information. Replacement coverage =
    `test_scope_nesting`.
- **3 retained**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that “does
    not use serialization to connect directly to execution” (the serialization variant is a
    different path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization roundtrip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit-level observation point for the
    `id > 255` rejection branch of `OperandResolver::to_reg` (the bytecode layer has its own
    independent overflow checks and does not cover this function; no second test in the repository
    makes `to_reg` exceed the limit). `is_err()` is a discriminating assertion (removing the guard
    would turn it red), not merely a check that “it did not panic.”
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

### One Census Blind Spot Found in This Batch (**Fixed in 0.6-15**, See “Generator Criterion Correction” Below)

The census’s “delegated assertion” recognition pattern `\b(assert|check)[A-Za-z_]*\s*\(` **could not
match helpers with digits in their names**: `[A-Za-z_]*` does not include digits, so a call such as
`assert_slot_is_42(&interp)` **was not recognized as an assertion**— 2 of the 5 candidates in this
batch were consequently misclassified into the “no assertions in the body” bucket.

After all 15 repository-wide “Neither gate passes” items were reviewed individually (by allowing
digits in helper names and additionally recognizing the `match` + `panic!` form), **4 actually
contained assertions**—3 used the numeric helper above (`assert_slot_is_42`), and 1 was
`test_weak_from_arc` in `src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form;
test-specification rule 6.1 requires writing `assert!(matches!(...))`, but it does in fact assert).
The remaining 11 truly contained no assertions.

**Taking these 4 directly based on the machine’s “Neither gate passes” result would have wrongly
deleted tests with criteria**—the human review in this batch prevented that; the generator-side fix
is described in “Generator Criterion Correction (0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots have been fixed: digits in helper names, and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411); only the
generator’s recognition of whether a body contains assertions is changed.

### What Was Fixed

1. **Digits are now allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so a test containing one was treated
   as “having no assertions in its body.”
2. **Added recognition of the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the
   assertion is written in one arm, so the `assert!` macro is not visible to the lexer
   (test-specification rule 6.1 requires writing `assert!(matches!(...))`, but this form exists in
   the existing code). Recognition method: scan the expression from the `match` keyword to the first
   `{`, determine the block boundary by **balancing braces**, and count it as an assertion only if
   the block contains `panic!(`. **The determination is made on masked lines**— braces in strings
   and comments do not participate in balancing, and `panic!` in a comment is not treated as an
   assertion.

### Bucket Differences: 3 (the gate and disposition changed)

| Test Name                                           | File                                           | Old Bucket                                 | New Bucket                                  | Basis                                                                                                                                                                                                                                                    |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------------ | ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither gate passes / Retirement candidate | Pending adjudication / Pending adjudication | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts a variant, but the form was previously unrecognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither gate passes / Retirement candidate | Pending adjudication / Pending adjudication | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern did not match; it is now recognized as an assertion                                                                                   |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither gate passes / Retirement candidate | Pending adjudication / Pending adjudication | Same as above (the serialization-roundtrip test)                                                                                                                                                                                                         |

Another **77 tests changed only the “Assertion Summary” column** (their gates and dispositions were
unchanged): most gained `match_panicxN` in the summary (the body contains the `match` + `panic!`
form), while 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3;
`src/package/tests/cache.rs`: 1—helper names contain digits).

**The relaxation changes only the “Neither gate passes” tier**: this tier is the machine’s
**confident verdict** (not a single assertion in the body), and its criterion is a hard requirement;
a “Retirement candidate” is merely a pool awaiting manual review, and the size of that pool is not a
verdict. Consequently, all other buckets, including the weak-assertion pool, continue to use the
original criteria. Results: gate Neither gate passes 14 → 11 · Pending adjudication 923 → 926;
disposition Retirement candidate 103 → 100 · Pending adjudication 411 → 414.

### Existing “Neither Gate Passes” Items: 11 — Individual Review Conclusions

After all 11 repository-wide “Neither gate passes” items were read and reviewed against the original
source, **all 11 truly contain no assertions** (including no `assert*` / `check*` helper, no
`match` + `panic!` form, and no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                     | Review Conclusion                                           |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 calls to `.expect(...)`, guaranteeing only that it does not crash                             | Genuinely no assertion (only proves that it does not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                  | Genuinely no assertion                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer`s, obtains their contexts, and discards them                       | Genuinely no assertion                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Genuinely no assertion                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Genuinely no assertion                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                              | Genuinely no assertion                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` with invalid content and discards the result                      | Genuinely no assertion (only proves that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Genuinely no assertion (only proves that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Genuinely no assertion (only proves that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                            | Genuinely no assertion                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertion                                      |

Of these, 5 (`test_normalizer_context` and the 4 in reducer.rs) were retained because the 0.6-11a
decision table **could not identify replacement coverage**; this review reached the same conclusion.

### Reverse Spot Check: Was the Relaxation Too Broad?

Five items were read from the 88 newly recognized assertions using a fixed seed (20261107),
confirming that they really do assert:

| Spot-Check Item                                             | File                                                 | Assertions in the Source                                                                                                                             |
| ----------------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching arm, with `panic!("Expected AssocType")` if it does not match                                                 |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm, with `panic!("expected InvalidPackage, got …")` if it does not match                     |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | An `assert_eq!` comparing the constraint type in the matching arm, with `panic!("不是 Dynamic 变体…")` if it does not match                          |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm, with `panic!("Expected TokensDelta variant")` if it does not match                           |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm to verify the error code and message, with `panic!("不是 Disproved…")` if it does not match |

All 5 assert a concrete value or concrete variant (replacing the implementation with another correct
path would turn them red)—**the relaxation did not go too far**. Reverse controls were also added to
`--selftest`: a `match` without `panic!`, and a `panic!` appearing only in a comment; neither may be
recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

When the generator is rolled back to the old implementation (`HELPER_RE` without digits + `match` +
`panic!` recognition disabled), all the newly added `--selftest` cases **turn red**, with exit code
1; the new implementation passes them all.

### Rollback

The generator and census table are in the same commit; `git revert <sha>` restores the whole state.

## Fourth Retirement Batch (Executed 2026-11-07)

**Fourth batch: of 10 “Retirement candidate” tests in the directory cluster
`src/backends/interpreter/` (the interpreter backend’s test directory), 1 was retired and 9
retained** (single directory cluster; all three files were read together: `tests/bytecode_load.rs` 4
· `tests/weak.rs` 2 · `executor/tests/debug.rs` 4). Criterion = the **three gates** under Part Seven
of coding-rules; retention criterion = “a unit-level observation point can only be replaced by
another unit test; corpus/integration operate across a process boundary”; **deletion must identify a
concrete replacement coverage test name; if one cannot be identified or the source cannot be
understood, retain the test**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, one step, then obtain
  `capture_stack`) and makes **strictly stronger** assertions: it asserts that the stack is nonempty
  and that `stack[0].function_name == "main"` (pinning the top-of-stack position), whereas the
  listed test asserts only `stack.iter().any(|f| f.function_name == "main")` (any position is
  sufficient) —the former being true necessarily implies the latter ⇒ zero criterion delta.
- **9 retained**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each one, each is the
  **sole unit-level observation point** for some property (the load magic-number and version
  branches differ, the YXBC routing layer, the Some/None sides of Weak, stepping with no frames,
  `step_out` popping the stack, and running to completion with an empty stack); no unit test capable
  of replacing them could be identified ⇒ retained under the hard requirement. The individual bases
  are in the commit message (the decision table is not included on this page).
- **Figures**: repository-wide `#[test]` 3409 → 3408; 1 deletion = `git diff` counted `-#[test]` on
  1 line and `+#[test]` on 0 lines; the dominant test `test_capture_stack_during_execution` passes
  on its own. Census: Retirement candidate 99 → 98 · Pending adjudication 925 → 924 · Clause 1713 ·
  Incident history 313 · Proof 447 · Neither gate passes 11 · Pending verification 2121 ·
  Retain 775.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Fifth Retirement Batch (Executed 2026-11-07)

**Fifth batch: 1353 unit tests were retired in bulk by the mechanical rule “for the same symbol and
result category, retain only the strongest item”** (only unit tests under `src/**/tests/**` were
changed; not a single test on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (executed mechanically, without individual arguments)**: For the same symbol under test +
  the same assertion-result category, retain only the item with the strongest assertion and delete
  the rest.
  - Symbol under test = an in-repository symbol called in the test body (including one level of
    delegation through a helper function in the same file); when multiple symbols are touched,
    classify by the **primary symbol** (the first in-repository call). Symbols are qualified by
    their in-repository path; bare names are not merged across modules.
  - Result category = concrete value / specific error code (E code) / specific variant match / only
    ok / only err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; within the same group and strength, retain
    the first item in the file.
  - If a test asserts multiple categories at once (and the primary category cannot be determined),
    **retain it**; do not guess.
- **Three exceptions (not one item was lost)**: ① the name or comments name `issue #` / regression /
  defect / incident (incident-history gate); ② a symbol has only one test (deleting it would leave
  the symbol without a unit-level observation point); ③ `#[should_panic]` is the symbol’s only
  should_panic case. **Zero tests were retired from the 313 tests on the incident-history gate or
  the 11 items on the “Neither gate passes” line.**
- **Figures**: repository-wide `#[test]` 3408 → 2055; 1353 deletions = `git diff` counted `-#[test]`
  on 1353 lines and `+#[test]` on 0 lines. Census by gate (current values after the 0.6-19 abolition
  of the sample audit): Clause 808 · Incident history 313 · Proof 424 · Pending adjudication 499 ·
  Neither gate passes 11. Census by disposition (current values): Retain 737 · Pending verification
  1054 · Pending adjudication 203 · Retirement candidate 61. This batch’s retirement count by gate:
  Clause 902 · Pending adjudication 428 · Proof 23 (0.6-19 reclassified 9 items “confirmed by the
  audit as genuinely supported” as Pending verification, so the current by-gate and by-disposition
  values differ from those at the close of the fifth batch).
- **Cleanup accompanying the deletions**: Helper functions used exclusively by deleted tests,
  imports invalidated by the deletions, and struct fields rendered unused were removed as well
  (`cargo clippy --all --all-features --all-targets -- -D warnings` produced 0 warnings).
- **Two out-of-bounds deletions requiring an archival record**: There were 29 items that the census
  originally classified as “Retain” but that this batch’s rule nevertheless deleted—23 current-state
  anchors (names containing unchanged/anchor/guard/preserve/baseline, many of which were
  **domain-term** false positives such as “guard” and “preserves,” for example
  `test_while_guard_scope_balanced`) and 6 clauses whose support was genuinely confirmed by the
  audit. This batch followed the closed list of three exceptions on the gate card and did not add
  protection for these two categories; to restore them, backfill according to the retention/deletion
  comparison table on this page.
- **Failure and abolition of the sample audit**: The 40-item sample was drawn from the test pool
  using a fixed seed. After the fifth batch’s retirements, 38 items in the original sample no longer
  existed, so the audit’s `--check` consistently reported “missing 38, extra 38.” The sample audit
  was therefore abolished and removed in 0.6-19 (the script, data file, and the generator’s
  dependency on it were all removed); see “Abolition of the Sample Audit” below.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Sixth Retirement Batch (Executed 2026-11-26)

**Sixth batch: mechanical rollback from the 4.9.9 narrowing round—28 tests were retired along with
their mechanism, and 4 prohibition criteria had never been committed.** This batch does not use the
three gates to decide whether tests “should be retained”; instead, the mechanism under test was
**rolled back as a whole**: criteria written during the first six rounds before 4.9.9 for downstream
ambiguity mechanisms (candidate-provider priority / candidate-signature comparison / owner
compatibility inference / IR-side named-argument fallback / import-name resolution-point E5006
prohibition) were retired along with their mechanisms—the mechanism no longer exists, so the tests
have no object under test. Such deletions are registered as a “batch” under coding-rules 7.3 and do
not enter the retirement-candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 (28 deleted: 27 downstream-mechanism
    criteria from the first six rounds
    - 1 alias-import named-argument criterion). The 2 retained tests pin the minimal fix for
      bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: all 4 E5006 prohibition criteria were retired—the
    file existed only in an intermediate 4.9.9 commit; neither the parent commit nor that version’s
    committed table included it. After the prohibition was withdrawn, there was no object under
    test, so this batch does not decrement the committed table.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the alias-import candidate-injection surface was rolled back, and the fixture’s
    behavior no longer holds); the same-named corpus-diff baseline entry was deleted in the same
    batch. The baseline figures are recorded separately: **relative to direct parent commit
    `ac810e97`, 405 → 404 lines** (the intermediate state had included the fixture); **relative to
    the pre-4.9.9 baseline `08ccacc5`, 403 → 404 lines** (+1 base fixture
    `named-args-cross-unit-method`; the retired `alias-overload` never entered the final history).
- **Which gate**: The 2 retained tests = **incident-history gate** (pinning a real defect registered
  in 4.9.9: cross-unit named arguments could not obtain declaration-time parameter names, leaving
  `E1014 available` empty); among the 28 retired tests, the mechanism criteria added during the
  first six rounds had mostly been on the incident-history gate. The reason for this batch’s
  deletion **is not** the three gates (the tests were not judged worthless); rather, the mechanism
  under test was rolled back.
- **Replacement coverage**: Cross-unit named arguments using bare-name imports = the 2 retained
  driver-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity surface
  no longer has rejection logic (the requirement was rolled back), so there is no replacement
  criterion; the 4.9.8 dispatch-attribution criteria (10 driver + 5 integration) are retained
  unchanged.
- **Restoration**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the empty-name-table guard for
  `method_param_info` is retained; this form is unreachable in the public syntax and is a defensive
  guard, and the unit test only guards against accidental deletion).
- **Figures** (after regenerating for this batch): repository-wide `#[test]` 2680 across 334 files;
  the previously committed table had 2688 items → the current tree has 2680 (−8); 1 item is in the
  current tree but not the old table, 9 items are in the old table but not the current tree, and 0
  same-key rows were recomputed.
- **Rollback**: The entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit); `git revert <sha>` rolls back the whole batch.

## Abolition of the Sample Audit (0.6-19)

**Criterion**: The sample audit (0.6-10) measured the “true support rate of clause citations,” but
after 0.6-18 the retention rule no longer examines citations (it changed to “for the same symbol and
result category, retain only the strongest item,” and the test pool shrank from 3408 to 2055). Of
the original 40 sample items, 38 no longer existed, so the audit’s `--check` consistently reported
“missing 38, extra 38”—keeping it would only produce continuous alarms. It was therefore **abolished
and removed**.

**What was deleted**: The sample-audit script under `scripts/dev/` (the one that reproduced the
40-item sample using a fixed seed) and its JSON data file (40 item-by-item decisions and bases).

**Synchronizing the generator**: The generator previously read that JSON and classified “genuinely
supported by the audit” as “Clause gate + Retain”; that branch has been removed—**clause citations
no longer have an “audit-confirmed” tier** and always fall under “Clause / Pending verification”.

**Distribution changes** (not one of the repository-wide 2055 `#[test]` items changed; the TSV was
regenerated): Retain 746 → 737 (−9: the 9 items previously confirmed by the audit as genuinely
supported moved back to Pending verification); Pending verification 1045 → 1054; by gate: Clause 811
→ 808, Pending adjudication 496 → 499 (the others remain unchanged: Incident history 313 · Proof 424
· Neither gate passes 11).

**Rollback**: The entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` rolls back the whole batch.

## Rerun and Self-Check

```bash
python scripts/dev/test-gate-census.py            # Recalculate and overwrite the TSV (both runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs only lexical determination; its boundaries are stated honestly:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as “no assertions”—the first batch’s 26 tests (21 retired / 5 retained) and the
  second batch’s 18 tests (11 retired / 7 retained) were all manually reviewed against their
  function bodies.
- **Function-body boundaries (fixed in 0.6-13)**: When extracting a function body, the generator
  balances braces; braces in string/character literals and comments skew that balance, truncating
  the body—`test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was once truncated to the
  end of the file, causing the “Assertion Summary” to pick up the remainder of the file and the item
  to be classified as “Pending adjudication” rather than “Retirement candidate.” It now masks
  non-code regions first (string/character literals / line comments / block comments / raw strings)
  and then balances the braces; see the “Generator Correction” section.
- **Delegated-assertion recognition (fixed in 0.6-15)**: The old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not allow digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions, and tests containing such calls
  were misclassified as “having no assertions in the body.” At the time, 4 of the repository-wide 15
  “Neither gate passes” items actually contained assertions (3 numeric helpers + 1 `match` +
  `panic!` form). The name portion has now been broadened to `[A-Za-z0-9_]*`, and recognition of the
  `match` + `panic!` form has been added; see the “Generator Criterion Correction” section.
- **The proof gate is not judged**: Every test that fails both preceding gates is classified only as
  “Pending adjudication” or “Retirement candidate”; the generator never declares that a test “should
  be deleted.”
- **No cross-file deduplication**: Completely duplicated tests are not identified in this table (the
  0.6-2 survey measured them at only 1%, so they do not constitute a retirement surface).
- **All clause citations require verification**: A verifiable section number in the file header only
  means that “the test maps to a clause that can be named explicitly”; it does not mean that the
  citation supports the assertion. The machine does not judge whether it does, so even a clause-gate
  match is classified only as “Pending verification” and must be verified by a human before the test
  can be retained.
