---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement-candidate list and retirement records for each batch'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6, “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **three gates (clause / incident history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), producing a rerunnable per-test decision table. A
40-case sampling audit of the clause gate once measured the proportion of tests that “cited a clause
but could not support the assertions” and was therefore used to **tighten the criteria**. That audit
was abolished and deleted in 0.6-19. **This page and the generator itself are read-only and do not
modify tests**; test retirement is performed in batches (one commit per batch, fully revertible with
`git revert`), and the results are recorded in each batch’s retirement subsection below and in the
commit messages.

Generated artifacts:

| Artifact                            | Purpose                                                                                  |
| ----------------------------------- | ---------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Per-test decision table (2,748 rows plus the header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                              |

## Criteria

### Scan Surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                 |
| ------------------------------------------------------------------------ | --------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 2,138                 |
| `tests/integration`                                                      | 463                   |
| `tests/formatter_integration.rs`                                         | 52                    |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                 |
| `tools/`                                                                 | 83                    |
| `benches/`                                                               | 9                     |
| **Total**                                                                | **2,748** (346 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                       | Meaning of a hit                                           |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | A **verifiable reference** appears in the first 25 lines of the file header: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); for files under `src/**/tests/`, the parent `mod.rs` is considered alongside them                                                                                                                                                                                               | The test maps to a clause that can be named explicitly     |
| Incident-history gate             | The test function body, function doc comment, or **file header** (such as `//! Regression context: issue #226`) explicitly names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                            | The test pins down an incident that has actually occurred  |
| Proof gate                        | **Cannot be judged by machine** (“If it were replaced by another correct implementation, would it turn red?” requires human judgment). Therefore, anything that fails the first two gates is classified as `Pending Judgment`, not automatically as `Neither Gate Passes`; the sole exception is a test with **no assertions whatsoever** (including no delegation to `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only `Pending Judgment` or `No Assertions` can be assigned |

Rationale for tightening: the clause gate in 0.6-9 only required the file header to **contain
`§`/`RFC`/`SPEC`**, so 2,403 tests were classified as `Keep`. The 40-case stratified sample measured
in 0.6-10 showed that **half of the citations could not support the assertions** (mis-citation rate:
50%, 95% interval: 35%–65%)—this measurement was the **direct reason for abandoning “keep based on a
single citation.”** After tightening, only the section-number form is accepted, and **having a
citation still does not mean the citation supports the assertion**—every clause citation is
classified only as `Pending Verification`. (This sampling audit was abolished and deleted in 0.6-19:
as the test pool shrank through retirement, 38 entries in the original sample no longer existed, so
`--check` consistently reported “38 missing, 38 extra”; the conclusion is retained, but the audit
artifacts are no longer maintained.)

Rationale for the file-header radius: this repository conventionally places text such as “Regression
context: issue #N” in the file header rather than repeating it in every test comment; looking only
at each test’s own comments would miss the incident history of an entire file (measured: 7 entries
within the test itself, 291 within the file header).

### Categories That Are Never Retired

The following tests are always classified as `Keep` and never enter the retirement-candidate pool:

- Corpus tests under `tests/yaoxiang/**` and project-form tests under `tests/project-corpus/**`
  (behavioral criterion surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and running across a process boundary);
- Tests whose names contain `unchanged` / `anchor` / `guard` / `preserv` / `baseline`, which serve
  as status anchors.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                       |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Only two sources: a hit on the incident-history gate, or a hit on the proof gate (spec-shaped criterion surface / status-anchor name)                                                         |
| Pending Verification | Cites a clause (with a section number, or merely cites an entire RFC without a section number), but the citation has not been manually confirmed—a human must verify it before it can be kept |
| Retirement Candidate | Both gates fail and the assertion surface contains only weak assertions (or no assertions)—a candidate is not a verdict; the proof gate still requires human review                           |
| Pending Judgment     | Both gates fail, but strong assertions are present; the machine provides no verdict                                                                                                           |

## Distribution (regenerated on 2026-11-28; 4.9.9 narrowing round, batch 8.7, 4.9.10-L0, init-test rewrite, batch D53, L3-1b S1, batch 7e, #593 criteria, and L3-1 identity switch)

| Gate                                                                 | Count     |
| -------------------------------------------------------------------- | --------- |
| Clause (has a verifiable reference)                                  | 1,048     |
| Incident history                                                     | 568       |
| Proof (spec-shaped criterion surface / status anchor)                | 574       |
| Pending Judgment (both gates fail; proof gate awaits human judgment) | 552       |
| Neither Gate Passes (no assertions in the body)                      | 6         |
| **Total**                                                            | **2,748** |

| Disposition          | Count     | Compared with 0.6-19 (2026-11-07) |
| -------------------- | --------- | --------------------------------- |
| Keep                 | **1,142** | 737 (+405)                        |
| Pending Verification | 1,320     | 1,054 (+266)                      |
| Pending Judgment     | 227       | 203 (+24)                         |
| Retirement Candidate | 59        | 61 (−2)                           |
| **Total**            | **2,748** | 2,055 (+693)                      |

Keep breakdown: 1,142 = 574 proof-gate cases (518 spec-shaped criterion surface + 56 status
anchors) + 568 incident-history-gate cases. File-level metric: of 346 test files, the **clause
gate** covers **128**.

Regeneration reconciliation: this regeneration matches the committed table key by key (0 added, 0
removed, 0 recomputed), with 2,748 rows. The key-by-key record is in the TSV.

The latest full regeneration (after the L3-1 identity switch and lower finalization were committed,
and upstream #591/#593/opcode convergence was merged) produced 2,748 rows and matches the current
tree key by key (`--check` exits with 0).

### Two Criteria Changes at the 0.6-19 Tightening Point

1. **Keep breakdown**: 737 = 424 proof-gate cases (390 spec-shaped criterion surface + 34 status
   anchors) + 313 incident-history-gate cases. Of the 2,121 tests in the original 2,897 that were
   “kept based on a single citation,” those are now classified as `Pending Verification` (after the
   0.6-19 sampling audit was abolished, the 9 tests previously classified as “audit-confirmed as
   genuinely supporting” were also moved to `Pending Verification`—clause citations no longer have
   an “audit-confirmed” tier).
2. **Incident-history gate 47 → 313**: this is not an increase in incident history, but **label
   realignment**—in 0.6-9, tests that hit both the clause and incident-history gates were recorded
   under the clause gate. After tightening, the clause gate no longer automatically yields `Keep`,
   so these tests must be caught by the incident-history gate and their counts folded into that
   gate. The total number kept is therefore unaffected (those tests were also classified as `Keep`
   in 0.6-9).

File-level metric at the time: of 285 test files, **136** had a verifiable section number in the
file header (the old 0.6-9 criterion covered 221).

## Reconciliation Between the 0.6-19 Snapshot and the 0.6-2 Survey

The figures from the 0.6-2 survey form the first comparison baseline. The differences and their
causes are itemized below:

| Item                            | 0.6-2 Survey                   | Current                                                                                                      | Reason for the Difference                                                                                                                                                                                                                                                                                |
| ------------------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3,443                          | 2,055                                                                                                        | 2 tests were added after 0.6-2 (the check-criterion regression tests for E3005 and E3022), then 21 assertion-free tests were deleted under the first batch, 11 weak-assertion tests under the second batch, 2 under the third batch, 1 under the fourth batch, and 1,353 under the fifth batch; net −388 |
| Number of test files            | 296                            | 285                                                                                                          | 11 entire files were retired in the fifth batch                                                                                                                                                                                                                                                          |
| Clause gate                     | 227 files (77%)                | After tightening: 136 files; 808 tests                                                                       | The criterion was tightened from “the token appears” to “there is a section number `§<number>`,” so the file count was nearly halved; the fifth batch retired 902 tests, and after the 0.6-19 sampling audit was abolished, another 9 tests moved from `Keep` to `Pending Verification`                  |
| Incident-history signals        | 442 tests / 148 files          | 313 tests in the gate; three radii: test itself 7 / file header 291 (38 files) / whole file 1,218 (79 files) | The 148 files in the 0.6-2 survey covered a wider range than the current whole-file radius of 79 files, because its vocabulary and scope were broader; the current scan strictly uses the four signals on the criterion card                                                                             |
| Characterization strong signals | 1,176 (34%) / strong 631 (18%) | Not measured                                                                                                 | Characterization is not one of the three gates; mixing it into the retirement conclusion would exceed the criteria’s authority                                                                                                                                                                           |
| Weak assertions                 | 473 (14%)                      | See the “Assertion Summary” column in the TSV                                                                | The counting units differ: 0.6-2 counted patterns that asserted only `is_ok`/`is_some`; the current scan uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                                                                  |

## 0.6-19 Conclusion Summary (Historical)

1. **The clause gate had previously been significantly overstated**: under the old criterion, 2,403
   tests were classified as “Clause/Keep,” while the sampling audit measured a 50% mis-citation rate
   (95% interval: 35%–65%)—“citation” and “a citation that supports the assertion” are different
   things, and the old criterion treated the former as the latter. (This sampling audit was
   abolished and deleted in 0.6-19; the conclusion is retained. See “Abolition of the Sampling
   Audit” below for the reason for deletion.)
2. **After tightening, approximately 1,202 tests genuinely have clause support** (range: 846–1,557);
   the rest either require pending verification or should have been classified through the
   incident-history or proof gate.
3. **The retirement surface is unrelated to clause mis-citations**: the 136 candidates have no
   clause support (no assertions/weak assertions); the first batch retired 21 tests, the second
   batch retired 11, the generator correction added 1, the third batch removed 2, the 0.6-15
   criteria correction removed 3, the redundancy scan removed 1, the fourth batch removed 1, and the
   fifth batch removed 1,353 (using the mechanical rule “for the same symbol and result category,
   keep only the strongest test”), reducing the candidate count to 61. The fifth batch’s retirement
   count by gate was: Clause 902 · Pending Judgment 428 · Proof 23 · Incident History 0 · Neither
   Gate Passes 0—**not a single incident-history-gate or zero-assertion case was removed**.
4. **The incident-history gate is the largest legitimate source of `Keep`** (313 tests): it
   demonstrates that “pinning down a real incident that occurred” is more reliable than “copying a
   line from an RFC.”

## First Retirement Batch (Executed on 2026-11-06)

**First batch: of the 26 assertion-free tests in `src/frontend/core/types/eval/tests/`, 21 were
retired and 5 were kept** (single directory cluster; criterion = all **three gates fail** under Part
Seven of coding-rules: no clause gate, no incident-history gate, and no assertions in the body). The
per-test decisions and “same-cell coverage” mappings are provided by the 0.6-11a decision table
(delete 21 / keep 5). This page does not repeat the reasons; it records only the execution result.

- **21 tests retired** (`normalizer.rs`: 2; `reducer.rs`: 19): `test_normalizer_with_config`,
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
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Second Retirement Batch (Executed on 2026-11-06)

**Second batch: of the 18 weak-assertion tests in `src/frontend/core/parser/pratt/tests/nud.rs`, 11
were retired and 7 were kept** (single directory cluster = the Pratt prefix-parsing module;
criterion = all **three gates fail** under Part Seven of coding-rules: no clause gate, no
incident-history gate, and only weak assertions such as `is_some()` / `is_none()` in the
body—neither binding power nor the parse result is pinned down).

The difference between this batch and the first batch must be stated clearly: these tests **contain
assertions** and are not empty tests. The reason for retirement is “all three gates fail + zero
criterion increment,” not “no assertions”—every test approved for deletion identifies a **unit-level
replacement coverage test in the same module**.

- **11 tests retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Every replacement
  follows the **same** `parse_expression → parse_prefix → prefix_info` path and makes stronger
  assertions (pinning the AST shape rather than merely checking `is_some`): `test_parse_int_literal`
  / `test_parse_float_literal` / `test_parse_string_literal` / `test_parse_bool_true` /
  `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` / `test_parse_return_expr`
  in the same file, plus `test_grouped_expr` / `test_tuple` / `test_list_literal` /
  `test_list_empty` / `test_ref_expr` in `src/frontend/core/parser/tests/expressions.rs`.
- **7 tests kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the commit message for the per-test reasons (task 0.6, second
  test-retirement batch). The reasons fall into two categories: **the proof gate applies** (the
  first three, together with `test_prefix_info_at` and `test_prefix_info_eof`, are the corresponding
  facts’ **only unit-level observation points**, and the repository contains no second unit test
  that observes them), or **the incident-history gate applies** (`break_removed` /
  `continue_removed` explicitly name #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Generator Correction (0.6-13)

**`body_span` truncated function bodies when braces appeared inside literals—fixed.**

The old implementation located function bodies by balancing braces without distinguishing real code
from braces in strings, character literals, or comments. Source such as `tokenize("{")` is common in
Rust, so the balance was skewed and the function body was truncated: the body of
`test_prefix_info_lbrace` was previously truncated at the end of the file, causing the “Assertion
Summary” to capture the rest of the file (`matchesx11 assertx24 …`) and the test to be binned as
`Pending Judgment` rather than `Retirement Candidate`—**a binning error**.

Fix: add `mask_noncode`, which first replaces the contents of string literals, character literals,
line comments, block comments (which may be nested), and raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (preserving newlines, line numbers, and line counts), then balances braces.
**The mask is used only to locate function-body boundaries**; assertion counting and the
incident-history gate still read the original source, so the criteria themselves were not relaxed.
`--selftest` now has 4 cases (paired and unpaired braces in ordinary strings, braces in raw strings
and block comments, and braces in character literals), each followed by a sentinel test—the old
implementation found the wrong boundary in all 4 cases, while the new implementation finds all of
them correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** test by test and
file by file (the old implementation balanced braces directly; the new implementation balances them
after masking). Any test with different boundaries was affected. Of 3,413 repository-wide tests, the
old implementation found the wrong boundary for **11**:

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

### Binning Differences Before and After the Fix: 2 Tests

Of the other 9 tests, **8 affected only the “Assertion Summary” column** (their binning did not
change), while **1 did not even change its summary**
(`test_fstring_close_brace_escape_preserved_raw`: the old implementation omitted the closing `}`
line, which contains no assertion token). The 8 summary changes are listed individually in the
commit message.

| Test Name                   | File                                          | Old Bin                             | New Bin                                     | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| --------------------------- | --------------------------------------------- | ----------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending Judgment / Pending Judgment | Pending Judgment / **Retirement Candidate** | The real function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; the file header and function body contain no § section number, issue/regression/defect/incident marker ⇒ both the clause and incident-history gates fail, and the body contains only a weak assertion such as `is_some()` ⇒ Retirement Candidate (a candidate is not a verdict; the proof gate still requires human review)                                                                                            |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident History / Keep             | Clause / **Pending Verification**           | The old implementation truncated the function body to the end of the file, swallowing text containing “regression / defect / incident” and incorrectly classifying it under the incident-history gate; the real function body contains none of these terms ⇒ the incident-history gate no longer applies; the file header has verifiable references (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate is hit, but the citation has not been confirmed by the sampling audit ⇒ Pending Verification |

The distribution changes accordingly: Clause 1,713 → 1,714 · Incident History 314 → 313 · Keep 776 →
775 · Pending Verification 2,121 → 2,122 · Pending Judgment 412 → 411 · Retirement Candidate 104
→ 105. **The repository-wide `#[test]` count is unchanged (3,413)**—this round changed only the
implementation of the criteria, not any test.

## Third Retirement Batch (Executed on 2026-11-07)

**Third batch: the 5 retirement candidates in `src/middle/passes/codegen/tests/` were evaluated test
by test; 2 were retired and 3 were kept.** (single directory cluster = the `middle::passes::codegen`
module under test; criterion = all **three gates fail** under Part Seven of coding-rules.)

This batch retains the **keep criterion** established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus and integration tests belong
to the process-boundary layer and cannot replace a unit-level observation point.

- **2 tests retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is **verbatim
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (after comments are
    removed, only the function name differs): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that the “`execute_module` entry
    path returns a non-Void value,” but the body never observes the return value from
    `execute_module` (`run_fixture` discards it with `.expect(...)`)— the claimed intent is not
    implemented, and the function body is a pure duplicate ⇒ zero criterion increment. Replacement
    coverage = `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file provides strictly
    stronger coverage: a positive lookup (`lookup("a").is_some()`), a negative lookup
    (`lookup("b").is_none()`), the `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` has the same shape as the inline `Symbol` in this
    test). “A name that was never inserted cannot be found” and “it cannot be found after being
    popped” follow the same path in `SymbolScopeManager::lookup` (walk each scope with
    `scope.get(name)` and return `None` if not found) ⇒ this test contributes zero increment.
    Replacement coverage = `test_scope_nesting`.
- **3 tests kept**:
  - `test_global_slot_store_then_load_roundtrip`—the only observation point for the path that does
    **not** go directly from serialization to execution (the serialization variant is a different
    path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the only observation point for the
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **only** unit-level observation point for the
    rejection branch when `OperandResolver::to_reg` receives `id > 255` (the bytecode layer has its
    own independent overflow checks and does not cover this function; no second use case in the
    repository causes `to_reg` to go out of range). `is_err()` is a discriminating assertion
    (removing the guard would turn the test red), not merely a check that nothing panicked.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

### Census Blind Spot Found in This Batch (**Fixed in 0.6-15**; see “Generator Criteria Correction” below)

The census’s recognition of “delegated assertions” uses `\b(assert|check)[A-Za-z_]*\s*\(` and
**cannot match helpers whose names contain digits**: because `[A-Za-z_]*` does not include digits, a
call such as `assert_slot_is_42(&interp)` is **not recognized as an assertion**— so 2 of the 5
candidates in this batch were incorrectly binned as “no assertions in the body.”

After reviewing all 15 repository-wide “Neither Gate Passes” entries (allowing digits in helper
names and additionally recognizing the `match` + `panic!` form), **4 actually contain assertions**—3
use the numeric helpers above (`assert_slot_is_42`), and 1 is `test_weak_from_arc` in
`src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule 6.1
requires `assert!(matches!(...))`, but it does assert). The other 11 truly contain no assertions.

**Taking these 4 directly from the machine’s “Neither Gate Passes” category would wrongly delete
tests carrying criteria**—the per-test human review in this batch prevented that; the generator-side
fix is described in “Generator Criteria Correction (0.6-15)” below.

## Generator Criteria Correction (0.6-15)

**Two criteria blind spots have been fixed: digits in helper names, and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3,411); it
changes only how the generator recognizes whether an assertion is present in a body.

### What Was Fixed

1. **Digits are now allowed in delegated-assertion names**: `\b(assert|check)[A-Za-z_]*\s*\(` →
   `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could not match helpers such as
   `assert_slot_is_42(&interp)`, so any test containing such a call was treated as “having no
   assertions in the body.”
2. **Added recognition of the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the
   assertion is written in one of the match arms, so no `assert!` macro is visible lexically
   (test-specification rule 6.1 requires `assert!(matches!(...))`, but some existing code uses this
   form). Recognition method: scan the expression from the `match` keyword to the first `{`,
   determine the block boundary by **brace balancing**, and require a `panic!(` within the block.
   **The decision is made on masked lines**— braces in strings and comments do not affect balancing,
   and `panic!` in comments is not treated as an assertion.

### Binning Differences: 3 Tests (Gate and Disposition Changed)

| Test Name                                           | File                                           | Old Bin                                    | New Bin                             | Basis                                                                                                                                                                                                                                                                   |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------------ | ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither Gate Passes / Retirement Candidate | Pending Judgment / Pending Judgment | The body contains `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but that form was not previously recognized; it is now recognized as an assertion, and the machine does not determine its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither Gate Passes / Retirement Candidate | Pending Judgment / Pending Judgment | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern could not match; it is now recognized as an assertion                                                                                                |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither Gate Passes / Retirement Candidate | Pending Judgment / Pending Judgment | Same as above (the serialization round-trip test)                                                                                                                                                                                                                       |

Another **77 tests changed only in the “Assertion Summary” column** (the gate and disposition did
not change): most gained `match_panicxN` in the summary (the body contains the `match` + `panic!`
form), and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3;
`src/package/tests/cache.rs`: 1—helper names containing digits).

**The relaxation changes only the “Neither Gate Passes” category**: that category is the machine’s
**confident verdict** (no assertions in the body), and its criterion is a hard requirement;
“Retirement Candidate” is only a pool awaiting human review, so the size of that pool is not a
verdict. Result: Gate Neither Gate Passes 14 → 11 · Pending Judgment 923 → 926; Disposition
Retirement Candidate 103 → 100 · Pending Judgment 411 → 414.

### Existing 11 “Neither Gate Passes” Tests: Per-Test Review Conclusions

After reading all 11 repository-wide “Neither Gate Passes” tests in the original source, **all 11
truly contain no assertions** (no `assert*` / `check*` helpers, no `match` + `panic!` form, and no
`#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                     | Review Conclusion                                        |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 calls to `.expect(...)`, merely guaranteeing that it does not crash                           | Truly assertion-free (only proves that it did not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                  | Truly assertion-free                                     |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them              | Truly assertion-free                                     |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Truly assertion-free                                     |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Truly assertion-free                                     |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                              | Truly assertion-free                                     |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                        | Truly assertion-free (only proves that it did not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Truly assertion-free (only proves that it did not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Truly assertion-free (only proves that it did not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                            | Truly assertion-free                                     |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Truly assertion-free                                     |

Five of these (`test_normalizer_context` and the 4 in reducer.rs) were classified as `Keep` in the
0.6-11a decision table because **no replacement coverage could be identified**; this review reaches
the same conclusion.

### Reverse Spot Check: Did the Relaxation Go Too Far?

Five tests were selected with a fixed seed (20261107) from the 88 entries newly recognized as
assertions, and their original source was read to confirm that they really assert:

| Spot-Check Entry                                            | File                                                 | Assertion in the Original Source                                                                                                             |
| ----------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching arm; `panic!("Expected AssocType")` if it does not match                                              |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm; `panic!("expected InvalidPackage, got …")` if it does not match                  |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | An `assert_eq!` compares constraint types in the matching arm; `panic!("不是 Dynamic 变体…")` if it does not match                           |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm; `panic!("Expected TokensDelta variant")` if it does not match                        |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm verify the error code and message; `panic!("不是 Disproved…")` if it does not match |

All 5 assert concrete values or concrete variants (replacing the implementation with another correct
path would turn them red)—**the relaxation did not go too far**. The reverse controls were also
added to `--selftest`: a `match` without `panic!`, and a `panic!` that appears only in a comment;
neither may be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

When the generator is reverted to the old implementation (`HELPER_RE` without digits + `match` +
`panic!` recognition disabled), all the newly added `--selftest` cases **turn red**, and the exit
code is 1; the new implementation passes them all.

### Rollback

The generator and the census table are in the same commit; `git revert <sha>` restores the whole
state.

## Fourth Retirement Batch (Executed on 2026-11-07)

**Fourth batch: of the 10 retirement candidates in the directory cluster `src/backends/interpreter/`
(the interpreter backend’s test directories), 1 was retired and 9 were kept** (single directory
cluster; three files were read together before decisions were made: `tests/bytecode_load.rs`: 4 ·
`tests/weak.rs`: 2 · `executor/tests/debug.rs`: 4). Criterion = the **three gates** in Part Seven of
coding-rules; keep criterion = “a unit-level observation point can only be replaced by another unit
test; corpus and integration tests belong to the process-boundary layer”; **deletion must identify a
concrete replacement coverage test name; if none can be identified, or the source cannot be
understood, retain the test**.

- **1 test retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, one step, then obtain
  `capture_stack`), and its assertions are **strictly stronger**: it asserts that the stack is
  nonempty and that `stack[0].function_name == "main"` (pinning the top-of-stack position), while
  the listed test asserts only `stack.iter().any(|f| f.function_name == "main")` (any position is
  sufficient) —the former being true necessarily implies the latter ⇒ zero criterion increment.
- **9 tests kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each test in the
  original source, each is the **only unit-level observation point** for some property (load’s
  magic-number and version branches are different, YXBC routing layer, both the Some and None sides
  of Weak, stepping with no frames, step_out popping the stack, and running to completion with an
  empty stack), and no replacement unit test could be identified ⇒ retained as required. See the
  commit message for the per-test basis (the decision table is not included in this page).
- **Figures**: repository-wide `#[test]` 3,409 → 3,408; 1 deletion = `git diff` counted 1 `-#[test]`
  line and 0 `+#[test]` lines; the dominating test `test_capture_stack_during_execution` passes on
  its own. Census: Retirement Candidate 99 → 98 · Pending Judgment 925 → 924 · Clause 1,713 ·
  Incident History 313 · Proof 447 · Neither Gate Passes 11 · Pending Verification 2,121 · Keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Fifth Retirement Batch (Executed on 2026-11-07)

**Fifth batch: mechanically retired 1,353 unit tests using the rule “for the same symbol and result
category, keep only the strongest test”** (only unit tests under `src/**/tests/**` were changed; not
a single test on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (executed mechanically, without case-by-case rationale)**: for the same symbol under test
  and the same assertion-result category, keep only the test with the strongest assertions and
  delete the rest.
  - Symbol under test = a repository symbol called in the test body (including one layer of
    delegation through a helper in the same file); when multiple symbols are reached, classify by
    the **primary symbol** (the first repository call). Symbols are qualified by repository path and
    are not merged across modules by bare name.
  - Result category = a concrete value / a particular error code (E code) / a particular variant
    match / only ok / only err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when tests in a group have equal strength,
    keep the first one to appear in the file.
  - If a test asserts multiple categories (so the primary category cannot be determined), **keep
    it**; do not guess.
- **Three exceptions (not one case was removed)**: ① the name or comment explicitly mentions
  `issue #` / regression / defect / incident (incident-history gate); ② a symbol has only one test
  (deleting it would leave the symbol with no unit-level observation point); ③ `#[should_panic]` is
  the symbol’s only should_panic use case. Not one of the 313 incident-history-gate tests or 11
  “Neither Gate Passes” tests was retired.
- **Figures**: repository-wide `#[test]` 3,408 → 2,055; 1,353 deletions = `git diff` counted 1,353
  `-#[test]` lines and 0 `+#[test]` lines. Census (by gate, current values after the 0.6-19 sampling
  audit was abolished): Clause 808 · Incident History 313 · Proof 424 · Pending Judgment 499 ·
  Neither Gate Passes 11. Census (by disposition, current values): Keep 737 · Pending Verification
  1,054 · Pending Judgment 203 · Retirement Candidate 61. This batch’s retirement count by gate was:
  Clause 902 · Pending Judgment 428 · Proof 23 (because 0.6-19 reclassified 9 entries previously
  described as “audit-confirmed as genuinely supporting” as `Pending Verification`, the current
  gate/disposition values differ from those at the close of the fifth batch).
- **Cleanup alongside deletion**: helper functions used only by deleted tests, imports that became
  invalid, and struct fields were removed
  (`cargo clippy --all --all-features --all-targets -- -D warnings` produced 0 warnings).
- **Two categories of out-of-bounds deletion requiring a record**: 29 tests originally classified as
  `Keep` were still deleted under this batch’s rule—23 status anchors (names containing
  unchanged/anchor/guard/preserve/baseline, many of which are false matches on domain terms such as
  “guard” or “preserves”, for example `test_while_guard_scope_balanced`) and 6 entries whose clause
  support had been audit-confirmed. This batch followed the closed three-item exception list on the
  criterion card and did not additionally protect these two categories; to restore them, fill them
  back according to the keep/delete comparison table on this page.
- **Failure and abolition of the sampling audit**: the 40-case sample was drawn from the test pool
  with a fixed seed, and after the fifth batch retired tests, 38 entries in the original sample no
  longer existed, so the audit’s `--check` consistently reported “38 missing, 38 extra.” This
  sampling audit was abolished and deleted in 0.6-19 (the script, data file, and generator
  dependency on it were all removed); see “Abolition of the Sampling Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Sixth Retirement Batch (Executed on 2026-11-26)

**Sixth batch: mechanism rollback from the 4.9.9 narrowing round—28 tests deleted along with their
mechanisms were retired, along with 4 prohibition criteria that had never been committed.** This
batch does not use the three gates to decide whether a test should be kept. Instead, the mechanism
under test was **rolled back wholesale**: the criteria written during the first six rounds of 4.9.9
for downstream ambiguity mechanisms (candidate-provider priority / candidate-signature comparison /
owner-compatibility reasoning / IR-side named-argument fallback / imported-name resolution-point
E5006 prohibition) were retired along with the mechanisms; the mechanisms no longer exist, so the
tests have no subject. These deletions are recorded under the “batch” concept in coding-rules 7.3
and do not enter the retirement-candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 tests (28 deleted: 27
    downstream-mechanism criteria from the first six rounds and 1 alias-import named-argument
    criterion). The 2 retained tests pin down the minimal fix for bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: all 4 E5006 prohibition criteria were retired—the
    file existed only in an intermediate 4.9.9 commit and was not included in either the parent
    commit or that version’s committed table; after the prohibition was withdrawn, the tests had no
    subject, so this batch does not decrement the committed table.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the alias-import candidate-injection surface was rolled back, so the fixture’s behavior
    no longer holds); the baseline-diff entry with the same name was deleted in the same batch. The
    baseline is stated separately: **relative to the direct parent commit `ac810e97`, 405 → 404
    lines** (the intermediate state had included the fixture); **relative to the pre-4.9.9 baseline
    `08ccacc5`, 403 → 404 lines** (+1 base fixture `named-args-cross-unit-method`; the retired
    `alias-overload` never entered the final history).
- **Which gate applies**: the 2 retained tests satisfy the **incident-history gate** (they pin down
  a real defect registered in 4.9.9: a cross-unit named argument could not obtain the
  declaration-time formal-parameter name, leaving `E1014 available` empty); most of the 28 retired
  tests were originally under the incident-history gate, but the reason for this batch’s deletion is
  **not** the three gates (it does not mean they were judged worthless); the mechanism under test
  was rolled back.
- **Replacement coverage**: cross-unit named arguments with bare-name imports = the 2 retained
  driver-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity surface
  no longer has rejection logic (the requirement was rolled back), so there is no replacement
  criterion; the 4.9.8 dispatch-attribution criteria (10 driver tests + 5 integration tests) remain
  unchanged.
- **Restoration**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the empty-name-table guard for
  `method_param_info` is retained; this form is unreachable in the public syntax and is a defensive
  guard, so the unit test only prevents accidental deletion).
- **Figures** (after this batch’s regeneration): 2,680 repository-wide `#[test]` functions / 334
  files; the previous committed table had 2,688 tests → the current tree has 2,680 tests (−8); the
  current tree contains 1 test absent from the old table, while the old table contains 9 tests
  absent from the current tree, and 0 same-key rows were recomputed.
- **Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit); `git revert <sha>` rolls back the whole batch.

## Abolition of the Sampling Audit (0.6-19)

**Rationale**: the sampling audit (0.6-10) measured the “true support rate” of clause citations, but
after 0.6-18 the keep rule no longer examines citations (it changed to “for the same symbol and
result category, keep only the strongest test,” and the test pool fell from 3,408 to 2,055). Of the
original 40 sample entries, 38 no longer exist, so the audit’s `--check` consistently reported “38
missing, 38 extra”—retaining it would only produce continuous alarms. Therefore, it was **abolished
and deleted**.

**What was deleted**: the sampling-audit script under `scripts/dev/` (the one that reproduced the
40-case sample with a fixed seed) and its JSON data file (the 40 per-test decisions and bases).

**Generator dependency removed in sync**: the generator previously read that JSON and classified
“audit-confirmed as genuinely supporting” as “clause gate + Keep”; that branch has been
removed—**clause citations no longer have an “audit-confirmed” tier** and are uniformly classified
as “Clause / Pending Verification.”

**Distribution changes** (none of the repository-wide 2,055 `#[test]` functions was changed; the TSV
was regenerated): Keep 746 → 737 (−9: 9 entries previously confirmed by the audit as genuinely
supporting moved back to Pending Verification); Pending Verification 1,045 → 1,054; by gate: Clause
811 → 808, Pending Judgment 496 → 499 (the others remain unchanged: Incident History 313 · Proof 424
· Neither Gate Passes 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` rolls back the whole batch.

## Reruns and Self-Checks

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (byte-for-byte identical output across two runs)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical classification only, and its boundaries are stated as they are:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test hides its assertions in a helper whose name lacks both prefixes, it will be misclassified as
  “No Assertions”—the first batch’s 26 tests (21 retired / 5 kept) and the second batch’s 18 tests
  (11 retired / 7 kept) have all had their function bodies manually reviewed.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator locates a function body, it
  balances braces; braces in strings, character literals, and comments skew the balance, so the
  function body was truncated—`test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was
  previously truncated at the end of the file, causing the “Assertion Summary” to capture the rest
  of the file and the test to be binned as `Pending Judgment` rather than `Retirement Candidate`.
  The generator now first masks non-code regions (strings / character literals / line comments /
  block comments / raw strings) and then balances braces; see “Generator Correction” below.
- **Delegated-assertion recognition (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` does not allow digits in `[A-Za-z_]*`, so a call such as
  `assert_slot_is_42(&interp)` is not recognized as an assertion—any test containing such a call is
  misclassified as “having no assertions in the body.” At the time, 4 of the 15 repository-wide
  “Neither Gate Passes” tests actually contained assertions (3 numeric helpers + 1 `match` +
  `panic!` form). The name portion has now been widened to `[A-Za-z0-9_]*`, and recognition of the
  `match` + `panic!` form has been added; see “Generator Criteria Correction” below.
- **The proof gate is not adjudicated**: any test that fails both other gates is classified only as
  `Pending Judgment` or `Retirement Candidate`; the generator never declares that a test “should be
  deleted.”
- **No deduplication across files**: exactly duplicate tests are not identified in this table (0.6-2
  measured only 1%, which does not constitute a retirement surface).
- **Every clause citation requires verification**: a verifiable section number in the file header
  only means that the test “maps to a clause that can be named explicitly”; it does not mean that
  the citation supports the assertion. The machine does not determine whether it does, so even a
  clause-gate hit is classified only as `Pending Verification`; a human must verify it before the
  test can be kept.
