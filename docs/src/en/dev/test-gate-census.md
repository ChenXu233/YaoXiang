---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository using the three gates from Part Seven of
  coding-rules, and provide a retirement-candidate list along with retirement records for each batch'
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
and in the commit message.

Generated artifacts:

| Artifact                            | Purpose                                                                                     |
| ----------------------------------- | ------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Item-by-item decision table (2646 rows plus the header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, with `--check` and `--selftest`)                                     |

## Scope

### Scan Surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root-package lib (`src/`, including each module’s `tests/` subdirectory) | 2044                 |
| `tests/integration`                                                      | 461                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2646** (328 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                 | Meaning of a match                                        |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | A **verifiable entry** appears in the first 25 lines of the file header: section-number form `§<数字>` (including `RFC-<数字> §<数字>` and `SPEC §<数字>`); files under `src/**/tests/` are inspected together with their parent `mod.rs`                                                                                                                                                                                                         | The test maps to a clause that can be named explicitly    |
| Incident-history gate             | The test function body, function doc comment, or **file header** (such as `//! 回归背景：issue #226`) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                           | The test pins down an incident that has actually occurred |
| Proof gate                        | **Cannot be machine-judged** (“If it were replaced with another correct implementation, would it fail?” requires human judgment). Therefore, anything that misses the first two gates falls into “Pending judgment” and is not automatically classified as “Neither passes”; the sole exception is a body with **no assertions at all** (including no delegated `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only “Pending judgment” or “No assertions” can be given   |

Rationale for tightening: the 0.6-9 clause gate only required the file header to **contain
`§`/`RFC`/`SPEC`**, so 2403 tests were classified as “Keep”. The 40-item stratified sample measured
in 0.6-10 found that **half of the citations could not support their assertions** (mis-citation
rate: 50%, 95% interval: 35%–65%)— this measurement was the **direct reason for abandoning the rule
of keeping a test on the strength of a citation**. After tightening, only section-number forms are
accepted, and **a citation still does not mean that the citation supports the assertion**—all clause
citations stop at “Pending verification”. (The sampling audit was repealed and deleted in 0.6-19: as
the test pool shrank through retirement, 38 of the original sample items no longer existed, so
`--check` consistently reported “38 missing, 38 extra”; the conclusion is retained, but the audit
artifacts are no longer maintained.)

Rationale for the file-header radius: this repository’s convention is to put “Regression context:
issue #N” in the file header without repeating it in individual test comments; looking only at each
test’s own comments would miss the incident history of the entire file (measured: 7 items at the
test’s own radius, 291 items at the file-header radius).

### Categories That Are Never Retired

The following tests are **always classified as “Keep”** and never enter the retirement-candidate
pool:

- Corpus tests under `tests/yaoxiang/**` and project-shape tests under `tests/project-corpus/**`
  (behavioral criterion surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and operating across a process boundary);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                   |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Only two cases: the incident-history gate matches, or the proof gate applies (spec-shaped directory / current-state-anchor name)                                                          |
| Pending verification | A clause is cited (with a section number, or only a whole RFC and no section number), but the citation has not been manually verified—it must be checked by a human before it can be kept |
| Retirement candidate | Both gates miss and the assertion surface contains only weak assertions (or no assertions)—a candidate is not a verdict; the proof gate still requires manual review                      |
| Pending judgment     | Both gates miss, but strong assertions are present; the machine reaches no conclusion                                                                                                     |

## Distribution (Regenerated 2026-11-24)

| Gate                                                             | Count    |
| ---------------------------------------------------------------- | -------- |
| Clause (has a verifiable entry)                                  | 997      |
| Incident history                                                 | 539      |
| Proof (spec-shaped criterion surface / current-state anchor)     | 568      |
| Pending judgment (misses both gates; proof awaits manual review) | 536      |
| Neither passes (no assertions in the body)                       | 6        |
| **Total**                                                        | **2646** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1107** | 737 (+370)                        |
| Pending verification | 1259     | 1054 (+205)                       |
| Pending judgment     | 222      | 203 (+19)                         |
| Retirement candidate | 58       | 61 (−3)                           |
| **Total**            | **2646** | 2055 (+591)                       |

Composition of “Keep”: 1107 = 568 proof-gate tests (516 on the spec-shaped criterion surface + 52
current-state anchors) + 539 incident-history-gate tests. File-level breakdown: among 328 test
files, **160** have a verifiable section number in the file header.

Regeneration reconciliation: the old table contained 2055 entries, while the current tree
contains 2646. There are 639 keys present in the current tree but absent from the old table: 25
entries added by 8.8 in this round, 606 existing entries that had not previously been regenerated,
and 8 entries added by `grammar_probe` in the 8.3 go/no-go probe. There are 48 keys present in the
old table but absent from the current tree. Per-key records for the 25 new tests, 606 existing
entries, and 8 probes are all in the TSV; the per-item destinations of the 48 stale keys under
“deleted / renamed / moved” are listed in
[09a §P8 8.3 Pre-batch Entry](./architecture/09a-wbs-history.md). Another 46 entries with the same
keys had their row contents recomputed from the current source; these do not count as added or
removed keys.

### Two Criteria Changes at the Time of the 0.6-19 Tightening

1. **Composition of “Keep”**: 737 = 424 proof-gate tests (390 on the spec-shaped criterion surface +
   34 current-state anchors) + 313 incident-history-gate tests. Of the original 2897 entries, the
   2121 that were kept “on the strength of a single citation” now fall into Pending verification.
   After 0.6-19 repealed the sampling audit, the 9 entries previously kept because the audit had
   “confirmed genuine support” also fell into Pending verification—clause citations no longer have
   an “audit-confirmed” tier.
2. **Incident-history gate 47 → 313**: this is not an increase in incident history, but **relocation
   of labels**. In 0.6-9, a test that matched both the clause and incident-history gates was
   recorded under the clause gate. After tightening, a clause match no longer automatically resulted
   in “Keep”, so these entries had to be caught by the incident-history gate and were folded into
   that count. The total number kept was therefore unaffected (the tests in this batch were also
   classified as “Keep” in 0.6-9).

File-level breakdown at that time: among 285 test files, **136** had a verifiable section number in
the file header (the old 0.6-9 criterion yielded 221).

## Reconciliation Between 0.6-19 and the 0.6-2 Survey

The 0.6-2 survey figures are the first comparison baseline. Each difference and its cause are listed
below:

| Item                            | 0.6-2 survey                  | This census                                                                                                | Cause of the difference                                                                                                                                                                                                                                                                          |
| ------------------------------- | ----------------------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                       | 2 tests were added after 0.6-2 (check-criterion regression tests for E3005 and E3022), followed by the removal of 21 no-assertion tests from the first list, 11 weak-assertion tests from the second list, 2 from the third list, 1 from the fourth list, and 1353 from the fifth list; net −388 |
| Number of test files            | 296                           | 285                                                                                                        | 11 entire files were retired in the fifth batch                                                                                                                                                                                                                                                  |
| Clause gate                     | 227 files (77%)               | 136 files after tightening; 808 by item                                                                    | The criterion was tightened from “the token appears” to “there is a section number `§<number>`”, nearly halving the file count; the fifth batch retired 902 tests, and 9 more moved from “Keep” to Pending verification after 0.6-19 repealed the sampling audit                                 |
| Incident-history signals        | 442 tests / 148 files         | 313 within the gate; three radii: test itself 7 / file header 291 (38 files) / entire file 1218 (79 files) | The 148 files in the 0.6-2 survey represent a broader scope than the current entire-file radius of 79 files because its vocabulary and scope were broader; this census strictly uses the four signals on the criterion card                                                                      |
| Characterization strong signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                               | Characterization is not one of the three gates; mixing it into a retirement conclusion would exceed the criteria’s authority                                                                                                                                                                     |
| Weak assertions                 | 473 (14%)                     | See the TSV “assertion summary” column                                                                     | The counting units differ: 0.6-2 counted patterns that assert only `is_ok`/`is_some`, whereas this census uses lexical counts and distinguishes strong, weak, and delegated assertions                                                                                                           |

## Summary of the 0.6-19 Conclusions (Historical)

1. **The clause gate had previously been significantly overstated**: under the old criterion, 2403
   tests were classified as “Clause/Keep”. The sampling audit measured a 50% mis-citation rate (95%
   interval: 35%–65%)— “citation” and “a citation that supports the assertion” are two different
   things, and the old criterion treated the former as the latter. (The sampling audit was repealed
   and deleted in 0.6-19, but its conclusion is retained; see “Repeal of the Sampling Audit” below
   for the reason for deletion.)
2. **After tightening, approximately 1202 tests genuinely had clause support** (range: 846–1557).
   The remainder must either obtain Pending verification status or should have been classified under
   the incident-history gate/proof gate in the first place.
3. **The retirement surface was unrelated to clause mis-citations**: the 136 candidates were
   unrelated to clauses (no assertions/weak assertions). The first batch retired 21 tests and the
   second batch retired 11; the generator correction contributed +1, the third batch −2, the 0.6-15
   criterion correction −3, the redundancy scan −1, the fourth batch −1, and the fifth batch −1353
   under the mechanical rule of retaining only the strongest test for the same symbol and result
   category, reducing the candidates to 61. The fifth batch’s retirement count by gate was: Clause
   902 · Pending judgment 428 · Proof 23 · Incident history 0 · Neither passes 0—**not a single
   incident-history-gate or zero-assertion test was removed**.
4. **The incident-history gate was the largest legitimate source of “Keep”** (313 tests): it
   demonstrates that “pinning down an actual incident” is more reliable than “copying a sentence
   from an RFC”.

## First Retirement Batch (Executed 2026-11-06)

**First batch: 21 of the 26 “no-assertion” tests under `src/frontend/core/types/eval/tests/` were
retired and 5 were kept** (a single directory cluster; criterion = all **three gates fail**
according to Part Seven of coding-rules: the clause gate misses, the incident-history gate misses,
and the body contains no assertions). The item-by-item decisions and “same-cell coverage” are
provided by the 0.6-11a decision table (21 deleted / 5 kept). This page does not repeat the reasons;
it records only the execution results.

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
- **5 kept** (the decision table could not identify alternative coverage, so they were kept under
  the hard requirement): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts the whole batch.

## Second Retirement Batch (Executed 2026-11-06)

**Second batch: 11 of the 18 “weak-assertion” tests in `src/frontend/core/parser/pratt/tests/nud.rs`
were retired and 7 were kept** (a single directory cluster = the Pratt prefix-parsing module;
criterion = all **three gates fail** according to Part Seven of coding-rules: the clause gate
misses, the incident-history gate misses, and the body contains only weak assertions such as
`is_some()` / `is_none()`—neither binding power nor the parse result is pinned down).

The difference from the first batch must be stated clearly: these tests **contain assertions** and
are not empty tests. The reason for retirement is “all three gates fail + zero criterion increment,”
not “no assertions”; every deleted test names an alternative unit-level coverage test in the same
module.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The alternative
  coverage for each follows the **same** `parse_expression → parse_prefix → prefix_info` path and
  makes stronger assertions (pinning down the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, plus `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the decision table in “VI. Second Test Retirement Batch” in
  the commit message for the per-test reasons. There are two categories of reason for keeping
  them: **the Proof gate applies** (the first three, together with `test_prefix_info_at` and
  `test_prefix_info_eof`, are the sole unit-level inspection points for their corresponding facts,
  and the repository contains no second unit test that observes them), and **the History gate
  applies** (`break_removed` / `continue_removed` identify #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts the whole batch.

## Generator Correction (0.6-13)

**`body_span` truncates the function body when it encounters braces inside literals — fixed.**

The old implementation extracted the function body by balancing braces without distinguishing real
code from braces in string/character literals or comments. Forms such as `tokenize("{")` are common
in Rust source, so balancing was thrown off and the function body was truncated: the body of
`test_prefix_info_lbrace` was once truncated to the end of the file, causing the “assertion summary”
to capture the rest of the file (`matchesx11 assertx24 …`) and the test to be bucketed as “Pending
judgment” rather than “Retirement candidate”—**a bucket error**.

Fix: add `mask_noncode` to first replace the contents of string literals, character literals, line
comments, block comments (which may be nested), and raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (newlines remain unchanged, as do line numbers and line counts), then balance
the braces. **The mask is used only to locate function-body boundaries**; assertion statistics and
the incident-history gate still read the original source, and the criterion itself has not been
relaxed. `--selftest` gained four cases (paired and unpaired braces in ordinary strings, braces in
raw strings and block comments, and braces in character literals), each followed by a sentinel
test—the old implementation found the wrong boundary in all four cases, while the new implementation
finds all four correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** test by test
within each file (the old implementation balanced braces literally; the new implementation balances
them after masking). Any test whose boundaries differ is affected. Of 3413 tests in the repository,
the old implementation found the wrong boundaries for **11**:

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
summary changes are listed individually in the commit message.

| Test name                   | File                                          | Old bucket                          | New bucket                                  | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --------------------------- | --------------------------------------------- | ----------------------------------- | ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending judgment / Pending judgment | Pending judgment / **Retirement candidate** | The real function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; neither the file header nor the body has a § section number, issue/regression/defect/incident signal ⇒ both the clause and incident-history gates miss, and the body contains only a weak assertion such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires manual review)                                                                              |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident history / Keep             | Clause / **Pending verification**           | The old implementation truncated the body to the end of the file, causing the absorbed text to contain “regression / defect / incident” and be misclassified as matching the incident-history gate; the real function body contains none of these terms ⇒ the incident-history gate no longer applies; the file header has verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate matches, but it has not been confirmed by the sampling audit ⇒ Pending verification |

The distribution changed accordingly: Clause 1713 → 1714 · Incident history 314 → 313 · Keep 776 →
775 · Pending verification 2121 → 2122 · Pending judgment 412 → 411 · Retirement candidate 104
→ 105. **The repository-wide `#[test]` count remains unchanged (3413)**—this round changed only the
implementation of the criterion, not any test.

## Third Retirement Batch (Executed 2026-11-07)

**Third batch: 2 of the 5 retirement candidates under `src/middle/passes/codegen/tests/` were
retired and 3 were kept.** (A single directory cluster = the `middle::passes::codegen` module under
test; criterion = all **three gates fail** according to Part Seven of coding-rules.)

This batch continues the **criterion for keeping tests** established by the previous batch: a
unit-level observation point may be replaced only by another **unit test**; corpus and integration
tests operate at the process-boundary layer and cannot replace a unit-level observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (apart from the
    function name after comments are removed): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that the `execute_module` entry
    path returns non-Void, but the body never observes the return value of `execute_module`
    (`run_fixture` discards it with `.expect(...)`)— the claimed intent is not implemented, and the
    function body is pure duplication ⇒ zero criterion increment. Alternative coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file covers it strictly:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` is the same shape as the inline `Symbol` in this
    test). “A name that was never inserted cannot be found” and “it cannot be found after being
    popped” follow the same path in `SymbolScopeManager::lookup` (walk each `scope.get(name)` layer;
    return `None` when not found) ⇒ this test adds zero increment. Alternative coverage =
    `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that
    “executes directly without going through serialization” (the serialization variant is a
    different path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit-level observation point for the
    `id > 255` rejection branch of `OperandResolver::to_reg` (the bytecode layer has its own
    independent overflow checks and does not cover this function; no second test in the repository
    makes `to_reg` go out of range). `is_err()` is a discriminating assertion (removing the guard
    would make the test fail), not merely a check that it “does not panic”.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts the whole batch.

### One Census Blind Spot Found in This Batch (**Fixed in 0.6-15**, See “Generator Criterion Correction” Below)

The census’s delegated-assertion recognition pattern, `\b(assert|check)[A-Za-z_]*\s*\(`, **could not
match helpers whose names contain digits**: `[A-Za-z_]*` does not include digits, so a call such as
`assert_slot_is_42(&interp)` **was not recognized as an assertion**— 2 of the 5 candidates in this
batch were consequently misbucketed as “no assertions in the body”.

After all 15 repository-wide “Neither passes” entries were reviewed item by item (by allowing digits
in helper names and additionally recognizing the `match` + `panic!` form), **4 actually contained
assertions**: 3 used the numbered helper above (`assert_slot_is_42`), and 1 was `test_weak_from_arc`
in `src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule
6.1 requires `assert!(matches!(...))`, but the test does in fact assert). The other 11 genuinely
contained no assertions.

**Taking these 4 directly from the machine’s “Neither passes” bucket would have wrongly deleted
tests with criteria**—they were not deleted because this batch included manual, item-by-item
judgment; the generator-side fix is described in “Generator Criterion Correction (0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots have been fixed: helper names containing digits, and the `match` +
`panic!` form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411);
it changes only how the generator recognizes whether a body contains assertions.

### What Was Fixed

1. **Digits are now allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing such calls were
   treated as “no assertions in the body”.
2. **The `match` + `panic!` form was added**: `match <expr> { … panic!(…) … }`—the assertion is
   written in one of the match arms, so the `assert!` macro is not visible lexically
   (test-specification rule 6.1 requires `assert!(matches!(...))`, but existing code contains this
   form). Recognition method: scan the expression after the `match` keyword up to the first `{` to
   find the block that it opens, determine the block boundary by **balancing braces**, and require a
   `panic!(` within that block. **The judgment is performed on masked lines**— braces in strings and
   comments do not participate in balancing, and `panic!` in comments is not treated as an
   assertion.

### Bucket Differences: 3 (Gate and Disposition Changed)

| Test name                                           | File                                           | Old bucket                            | New bucket                          | Basis                                                                                                                                                                                                                                                      |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------- | ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither passes / Retirement candidate | Pending judgment / Pending judgment | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts a variant, but its form was not previously recognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending judgment / Pending judgment | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit and the old pattern did not match it; it is now recognized as an assertion                                                                                     |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending judgment / Pending judgment | The same applies (the serialization round-trip test)                                                                                                                                                                                                       |

Another **77 tests changed only the “assertion summary” column** (the gate and disposition did not
change): most summaries gained `match_panicxN` (the body contains the `match` + `panic!` form),
while 4 gained `helperxN` (3 in `src/driver/tests/manifest_error.rs`, and 1 in
`src/package/tests/cache.rs`—the helper names contain digits).

**The relaxation changes only the “Neither passes” tier**: that tier is the machine’s **confident
verdict** (there is not one assertion in the body), and its criterion is a hard requirement;
“Retirement candidate” is only a pool awaiting manual review, so the size of the pool is not a
verdict. Consequently, the weak-assertion pool and all other buckets retain their original criteria.
Result: Gate—Neither passes 14 → 11 · Pending judgment 923 → 926; Disposition—Retirement candidate
103 → 100 · Pending judgment 411 → 414.

### Existing “Neither Passes” Entries: 11, with Item-by-Item Review Conclusions

After all 11 repository-wide “Neither passes” tests were read against the original source, **all 11
genuinely contained no assertions** (including no `assert*` / `check*` helpers, no `match` +
`panic!` form, and no `#[should_panic]`):

| Test name                                       | File                                             | What the body actually does                                                                     | Review conclusion                                            |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 calls to `.expect(...)`, which merely guarantee that it does not crash                        | Genuinely no assertions (only proves that it does not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                  | Genuinely no assertions                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains the context, and discards it                   | Genuinely no assertions                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Genuinely no assertions                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Genuinely no assertions                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                              | Genuinely no assertions                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                        | Genuinely no assertions (only proves that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and then discards the edits                                           | Genuinely no assertions (only proves that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and then discards the edits                                     | Genuinely no assertions (only proves that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (within the wasm32 cfg gate)                            | Genuinely no assertions                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertions                                      |

Five of these (`test_normalizer_context` and the four in reducer.rs) were kept in the 0.6-11a
decision table because it **could not identify alternative coverage**; the conclusion of this review
agrees with that decision.

### Reverse Sampling: Did the Relaxation Go Too Far?

Five of the 88 entries newly recognized as assertions were sampled with a fixed seed (20261107) and
read against the original source to confirm that they really assert:

| Sampled entry                                               | File                                                 | Assertion in the original source                                                                                                                         |
| ----------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | The matching arm contains two `assert_eq!` calls; the non-matching arm executes `panic!("Expected AssocType")`                                           |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | The matching arm contains `assert!(msg.contains("src/evil.yx"))`; the non-matching arm executes `panic!("expected InvalidPackage, got …")`               |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | The matching arm contains `assert_eq!` comparisons of constraint types; the non-matching arm executes `panic!("不是 Dynamic 变体…")`                     |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | The matching arm contains `assert!(!delta.edits.is_empty())`; the non-matching arm executes `panic!("Expected TokensDelta variant")`                     |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | The matching arm contains three `assert_eq!` / `assert!` checks of the error code and message; the non-matching arm executes `panic!("不是 Disproved…")` |

All five assert a concrete value or variant (replacing the implementation with another correct path
would make them fail)—**the relaxation did not go too far**. Reverse-control cases were also added
to `--selftest`: a `match` without `panic!`, and `panic!` appearing only in a comment; neither may
be recognized as an assertion.

### The Criterion Has Teeth (Developmental Proof)

When the generator was reverted to the old implementation (`HELPER_RE` without digits + recognition
of the `match` + `panic!` form disabled), the newly added `--selftest` cases **all failed**, with
exit code 1; the new implementation passes all of them.

### Rollback

The generator and the census table are in the same commit; `git revert <sha>` restores the entire
state.

## Fourth Retirement Batch (Executed 2026-11-07)

**Fourth batch: 1 of the 10 “retirement candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend’s test directories) was retired and 9 were
kept** (a single directory cluster; all three files were read in one pass: 4 in
`tests/bytecode_load.rs` · 2 in `tests/weak.rs` · 4 in `executor/tests/debug.rs`). Criterion = the
**three gates** from Part Seven of coding-rules; criterion for keeping = “a unit-level observation
point may be replaced only by another unit test; corpus/integration tests operate at the
process-boundary layer”; **deletion must identify a concrete alternative coverage test name; if one
cannot be identified or understood, the test is kept**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step` (in
  `src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, followed by one step
  and then `capture_stack`), and its assertions are **strictly stronger**: it asserts that the stack
  is nonempty and that `stack[0].function_name == "main"` (pinning down the top-of-stack position),
  whereas the listed test asserts only `stack.iter().any(|f| f.function_name == "main")` (any
  position is sufficient)— the former being true necessarily implies the latter ⇒ zero criterion
  increment.
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
  the commit message for the per-test evidence (the decision table is
  recorded there and is not included on this page).
- **Numbers**: repository-wide `#[test]` 3409 → 3408; the 1 deletion is `-#[test]` 1 line and
  `+#[test]` 0 line as counted by `git diff`; the dominant test
  `test_capture_stack_during_execution` passes when run on its own. Census: Retirement Candidate 99
  → 98 · Pending Decision 925 → 924 · Clause 1713 · History 313 · Proof 447 · Fail All 11 · Pending
  Verification 2121 · Keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts the whole batch.

## Fifth Retirement Batch (Executed 2026-11-07)

**Fifth batch: 1353 unit tests were retired in bulk under the mechanical rule “for the same symbol
and result category, retain only the strongest one”** (only unit tests under `src/**/tests/**` were
changed; not a single test on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rules (executed mechanically, without an item-by-item argument)**: for the same symbol under
  test and the same assertion result category, retain only the test with the strongest assertion and
  delete the rest.
  - Symbol under test = a repository symbol called in the test body (including one level of
    propagation through a helper function in the same file); when multiple symbols are touched,
    classify by the **primary symbol** (the first repository call). Symbols are qualified by
    repository path and are not merged across modules by bare name.
  - Result category = a concrete value / a specific error code (E code) / a specific variant match /
    only checking ok / only checking err / only `should_panic`.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or `match` + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when tests in the same group have equal
    strength, retain the first one to appear in the file.
  - When a test asserts multiple categories (making the primary category indeterminate), **retain
    it** rather than guessing.
- **Three exceptions (not one was removed)**: ① the name or comment explicitly cites `issue #` /
  regression / defect / incident (incident-history gate); ② a symbol has only one test (removing it
  would leave the symbol with no unit-level observation point); ③ a `#[should_panic]` test is the
  symbol’s only should_panic case. **Zero** of the 313 incident-history-gate tests or 11 “Neither
  passes” tests were retired.
- **Numbers**: repository-wide `#[test]` 3408 → 2055; 1353 deletions = `-#[test]` 1353 lines and
  `+#[test]` 0 lines in `git diff`. Census by gate (current values after the 0.6-19 repeal of the
  sampling audit): Clause 808 · Incident history 313 · Proof 424 · Pending judgment 499 · Neither
  passes 11. Census by disposition (current values): Keep 737 · Pending verification 1054 · Pending
  judgment 203 · Retirement candidate 61. The retirement count for this batch by gate was: Clause
  902 · Pending judgment 428 · Proof 23 (0.6-19 reclassified 9 entries that the audit had “confirmed
  as genuinely supporting” as Pending verification, so the current values by gate and disposition
  differ from those when the fifth batch was closed).
- **Cleanup accompanying the deletions**: helper functions used only by deleted tests, imports that
  became invalid, and struct fields that became unused were removed as well
  (`cargo clippy --all --all-features --all-targets -- -D warnings` produced 0 warnings).
- **Two out-of-bounds deletions requiring a record**: 29 entries that the census originally
  classified as “Keep” were still deleted by this batch’s rules—23 current-state anchors (names
  containing unchanged/anchor/guard/preserve/baseline, many of which were false matches on domain
  terms such as “guard” or “preserves”, for example `test_while_guard_scope_balanced`) and 6 entries
  whose clause support the audit had confirmed as genuine. This batch followed the closed list of
  three exceptions on the criterion card and did not add protection for these two categories; to
  restore them, backfill them using the keep/delete comparison table on this page.
- **Failure and repeal of the sampling audit**: the 40-item sample was drawn from the test pool with
  a fixed seed. After the fifth batch’s retirements, 38 of the original sample items no longer
  existed, so the audit’s `--check` consistently reported “38 missing, 38 extra”. The sampling audit
  was repealed and deleted in 0.6-19 (the script, data file, and the generator’s dependency on it
  were all removed); see “Repeal of the Sampling Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts the whole batch.

## Repeal of the Sampling Audit (0.6-19)

**Criterion**: the sampling audit (0.6-10) measured the “true support rate” of clause citations, but
the retention rules after 0.6-18 no longer inspect citations (they were changed to “for the same
symbol and result category, retain only the strongest one”, reducing the test pool from 3408 to
2055). Of the original 40 sample items, 38 no longer existed, so the audit’s `--check` consistently
reported “38 missing, 38 extra”—retaining it would only produce continuing alarms. It was therefore
**repealed and deleted**.

**What was deleted**: the sampling-audit script under `scripts/dev/` (the one that reproduced the
40-item sample with a fixed seed) and its JSON data file (the 40 item-by-item decisions and their
bases).

**The generator’s dependency was removed in sync**: the generator previously read that JSON and
classified “audit-confirmed genuine support” as “Clause gate + Keep”; that branch has been
deleted—**clause citations no longer have an “audit-confirmed” tier** and uniformly fall into
“Clause / Pending verification”.

**Distribution changes** (all 2055 repository-wide `#[test]` functions were left unchanged, and the
TSV was regenerated): Keep 746 → 737 (−9: the 9 entries previously confirmed by the audit as
genuinely supporting moved back to Pending verification); Pending verification 1045 → 1054; by gate:
Clause 811 → 808, Pending judgment 496 → 499 (the others are unchanged: Incident history 313 · Proof
424 · Neither passes 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` reverts the whole batch.

## Rerunning and Self-Checks

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (two runs are byte-for-byte identical)
python scripts/dev/test-gate-census.py --check    # Before CI/commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-tests (does not scan the repository)
```

The generator performs lexical judgment only, and states its boundaries honestly:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as having “no assertions”—the first batch of 26 (21 retired / 5 kept) and the second
  batch of 18 (11 retired / 7 kept) were each manually reviewed at the function-body level.
- **Function-body boundaries (fixed in 0.6-13)**: the generator used to balance braces when
  extracting a function body, and braces in strings/character literals and comments would throw the
  balance off, truncating the body—`test_prefix_info_lbrace` (which contains `tokenize("{")`) was
  once truncated to the end of the file, causing the “assertion summary” to capture the rest of the
  file and the test to be bucketed as “Pending judgment” rather than “Retirement candidate”. It now
  masks non-code regions (strings / character literals / line comments / block comments / raw
  strings) before balancing; see “Generator Correction” below.
- **Delegated-assertion recognition (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` has no digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions, and tests containing such calls
  were misclassified as having “no assertions in the body”. At the time, 4 of the 15 repository-wide
  “Neither passes” tests actually contained assertions (3 numbered helpers + 1 in the `match` +
  `panic!` form). The name portion is now `[A-Za-z0-9_]*`, and the `match` + `panic!` form is also
  recognized; see “Generator Criterion Correction” below.
- **The proof gate is not judged**: every test that misses both gates falls into “Pending judgment”
  or “Retirement candidate”; the generator never declares that it “should be deleted”.
- **No cross-file deduplication**: exact duplicate tests are not identified in this table (0.6-2
  measured them at only 1%, so they do not constitute a retirement surface).
- **Every clause citation is pending verification**: a verifiable section number in the file header
  only means that the test “maps to a clause that can be named explicitly”; it does not mean that
  the citation supports the assertion. The machine does not judge whether the citation supports the
  assertion, so even a clause-gate match stops at “Pending verification” and must be checked by a
  human before it can be kept.
