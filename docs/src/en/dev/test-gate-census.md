---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement candidate list plus retirement records for each batch'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **three gates (clause / incident history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), producing a rerunnable, per-test decision table. A
sampling audit of the clause gate (40 entries) once measured the proportion of tests that “cite a
clause but whose assertions are unsupported by it” and thereby **tightened the criteria**. That
audit was abolished and removed in 0.6-19. **This page and the generator itself are read-only and do
not modify tests**; test retirement is performed in batches (one commit per batch, revertible as a
whole with `git revert`), and the outcomes are recorded in the batch retirement subsections below
and in `09a`.

Generated artifacts:

| Artifact                            | Purpose                                                                                 |
| ----------------------------------- | --------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Per-test decision table (2055 rows plus the header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                             |

## Criteria

### Scan Surface

Every `#[test]` in all `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module's `tests/` subdirectory) | 1581                 |
| `tests/integration`                                                      | 335                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 75                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2055** (285 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                   | Meaning of a hit                                           |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | A **verifiable reference** appears within the first 25 lines of the file header: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); files under `src/**/tests/` are considered together with their parent `mod.rs`                                                                                                                                                                                          | The test maps to an identifiable clause                    |
| History gate                      | The test function body, function documentation comment, or **file header** (such as `//! Regression background: issue #226`) identifies a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                 | The test pins down an actual incident                      |
| Proof gate                        | **The machine cannot decide this** (“If it were replaced by another correct implementation, would it fail?” requires human judgment). Therefore, every test that fails the first two gates falls into “Pending Decision”; it is not automatically classified as “Fail All”. The sole exception is a body with **no assertion at all** (including no delegation to `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only “Pending Decision” or “No Assertions” can be produced |

Rationale for tightening: the 0.6-9 clause gate only required the file header to **contain the
string `§`/`RFC`/`SPEC`**, so 2403 tests were classified as “Keep”. A 40-entry stratified sample in
0.6-10 found that **half of the citations did not support the assertions** (mis-citation rate: 50%,
95% CI: 35%–65%)—the direct reason for abandoning “keep it based on one citation.” After tightening,
only section-number forms are accepted, and **having a citation still does not mean the citation
supports the assertion**—clause references always stop at “Pending Verification”.

(The sampling audit was abolished and removed in 0.6-19: as the audit pool shrank with test
retirement, 38 entries in the original sample no longer existed, and `--check` consistently reported
“38 missing, 38 extra.” The findings are retained, but the audit artifacts are no longer
maintained.)

Rationale for the file-header radius: repository convention is to put text such as “Regression
background: issue #N” in the file header and not repeat it in each test comment. Looking only at
each test's own comments would miss the history for the entire file (measured: 7 entries within the
tests themselves, 291 within file headers).

### Categories That Are Never Retired

The following tests are **always classified as “Keep”** and never enter the retirement candidate
pool:

- The `tests/yaoxiang/**` corpus and the project-shape corpus at `tests/project-corpus/**`
  (behavioral criterion surfaces);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and operating across process boundaries);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                          |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Keep                 | Only two cases: the History gate is hit, or the Proof gate applies (spec-shaped directory / current-state anchor name)                                                                           |
| Pending Verification | Cites a clause (with a section number, or only citing an entire RFC without a section number), but the reference has not been manually confirmed—it must be human-verified before it can be kept |
| Retirement Candidate | Fails both gates and has only weak assertions (or no assertions)—a candidate is not a verdict; the Proof gate still requires manual review                                                       |
| Pending Decision     | Fails both gates but has strong assertions; the machine gives no conclusion                                                                                                                      |

## Distribution (After Tightening)

| Gate                                                            | Count    |
| --------------------------------------------------------------- | -------- |
| Clause (has a verifiable reference)                             | 808      |
| History                                                         | 313      |
| Proof (spec-shaped criterion surface / current-state anchor)    | 424      |
| Pending Decision (fails both gates; Proof awaits manual review) | 499      |
| Fail All (no assertions in the body)                            | 11       |
| **Total**                                                       | **2055** |

| Disposition          | Count    | Compared with 0.6-9                                                                                                                                                                                            |
| -------------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | **737**  | 2897 (−2160)                                                                                                                                                                                                   |
| Pending Verification | 1054     | 0 (new category)                                                                                                                                                                                               |
| Pending Decision     | 203      | 412 (−1 after the second batch, +3 after the 0.6-15 criteria correction, −211 after the fifth batch)                                                                                                           |
| Retirement Candidate | 61       | 136 (−21 in the first batch, −11 in the second, +1 after the generator correction, −2 in the third, −3 after the 0.6-15 criteria correction, −1 after the redundancy scan, −1 in the fourth, −37 in the fifth) |
| **Total**            | **2055** | —                                                                                                                                                                                                              |

Two changes to the criteria require explanation:

1. **Structure of “Keep”**: 737 = Proof gate 424 (spec-shaped criterion surface 390 + current-state
   anchors 34) + History gate 313. Of the original 2897, the 2121 tests that were “kept based on a
   single citation” now fall into Pending Verification. After the sampling audit was abolished in
   0.6-19, the 9 tests that had been kept as “confirmed by the audit to be genuinely supported” also
   fall into Pending Verification—clause references no longer have an “audit-confirmed” category.
2. **History gate 47 → 313**: these are not new incidents, but **label realignment**. In 0.6-9,
   tests that hit both the Clause and History gates were recorded under Clause. After tightening, a
   Clause hit no longer automatically means “Keep”, so the History gate must catch these tests, and
   its count absorbs them. The total number kept is therefore unaffected (these tests were also
   “Keep” in 0.6-9).

File criterion: of the 285 test files, **136** have a verifiable section number in the file header
(the old 0.6-9 criterion produced 221).

## Reconciliation with the 0.6-2 Survey

The 0.6-2 survey figures are the first-version baseline for comparison. The reasons for each
difference are listed below:

| Item                           | 0.6-2 survey                  | This census                                                                                                                                 | Reason for the difference                                                                                                                                                                                                                                                                              |
| ------------------------------ | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Repository-wide `#[test]`      | 3443                          | 2055                                                                                                                                        | 2 tests were added after 0.6-2 (the check-semantics regression tests E3005 and E3022); 21 no-assertion tests were deleted according to the first list, 11 weak-assertion tests according to the second, 2 according to the third, 1 according to the fourth, and 1353 according to the fifth; net −388 |
| Number of test files           | 296                           | 285                                                                                                                                         | 11 whole files were retired in the fifth batch                                                                                                                                                                                                                                                         |
| Clause gate                    | 227 files (77%)               | 136 files after tightening; 808 tests by entry                                                                                              | The criterion was tightened from “the string appears” to “a section number `§<number>` is present”, nearly halving the file count; the fifth batch retired 902 tests, and after the sampling audit was abolished in 0.6-19, another 9 moved from “Keep” to Pending Verification                        |
| History signal                 | 442 tests / 148 files         | 313 tests within the gate; three radii: 7 within the test itself / 291 within file headers (38 files) / 1218 across entire files (79 files) | The 148 files in 0.6-2 were broader than the current 79 files under the entire-file radius because its vocabulary and scope were larger; this census strictly uses the four signals defined by the gate                                                                                                |
| Characterization strong signal | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                                                | Characterization is not one of the three gates; incorporating it into retirement conclusions would exceed the gate's authority                                                                                                                                                                         |
| Weak assertions                | 473 (14%)                     | See the “Assertion Summary” column in the TSV                                                                                               | The counting units differ: 0.6-2 counted the pattern “only asserts is_ok/is_some”; this census uses lexical counts and distinguishes strong / weak / delegated assertions                                                                                                                              |

## Conclusion Summary

1. **The Clause gate was previously significantly inflated**: under the old criterion, 2403 tests
   were classified as “Clause/Keep”. The sample measured a 50% mis-citation rate (95% CI: 35%–65%)—a
   “reference” and a “reference that supports the assertion” are different things, and the old
   criterion treated them as the same. (The sampling audit was abolished and removed in 0.6-19; its
   findings are retained. See “Abolition of the Sampling Audit” below for the reason.)
2. **After tightening, approximately 1202 tests truly have clause support** (range: 846–1557). The
   rest either await verification or should be judged through the History / Proof gates.
3. **The retirement surface is unrelated to the clause mis-citation surface**: the 136 candidates
   were unrelated to clauses (no assertions / weak assertions). The first batch retired 21 tests,
   the second retired 11, the generator correction added 1, the third retired 2, the 0.6-15 criteria
   correction removed 3, the redundancy scan removed 1, the fourth retired 1, and the fifth retired
   1353 (using the mechanical rule “for the same symbol and result category, keep only the strongest
   test”), reducing the candidate count to 61. The fifth batch's retirement count by gate was:
   Clause 902 · Pending Decision 428 · Proof 23 · History 0 · Fail All 0—**not a single History gate
   test or zero-assertion test was retired**.
4. **The History gate is the largest legitimate source of “Keep”** (313 tests): it demonstrates that
   “pinning down an actual incident” is more reliable than “copying a sentence from an RFC”.

## First Retirement Batch (Executed, 2026-11-06)

**First batch: of the 26 “no assertion” tests in `src/frontend/core/types/eval/tests/`, 21 were
retired and 5 were kept** (single directory cluster; criterion = all **three gates fail** under Part
Seven of coding-rules: no Clause gate, no History gate, and no assertions in the body). The per-test
decisions and “same-cell coverage” are provided by the 0.6-11a decision table (21 deleted / 5 kept);
this page does not repeat the reasons and only records the execution outcome.

- **21 retired** (`normalizer.rs`: 2 + `reducer.rs`: 19): `test_normalizer_with_config`,
  `test_normalizer_evaluator`, `test_type_computer_register_alias`,
  `test_type_computer_compute_primitive`, `test_type_computer_set_context`,
  `test_type_computer_compute_type_var`, `test_type_computer_compute_list`,
  `test_type_computer_compute_tuple`, `test_type_computer_compute_fn`,
  `test_reducer_reduce_with_limit`, `test_unifier_different_types`,
  `test_unifier_substitution_after_unify`, `test_unifier_unify_option_same`,
  `test_unifier_unify_result_same`, `test_unifier_unify_dict_same`, `test_unifier_unify_set_same`,
  `test_unifier_unify_range_same`, `test_unifier_unify_arc_same`, `test_unifier_unify_weak_same`,
  `test_unifier_unify_enum_same`, `test_unifier_unify_struct_same`.
- **5 kept** (the decision table could not identify replacement coverage, so the hard requirement
  applies): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Second Retirement Batch (Executed, 2026-11-06)

**Second batch: of the 18 “weak assertion” tests in `src/frontend/core/parser/pratt/tests/nud.rs`,
11 were retired and 7 were kept** (single directory cluster = the Pratt prefix parser module;
criterion = all **three gates fail** under Part Seven of coding-rules: no Clause gate, no History
gate, and only weak assertions such as `is_some()` / `is_none()` in the body—neither pinning binding
power nor the parsing result).

The difference between this batch and the first must be stated clearly: these tests **do contain
assertions**; they are not empty tests. The reason for retirement is “all three gates fail + zero
criterion increment,” not “no assertions”. Every test slated for deletion identifies a **unit-level
replacement coverage test in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The replacement
  coverage for every one follows the **same** `parse_expression → parse_prefix → prefix_info` path
  and makes a stronger assertion (pinning the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, and `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the decision table in “VI. Second Test Retirement Batch” in
  `09a-wbs-history.md` for the per-test reasons. There are two categories of reason for keeping
  them: **the Proof gate applies** (the first three, together with `test_prefix_info_at` and
  `test_prefix_info_eof`, are the sole unit-level inspection points for their corresponding facts,
  and the repository contains no second unit test that observes them), and **the History gate
  applies** (`break_removed` / `continue_removed` identify #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Generator Correction (0.6-13)

**`body_span` truncated function bodies when it encountered braces inside literals—this has been
fixed.**

The old implementation obtained the function body by balancing braces without distinguishing actual
code from braces inside string / character literals or comments. Such code is common in Rust source,
for example `tokenize("{")`, so the balance was thrown off and the function body was truncated. The
body of `test_prefix_info_lbrace` was once truncated to the end of the file, causing the “Assertion
Summary” to include the remainder of the file (`matchesx11 assertx24 …`). It was therefore bucketed
as “Pending Decision” rather than “Retirement Candidate”—the **wrong bucket**.

Fix: add `mask_noncode` to replace the contents of string literals / character literals / line
comments / block comments (nestable) / raw strings (`r"…"` / `r#"…"#` / `br"…"` / `cr#"…"#`) with
spaces (newlines are preserved unchanged, so line numbers and line counts do not change), then
balance the braces. **The mask is used only to locate function-body boundaries**; assertion
statistics and the History gate still read the original source, and the criterion itself has not
been loosened. `--selftest` gained four cases (paired and unpaired braces in ordinary strings,
braces in raw strings and block comments, and braces in character literals), each followed by a
sentinel test. The old implementation extracted the wrong boundary in all four cases; the new
implementation extracts all four correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** for each test in
each file (the old implementation balanced literals, while the new implementation balances after
masking). Any test with different boundaries is affected. Of the 3413 tests in the repository, the
old implementation extracted **11** boundaries incorrectly:

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

### Bucket Differences Before and After the Fix: 2

For the other 9 tests, **8 affected only the “Assertion Summary” column** (the bucket did not
change), and **1 did not even change the summary** (`test_fstring_close_brace_escape_preserved_raw`:
the old implementation omitted the closing `}` line, which contained no assertion token). The eight
summary changes are listed individually in this section of `09a-wbs-history.md`.

| Test name                   | File                                          | Old bucket                          | New bucket                                  | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --------------------------- | --------------------------------------------- | ----------------------------------- | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending Decision / Pending Decision | Pending Decision / **Retirement Candidate** | The real function body contains only one weak assertion, `assert!(state.prefix_info().is_some())`; the file header and function body contain no § section number, issue/regression/defect/incident ⇒ both the Clause and History gates fail; the body contains only a weak assertion such as `is_some()` ⇒ Retirement Candidate (a candidate is not a verdict; the Proof gate still requires manual review)                                                                        |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | History / Keep                      | Clause / **Pending Verification**           | The old implementation truncated the function body to the end of the file, swallowed text containing “regression / defect / incident”, and falsely matched the History gate; the real function body contains no such wording ⇒ the History gate no longer applies; the file header has verifiable references (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the Clause gate is hit, but the reference has not been confirmed by the sampling audit ⇒ Pending Verification |

The distribution changes accordingly: Clause 1713 → 1714 · History 314 → 313 · Keep 776 → 775 ·
Pending Verification 2121 → 2122 · Pending Decision 412 → 411 · Retirement Candidate 104 → 105.
**The repository-wide `#[test]` count is unchanged (3413)**—this round only changes the
implementation of the criterion and does not modify any tests.

## Third Retirement Batch (Executed, 2026-11-07)

**Third batch: the 5 retirement candidates in `src/middle/passes/codegen/tests/` were adjudicated
individually; 2 were retired and 3 were kept.** (single directory cluster = the
`middle::passes::codegen` module under test; criterion = all **three gates fail** under Part Seven
of coding-rules.)

This batch retains the **retention criterion** established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; a corpus / integration test
operates at the process-boundary layer and cannot replace a unit-level observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (after comments are
    removed, only the function name differs): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its documentation comment claims to verify that the
    `execute_module` entry path returns a non-Void value, but the body never observes the return
    value from `execute_module` (`run_fixture` discards it with `.expect(...)`)—the claimed intent
    was not implemented, and the function body is a pure duplicate ⇒ zero criterion increment.
    Replacement coverage = `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file provides strictly
    stronger coverage: a positive lookup (`lookup("a").is_some()`), a negative lookup
    (`lookup("b").is_none()`), the `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)`, identical to the inline `Symbol` in this test). “A
    name that was never inserted cannot be found” and “it cannot be found after being popped” use
    the same path in `SymbolScopeManager::lookup` (each layer performs `scope.get(name)`, returning
    `None` when not found) ⇒ this test adds zero criterion. Replacement coverage =
    `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that
    “executes directly without serialization” (the serialized variant is a different path), and
    `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit-level observation point for the
    `OperandResolver::to_reg` rejection branch when `id > 255` (the bytecode layer has its own,
    independent overflow checks and does not cover this function; nowhere in the repository is there
    a second test case that drives `to_reg` out of range). `is_err()` is a discriminating assertion
    (it would fail if the guard were removed), not merely proof that the test did not panic.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

### A Census Blind Spot Found in This Batch (**Fixed in 0.6-15**, See “Generator Criteria Correction” Below)

The census's recognition of “delegated assertions” used `\b(assert|check)[A-Za-z_]*\s*\(` and
therefore **could not match helper names containing digits**: `[A-Za-z_]*` does not include digits,
so calls such as `assert_slot_is_42(&interp)` were not recognized as assertions. Consequently, 2 of
the 5 candidates in this batch were mistakenly bucketed as “no assertions in the body”.

After reviewing all 15 repository-wide “Fail All” tests (allowing digits in helper names and
additionally recognizing the `match` + `panic!` form), **4 actually contained assertions**—3 used
the numeric helpers above, and 1 was `test_weak_from_arc` in
`src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule 6.1
requires `assert!(matches!(...))`, but it does in fact assert). The other 11 truly contain no
assertion at all.

**If these 4 had been removed directly under the machine's “Fail All” classification, tests with
criteria would have been mistakenly deleted**—the per-test human adjudication in this batch
prevented that; the generator-side fix is described under “Generator Criteria Correction (0.6-15)”
below.

## Generator Criteria Correction (0.6-15)

**Two criteria blind spots have been fixed: helper names containing digits, and the `match` +
`panic!` form.** This round **does not delete any tests** (the repository-wide `#[test]` count
remains 3411); it only changes how the generator recognizes whether a body contains an assertion.

### What Was Fixed

1. **Digits are now allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern did not
   match helper names such as `assert_slot_is_42(&interp)`, so tests containing such calls were
   treated as having “no assertions in the body”.
2. **The `match` + `panic!` form is now recognized**: `match <expr> { … panic!(…) … }`—the assertion
   is written in one of the arms, so `assert!` is not visible lexically (test-specification rule 6.1
   requires `assert!(matches!(...))`, but existing code contains this form). Recognition method:
   scan from the `match` keyword to the first `{` (the expression) to find the block that it opens;
   determine the block boundary by **brace balancing**; and require `panic!(` within the block.
   **The decision is made on masked lines**—braces inside strings and comments do not participate in
   the balancing, and `panic!` inside a comment is not treated as an assertion.

### Bucket Differences: 3 (Gate and Disposition Changed)

| Test name                                           | File                                           | Old bucket                      | New bucket                          | Basis                                                                                                                                                                                                                                                      |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------- | ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Fail All / Retirement Candidate | Pending Decision / Pending Decision | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts a variant, but the form was not previously recognized; it is now recognized as an assertion, but the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Fail All / Retirement Candidate | Pending Decision / Pending Decision | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern did not match; it is now recognized as an assertion                                                                                     |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Fail All / Retirement Candidate | Pending Decision / Pending Decision | Same as above (the serialization round-trip test)                                                                                                                                                                                                          |

Another **77 tests changed only the “Assertion Summary” column** (the gate and disposition were
unchanged): most gained `match_panicxN` in the summary (the body contains the `match` + `panic!`
form), and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3;
`src/package/tests/cache.rs`: 1—helper names containing digits).

**The relaxation changes only the “Fail All” category**: this category is the machine's **confident
verdict** (the body contains no assertion at all), and its criterion is a hard requirement;
“Retirement Candidate” is only a pool awaiting manual review, and the size of that pool is not a
verdict. All other buckets, including the weak-assertion pool, therefore retain their original
criteria. Results: gate Fail All 14 → 11 · Pending Decision 923 → 926; disposition Retirement
Candidate 103 → 100 · Pending Decision 411 → 414.

### Review of the Existing 11 “Fail All” Tests

After reading all 11 repository-wide “Fail All” tests individually, **all 11 truly contain no
assertion at all** (including no `assert*` / `check*` helpers, no `match` + `panic!` form, and no
`#[should_panic]`):

| Test name                                       | File                                             | What the body actually does                                                                   | Review conclusion                                     |
| ----------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 calls to `.expect(...)`, only ensuring that it does not crash                               | Truly no assertion (only proof that it did not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains a `context()` handle and discards it                                                  | Truly no assertion                                    |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them            | Truly no assertion                                    |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                           | Truly no assertion                                    |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                     | Truly no assertion                                    |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                            | Truly no assertion                                    |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` with invalid content and discards the result                    | Truly no assertion (only proof that it did not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                              | Truly no assertion (only proof that it did not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                        | Truly no assertion (only proof that it did not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                          | Truly no assertion                                    |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Truly no assertion                                    |

Of these, 5 (`test_normalizer_context` and the 4 tests in reducer.rs) were kept because the 0.6-11a
decision table **could not identify replacement coverage**. The findings of this review agree with
that decision.

### Reverse Sampling: Did the Relaxation Go Too Far?

Five tests were sampled with the fixed seed (20261107) from the 88 entries newly recognized as
assertions, and their source was read to confirm that they really assert:

| Sampled entry                                               | File                                                 | Assertion in the source                                                                                                                          |
| ----------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching arm, and `panic!("Expected AssocType")` if it does not match                                              |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm, and `panic!("expected InvalidPackage, got …")` if it does not match                  |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | `assert_eq!` in the matching arm compares the constraint type, and `panic!("不是 Dynamic 变体…")` if it does not match                           |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm, and `panic!("Expected TokensDelta variant")` if it does not match                        |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm verify the error code and message, and `panic!("不是 Disproved…")` if it does not match |

All five assert concrete values or concrete variants (replacing the implementation with another
correct path would make them fail)—**the relaxation did not go too far**. The reverse controls were
also added to `--selftest`: a `match` without `panic!`, and `panic!` appearing only in a comment;
neither may be recognized as an assertion.

### The Criterion Has Teeth (Developmental Proof)

Reverting the generator to the old implementation (`HELPER_RE` without digits + recognition of
`match` + `panic!` disabled) makes all the newly added `--selftest` cases **fail**, with exit code
1; the new implementation passes them all.

### Rollback

The generator and the census table are in the same commit, so `git revert <sha>` restores them as a
whole.

## Fourth Retirement Batch (Executed, 2026-11-07)

**Fourth batch: of the 10 “Retirement Candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend's test directories), 1 was retired and 9 were
kept** (single directory cluster; three files were read together before judgment:
`tests/bytecode_load.rs`: 4 · `tests/weak.rs`: 2 · `executor/tests/debug.rs`: 4). Criterion = the
**three gates** under Part Seven of coding-rules; retention criterion = “a unit-level observation
point can only be replaced by another unit test; corpus / integration tests operate at the
process-boundary layer”; **a deletion must identify a specific replacement coverage test by name; if
it cannot be identified or understood, the test is kept**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, followed by one step
  and then `capture_stack`) and makes a **strictly stronger** assertion: it asserts that the stack
  is non-empty and that `stack[0].function_name == "main"` (pinning the top-of-stack position),
  whereas the listed test only asserts `stack.iter().any(|f| f.function_name == "main")` (any
  position is sufficient)—the former being true implies the latter ⇒ zero criterion increment.
- **9 kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading their source
  individually, each is the **sole unit-level observation point** for some property (the load
  magic-number and version branches are distinct, the YXBC routing layer, both sides of Weak's
  Some/None behavior, stepping with no frame, `step_out` popping the stack, and running to
  completion leaving the stack empty). No unit test that could replace them could be identified ⇒
  they are kept under the hard requirement. See Section Eleven of
  `docs/src/dev/architecture/09a-wbs-history.md` for the per-test evidence (the decision table is
  recorded there and is not included on this page).
- **Numbers**: repository-wide `#[test]` 3409 → 3408; the 1 deletion is `-#[test]` 1 line and
  `+#[test]` 0 line as counted by `git diff`; the dominant test
  `test_capture_stack_during_execution` passes when run on its own. Census: Retirement Candidate 99
  → 98 · Pending Decision 925 → 924 · Clause 1713 · History 313 · Proof 447 · Fail All 11 · Pending
  Verification 2121 · Keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Fifth Retirement Batch (Executed, 2026-11-07)

**Fifth batch: 1353 unit tests were retired in bulk using the mechanical rule “for the same symbol
and result category, keep only the strongest test”** (only unit tests under `src/**/tests/**` were
modified; not a single test on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (executed mechanically, without individual arguments)**: for the same tested symbol and the
  same assertion result category, keep only the test with the strongest assertion and delete the
  rest.
  - Tested symbol = a repository symbol called by the test body (including calls passed through one
    layer of helper functions in the same file); when multiple symbols are touched, classify by the
    **primary symbol** (the first repository call). Symbols are qualified by repository path and are
    not merged across modules by bare name.
  - Result category = a concrete value / a specific error code (E code) / a specific variant match /
    only ok / only err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when tests in a group have equal strength,
    keep the first one in the file.
  - If a test asserts multiple categories at once (making the primary category indeterminate),
    **keep it**; do not guess.
- **Three exceptions (not one was retired)**: ① the name or comment identifies `issue #` /
  regression / defect / incident (History gate); ② a symbol has only one test (retiring it would
  leave the symbol without a unit-level observation point); ③ `#[should_panic]` and the test is the
  symbol's only should_panic case. **0 of the 313 History gate tests and 0 of the 11 “Fail All”
  tests were retired.**
- **Numbers**: repository-wide `#[test]` 3408 → 2055; the 1353 deletions are `-#[test]` 1353 lines
  and `+#[test]` 0 line as counted by `git diff`. Census by gate (current values after the sampling
  audit was abolished in 0.6-19): Clause 808 · History 313 · Proof 424 · Pending Decision 499 · Fail
  All 11. Census by disposition (current values): Keep 737 · Pending Verification 1054 · Pending
  Decision 203 · Retirement Candidate 61. The retirement count for this batch by gate was: Clause
  902 · Pending Decision 428 · Proof 23 (after 0.6-19 reclassified the 9 “audit-confirmed as
  genuinely supported” entries as Pending Verification, the current by-gate and by-disposition
  values differ from those at the close of the fifth batch).
- **Cleanup alongside deletions**: helper functions used only by deleted tests, imports that became
  unused, and struct fields that became obsolete were all removed
  (`cargo clippy --all --all-features --all-targets -- -D warnings` produced 0 warnings).
- **Two out-of-bounds deletions that must be recorded**: 29 tests that the census originally
  classified as “Keep” were nevertheless deleted by this batch's rule—23 current-state anchors
  (names containing unchanged/anchor/guard/preserve/baseline, many of which were accidental matches
  on domain words such as “guard” and “preserves”, for example `test_while_guard_scope_balanced`)
  and 6 clauses whose genuine support had been confirmed by the audit. This batch followed the
  closed list of three gate exceptions and did not additionally protect these two categories; to
  restore them, simply backfill according to the keep/delete comparison table on this page.
- **Failure and abolition of the sampling audit**: the 40-entry sample was drawn from the test pool
  using a fixed seed. After the fifth batch retired tests, 38 entries in the original sample no
  longer existed, and the audit's `--check` therefore consistently reported “38 missing, 38 extra.”
  The sampling audit was abolished and removed in 0.6-19 (its script, data file, and the generator's
  dependencies on it were all removed); see “Abolition of the Sampling Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Abolition of the Sampling Audit (0.6-19)

**Criterion**: the sampling audit (0.6-10) measured the “true support rate” of clause references,
but after 0.6-18 the retention rule no longer examines references (it changed to “for the same
symbol and result category, keep only the strongest test”, and the test pool shrank from 3408 to
2055). Of the original 40 sample entries, 38 no longer existed, and the audit's `--check`
consistently reported “38 missing, 38 extra”—keeping it would only produce a continuous alarm.
Therefore, it was **abolished and removed**.

**What was removed**: the sampling audit script under `scripts/dev/` (the one that reproduced the
40-entry sample using a fixed seed) and its JSON data file (the 40 per-entry decisions and
supporting evidence).

**The generator's dependencies were removed in sync**: the generator previously read that JSON and
classified “audit-confirmed genuine support” as “Clause gate + Keep”. That branch has been
removed—**clause references no longer have an “audit-confirmed” category**, and always fall under
“Clause / Pending Verification”.

**Distribution changes** (the repository-wide `#[test]` count of 2055 was **unchanged**, and the TSV
was regenerated): Keep 746 → 737 (−9: the 9 entries that the audit had confirmed as genuinely
supported fell back to Pending Verification); Pending Verification 1045 → 1054; by gate: Clause 811
→ 808, Pending Decision 496 → 499 (all others unchanged: History 313 · Proof 424 · Fail All 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit), so `git revert <sha>` rolls it back as a whole.

## Rerunning and Self-Checks

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-check (does not scan the repository)
```

The generator performs lexical judgment only, and its boundaries are stated accurately:

- **Delegated assertions are recognized by name only** (`assert*`/`check*` helper functions). If a
  test's assertion is hidden in a helper whose name does not begin with either prefix, the test will
  be misclassified as having “no assertions”—the first batch's 26 tests (21 retired / 5 kept) and
  the second batch's 18 tests (11 retired / 7 kept) were all manually reviewed function body by
  function body.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator obtains a function body by
  balancing braces, braces inside strings / character literals and comments throw off the balance
  and truncate the body—`test_prefix_info_lbrace` (which contains `tokenize("{")`) was once
  truncated to the end of the file, and the “Assertion Summary” included the rest of the file,
  causing it to be bucketed as “Pending Decision” rather than “Retirement Candidate”. The generator
  now masks non-code regions (strings / character literals / line comments / block comments / raw
  strings) before balancing the braces; see “Generator Correction” below.
- **Recognition of delegated assertions (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` used `[A-Za-z_]*`, which does not include digits, so calls such
  as `assert_slot_is_42(&interp)` were not recognized as assertions, and tests containing such calls
  were misclassified as having “no assertions in the body”. At the time, 4 of the repository's 15
  “Fail All” tests actually contained assertions (3 numeric helpers + 1 `match` + `panic!` form).
  The name portion has now been widened to `[A-Za-z0-9_]*`, and the `match` + `panic!` form has been
  added; see “Generator Criteria Correction” below.
- **The Proof gate is not adjudicated**: any test that fails both gates only falls into “Pending
  Decision” or “Retirement Candidate”; the generator never declares that it “should be deleted”.
- **No deduplication across files**: completely duplicate tests are not identified in this table
  (0.6-2 measured them at only 1%, so they do not constitute a retirement surface).
- **Clause references always require verification**: a verifiable section number in the file header
  only means that the test maps to an identifiable clause; it does not mean that the reference
  supports the assertion. The machine does not judge whether it does, so a Clause hit still only
  reaches “Pending Verification” and must be human-verified before the test can be kept.
