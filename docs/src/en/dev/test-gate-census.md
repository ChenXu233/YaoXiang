---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository using the three gates from Part 7 of
  coding-rules, and provide a retirement candidate list and retirement records for each batch'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **three gates (clauses / incident history / proof)** in
[coding-rules Part 7](./coding-rules.md), and produce a rerunnable item-by-item decision table. A
sampling audit of the clause gate (40 items) measured the proportion of cases that “cited a clause
but could not support the assertion” and consequently **tightened the criteria**; that audit was
abolished and deleted in 0.6-19. **This page and the generator itself are read-only and do not
modify tests**; test retirement is executed in batches (one commit per batch, with the whole batch
eligible for `git revert`), and the results are recorded in the retirement subsections below and in
the commit messages.

Outputs:

| Output                              | Purpose                                                                                      |
| ----------------------------------- | -------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Item-by-item decision table (2,727 rows plus the header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                                  |

## Criteria

### Scan Scope

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root-package lib (`src/`, including each module’s `tests/` subdirectory) | 2123                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2727** (341 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                     | Meaning of a hit                                            |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | The first 25 lines of the file header contain a **verifiable entry**: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); files under `src/**/tests/` are evaluated together with the parent `mod.rs`                                                                                                                                                                                                          | The test maps to an individually identifiable clause        |
| Incident-history gate             | The test body, function doc comment, or **file header** (such as `//! 回归背景：issue #226`) explicitly identifies a real failure: `issue #N` / `回归` / `缺陷` / `事故` (regression / defect / incident)                                                                                                                                                                                                                                             | The test pins an incident that has actually occurred        |
| Proof gate                        | **Cannot be determined mechanically** (“If this were replaced with another correct implementation, would it turn red?” requires human judgment). Therefore, anything that fails both preceding gates is classified as “pending adjudication,” not automatically as “none pass”; the only exception is a body with **not a single assertion** (including no delegated `assert*`/`check*` helper, no `match` + `panic!` form, and no `#[should_panic]`) | Only “pending adjudication” or “no assertions” can be given |

Rationale for tightening: the clause gate in 0.6-9 only required the words `§`/`RFC`/`SPEC` to
appear in the file header, so 2,403 tests were classified as “retain.” The measured result of a
40-item stratified sample in 0.6-10: **half of the citations could not support the assertions**
(miscitation rate: 50%; 95% interval: 35%–65%)—this measurement was the **direct reason for
abandoning “retain on the strength of a single citation.”** After tightening, only section-number
forms are recognized, and **a citation still does not mean that the citation supports the
assertion**—all clause citations stop at “pending verification.” (The sampling audit was abolished
and deleted in 0.6-19: as the test pool shrank through retirement, 38 items from the original sample
no longer existed, so `--check` consistently reported “38 missing, 38 extra”; the conclusion is
retained, but the audit artifact is no longer maintained.)

Rationale for the file-header radius: this repository’s convention is to put “regression context:
issue #N” in the file header rather than repeat it in each test comment; looking only at each test’s
own comments would miss the incident history of the entire file (measured: 7 items at the test’s own
radius, 291 items at the file-header radius).

### Categories That Are Never Retired

The following tests are **always “retain”** and do not enter the retirement candidate pool:

- Corpus `tests/yaoxiang/**` and project-form `tests/project-corpus/**` (behavior-criterion
  surfaces);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and traversing a process boundary);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                           |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Retain               | Only two possibilities: an incident-history gate hit, or a proof gate (spec-shaped directory / current-state anchor name)                                                                         |
| Pending verification | Cites a clause (with a section number, or only an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a human before it can be retained |
| Retirement candidate | Fails both gates and the assertion surface contains only weak assertions (or no assertions)—a candidate is not a verdict; the proof gate still requires manual review                             |
| Pending adjudication | Fails both gates but has strong assertions; the machine reaches no conclusion                                                                                                                     |

## Distribution (Regenerated 2026-11-28; 4.9.9 narrowing round, batch 8.7, 4.9.10-L0, init test rewrite, batch D53, L3-1b S1, and batch 7e)

| Gate                                                                 | Count    |
| -------------------------------------------------------------------- | -------- |
| Clause (with a verifiable entry)                                     | 1037     |
| Incident history                                                     | 568      |
| Proof (spec-shaped criterion surface / current-state anchor)         | 574      |
| Pending adjudication (fails both gates; proof awaits human judgment) | 542      |
| None pass (no assertions in the body)                                | 6        |
| **Total**                                                            | **2727** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Retain               | **1142** | 737 (+405)                        |
| Pending verification | 1297     | 1054 (+241)                       |
| Pending adjudication | 229      | 203 (+26)                         |
| Retirement candidate | 59       | 61 (−2)                           |
| **Total**            | **2727** | 2055 (+651)                       |

Composition of retained tests: 1142 = 574 from the proof gate (518 spec-shaped criterion surfaces +
56 current-state anchors) + 568 from the incident-history gate. File-level criteria: of 341 test
files, the **clause gate** covers **125**; using the first 25 lines of the file header (including
the parent `mod.rs` for `src/**/tests*/`), a verifiable section number (`§` + digits) can be
detected in **147**.

Regeneration reconciliation: the previous committed table had 2,712 entries, while the current tree
has 2,727; 15 are present in the current tree but absent from the old table, 0 are present in the
old table but absent from the current tree, and 0 rows with the same key have recalculated content.
Every key is recorded in the TSV.

The latest full regeneration (after batch 7e was committed): 2,727 rows, consistent with the current
tree key by key (`--check` exits with 0).

### Two Criteria Changes at the 0.6-19 Tightening Point

1. **Composition of retained tests**: 737 = 424 from the proof gate (390 spec-shaped criterion
   surfaces + 34 current-state anchors) + 313 from the incident-history gate. Of the original 2,897,
   the 2,121 tests “retained on the strength of a single citation” now become pending-verification
   tests (after the sampling audit was abolished in 0.6-19, the 9 tests previously classified as
   “audit-confirmed as genuinely supporting the assertion” also moved to pending verification—clause
   citations no longer have an “audit-confirmed” tier).
2. **Incident-history gate 47 → 313**: this was not an increase in incident history, but a
   **relabeling**—in 0.6-9, cases where the clause and incident-history gates both hit were recorded
   under the clause gate; after tightening, a clause hit no longer automatically meant retention, so
   these cases had to be caught by the incident-history gate, and their counts were folded into
   incident history. The total number retained was therefore unaffected (these tests were also
   retained in 0.6-9).

File-level criteria at the time: of 285 test files, **136** had a verifiable section number in the
file header (the old 0.6-9 criterion found 221).

## Reconciliation Between the 0.6-19 Snapshot and the 0.6-2 Survey

The 0.6-2 survey figures form the baseline for the first comparison. The reasons for each difference
are itemized below:

| Item                            | 0.6-2 Survey                  | This Review                                                                                                                    | Reason for the Difference                                                                                                                                                                                                                                                                          |
| ------------------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                                           | 2 tests were added after 0.6-2 (check-criterion regression tests for E3005 and E3022), then 21 no-assertion tests were deleted under the first batch, 11 weak-assertion tests under the second batch, 2 under the third batch, 1 under the fourth batch, and 1,353 under the fifth batch; net −388 |
| Number of test files            | 296                           | 285                                                                                                                            | 11 entire files were retired in the fifth batch                                                                                                                                                                                                                                                    |
| Clause gate                     | 227 files (77%)               | 136 files after tightening; 808 by item                                                                                        | The criterion was tightened from “the words appear” to “a section number `§<number>` is present,” nearly halving the file count; the fifth batch retired 902 tests, and after the sampling audit was abolished in 0.6-19, another 9 moved from “retain” to pending verification                    |
| Incident-history signals        | 442 items / 148 files         | 313 within the gate; three radii: 7 at test scope / 291 at file-header scope (38 files) / 1,218 across entire files (79 files) | The 148 files in the 0.6-2 survey were broader even than the current 79 files at the entire-file radius, and its vocabulary/scope was larger; this review strictly uses the four card-surface signals                                                                                              |
| Characterization strong signals | 1176 (34%) / strong 631 (18%) | Not tested                                                                                                                     | Characterization is not one of the three gates; mixing it into a retirement conclusion would exceed the mandate                                                                                                                                                                                    |
| Weak assertions                 | 473 (14%)                     | See the “assertion summary” column in the TSV                                                                                  | The unit of counting differs: 0.6-2 counted patterns that only assert `is_ok`/`is_some`; this review uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                                                                |

## 0.6-19 Conclusion Summary (Historical)

1. **The clause gate had previously been significantly overstated**: under the old criterion, 2,403
   tests were classified as “clause/retain,” and the measured miscitation rate was 50% (95%
   interval: 35%–65%)—“a citation” and “a citation that supports the assertion” are different
   things, and the old criterion treated the former as the latter. (The sampling audit was abolished
   and deleted in 0.6-19, and the conclusion was retained; see “Abolition of the Sampling Audit”
   below for the reason for deletion.)
2. **After tightening, approximately 1,202 tests genuinely have clause support** (range: 846–1,557);
   the rest must either acquire pending verification or be judged through the incident-history/proof
   gates.
3. **The retirement surface is unrelated to miscited clauses**: the 136 candidates had no clause
   basis (no assertions/weak assertions); the first batch retired 21 tests, the second batch retired
   11, the generator correction added 1, the third batch removed 2, the 0.6-15 criterion correction
   removed 3, the redundancy scan removed 1, the fourth batch removed 1, and the fifth batch removed
   1,353 (using the mechanical rule “for the same symbol and result category, retain only the
   strongest test”), bringing the candidates down to 61. The fifth-batch retirement count by gate
   was: clause 902 · pending adjudication 428 · proof 23 · incident history 0 · none pass 0—**not a
   single incident-history test or zero-assertion test was retired**.
4. **The incident-history gate is the largest legitimate source of retention** (313 tests): it
   demonstrates that “pinning an actual incident” is more reliable than “copying a sentence from an
   RFC.”

## First Retirement Batch (Executed 2026-11-06)

**First batch: 21 of the 26 “no-assertion” tests under `src/frontend/core/types/eval/tests/` were
retired, and 5 were retained** (a single directory cluster; criterion = all **three gates fail**
under Part 7 of coding-rules: no clause gate, no incident-history gate, and no assertions in the
body). The item-by-item decisions and “same-slot coverage” are provided by the 0.6-11a decision
table (21 deletions / 5 retentions); this page does not repeat the rationale and records only the
execution result.

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
- **5 tests retained** (the decision table could not identify replacement coverage, so they are
  retained under the hard requirement): `test_normalizer_context`,
  `test_type_computer_new_and_with_config`, `test_type_computer_compute_type_ref`,
  `test_reducer_reduce_type_ref_with_alias_chain`, `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Second Retirement Batch (Executed 2026-11-06)

**Second batch: 11 of the 18 “weak-assertion” tests in `src/frontend/core/parser/pratt/tests/nud.rs`
were retired, and 7 were retained** (a single directory cluster = the Pratt prefix-parsing module;
criterion = all **three gates fail** under Part 7 of coding-rules: no clause gate, no
incident-history gate, and the body contains only weak assertions such as `is_some()` /
`is_none()`—neither binding power nor the parse result is pinned).

The difference from the first batch must be stated clearly: these tests **do contain assertions**
and are not empty. The retirement rationale is “all three gates fail + zero criterion increment,”
not “no assertions”—every test selected for deletion names a **unit-level replacement test in the
same module**.

- **11 tests retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The replacement
  coverage for each follows the **same** `parse_expression → parse_prefix → prefix_info` path and
  has stronger assertions (pinning the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, and `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 tests retained**: `test_prefix_info_match`, `test_prefix_info_unsafe`,
  `test_prefix_info_spawn`, `test_prefix_info_at`, `test_prefix_info_break_removed`,
  `test_prefix_info_continue_removed`, `test_prefix_info_eof`—the item-by-item rationales are in the
  commit message (Task 0.6, second test-retirement batch). The retention rationales fall into two
  categories: **the proof gate holds** (the first three, together with `test_prefix_info_at` /
  `test_prefix_info_eof`, are the sole **unit-level** inspection points for the corresponding facts,
  and the repository contains no second unit test that observes them), and **the incident-history
  gate holds** (`break_removed` / `continue_removed` explicitly name #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Generator Correction (0.6-13)

**`body_span` truncated function bodies when braces appeared in literals — fixed.**

The old implementation obtained function bodies by balancing braces without distinguishing real code
from braces in string/character literals or comments. Forms such as `tokenize("{")` are common in
Rust source, so balancing was thrown off and the function body was truncated: the body of
`test_prefix_info_lbrace` was formerly truncated to the end of the file, causing the “assertion
summary” to consume the remainder of the file (`matchesx11 assertx24 …`) and the test to be binned
as “pending adjudication” rather than “retirement candidate”—**the wrong bin**.

Fix: add `mask_noncode`, first replacing the contents of string literals / character literals / line
comments / block comments (which may be nested) / raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (newlines are preserved as-is, so line numbers and line counts do not
change), and then balance the braces. **The mask is used only to locate function-body boundaries**;
assertion statistics and the incident-history gate still read the original text, and the criteria
themselves have not been relaxed. `--selftest` gained 4 cases (paired / unpaired braces in an
ordinary string, braces in a raw string and block comment, and braces in a character literal), each
followed by a sentinel test—the old implementation found the wrong boundary in all 4 cases, while
the new implementation finds all of them correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** file by file and
test by test (the old implementation balanced braces directly, while the new implementation balances
them after masking); any test with differing boundaries is affected. Of 3,413 tests in the
repository, the old implementation found the wrong boundary for **11**:

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

Of the other 9, **8 affected only the “assertion summary” column** (their bin did not change), while
**1 did not even change the summary** (`test_fstring_close_brace_escape_preserved_raw`: the old
implementation omitted the closing `}` line, and that line contained no assertion keywords). The 8
summary changes are itemized in the commit message.

| Test Name                   | File                                          | Old Bin                                     | New Bin                                         | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| --------------------------- | --------------------------------------------- | ------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending adjudication / Pending adjudication | Pending adjudication / **Retirement candidate** | The actual function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; the file header and function body have no § section number and no issue/regression/defect/incident signal ⇒ both the clause and incident-history gates fail, and the body contains only a weak assertion such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires manual review)                                                         |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident history / Retain                   | Clause / **Pending verification**               | The old implementation truncated the body to the end of the file and swallowed text containing “regression / defect / incident,” incorrectly triggering the incident-history gate; the actual function body contains no such wording ⇒ the incident-history gate no longer holds; the file header has verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate hits, but the citation has not been confirmed by the sampling audit ⇒ pending verification |

The distribution changes accordingly: clause 1713 → 1714 · incident history 314 → 313 · retain 776 →
775 · pending verification 2121 → 2122 · pending adjudication 412 → 411 · retirement candidate 104
→ 105. **The repository-wide `#[test]` count remains unchanged (3413)**—this round changed only the
implementation of the criteria, not any test.

## Third Retirement Batch (Executed 2026-11-07)

**Third batch: of the 5 retirement candidates under `src/middle/passes/codegen/tests/`, 2 were
retired and 3 were retained.** (A single directory cluster = the `middle::passes::codegen` module
under test; criterion = all **three gates fail** under Part 7 of coding-rules.)

This batch continues the **retention criterion** established by the preceding batch: a unit-level
observation point may only be replaced by another **unit test**; corpus/integration tests operate at
the process-boundary layer and cannot replace a unit-level observation point.

- **2 tests retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—its function body is **literally
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (after comments are
    removed, only the function name differs): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that “the `execute_module` entry
    path returns non-Void,” but the body never observes the return value of `execute_module`
    (`run_fixture` discards it with `.expect(...)`)— the claimed intent was not implemented, and the
    function body is a pure duplicate ⇒ zero criterion increment. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file covers it strictly:
    a positive lookup (`lookup("a").is_some()`), a negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the inserted `Symbol` shape (`symbol("a", Int(64), Local(0), false, 0)`
    is identical to the inline `Symbol` in this test). “A name that was never inserted cannot be
    found” and “a name cannot be found after being popped” follow the same path in
    `SymbolScopeManager::lookup` (look through each layer with `scope.get(name)` and return `None`
    if not found) ⇒ this test contributes zero increment. Replacement coverage =
    `test_scope_nesting`.
- **3 tests retained**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that
    “executes without a serialization direct connection” (the serialization variant is a different
    path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for serialization
    round-tripping (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit-level observation point for the
    `id > 255` rejection branch of `OperandResolver::to_reg` (the bytecode layer has its own
    independent overflow checks and does not cover this function; no second test in the repository
    drives `to_reg` out of range). `is_err()` is a discriminating assertion (removing the guard
    would turn it red), not merely a check that nothing panicked.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

### One Census Blind Spot Exposed by This Batch (**Fixed in 0.6-15**, see “Generator Criterion Correction” below)

The census’s recognition of “delegated assertions” uses `\b(assert|check)[A-Za-z_]*\s*\(` and
**cannot match helper names containing digits**: `[A-Za-z_]*` does not include digits, so a call
such as `assert_slot_is_42(&interp)` **is not recognized as an assertion**— 2 of the 5 candidates in
this batch were consequently mis-binned as “no assertions in the body.”

After all 15 repository-wide “none pass” cases were reviewed item by item (with helper names
broadened to include digits and `match` + `panic!` additionally recognized), **4 actually contained
assertions**—3 were the numeric helpers above (`assert_slot_is_42`), and 1 was the `match` +
`panic!` form in `test_weak_from_arc` in `src/backends/interpreter/tests/weak.rs`
(test-specification rule 6.1 requires `assert!(matches!(...))`, but it does in fact assert). The
other 11 genuinely contained no assertions at all.

**Directly removing these 4 under the machine’s “none pass” classification would have wrongly
deleted tests with criteria**—the item-by-item human review in this batch prevented that; the
generator-side fix is described in “Generator Criterion Correction (0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots were fixed: helper names containing digits, and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411); it
changes only how the generator recognizes whether a body contains assertions.

### What Was Fixed

1. **Digits were permitted in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing them were
   treated as “no assertions in the body.”
2. **Added the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the assertion is written
   in one of the arms, but `assert!` is not visible lexically (test-specification rule 6.1 requires
   `assert!(matches!(...))`, but existing code contains this form). Recognition method: scan the
   expression from the `match` keyword to the first `{`, determine the block boundary by **brace
   balancing**, and count it only if `panic!(` appears within the block. **The decision is made on
   masked lines**— braces in strings and comments do not participate in balancing, and `panic!` in
   comments is not treated as an assertion.

### Bin Differences: 3 (the gate and disposition changed)

| Test Name                                           | File                                           | Old Bin                          | New Bin                                     | Basis                                                                                                                                                                                                                                                        |
| --------------------------------------------------- | ---------------------------------------------- | -------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | None pass / Retirement candidate | Pending adjudication / Pending adjudication | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | None pass / Retirement candidate | Pending adjudication / Pending adjudication | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern could not match; it is now recognized as an assertion                                                                                     |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | None pass / Retirement candidate | Pending adjudication / Pending adjudication | Same as above (the serialization round-trip test)                                                                                                                                                                                                            |

Another **77 tests changed only the “assertion summary” column** (the gate and disposition remained
unchanged): most gained `match_panicxN` in the summary (they contain the `match` + `panic!` form),
and 4 gained `helperxN` (3 in `src/driver/tests/manifest_error.rs` and 1 in
`src/package/tests/cache.rs`—helper names containing digits).

**The relaxation changes only the “none pass” bin**: that bin is the machine’s **high-confidence
verdict** (not a single assertion in the body), and its criterion is a hard requirement; a
“retirement candidate” is only a pool awaiting manual review, so its size is not a verdict. All
other bins, including the weak-assertion pool, continue to follow the original criteria. As a
result: gate none pass 14 → 11 · pending adjudication 923 → 926; disposition retirement candidate
103 → 100 · pending adjudication 411 → 414.

### Existing 11 “None Pass” Cases: Item-by-item Review Conclusions

After all 11 repository-wide “none pass” cases were read and reviewed in the original text, **all 11
genuinely contained not a single assertion** (including no `assert*` / `check*` helper, no `match` +
`panic!` form, and no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                   | Review Conclusion                                            |
| ----------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 uses of `.expect(...)`, only guaranteeing that it does not crash                            | Genuinely no assertions (only proves that it does not panic) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                | Genuinely no assertions                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer`s, obtains their contexts, and discards them                     | Genuinely no assertions                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                           | Genuinely no assertions                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                     | Genuinely no assertions                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                            | Genuinely no assertions                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` with invalid content and discards the result                    | Genuinely no assertions (only proves that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                              | Genuinely no assertions (only proves that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                        | Genuinely no assertions (only proves that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (within the wasm32 cfg gate)                          | Genuinely no assertions                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertions                                      |

Five of these (`test_normalizer_context` and the 4 in reducer.rs) were retained because the 0.6-11a
decision table **could not identify replacement coverage**; this review reaches the same conclusion.

### Reverse Spot Check: Did the Relaxation Go Too Far?

Five entries were read in their original text from the 88 items newly recognized as assertions,
using a fixed seed (20261107), to confirm that they really assert:

| Spot-Checked Entry                                          | File                                                 | Assertions in the Original Text                                                                                                                |
| ----------------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching `match` arm; `panic!("Expected AssocType")` if it does not match                                        |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm; `panic!("expected InvalidPackage, got …")` if it does not match                    |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | `assert_eq!` in the matching arm compares the constraint type; `panic!("不是 Dynamic 变体…")` if it does not match                             |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm; `panic!("Expected TokensDelta variant")` if it does not match                          |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm validate the error code and message; `panic!("不是 Disproved…")` if it does not match |

All 5 assert a concrete value or concrete variant (replacing the implementation with another correct
path would turn them red)—**the relaxation did not go too far**. The reverse cases were also added
to `--selftest`: a `match` without `panic!`, and a `panic!` that appears only in a comment; neither
may be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

When the generator is reverted to the old implementation (`HELPER_RE` without digits and with
`match` + `panic!` recognition disabled), the newly added `--selftest` cases **all turn red** and
the exit code is 1; the new implementation passes all of them.

### Rollback

The generator and census table are in the same commit; `git revert <sha>` restores the whole thing.

## Fourth Retirement Batch (Executed 2026-11-07)

**Fourth batch: of the 10 “retirement candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend’s test directory), 1 was retired and 9 were
retained** (a single directory cluster; three files were read together before making the decision:
`tests/bytecode_load.rs` 4 items · `tests/weak.rs` 2 items · `executor/tests/debug.rs` 4 items).
Criterion = the **three gates** in Part 7 of coding-rules; retention criterion = “a unit-level
observation point may only be replaced by another unit test; corpus/integration tests operate at the
process-boundary layer”; **a deletion must identify a concrete replacement coverage test name; if
none can be identified or the test cannot be understood, retain it**.

- **1 test retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, followed by one step
  and then `capture_stack`), and its assertions are **strictly stronger**: it asserts that the stack
  is nonempty and that `stack[0].function_name == "main"` (pinning the top-of-stack position),
  whereas the listed test asserts only `stack.iter().any(|f| f.function_name == "main")` (any
  position is sufficient) —the former being true necessarily implies the latter ⇒ zero criterion
  increment.
- **9 tests retained**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic`
  · `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each one in its
  original text, each is the **sole unit-level observation point** for a property (the load
  magic-number/version branches differ, the YXBC routing layer, both sides of Weak’s Some/None,
  stepping with no frames, step_out popping the stack, and running to completion with an empty
  stack). No unit test capable of replacing them could be identified ⇒ retained under the hard
  requirement. The item-by-item rationales are in the commit message (the decision table is not
  included in this page).
- **Figures**: repository-wide `#[test]` 3409 → 3408; 1 deletion = `-#[test]` on 1 line and
  `+#[test]` on 0 lines in the `git diff`; the dominating test `test_capture_stack_during_execution`
  passes independently. Census: retirement candidate 99 → 98 · pending adjudication 925 → 924 ·
  clause 1713 · incident history 313 · proof 447 · none pass 11 · pending verification 2121 ·
  retain 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Fifth Retirement Batch (Executed 2026-11-07)

**Fifth batch: bulk retirement of 1,353 unit tests using the mechanical rule “for the same symbol
and result category, retain only the strongest test”** (only unit tests under `src/**/tests/**` were
changed; not a single test on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (mechanically executed, without item-by-item argument)**:
  - Tested symbol = a repository-internal symbol called in the test body (including one level of
    propagation through a helper function in the same file); when multiple symbols are reached,
    classify by the **primary symbol** (the first repository-internal call). Symbols are qualified
    by their path within the repository and are not merged across modules by bare name.
  - Result category = a concrete value / a particular error code (E code) / a particular variant
    match / only ok / only err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when equally strong within a group, retain
    the first one to appear in the file.
  - When a test asserts multiple categories simultaneously (making the primary category
    indeterminate), **retain it**; do not guess.
- **Three exceptions (not a single item exempted)**: ① the name or comment identifies `issue #` /
  regression / defect / incident (incident-history gate); ② a symbol has only one test (deleting it
  would leave the symbol without a unit-level observation point); ③ a `#[should_panic]` test is the
  symbol’s only should_panic case. The 313 incident-history tests and the 11 “none pass” tests had
  **0 retirements**.
- **Figures**: repository-wide `#[test]` 3408 → 2055; 1,353 deletions = `-#[test]` on 1,353 lines
  and `+#[test]` on 0 lines in the `git diff`. Census by gate (current values after the 0.6-19
  abolition of the sampling audit): clause 808 · incident history 313 · proof 424 · pending
  adjudication 499 · none pass 11. Census by disposition (current values): retain 737 · pending
  verification 1054 · pending adjudication 203 · retirement candidate 61. The retirement count for
  this batch by gate was: clause 902 · pending adjudication 428 · proof 23 (because 0.6-19
  reclassified 9 “audit-confirmed as genuinely supporting the assertion” entries as pending
  verification, the current values by gate/by disposition differ from those when the fifth batch was
  closed).
- **Cleanup accompanying the deletions**: helper functions used only by deleted tests, imports that
  became invalid, and struct fields that became unused were all removed
  (`cargo clippy --all --all-features --all-targets -- -D warnings` reported 0 warnings).
- **Two out-of-bounds deletions that must be recorded**: the census originally classified 29 tests
  as “retain,” but the rule for this batch still deleted them—23 current-state anchors (names
  containing unchanged/anchor/guard/preserve/baseline, many of which were false positives caused by
  **domain words** such as “guard” or “preserves,” for example `test_while_guard_scope_balanced`)
  and 6 entries whose clause support had been confirmed by the audit. This batch followed the closed
  list of three exceptions on the card surface and did not provide additional protection for these
  two categories; to restore them, fill them back in from the retain/delete comparison table on this
  page.
- **Failure and abolition of the sampling audit**: the 40-item sample was drawn from the test pool
  using a fixed seed; after the fifth batch retired tests, 38 items in the original sample no longer
  existed, so the audit’s `--check` consistently reported “38 missing, 38 extra.” The sampling audit
  was abolished and deleted in 0.6-19 (the script, data file, and the generator’s dependency on it
  were all removed); see “Abolition of the Sampling Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Sixth Retirement Batch (Executed 2026-11-26)

**Sixth batch: the mechanism rollback from the 4.9.9 narrowing round—28 tests retired along with the
mechanism they tested, while 4 prohibition criteria had never been committed.** This batch does not
use the three gates to decide whether the tests “should be retained”; instead, the mechanism under
test was **rolled back as a whole**: the criteria written for the first six rounds of 4.9.9 for
downstream ambiguity mechanisms (candidate-provider priority / candidate-signature comparison /
owner-compatibility inference / IR-side named-argument fallback / import-name resolution-point E5006
prohibition) were retired along with the mechanism itself—when the mechanism does not exist, the
test has no object under test. These deletions are registered under the “batch” provision of
coding-rules 7.3 and do not enter the retirement candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 (28 deleted: 27 downstream-mechanism
    criteria from the first six rounds and 1 alias-import named-argument criterion). The 2 retained
    tests pin the minimal fix for bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: all 4 E5006 prohibition criteria were retired—the
    file existed only in an intermediate 4.9.9 commit, and neither the parent commit nor that
    version’s committed table included it; after the prohibition was withdrawn, there was no object
    under test, so this batch does not reduce the number of rows in the committed table.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the candidate-injection surface for alias imports was rolled back, so the fixture’s
    behavior no longer holds); the corresponding corpus-difference baseline entry was deleted in the
    same batch. The baseline counts are stated separately: **relative to direct parent commit
    `ac810e97`, 405 → 404 lines** (the intermediate state had included this fixture); **relative to
    pre-4.9.9 baseline `08ccacc5`, 403 → 404 lines** (+1 base fixture
    `named-args-cross-unit-method`; the retired `alias-overload` never entered the final history).
- **Which Gate Applied**: the 2 retained tests = **incident-history gate** (they pin a real defect
  registered in 4.9.9: cross-unit named arguments could not obtain declaration-phase parameter
  names, leaving `E1014 available` empty); most of the 28 retired tests were originally
  incident-history tests under the first six rounds, but the deletion rationale **is not** the three
  gates (they were not judged worthless); rather, the mechanism under test was rolled back.
- **Replacement Coverage**: cross-unit named arguments for bare-name imports = the 2 retained
  driver-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity surface
  no longer has rejection logic (the requirement was rolled back), so there is no replacement
  criterion; the 4.9.8 dispatch-attribution criteria (10 driver tests + 5 integration tests) are
  retained unchanged.
- **Restoration**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the empty-name-table guard for
  `method_param_info` is retained; this form is unreachable in the public syntax and is a defensive
  guard, and the unit test merely prevents accidental deletion).
- **Figures** (after this batch was regenerated): repository-wide `#[test]` 2680 / 334 files; the
  previous committed table had 2688 entries, while the current tree has 2680 (−8); 1 is present in
  the current tree but absent from the old table, 9 are present in the old table but absent from the
  current tree, and 0 rows with the same key have recalculated content.
- **Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit); `git revert <sha>` rolls back the whole batch.

## Abolition of the Sampling Audit (0.6-19)

**Criterion**: the sampling audit (0.6-10) measured the “actual support rate” of clause citations,
but after 0.6-18 the retention rule no longer examines citations (after changing to “for the same
symbol and result category, retain only the strongest test,” the test pool shrank from 3408 to
2055). Of the original 40-item sample, 38 no longer existed, so the audit’s `--check` consistently
reported “38 missing, 38 extra”—retaining it would only produce continuous alarms. Therefore, it was
**abolished and deleted**.

**What was deleted**: the sampling-audit script under `scripts/dev/` (the one that reproduced the
40-item sample using a fixed seed) and its JSON data file (the item-by-item decisions and rationales
for all 40 items).

**The generator’s dependency was removed at the same time**: the generator previously read that JSON
and classified “audit-confirmed as genuinely supporting the assertion” as “clause gate + retain”;
that branch has been deleted—**clause citations no longer have an “audit-confirmed” tier** and
always fall into “clause / pending verification.”

**Distribution changes** (all 2055 repository-wide `#[test]` functions **remain unchanged**, and the
TSV has been regenerated): retain 746 → 737 (−9: the 9 previously audit-confirmed as genuinely
supporting the assertions fall back to pending verification); pending verification 1045 → 1054; by
gate: clause 811 → 808, pending adjudication 496 → 499 (the others remain unchanged: incident
history 313 · proof 424 · none pass 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` rolls back the whole batch.

## Reruns and Self-Checks

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (both runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # Before CI/commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs only lexical determination, and its boundaries are stated accurately:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test hides its assertion in a helper whose name does not use either prefix, it will be
  misclassified as having “no assertions”—the first batch’s 26 tests (21 retired / 5 retained) and
  the second batch’s 18 tests (11 retired / 7 retained) were all manually reviewed function body by
  function body.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator obtains a function body, it
  balances braces; braces in strings / character literals and comments throw off the balance and
  truncate the body—`test_prefix_info_lbrace` (which contains `tokenize("{")`) was formerly
  truncated to the end of the file, and the “assertion summary” consumed the remainder of the file,
  binning it as “pending adjudication” rather than “retirement candidate.” The generator now masks
  non-code regions (strings / character literals / line comments / block comments / raw strings)
  before balancing; see “Generator Correction” below.
- **Recognition of “delegated assertions” (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` does not include digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions—tests containing such calls were
  misclassified as having “no assertions in the body.” At the time, 4 of the repository’s 15 “none
  pass” cases actually contained assertions (3 numeric helpers + 1 `match` + `panic!` form). The
  name portion has now been broadened to `[A-Za-z0-9_]*`, and the `match` + `panic!` form has been
  added; see “Generator Criterion Correction” below.
- **The proof gate is not adjudicated**: every test that fails both preceding gates falls into
  “pending adjudication” or “retirement candidate”; the generator never declares that it “should be
  deleted.”
- **No cross-file deduplication**: exactly duplicated tests are not identified in this table (0.6-2
  measured only 1%, which does not constitute a retirement surface).
- **All clause citations are pending verification**: a verifiable section number in the file header
  only shows that “the test maps to an individually identifiable clause”; it does not mean that the
  citation supports the assertion. The machine does not determine whether the citation supports it,
  so even a clause-gate hit stops at “pending verification” and must be verified by a human before
  it can be retained.
