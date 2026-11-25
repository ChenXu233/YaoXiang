---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Grand Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement candidate list along with retirement records for each batch'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Grand Cleanup)

Prerequisite material for WBS 0.6 “Grand Cleanup”: mechanically classify every repository-wide
`#[test]` according to the **three gates (clause / incident / proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a reproducible per-test decision table.
The clause gate’s sample audit (40 entries) once measured the proportion of tests that “cited a
clause but could not support their assertions” and thereby **tightened the criteria**; that audit
was deprecated and deleted in 0.6-19. **This page and the generator itself are read-only and do not
modify tests**; test retirement is carried out in batches (one commit per batch, revertible as a
whole with `git revert`), and execution results are recorded in the per-batch retirement sections
below and in commit messages.

Generated artifacts:

| Artifact                            | Purpose                                                                          |
| ----------------------------------- | -------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Per-test decision table (2688 rows + header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                      |

## Criteria

### Scan Surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 2084                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2688** (334 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine Criteria                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Meaning of a Match                                             |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | A **verifiable entry** appears within the first 25 lines of the file header: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); files under `src/**/tests/` are inspected together with their parent `mod.rs`                                                                                                                                                                                                                                         | The test maps to a clause that can be specifically identified  |
| Incident gate                     | The test function body, function doc comment, or **file header** (`//! Regression context: issue #226`, for example) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                                        | The test pins an incident that has actually occurred           |
| Proof gate                        | **The machine cannot adjudicate this** (“If it were replaced with another correct implementation, would it fail?” requires human judgment). Therefore, anything that fails both of the first two gates always lands in “pending adjudication”; it is never automatically classified as “neither passes.” The sole exception is a test with **no assertions at all** in its body (including no delegation to `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only “pending adjudication” or “no assertions” can be assigned |

Rationale for tightening: the clause gate in 0.6-9 merely required the file header to **contain
`§`/`RFC`/`SPEC`**, so 2403 entries were classified as “keep.” The stratified sample of 40 entries
measured in 0.6-10 found that **half of the citations could not support their assertions**
(mis-citation rate 50%, 95% interval 35%–65%)—this measurement was the **direct reason for
abandoning the rule that a citation alone is sufficient to keep a test**. After tightening, only the
section-number form is recognized, and **having a citation still does not mean that the citation
supports the assertion**—all clause references now stop at “pending verification.” (This sample
audit was deprecated and deleted in 0.6-19: as the audit pool shrank with test retirement, 38
entries from the original sample no longer existed, and `--check` always reported “missing 38
entries, extra 38 entries.” The conclusion is retained, but the audit artifacts are no longer
maintained.)

Rationale for the file-header radius: this repository’s convention is to put “Regression context:
issue #N” in the file header rather than repeat it in each test’s comment. Looking only at each
test’s own comment would miss the incident history of an entire file (measured: 7 entries within the
test’s own radius, 291 entries within the file-header radius).

### Categories That Are Never Retired

The following tests are **always “kept”** and never become retirement candidates:

- Corpus `tests/yaoxiang/**` and project-shape `tests/project-corpus/**` (behavior criterion
  surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and crossing process boundaries);
- Tests whose names contain `unchanged` / `anchor` / `guard` / `preserv` / `baseline` (status-quo
  anchors).

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                        |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Assigned in only two cases: the incident gate matches, or the proof gate applies (spec-shaped directory / status-quo anchor name)                                                              |
| Pending verification | Cites a clause (with a section number, or only an entire RFC without a section number), but the reference has not been manually confirmed—it must be verified by a human before it can be kept |
| Retirement candidate | Neither gate passes and the assertion surface contains only weak assertions (or none)—a candidate is not a verdict; the proof gate still requires human review                                 |
| Pending adjudication | Neither gate passes, but there are strong assertions; the machine gives no conclusion                                                                                                          |

## Distribution (Regenerated on 2026-11-26)

| Gate                                                                       | Count    |
| -------------------------------------------------------------------------- | -------- |
| Clause (has a verifiable entry)                                            | 1017     |
| Incident                                                                   | 565      |
| Proof (spec-shaped criterion surface / status-quo anchors)                 | 571      |
| Pending adjudication (neither gate passes; proof gate awaits human review) | 529      |
| Neither passes (no assertions in the body)                                 | 6        |
| **Total**                                                                  | **2688** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1136** | 737 (+399)                        |
| Pending verification | 1271     | 1054 (+217)                       |
| Pending adjudication | 223      | 203 (+20)                         |
| Retirement candidate | 58       | 61 (−3)                           |
| **Total**            | **2688** | 2055 (+633)                       |

Structure of “keep”: 1136 = proof gate 571 (spec-shaped criterion surface 518 + status-quo
anchors 53) + incident gate 565. File-level criteria: among 334 test files, the **clause gate covers
122**; using the first 25 lines of the file header (including the parent `mod.rs` for
`src/**/tests*/`), a verifiable section number can be detected in **166** files.

Regeneration reconciliation: the previous checked-in table had 2646 entries, while the current tree
has 2688; 51 entries are in the current tree but not the old table, 9 entries are in the old table
but not the current tree, and 0 same-key rows have changed content. Per-key records are in the TSV;
the fate of stale keys is recorded in this batch’s commit message.

### Two Criteria Changes at the 0.6-19 Tightening Point

1. **Structure of “keep”**: 737 = proof gate 424 (spec-shaped criterion surface 390 + status-quo
   anchors 34) + incident gate 313. Of the original 2897 entries, the 2121 that were “kept based on
   a single citation” now fall into pending verification (after the 0.6-19 abolition of the sample
   audit, the 9 entries previously classified as “audit-confirmed genuine support” also fall into
   pending verification—clause references no longer have an “audit-confirmed” tier).
2. **Incident gate 47 → 313**: these are not newly discovered incidents, but **relabelled
   classifications**—in 0.6-9, entries that matched both the clause and incident gates were recorded
   as clause entries. After tightening, a clause match no longer automatically means “keep,” so
   these entries must be caught by the incident gate, and their counts are folded into it. The total
   number kept is therefore unaffected (these tests were also kept in 0.6-9).

File-level criteria at the time: of 285 test files, **136** had a verifiable section number in the
file header (the old 0.6-9 criterion yielded 221).

## Reconciliation Between the 0.6-19 Snapshot and the 0.6-2 Survey

The 0.6-2 survey figures are the baseline for the first comparison. Each difference is explained
below:

| Item                           | 0.6-2 Survey                  | Current                                                                                                           | Reason for Difference                                                                                                                                                                                                                                                                                       |
| ------------------------------ | ----------------------------- | ----------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`      | 3443                          | 2055                                                                                                              | 2 entries were added after 0.6-2 (the check-criteria regression tests for E3005 and E3022), followed by the deletion of 21 no-assertion tests from the first batch, 11 weak-assertion tests from the second batch, 2 from the third batch, 1 from the fourth batch, and 1353 from the fifth batch; net −388 |
| Number of test files           | 296                           | 285                                                                                                               | The fifth batch retired 11 whole files                                                                                                                                                                                                                                                                      |
| Clause gate                    | 227 files (77%)               | 136 files after tightening; 808 entries                                                                           | The criterion was tightened from “the token appears” to “a section number `§<number>` is present,” nearly halving the file count; the fifth batch retired 902 entries, and the abolition of the 0.6-19 sample audit moved another 9 entries from “keep” to pending verification                             |
| Incident signal                | 442 entries / 148 files       | 313 entries within the gate; three radii: test itself 7 / file header 291 (38 files) / whole file 1218 (79 files) | The 148 files from 0.6-2 were broader than even the current whole-file radius of 79 files, with a larger vocabulary and scope; the current run strictly uses the gate card’s four signals                                                                                                                   |
| Strong characterization signal | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                      | Characterization is not one of the three gates; mixing it into a retirement conclusion would exceed the gate’s authority                                                                                                                                                                                    |
| Weak assertions                | 473 (14%)                     | See the TSV “assertion summary” column                                                                            | The counting unit differs: 0.6-2 counted patterns that asserted only `is_ok`/`is_some`; the current run uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                                                                      |

## Summary of the 0.6-19 Conclusions (Historical)

1. **The clause gate had previously been significantly inflated**: under the old criterion, 2403
   entries were classified as “clause / keep,” and the sample audit measured a 50% mis-citation rate
   (95% interval 35%–65%)—“citation” and “a citation that supports the assertion” are different
   things, and the old criterion treated the former as the latter. (The sample audit was deprecated
   and deleted in 0.6-19; its conclusion is retained. The reason for deletion is given in “Abolition
   of the Sample Audit” below.)
2. **After tightening, approximately 1202 entries genuinely have clause support** (range 846–1557);
   the rest either need pending-verification paperwork or should originally have been assessed
   through the incident or proof gate.
3. **The retirement surface is unrelated to clause mis-citations**: the 136 candidates had no clause
   relationship (no assertions / weak assertions); the first batch retired 21, the second retired
   11, the generator correction added 1, the third batch removed 2, the 0.6-15 criterion correction
   removed 3, the redundancy scan removed 1, the fourth batch removed 1, and the fifth batch removed
   1353 under the mechanical rule “for the same symbol and result category, keep only the strongest
   entry,” reducing the candidates to 61. The fifth batch’s retirement count by gate was: clause 902
   · pending adjudication 428 · proof 23 · incident 0 · neither passes 0—**not a single entry was
   removed from either the incident gate or the zero-assertion line**.
4. **The incident gate is the largest legitimate source of “keep”** (313 entries): it demonstrates
   that “pinning a real incident” is more reliable than “copying a sentence from an RFC.”

## First Retirement Batch (Executed on 2026-11-06)

**First batch: 21 of the 26 “no assertions” tests under `src/frontend/core/types/eval/tests/` were
retired and 5 were kept** (a single directory cluster; criterion = all **three gates fail** under
Part Seven of coding-rules: no clause gate, no incident gate, and no assertions in the body). The
per-test decisions and “same-grid coverage” are provided by the 0.6-11a decision table (21 deleted /
5 kept); this page does not repeat the reasons and records only the execution result.

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
- **5 kept** (the decision table could not identify replacement coverage, so they were kept under
  the hard requirement): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Second Retirement Batch (Executed on 2026-11-06)

**Second batch: 11 of the 18 “weak assertions” tests in
`src/frontend/core/parser/pratt/tests/nud.rs` were retired and 7 were kept** (a single directory
cluster = the Pratt prefix parsing module; criterion = all **three gates fail** under Part Seven of
coding-rules: no clause gate, no incident gate, and the body contains only weak assertions such as
`is_some()` / `is_none()`—it does not pin down binding power or the parsing result).

The difference from the first batch must be stated clearly: these tests **do contain assertions**
and are not empty tests. The reason for retirement is “all three gates fail + zero criterion
increment,” not “no assertions”—every test selected for deletion identifies a **unit-level
replacement test in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Every replacement
  follows the **same** `parse_expression → parse_prefix → prefix_info` path and makes a stronger
  assertion (pins down the AST shape rather than checking only `is_some`): `test_parse_int_literal`
  / `test_parse_float_literal` / `test_parse_string_literal` / `test_parse_bool_true` /
  `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` / `test_parse_return_expr`
  in the same file, plus `test_grouped_expr` / `test_tuple` / `test_list_literal` /
  `test_list_empty` / `test_ref_expr` in `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—the per-test reasons are recorded in the commit message (Task 0.6, second
  test-retirement batch). There are two categories of reason: **the proof gate applies** (the first
  three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are the sole **unit-level**
  observation points for the corresponding facts, and the repository has no second unit test that
  observes them), and **the incident gate applies** (`break_removed` / `continue_removed` name
  #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Generator Correction (0.6-13)

**`body_span` truncates the function body when it encounters braces inside literals — fixed.**

The old implementation found function bodies by balancing braces without distinguishing real code
from string or character literals and comments. Forms such as `tokenize("{")` are common in Rust
source, so the balance was skewed and the function body was truncated: the body of
`test_prefix_info_lbrace` was once truncated to the end of the file, causing the “assertion summary”
to capture the rest of the file (`matchesx11 assertx24 …`) and therefore placing it in the “pending
adjudication” bucket rather than the “retirement candidate” bucket—**the wrong bucket**.

Fix: add `mask_noncode` to replace the contents of string literals / character literals / line
comments / block comments (nestable) / raw strings (`r"…"`, `r#"…"#`, `br"…"`, `cr#"…"#`) with
spaces (newlines remain unchanged, so line numbers and line counts do not change), then balance the
braces. **The mask is used only to locate function-body boundaries**; assertion statistics and the
incident gate still read the original source, and the criteria themselves have not been loosened.
`--selftest` gained 4 cases (paired and unpaired braces in ordinary strings, braces in raw strings
and block comments, and braces in character literals), each followed by a sentinel test—the old
implementation obtained the wrong boundary in all 4 cases, while the new implementation obtains all
of them correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** for each test in
each file (old implementation balancing against literals vs. new implementation balancing after
masking); entries with different boundaries are affected. Of 3413 tests in the repository, the old
implementation obtained the wrong boundary for **11**:

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

For the other 9 entries, **8 affected only the “assertion summary” column** (the bucket was
unchanged), and **1 did not even change the summary**
(`test_fstring_close_brace_escape_preserved_raw`: the old implementation omitted the closing `}`
line, which contained no assertion keyword). The 8 changed summaries are recorded individually in
the commit message.

| Test Name                   | File                                          | Old Bucket                                  | New Bucket                                      | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --------------------------- | --------------------------------------------- | ------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending adjudication / Pending adjudication | Pending adjudication / **Retirement candidate** | The actual function body contains only one weak assertion, `assert!(state.prefix_info().is_some())`; the file header and function body contain no § section number or issue/regression/defect/incident ⇒ both the clause and incident gates fail, and the body contains only a weak assertion of the `is_some()` kind ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires human review)                                                   |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident / Keep                             | Clause / **Pending verification**               | The old implementation extended the body to the end of the file and swallowed text containing “regression / defect / incident,” producing a false incident-gate match; the actual function body contains none of those terms ⇒ the incident gate no longer applies; the file header has verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate matches, but the citation was not confirmed by the sample audit ⇒ pending verification |

The distribution consequently changes: clause 1713 → 1714 · incident 314 → 313 · keep 776 → 775 ·
pending verification 2121 → 2122 · pending adjudication 412 → 411 · retirement candidate 104 → 105.
**The repository-wide `#[test]` count is unchanged (3413)**—this round changes only the
implementation of the criteria, not any test.

## Third Retirement Batch (Executed on 2026-11-07)

**Third batch: the 5 retirement candidates under `src/middle/passes/codegen/tests/` were adjudicated
one by one; 2 were retired and 3 were kept.** (a single directory cluster = the
`middle::passes::codegen` module under test; criterion = all **three gates fail** under Part Seven
of coding-rules.)

This batch retains the **keep criteria** established by the previous batch: a unit-level observation
point can only be replaced by another **unit test**; corpus and integration tests belong to the
process-boundary layer and cannot replace a unit observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (after comments are
    removed, only the function name differs): both use `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that the `execute_module` entry
    path returns non-Void, but the body never observes the return value of `execute_module`
    (`run_fixture` discards it with `.expect(...)`)— the claimed intent was not implemented, and the
    function body is a pure duplicate ⇒ zero criterion increment. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file covers it strictly:
    a positive lookup (`lookup("a").is_some()`), a negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the inserted `Symbol` shape (`symbol("a", Int(64), Local(0), false, 0)`
    is the same shape as the inline `Symbol` in this test). “A name that was never inserted cannot
    be found” and “it cannot be found after being popped” follow the same path in
    `SymbolScopeManager::lookup` (walk the layers with `scope.get(name)` and return `None` when not
    found) ⇒ zero increment for this test. Replacement coverage = `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that
    “executes directly without serialization” (the serialization variant is a different path), and
    `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization roundtrip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit observation point for the rejection
    branch `id > 255` in `OperandResolver::to_reg` (the bytecode layer has separate overflow checks
    and does not cover this function; no second test in the repository drives `to_reg` out of
    range). `is_err()` is a discriminating assertion (removing the guard makes the test fail), not
    merely a check that “nothing panicked.”
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

### One Census Blind Spot Exposed by This Batch (**Fixed in 0.6-15**; See “Generator Criterion Correction” Below)

The census’s recognition of “delegated assertions,” `\b(assert|check)[A-Za-z_]*\s*\(`, **does not
match helpers whose names contain digits**: `[A-Za-z_]*` does not include digits, so calls such as
`assert_slot_is_42(&interp)` are not recognized as assertions— 2 of the 5 candidates in this batch
were consequently misclassified as “no assertions in the body.”

After all 15 repository-wide “neither passes” entries were reviewed individually (helper names were
widened to include digits, and the `match` + `panic!` form was additionally recognized): **4
actually contain assertions**—3 use the numeric helpers above, and 1 is `test_weak_from_arc` in
`src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification Rule 6.1
requires `assert!(matches!(...))`, but it does assert in substance). The remaining 11 genuinely
contain no assertions.

**Directly removing these 4 under the machine’s “neither passes” classification would have wrongly
deleted tests with criteria**—they were preserved only because this batch included human
adjudication; the corresponding generator fix is described in “Generator Criterion Correction
(0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots have been fixed: helper names containing digits, and the `match` +
`panic!` form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411);
it changes only how the generator recognizes whether a body contains assertions.

### What Was Fixed

1. **Digits are now allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing such calls were
   treated as having “no assertions in the body.”
2. **Added the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the assertion is written
   in an arm, so no `assert!` macro is visible lexically (test-specification Rule 6.1 requires
   `assert!(matches!(...))`, but existing code contains this form). Recognition method: scan the
   expression after the `match` keyword up to the first `{`, determine the block boundary by **brace
   balancing**, and count it only if `panic!(` appears inside the block. **The decision is made on
   masked lines**— braces in strings and comments do not participate in balancing, and `panic!` in a
   comment is not treated as an assertion.

### Bucket Differences: 3 (Gate and Disposition Changed)

| Test Name                                           | File                                           | Old Bucket                            | New Bucket                                  | Basis                                                                                                                                                                                                                                                       |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------- | ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither passes / Retirement candidate | Pending adjudication / Pending adjudication | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts a variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not assess its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending adjudication / Pending adjudication | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern could not match; it is now recognized as an assertion                                                                                    |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending adjudication / Pending adjudication | Same as above (the serialization-roundtrip entry)                                                                                                                                                                                                           |

Another **77 entries changed only the “assertion summary” column** (the gate and disposition were
unchanged): most gained `match_panicxN` in the summary (the body contains the `match` + `panic!`
form), and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3 entries;
`src/package/tests/cache.rs`: 1 entry—helper names containing digits).

**The relaxation changes only the “neither passes” tier**: this tier is the machine’s **confident
verdict** (the body contains no assertions at all), and its criterion is a hard requirement; a
“retirement candidate” is only a pool awaiting human review, and the size of that pool is not a
verdict. All other buckets, including the weak-assertion pool, retain their original criteria.
Result: gates—neither passes 14 → 11 · pending adjudication 923 → 926; dispositions—retirement
candidate 103 → 100 · pending adjudication 411 → 414.

### Existing 11 “Neither Passes” Entries: Individual Review Conclusions

After individually reading all 11 repository-wide “neither passes” entries, **all 11 genuinely
contain no assertions at all** (neither `assert*` / `check*` helpers nor the `match` + `panic!`
form, and no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                     | Review Conclusion                                            |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 `.expect(...)` calls, which only guarantee that it does not crash                             | Genuinely no assertions (only proves that it does not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                  | Genuinely no assertions                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer`s, obtains their contexts, and discards them                       | Genuinely no assertions                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Genuinely no assertions                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Genuinely no assertions                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs `reduce` for a self-referential alias and discards the result                          | Genuinely no assertions                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                        | Genuinely no assertions (only proves that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Genuinely no assertions (only proves that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Genuinely no assertions (only proves that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs the lsp / repl and discards them (inside the wasm32 cfg gate)                        | Genuinely no assertions                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertions                                      |

Five of these (`test_normalizer_context` and the 4 entries in reducer.rs) were classified as “keep”
in the 0.6-11a decision table because **replacement coverage could not be identified**; the
conclusion of this review agrees with that result.

### Reverse Spot Check: Did the Relaxation Go Too Far?

Five entries were read in the source from among the 88 newly recognized as assertions, using a fixed
seed (20261107), to confirm that they really do assert:

| Spot-Check Entry                                            | File                                                 | Assertion in the Source                                                                                                                          |
| ----------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching `match` arm; the unmatched arm uses `panic!("Expected AssocType")`                                        |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm; the unmatched arm uses `panic!("expected InvalidPackage, got …")`                    |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | `assert_eq!` comparing the constraint type in the matching arm; the unmatched arm uses `panic!("不是 Dynamic 变体…")`                            |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm; the unmatched arm uses `panic!("Expected TokensDelta variant")`                          |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm checking the error code and message; the unmatched arm uses `panic!("不是 Disproved…")` |

All 5 assert concrete values or concrete variants (replacing the implementation with another correct
path would make them fail)—**the relaxation did not go too far**. The reverse controls were also
added to `--selftest`: a `match` without `panic!`, and a `panic!` that appears only in a comment.
Neither may be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

When the generator is reverted to the old implementation (`HELPER_RE` without digits + recognition
of `match` + `panic!` disabled), all the newly added `--selftest` cases **turn red** and the exit
code is 1; the new implementation passes them all.

### Rollback

The generator and census table are in the same commit; `git revert <sha>` restores them as a whole.

## Fourth Retirement Batch (Executed on 2026-11-07)

**Fourth batch: 1 of the 10 “retirement candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend’s test directories) was retired and 9 were
kept** (a single directory cluster, with three files read in one pass: `tests/bytecode_load.rs`: 4
entries · `tests/weak.rs`: 2 entries · `executor/tests/debug.rs`: 4 entries). Criterion = the
**three gates** in Part Seven of coding-rules; keep criteria = “a unit-level observation point can
only be replaced by another unit test; corpus/integration tests belong to the process-boundary
layer”; **retirement requires naming a concrete replacement coverage test; if no replacement can be
named or the source cannot be understood, the entry must be kept**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, plus one step before
  taking `capture_stack`), and its assertion is **strictly stronger**: it asserts that the stack is
  non-empty and that `stack[0].function_name == "main"` (pinning the top-of-stack position), whereas
  the listed test asserts only `stack.iter().any(|f| f.function_name == "main")` (any position is
  sufficient) —the former being true necessarily implies the latter ⇒ zero criterion increment.
- **9 kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each source, every one
  is the **sole unit observation point** for some property (the two distinct branches for load’s
  magic number/version, the YXBC routing layer, the Some/None sides of Weak, stepping with no
  frames, `step_out` popping the stack, and running to completion with an empty stack), and no unit
  test could be identified to replace them ⇒ they must be kept under the hard requirement. The
  per-test basis is recorded in the commit message (the decision table is not included on this
  page).
- **Numbers**: repository-wide `#[test]` 3409 → 3408; 1 deletion = `git diff` counts `-#[test]` 1
  line and `+#[test]` 0 lines; the dominating test, `test_capture_stack_during_execution`, passes on
  its own. Census: retirement candidate 99 → 98 · pending adjudication 925 → 924 · clause 1713 ·
  incident 313 · proof 447 · neither passes 11 · pending verification 2121 · keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Fifth Retirement Batch (Executed on 2026-11-07)

**Fifth batch: 1353 unit tests were retired in bulk under the mechanical rule “for the same symbol
and result category, keep only the strongest entry”** (only unit tests under `src/**/tests/**` were
changed; not a single entry under the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (executed mechanically, without per-test arguments)**: for the same symbol under test and
  the same assertion result category, keep only the strongest assertion and delete the rest.
  - The symbol under test is a repository symbol called in the test body (including one level of
    propagation through a helper function in the same file); when multiple symbols are reached,
    classify by the **primary symbol** (the first repository-internal call). Symbols are qualified
    by their path within the repository and are not merged across modules by bare name.
  - Result category = a concrete value / a particular error code (E code) / a particular variant
    match / ok only / err only / should_panic only.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when tests are equally strong, keep the
    first one in the file.
  - If a test asserts multiple categories (so the primary category cannot be determined), **keep
    it**; do not guess.
- **Three exceptions (not one entry removed from any)**: ① the name or comment names `issue #` /
  regression / defect / incident (incident gate); ② a symbol has only one test (removing it would
  leave the symbol with no unit observation point); ③ `#[should_panic]` is the symbol’s only
  should_panic test. **Zero of the incident gate’s 313 entries and zero of the 11 “neither passes”
  entries were retired.**
- **Numbers**: repository-wide `#[test]` 3408 → 2055; 1353 deletions = `git diff` counts `-#[test]`
  1353 lines and `+#[test]` 0 lines. Census by gate (current values after the 0.6-19 abolition of
  the sample audit): clause 808 · incident 313 · proof 424 · pending adjudication 499 · neither
  passes 11. Census by disposition (current values): keep 737 · pending verification 1054 · pending
  adjudication 203 · retirement candidate 61. The retirement count for this batch by gate was:
  clause 902 · pending adjudication 428 · proof 23 (0.6-19 reclassified 9 entries that had been
  “audit-confirmed as genuinely supported” as pending verification, so the current values by gate
  and disposition differ from those at the close of the fifth batch).
- **Cleanup alongside deletions**: helper functions used only by deleted tests, imports that became
  unused, and struct fields that became invalid were removed
  (`cargo clippy --all --all-features --all-targets -- -D warnings` produced 0 warnings).
- **Two out-of-bounds deletions that must be recorded**: there were 29 entries that the census
  originally classified as “keep” but that this batch’s rule still deleted—23 status-quo anchors
  (names containing unchanged/anchor/guard/preserve/baseline, many of which were false positives on
  **domain terms** such as “guard” or “preserves,” for example `test_while_guard_scope_balanced`)
  and 6 clauses genuinely confirmed by the audit as supported. This batch followed the closed list
  of three exceptions on the gate card and did not provide additional protection for these two
  categories; to restore them, backfill them using the keep/delete comparison table on this page.
- **Failure and abolition of the sample audit**: the 40-entry sample was drawn from the test pool
  using a fixed seed. After the fifth batch’s retirements, 38 entries from the original sample no
  longer existed, so the audit’s `--check` always reported “missing 38 entries, extra 38 entries.”
  The sample audit was deprecated and deleted in 0.6-19 (the script, data file, and the generator’s
  dependency on it were all removed); see “Abolition of the Sample Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Abolition of the Sample Audit (0.6-19)

**Criterion**: the sample audit (0.6-10) measured the “true support rate” of clause references, but
after 0.6-18 the retention rules no longer examine references (they changed to “for the same symbol
and result category, keep only the strongest entry,” and the test pool fell from 3408 to 2055). Of
the original 40 entries, 38 no longer existed, and the audit’s `--check` always reported “missing 38
entries, extra 38 entries”—leaving it in place would only produce continuous alarms. It was
therefore **deprecated and deleted**.

**What was deleted**: the sample-audit script under `scripts/dev/` (the one that reproduced the
40-entry sample using a fixed seed) and its JSON data file (40 per-entry decisions and bases).

**The generator’s dependency was removed in sync**: the generator previously read that JSON and
classified “audit-confirmed genuine support” as “clause gate + keep”; that branch was
deleted—**clause references no longer have an “audit-confirmed” tier** and uniformly fall into
“clause / pending verification.”

**Distribution changes** (all 2055 repository-wide `#[test]` entries remain untouched, and the TSV
was regenerated): keep 746 → 737 (−9: the 9 entries previously confirmed by the audit as genuinely
supported fell back to pending verification); pending verification 1045 → 1054; by gate: clause 811
→ 808, pending adjudication 496 → 499 (all others unchanged: incident 313 · proof 424 · neither
passes 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` reverts it as a whole.

## Rerun and Self-Check

```bash
python scripts/dev/test-gate-census.py            # Recalculate and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # Before CI/commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs only lexical adjudication; its boundaries are stated faithfully:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as “no assertions”—the first batch’s 26 entries (21 retired / 5 kept) and the second
  batch’s 18 entries (11 retired / 7 kept) all had their function bodies manually reviewed.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator obtains a function body, it
  balances braces, so braces in string / character literals and comments can skew the balance and
  truncate the body—`test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was once
  truncated to the end of the file, causing the “assertion summary” to capture the rest of the file
  and placing it in the “pending adjudication” bucket rather than the “retirement candidate” bucket.
  It now masks non-code regions (strings / character literals / line comments / block comments / raw
  strings) before balancing; see “Generator Correction” below.
- **Delegated-assertion recognition (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not include digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions, and tests containing such calls
  were misclassified as having “no assertions in the body.” At the time, 4 of the repository’s 15
  “neither passes” entries actually contained assertions (3 numeric helpers + 1 `match` + `panic!`
  form). The name portion has now been widened to `[A-Za-z0-9_]*`, and recognition of the `match` +
  `panic!` form has been added; see “Generator Criterion Correction” below.
- **The proof gate is not adjudicated**: every test that “fails both gates” lands only in “pending
  adjudication” or “retirement candidate”; the generator never announces “delete this.”
- **No cross-file deduplication**: completely duplicated tests are not identified in this table
  (0.6-2 measured only 1%, which does not constitute a retirement surface).
- **All clause references require verification**: a verifiable section number in the file header
  only means that “the test maps to a clause that can be specifically identified”; it does not mean
  that the reference supports the assertion. The machine does not adjudicate whether it does, so a
  clause match also stops at “pending verification” and must be verified by a human before it can be
  kept.
