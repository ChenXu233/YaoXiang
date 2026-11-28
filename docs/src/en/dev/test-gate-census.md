---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement-candidate list plus per-batch retirement records'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **three gates (clause / incident history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a rerunnable row-by-row decision table.
A sample audit of the Clause Gate (40 entries) measured the proportion of tests that “cited a clause
but could not support the assertion” and thereby **tightened the criteria**; that audit was
abolished and deleted in 0.6-19. **This page and the generator itself are read-only and do not
modify tests**; test retirement is performed in batches (one commit per batch, individually
revertible with `git revert`), and execution results are recorded in the per-batch retirement
sections below and in commit messages.

Outputs:

| Output                              | Purpose                                                                            |
| ----------------------------------- | ---------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Row-by-row decision table (2709 rows + header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                        |

## Criteria

### Scan Surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root-package lib (`src/`, including each module's `tests/` subdirectory) | 2105                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2709** (339 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine Criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Meaning of a Match                                     |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| Clause Gate (tightened in 0.6-10) | A **verifiable entry** appears in the first 25 lines of the file header: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); files under `src/**/tests/` are inspected together with their parent `mod.rs`                                                                                                                                                                                                                 | The test maps to an identifiable clause                |
| Incident-History Gate             | The test body, function doc comment, or **file header** (such as `//! 回归背景：issue #226`) identifies a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                               | The test pins down an incident that actually occurred  |
| Proof Gate                        | **Cannot be judged mechanically** (“If it were replaced with another correct implementation, would it turn red?” requires human judgment). Therefore, every test that fails the first two gates is placed in “Undetermined” rather than automatically being classified as “fails both”; the sole exception is a test with **no assertions at all** (including none delegated to `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only “Undetermined” or “No Assertions” can be assigned |

Rationale for tightening: the Clause Gate in 0.6-9 required only that the file header **contain the
characters `§`/`RFC`/`SPEC`**, so 2403 entries were classified as “Clause/Keep”. A stratified sample
of 40 entries in 0.6-10 measured that **half of the citations could not support their assertions**
(50% miscitation rate, 95% interval 35%–65%)—this measurement was the **direct reason for abandoning
“keep based on a single citation.”** After tightening, only section-number forms are recognized, and
**having a citation still does not mean that the citation supports the assertion**—all clause
citations stop at “Needs Verification.” (The sample audit was abolished and deleted in 0.6-19: as
the test pool shrank through retirement, 38 entries in the original sample no longer existed, and
`--check` consistently reported “missing 38, extra 38”; the conclusion is retained, but the audit
output is no longer maintained.)

Rationale for the file-header radius: this repository's convention is to write “回归背景：issue #N”
at the beginning of a file rather than repeat it in each test comment; inspecting only each test's
own comments would miss the incident history of entire files (measured: 7 entries within the test
itself versus 291 entries within the file header).

### Categories That Are Never Classified for Retirement

The following tests are **always classified as “Keep”** and never enter the retirement-candidate
pool:

- Corpus `tests/yaoxiang/**` and project-shaped `tests/project-corpus/**` (behavior-criterion
  surfaces);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and operating across a process boundary);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Meaning of Each Disposition

| Disposition          | Meaning                                                                                                                                                                                       |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Assigned for only two reasons: an Incident-History Gate match, or a Proof Gate match (spec-shaped criterion surface / current-state anchor name)                                              |
| Needs Verification   | Cites a clause (with a section number, or only an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a human before it can be kept |
| Retirement Candidate | Fails both gates and the assertion surface consists only of weak assertions (or no assertions)—a candidate is not a verdict; the Proof Gate still requires manual review                      |
| Undetermined         | Fails both gates but has strong assertions; the machine reaches no conclusion                                                                                                                 |

## Distribution (Regenerated 2026-11-28; 4.9.9 Narrowing Round, 8.7 Batch, 4.9.10-L0, Init Test Rewrite, D53 Batch, and L3-1b S1)

| Gate                                                                                    | Count    |
| --------------------------------------------------------------------------------------- | -------- |
| Clause (has a verifiable entry)                                                         | 1037     |
| Incident History                                                                        | 565      |
| Proof (spec-shaped criterion surface / current-state anchor)                            | 573      |
| Undetermined (neither of the first two gates passes; Proof Gate requires manual review) | 528      |
| Fails Both Gates (no assertions in the body)                                            | 6        |
| **Total**                                                                               | **2709** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1138** | 737 (+401)                        |
| Needs Verification   | 1287     | 1054 (+230)                       |
| Undetermined         | 226      | 203 (+23)                         |
| Retirement Candidate | 58       | 61 (−3)                           |
| **Total**            | **2709** | 2055 (+651)                       |

Structure of the Keep classification: 1138 = Proof Gate 573 (spec-shaped criterion surface 518 +
current-state anchors 55) + Incident-History Gate 565. File-level figures: the Clause Gate covers
**125** of the 339 test files; **147** files have a verifiable section number (`§` + digits)
detectable within the first 25 lines of the file header (`src/**/tests*/` together with the parent
`mod.rs`).

Regeneration reconciliation: the previously committed table had 2706 entries, while the current tree
has 2709; the current tree has 3 entries absent from the old table (the three-form unit tests for
`method_key_for_nominal` in `src/frontend/core/sema/import_face.rs`), the old table has 0 entries
absent from the current tree, and 0 same-key rows changed when recalculated. Per-key records are
stored in the TSV.

The latest full regeneration, after the three-form `method_key_for_nominal` unit tests from L3-1b S1
were committed: 2709 rows (+3); all new rows are tests in `src/frontend/core/sema/import_face.rs`;
the TSV is consistent with the current tree (`--check` exits with 0).

### Two Changes in Criteria at the 0.6-19 Tightening Point

1. **Keep structure**: 737 = Proof Gate 424 (spec-shaped criterion surface 390 + current-state
   anchors 34) + Incident-History Gate 313. Of the original 2897 entries, the 2121 that had been
   kept solely on the basis of a citation are now classified as Needs Verification (after the sample
   audit was abolished in 0.6-19, the 9 entries previously classified as “audit-confirmed genuine
   support” also moved to Needs Verification—there is no longer an “audit-confirmed” tier for clause
   citations).
2. **Incident-History Gate 47 → 313**: this is not an increase in incident history but a
   **relabeling**. In 0.6-9, an entry that matched both the Clause Gate and Incident-History Gate
   was recorded under Clause; after tightening, a Clause match no longer automatically meant Keep,
   so these entries had to be caught by the Incident-History Gate. The total number of Keep entries
   is therefore unchanged (those tests were also Keep in 0.6-9).

File-level figures at the time: of 285 test files, **136** had a verifiable section number in the
file header (the old 0.6-9 criterion classified 221 files).

## Reconciliation Between 0.6-19 and the 0.6-2 Survey

The 0.6-2 survey figures form the first comparison baseline. The reasons for each difference are
listed below:

| Item                            | 0.6-2 Survey                  | This Round                                                                                                                                       | Reason for Difference                                                                                                                                                                                                                                                                                      |
| ------------------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                                                             | 2 entries were added after 0.6-2 (check-criterion regression tests for E3005 and E3022), followed by deletion of 21 no-assertion tests from the first retirement list, 11 weak-assertion tests from the second list, 2 from the third list, 1 from the fourth list, and 1353 from the fifth list; net −388 |
| Number of test files            | 296                           | 285                                                                                                                                              | 11 entire files were retired in the fifth batch                                                                                                                                                                                                                                                            |
| Clause Gate                     | 227 files (77%)               | 136 files after tightening; 808 entries                                                                                                          | The criterion was tightened from “the characters appear” to “a section number `§<number>` is present,” nearly halving the file count; the fifth batch retired 902 entries, and after the sample audit was abolished in 0.6-19, another 9 moved from “Keep” to “Needs Verification”                         |
| Incident-history signals        | 442 entries / 148 files       | 313 entries within the gate; three radii: 7 within the test itself / 291 within the file header (38 files) / 1218 across entire files (79 files) | The 148 files in the 0.6-2 survey were broader than the 79 files found at the entire-file radius in this round; its vocabulary and scope were larger. This round strictly uses the four card-surface signals                                                                                               |
| Strong characterization signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                                                     | Characterization is not one of the three gates; mixing it into a retirement conclusion would exceed the gate's authority                                                                                                                                                                                   |
| Weak assertions                 | 473 (14%)                     | See the “Assertion Summary” column in the TSV                                                                                                    | The counting units differ: 0.6-2 counted patterns that assert only is_ok/is_some, whereas this round uses lexical counts and distinguishes strong, weak, and delegated assertions                                                                                                                          |

## 0.6-19 Conclusion Summary (Historical)

1. **The Clause Gate had previously been significantly inflated**: under the old criterion, 2403
   entries were classified as “Clause/Keep,” while the sample audit measured a 50% miscitation rate
   (95% interval 35%–65%)—“citation” and “a citation that supports the assertion” are different
   things, but the old criterion treated the former as the latter. (The sample audit was abolished
   and deleted in 0.6-19; its conclusion is retained, and the reason for deletion appears in
   “Abolition of the Sample Audit” below.)
2. **After tightening, approximately 1202 entries genuinely had clause support** (range 846–1557);
   the remainder either need verification or should have been classified through the
   Incident-History Gate or Proof Gate from the outset.
3. **The retirement surface is unrelated to clause miscitation**: the 136 candidates had no clause
   (no assertions/weak assertions); 21 were retired in the first batch, 11 in the second, a
   generator correction added 1, the third batch removed 2, the 0.6-15 criterion correction removed
   3, the redundancy scan removed 1, the fourth batch removed 1, and the fifth batch removed 1353
   (using the mechanical rule “for the same symbol and result category, retain only the strongest
   entry”), leaving 61 candidates. The retirement volume in the fifth batch by gate was: Clause 902
   · Undetermined 428 · Proof 23 · Incident History 0 · Fails Both Gates 0—**not one
   Incident-History Gate or zero-assertion entry was removed**.
4. **The Incident-History Gate is the largest legitimate source of Keep classifications** (313
   entries): it demonstrates that “pinning down one real incident” is more reliable than “copying a
   sentence from an RFC.”

## First Retirement Batch (Executed 2026-11-06)

**First batch: retire 21 and retain 5 of the 26 “no assertions” tests under
`src/frontend/core/types/eval/tests/`** (a single directory cluster; criterion = all **three gates
fail** under Part Seven of coding-rules: no Clause Gate match, no Incident-History Gate match, and
no assertions in the body). The row-by-row decisions and “same-slot coverage” are provided by the
0.6-11a decision table (delete 21 / keep 5); this page does not repeat the reasons and records only
the execution result.

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
  requirement is to keep them): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls it back as a whole.

## Second Retirement Batch (Executed 2026-11-06)

**Second batch: retire 11 and retain 7 of the 18 “weak assertions” tests in
`src/frontend/core/parser/pratt/tests/nud.rs`** (a single directory cluster = the Pratt
prefix-parsing module; criterion = all **three gates fail** under Part Seven of coding-rules: no
Clause Gate match, no Incident-History Gate match, and the body contains only weak assertions such
as `is_some()` / `is_none()`—neither pinning down binding power nor the parsed result).

The difference between this batch and the first must be stated clearly: these tests **do contain
assertions** and are not empty tests. The reason for retirement is “all three gates fail + zero
criterion increment,” not “no assertions”—every test selected for deletion identifies a **unit-level
replacement coverage test name in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The replacement
  coverage for every entry follows the **same** `parse_expression → parse_prefix → prefix_info` path
  and makes stronger assertions (pinning down the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, plus `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 retained**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—the reason for each is in the commit message (task 0.6, second
  test-retirement batch). The reasons fall into two categories: **the Proof Gate applies** (the
  first three, along with `test_prefix_info_at` and `test_prefix_info_eof`, are the sole unit-level
  observation points for their respective facts, and no second unit test in the repository observes
  them), and **the Incident-History Gate applies** (`break_removed` / `continue_removed` identify
  #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls it back as a whole.

## Generator Fix (0.6-13)

**`body_span` truncates the function body when it encounters braces inside literals — fixed.**

The old implementation extracted the function body by balancing braces without distinguishing real
code from braces in string/character literals or comments. Forms such as `tokenize("{")` are common
in Rust source, so the balance was skewed and the function body was truncated: the body of
`test_prefix_info_lbrace` was once truncated to the end of the file, causing the “Assertion Summary”
to capture the remainder of the file (`matchesx11 assertx24 …`) and the test to be classified as
“Undetermined” rather than “Retirement Candidate”—a **misclassification**.

Fix: add `mask_noncode`, first replacing the contents of string literals / character literals / line
comments / block comments (nestable) / raw strings (`r"…"` / `r#"…"#` / `br"…"` / `cr#"…"#`) with
spaces (newlines are preserved, and line numbers and line counts do not change), and then balance
the braces. **The mask is used only to locate function-body boundaries**; assertion statistics and
the Incident-History Gate still read the original source, and the criteria themselves have not been
relaxed. `--selftest` adds 4 cases (paired and unpaired braces in ordinary strings, braces in raw
strings and block comments, and braces in character literals), each followed by a sentinel test—the
old implementation found the wrong boundary in all 4 cases, while the new implementation finds all
of them correctly.

### Tests Affected by the Same Bug: 11 Entries

Counting method: compare the **function-body boundaries before and after the fix** test by test and
file by file (old implementation balances literal braces versus the new implementation balances
braces after masking); an entry is affected when the boundaries differ. The old implementation found
the wrong boundary for **11** of the repository's 3413 tests:

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

### Bucket Differences Before and After the Fix: 2 Entries

Of the other 9 entries, **8 affected only the “Assertion Summary” column** (their bucket did not
change), and **1 did not change even the summary** (`test_fstring_close_brace_escape_preserved_raw`:
the old implementation omitted the closing `}` line, which contains no assertion terms). The 8
summary changes are listed individually in the commit message.

| Test Name                   | File                                          | Old Bucket                  | New Bucket                              | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| --------------------------- | --------------------------------------------- | --------------------------- | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Undetermined / Undetermined | Undetermined / **Retirement Candidate** | The actual function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; neither the file header nor the function body contains a § section number, issue/regression/defect/incident term ⇒ both the Clause Gate and Incident-History Gate fail, and the body contains only a weak assertion such as `is_some()` ⇒ Retirement Candidate (a candidate is not a verdict; the Proof Gate still requires manual review)                                           |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident History / Keep     | Clause / **Needs Verification**         | The old implementation truncated the body to the end of the file and swallowed text containing “regression / defect / incident,” incorrectly causing an Incident-History Gate match; the actual function body contains none of these terms ⇒ the Incident-History Gate no longer applies. The file header has verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the Clause Gate matches, but it has not been confirmed by the sample audit ⇒ Needs Verification |

The distribution consequently changes as follows: Clause 1713 → 1714 · Incident History 314 → 313 ·
Keep 776 → 775 · Needs Verification 2121 → 2122 · Undetermined 412 → 411 · Retirement Candidate 104
→ 105. **The repository-wide `#[test]` count does not change (3413)**—this round changes only the
implementation of the criteria, not any test.

## Third Retirement Batch (Executed 2026-11-07)

**Third batch: classify the 5 Retirement Candidates under `src/middle/passes/codegen/tests/`
individually; retire 2 and retain 3.** (a single directory cluster = the `middle::passes::codegen`
module under test; criterion = the **three gates** under Part Seven of coding-rules.)

This batch retains the **Keep criterion** established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus and integration tests
operate at the process-boundary layer and cannot replace a unit observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the body is **byte-for-byte
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file after comments are
    removed (only the function name differs): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that the `execute_module` entry
    path returns non-Void, but the body never observes the return value of `execute_module`
    (`run_fixture` discards it with `.expect(...)`)— the claimed intent was not implemented, and the
    body is a pure duplicate ⇒ zero criterion increment. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file covers it strictly:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of an inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)`, identical in shape to the inline `Symbol` in this
    test). “Looking up a name that was never inserted” and “looking it up after it was popped”
    follow the same path in `SymbolScopeManager::lookup` (each layer runs `scope.get(name)`,
    returning `None` when not found) ⇒ this test contributes zero increment. Replacement coverage =
    `test_scope_nesting`.
- **3 retained**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that does
    **not** go through serialization into direct execution (the serialization variant is a different
    path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization roundtrip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit observation point for the `id > 255`
    rejection branch of `OperandResolver::to_reg` (the bytecode layer has independent overflow
    checks and does not cover this function; no second test in the repository drives `to_reg` out of
    bounds). `is_err()` is a discriminating assertion (removing the guard would turn the test red),
    not merely a check that nothing panicked.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls it back as a whole.

### One Census Blind Spot Found in This Batch (**Fixed in 0.6-15**, see “Generator Criterion Corrections” below)

The census's “delegated assertion” recognition pattern `\b(assert|check)[A-Za-z_]*\s*\(` **could not
match helpers with digits in their names**: `[A-Za-z_]*` contains no digits, so a call such as
`assert_slot_is_42(&interp)` is **not recognized as an assertion**— 2 of the 5 candidates in this
batch were therefore incorrectly classified as “no assertions in the body.”

After reviewing all 15 repository-wide “fails both gates” entries individually (by allowing digits
in helper names and additionally recognizing the `match` + `panic!` form), **4 actually contained
assertions**—3 used the numeric helpers above (`assert_slot_is_42`), and 1 was `test_weak_from_arc`
in `src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification Rule
6.1 requires `assert!(matches!(...))`, but it does in fact assert). The other 11 genuinely contained
no assertions.

**Directly removing these 4 under the machine's “fails both gates” classification would have
incorrectly deleted tests that have criteria**—the manual review in this batch prevented that; the
generator-side fix appears in “Generator Criterion Corrections (0.6-15)” below.

## Generator Criterion Corrections (0.6-15)

**Two criterion blind spots were fixed: helper names containing digits, and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411); it
changes only how the generator recognizes whether a test body contains assertions.

### What Was Fixed

1. **Digits are now allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing such calls were
   treated as “no assertions in the body.”
2. **The `match` + `panic!` form was added**: `match <expr> { … panic!(…) … }`—the assertion is
   written in one of the arms, so the `assert!` macro is not visible lexically (test-specification
   Rule 6.1 requires `assert!(matches!(...))`, but existing code contains this form). Recognition
   method: scan the expression after the `match` keyword up to the first `{`, determine the block
   boundary by **brace balancing**, and treat the block as an assertion only when it contains
   `panic!(`. **This determination is performed on masked lines**— braces in strings and comments do
   not participate in balancing, and `panic!` in comments is not treated as an assertion.

### Bucket Differences: 3 Entries (The Gate and Disposition Changed)

| Test Name                                           | File                                             | Old Bucket                              | New Bucket                  | Basis                                                                                                                                                                                                                                                     |
| --------------------------------------------------- | ------------------------------------------------ | --------------------------------------- | --------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs           | Fails Both Gates / Retirement Candidate | Undetermined / Undetermined | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts a variant, but the form was not previously recognized; it is now recognized as an assertion, while the machine does not judge its coverage power |
| `test_global_slot_store_then_load_roundtrip`        | `src/middle/passes/codegen/tests/global_slot.rs` | Fails Both Gates / Retirement Candidate | Undetermined / Undetermined | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern could not match; it is now recognized as an assertion                                                                                  |
| `test_global_slot_survives_serialization_roundtrip` | `src/middle/passes/codegen/tests/global_slot.rs` | Fails Both Gates / Retirement Candidate | Undetermined / Undetermined | The same applies (the serialization-roundtrip entry)                                                                                                                                                                                                      |

Another **77 entries changed only in the “Assertion Summary” column** (the gate and disposition did
not change): most summaries gained `match_panicxN` (the body contains a `match` + `panic!` form),
and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3; `src/package/tests/cache.rs`:
1—helper names containing digits).

**The relaxation changes only the “Fails Both Gates” bucket**: this bucket is the machine's
**confident verdict** (no assertions at all), whose criterion is a hard requirement; a “Retirement
Candidate” is merely a pool awaiting manual review, and the size of that pool is not a verdict. All
other buckets, including the weak-assertion pool, continue to use the original criteria. As a
result: Gates Fails Both Gates 14 → 11 · Undetermined 923 → 926; Dispositions Retirement Candidate
103 → 100 · Undetermined 411 → 414.

### Review of the Existing 11 “Fails Both Gates” Entries

After reading all 11 repository-wide “fails both gates” entries individually, **all 11 genuinely
contain no assertions** (including no `assert*` / `check*` helpers, no `match` + `panic!` form, and
no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                       | Review Conclusion                                           |
| ----------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | Uses `.expect(...)` in 5 places, guaranteeing only that it does not crash                         | Genuinely no assertions (proves only that nothing crashed)  |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                    | Genuinely no assertions                                     |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer`s, obtains the context, and discards it                              | Genuinely no assertions                                     |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                               | Genuinely no assertions                                     |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                         | Genuinely no assertions                                     |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                                | Genuinely no assertions                                     |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                          | Genuinely no assertions (proves only that nothing panicked) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                  | Genuinely no assertions (proves only that nothing panicked) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                            | Genuinely no assertions (proves only that nothing panicked) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                              | Genuinely no assertions                                     |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is merely a type annotation | Genuinely no assertions                                     |

Five of these (`test_normalizer_context` and the 4 entries in reducer.rs) were retained because the
**0.6-11a decision table could not identify replacement coverage**; the conclusion of this review
agrees with that decision.

### Reverse Sample: Did the Relaxation Go Too Far?

Five entries were selected from the 88 newly recognized as assertions using a fixed seed (20261107),
and their source was read to confirm that they genuinely assert:

| Sampled Entry                                               | File                                                 | Assertions in the Source                                                                                                          |
| ----------------------------------------------------------- | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching `match` arm; `panic!("Expected AssocType")` otherwise                                      |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm; `panic!("expected InvalidPackage, got …")` otherwise                  |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | `assert_eq!` in the matching arm compares the constrained type; `panic!("不是 Dynamic 变体…")` otherwise                          |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm; `panic!("Expected TokensDelta variant")` otherwise                        |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm verify the error code and message; `panic!("不是 Disproved…")` otherwise |

All 5 assert concrete values or concrete variants (they would turn red if the implementation were
replaced by another correct path)—**the relaxation did not go too far**. Reverse controls were also
added to `--selftest`: a `match` without `panic!`, and a `panic!` that appears only in a comment;
**neither** may be recognized as an assertion.

### The Criterion Has Teeth (Developmental Proof)

When the generator was reverted to the old implementation (`HELPER_RE` without digits + recognition
of the `match` + `panic!` form disabled), all of the newly added `--selftest` cases **failed**, with
exit code 1; the new implementation passes all of them.

### Rollback

The generator and census table are in the same commit; `git revert <sha>` restores everything as a
whole.

## Fourth Retirement Batch (Executed 2026-11-07)

**Fourth batch: retire 1 and retain 9 of the 10 “Retirement Candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend's test directory)** (a single directory
cluster; all three files were read together before making any decision: `tests/bytecode_load.rs`: 4
entries · `tests/weak.rs`: 2 entries · `executor/tests/debug.rs`: 4 entries). Criterion = the
**three gates** under Part Seven of coding-rules; Keep criterion = “a unit-level observation point
can only be replaced by another unit test, while corpus/integration tests operate at the
process-boundary layer”; **deletion must identify a concrete replacement coverage test name, and an
entry must be retained if no replacement can be identified or the source cannot be understood**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping` plus one step, then
  obtain `capture_stack`) and makes **strictly stronger** assertions: it asserts that the stack is
  nonempty and that `stack[0].function_name == "main"` (pinning down the top-of-stack position),
  whereas the listed test only asserts `stack.iter().any(|f| f.function_name == "main")` (any
  position is sufficient) —the former being true necessarily implies the latter ⇒ zero criterion
  increment.
- **9 retained**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each source
  individually, each is the **sole unit observation point** for some property (the distinct
  magic-number and version branches under load, the YXBC routing layer, both sides of Weak's
  Some/None, stepping with no frames, `step_out` popping the stack, and the stack being empty after
  running to completion); because no unit test that could replace them can be identified, the hard
  requirement is to retain them. The basis for each entry is in the commit message (the decision
  table is not included in this page).
- **Figures**: repository-wide `#[test]` 3409 → 3408; the deletion of 1 = `git diff` counts
  `-#[test]` on 1 line and `+#[test]` on 0 lines; the dominant replacement,
  `test_capture_stack_during_execution`, passes when run individually. Census: Retirement Candidate
  99 → 98 · Undetermined 925 → 924 · Clause 1713 · Incident History 313 · Proof 447 · Fails Both
  Gates 11 · Needs Verification 2121 · Keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls it back as a whole.

## Fifth Retirement Batch (Executed 2026-11-07)

**Fifth batch: mechanically retire 1353 unit tests using the rule “for the same symbol and result
category, retain only the strongest entry”** (only unit tests under `src/**/tests/**` were changed;
not one entry in the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (mechanically applied, without arguing each entry individually)**: for the same symbol
  under test + the same assertion-result category, retain only the entry with the strongest
  assertion and delete the rest.
  - Symbol under test = a repository symbol called in the test body (including one level of
    propagation through helpers in the same file); when multiple symbols are reached, classify by
    the **primary symbol** (the first repository call). Symbols are qualified by their repository
    path and are not merged across modules by bare name.
  - Result category = a concrete value / a specific error code (E code) / a specific variant match /
    only ok / only err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; within a group of equal strength, retain the
    first entry in the file.
  - When a test asserts multiple categories (making the primary category indeterminate), **retain
    it** rather than guess.
- **Three exceptions (not one entry was removed under them)**: ① names or comments identify
  `issue #` / regression / defect / incident (Incident-History Gate); ② a symbol has only one test
  (deleting it would leave the symbol without a unit observation point); ③ `#[should_panic]` is the
  symbol's only should_panic case. **Zero** of the 313 Incident-History Gate entries and 11 Fails
  Both Gates entries were retired.
- **Figures**: repository-wide `#[test]` 3408 → 2055; 1353 deletions = `git diff` counts `-#[test]`
  on 1353 lines and `+#[test]` on 0 lines. Census by gate (current values after the sample audit was
  abolished in 0.6-19): Clause 808 · Incident History 313 · Proof 424 · Undetermined 499 · Fails
  Both Gates 11. Census by disposition (current values): Keep 737 · Needs Verification 1054 ·
  Undetermined 203 · Retirement Candidate 61. The retirement volume in this batch by gate was:
  Clause 902 · Undetermined 428 · Proof 23 (the current values differ from those when the fifth
  batch was closed because 0.6-19 reclassified 9 entries previously marked “audit-confirmed genuine
  support”).
- **Cleanup accompanying the deletions**: helper functions used exclusively by deleted tests,
  imports that became invalid, and struct fields that became unused were removed as well
  (`cargo clippy --all --all-features --all-targets -- -D warnings` produced 0 warnings).
- **Two out-of-bounds deletions that must be recorded**: 29 entries that the census originally
  classified as “Keep” were still deleted by this batch's rule—23 current-state anchors (names
  containing unchanged/anchor/guard/preserve/baseline, many of which are false-positive matches on
  **domain terms** such as “guard” or “preserves,” such as `test_while_guard_scope_balanced`) and 6
  entries whose clause support was genuinely confirmed by the audit. This batch followed the closed
  list of three exceptions on the card surface and did not separately protect these two categories;
  to restore them, fill them back according to the Keep/Delete comparison table in this page.
- **Failure and abolition of the sample audit**: the 40-entry sample was drawn from the test pool
  using a fixed seed, and after the fifth batch retired tests, 38 entries in the original sample no
  longer existed. The audit's `--check` therefore consistently reported “missing 38, extra 38.” The
  sample audit was consequently abolished and deleted in 0.6-19 (the script, data file, and the
  generator's dependency on it were removed together); see “Abolition of the Sample Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls it back as a whole.

## Sixth Retirement Batch (Executed 2026-11-26)

**Sixth batch: the mechanism rollback from the 4.9.9 narrowing round—retire 28 tests removed along
with their mechanism, plus 4 prohibition criteria that were never committed.** This batch does not
decide whether tests “should be kept” through the three gates. Instead, the mechanisms under test
were **rolled back as a whole**: the criteria written for downstream ambiguity mechanisms in the
first six rounds before 4.9.9 (candidate-provider priority / candidate-signature comparison / owner
compatibility inference / IR-side named-argument fallback / import-name resolution-point E5006
prohibition) were retired along with the mechanisms— the mechanisms no longer exist, so the tests
have no subject. Such deletions are registered as a batch under coding-rules 7.3 and do not enter
the retirement-candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 (28 deleted: 27 downstream-mechanism
    criteria from the first six rounds
    - 1 alias-import named-argument criterion). The 2 retained entries pin down the minimal fix for
      bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: all 4 E5006 prohibition criteria were retired with
    the file—the file existed only in an intermediate 4.9.9 commit, and neither the parent commit
    nor that version's committed table included it; after the prohibition was withdrawn, there was
    no subject under test, so this batch does not subtract rows from the committed table.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the candidate-injection surface for alias imports was rolled back, so the fixture's
    behavior no longer holds); the same-named corpus differential-baseline entry was deleted in the
    same batch. The baseline figures are recorded separately: **relative to direct parent commit
    `ac810e97`, 405 → 404 lines** (the intermediate state had included the fixture); **relative to
    the pre-4.9.9 baseline `08ccacc5`, 403 → 404 lines** (+1 base fixture
    `named-args-cross-unit-method`; the retired `alias-overload` never entered the final history).
- **Which gate applied**: the 2 retained entries = **Incident-History Gate** (they pin a real defect
  registered in 4.9.9: cross-unit named arguments could not obtain declaration-time formal-parameter
  names, leaving `E1014 available` empty); most of the 28 retired criteria from the first six rounds
  were originally Incident-History Gate matches, but the reason for their deletion in this batch is
  **not** one of the three gates—it is not a judgment that they were unworthy; the mechanism under
  test was rolled back.
- **Replacement coverage**: cross-unit named arguments using bare-name imports = the 2 retained
  driver-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity surface
  no longer has rejection logic (the requirement was rolled back), so there is no replacement
  criterion; the 4.9.8 dispatch-attribution criteria (10 driver-level + 5 integration-level) remain
  unchanged.
- **Restoration**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the empty-name-table guard for
  `method_param_info` is retained; this form is unreachable under the public syntax and is a
  defensive guard, and the unit test merely prevents accidental deletion).
- **Figures** (after regeneration for this batch): repository-wide `#[test]` 2680 entries / 334
  files; the previously committed table had 2688 entries → the current tree has 2680 entries (−8);
  the current tree has 1 entry absent from the old table, the old table has 9 entries absent from
  the current tree, and 0 same-key rows changed when recalculated.
- **Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit); `git revert <sha>` rolls it back as a whole.

## Abolition of the Sample Audit (0.6-19)

**Criterion**: the sample audit (0.6-10) measured the “true support rate” of clause citations, but
the retention rule after 0.6-18 no longer examines citations (after changing to “for the same symbol
and result category, retain only the strongest entry,” the test pool shrank from 3408 to 2055). Of
the original 40 sampled entries, 38 no longer exist, so the audit's `--check` consistently reported
“missing 38, extra 38”—keeping it would only produce continuing alerts. It was therefore **abolished
and deleted**.

**What was deleted**: the sample-audit script under `scripts/dev/` (the one that reproduced the 40
sampled entries using a fixed seed) and its JSON data file (the row-by-row decisions and basis for
all 40 entries).

**The generator's dependency was removed synchronously**: the generator formerly read that JSON and
classified “audit-confirmed genuine support” as “Clause Gate + Keep”; that branch has been
deleted—**there is no longer an “audit-confirmed” tier for clause references**, and every such entry
is classified as “Clause / Needs Verification.”

**Distribution changes** (not one of the repository's 2055 `#[test]` entries was changed; the TSV
was regenerated): Keep 746 → 737 (−9: the 9 entries previously confirmed by the audit as genuinely
supported moved back to Needs Verification); Needs Verification 1045 → 1054; by gate: Clause 811 →
808, Undetermined 496 → 499 (the others remain unchanged: Incident History 313 · Proof 424 · Fails
Both Gates 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` rolls it back as a whole.

## Rerunning and Self-Checking

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical classification only, and states its boundaries honestly:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test's assertion is hidden in a helper whose name lacks these prefixes, it is misclassified as
  having “no assertions”—the first 26 entries (21 retired / 5 retained) and the second 18 entries
  (11 retired / 7 retained) have all had their function bodies manually reviewed.
- **Function-body boundaries (fixed in 0.6-13)**: when extracting a function body, the generator
  balances braces, but braces in string/character literals and comments skew the balance and
  truncate the body—`test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was once
  truncated to the end of the file, causing the “Assertion Summary” to capture the remainder of the
  file and the test to be classified as “Undetermined” rather than “Retirement Candidate.” The
  generator now masks non-code areas (strings / character literals / line comments / block comments
  / raw strings) before balancing; see “Generator Fix” below.
- **Delegated-assertion recognition (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` contains `[A-Za-z_]*`, which excludes digits, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions—tests containing such calls were
  misclassified as having “no assertions in the body.” At the time, 4 of the repository's 15 “fails
  both gates” entries actually contained assertions (3 numeric helpers + 1 `match` + `panic!` form).
  The name portion has now been widened to `[A-Za-z0-9_]*`, and recognition of the `match` +
  `panic!` form has been added; see “Generator Criterion Corrections” below.
- **The Proof Gate is not judged**: every test that fails the first two gates is classified only as
  “Undetermined” or “Retirement Candidate”; the generator never declares that it “should be
  deleted.”
- **No deduplication across files**: fully duplicated tests are not identified in this table (0.6-2
  measured only 1%, which does not constitute a retirement surface).
- **All clause citations require verification**: a verifiable section number in the file header only
  means that the test maps to an identifiable clause; it does not mean that the citation supports
  the assertion. The machine does not judge whether it is supported, so even a Clause Gate match
  stops at “Needs Verification”; a human must verify it before the test can be kept.
