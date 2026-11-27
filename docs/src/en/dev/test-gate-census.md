---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository against the three gates in Part Seven of
  coding-rules, and provide a retirement-candidate list plus the retirement record for each batch'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **Three Gates (Clause / Incident / Proof)** in
[Part Seven of coding-rules](./coding-rules.md), producing a rerunnable, item-by-item decision
table. The sampled audit of the Clause Gate (40 entries) measured the proportion of tests that
“cited a clause but could not support the assertions with it” and therefore **tightened the
criteria**. That audit was abolished and removed in 0.6-19. **This page and the generator itself are
read-only and do not modify tests**; test retirement is performed in batches (one commit per batch,
revertible as a whole), and the results are recorded in the batch-retirement subsections and commit
messages below.

Generated artifacts:

| Artifact                            | Purpose                                                                                      |
| ----------------------------------- | -------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Item-by-item decision table (2,701 rows plus the header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, with `--check` and `--selftest`)                                      |

## Criteria

### Scan Scope

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Scope                                                                    | Count                 |
| ------------------------------------------------------------------------ | --------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 2,097                 |
| `tests/integration`                                                      | 463                   |
| `tests/formatter_integration.rs`                                         | 52                    |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                 |
| `tools/`                                                                 | 77                    |
| `benches/`                                                               | 9                     |
| **Total**                                                                | **2,701** (337 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine Criteria                                                                                                                                                                                                                                                                                                                                                                                                                                            | Meaning of a Match                                             |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Clause Gate (tightened in 0.6-10) | A **verifiable entry** appears in the first 25 lines of the file header: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); files under `src/**/tests/` are inspected together with the parent `mod.rs`                                                                                                                                                                                                             | The test maps to an identifiable clause                        |
| Incident Gate                     | The test body, function documentation comment, or **file header** (`//! 回归背景：issue #226` and similar) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                | The test pins an incident that actually occurred               |
| Proof Gate                        | **Cannot be determined mechanically** (“If it were replaced with another correct implementation, would it fail?” requires human judgment). Therefore, every test that fails the first two gates falls into “Pending Adjudication” and is never automatically classified as “Fail Both”; the sole exception is a body with **no assertions at all** (including no delegated `assert*`/`check*` helper, no `match` + `panic!` form, and no `#[should_panic]`) | Only “Pending Adjudication” or “No Assertions” can be produced |

Rationale for tightening: the Clause Gate in 0.6-9 required only that `§`/`RFC`/`SPEC` appear in the
file header, so 2,403 entries were classified as “Retain”. The 40-entry stratified sample measured
in 0.6-10 found that **half of the citations could not support the assertions** (50% miscitation
rate, 95% interval 35%–65%)—this measurement was the **direct reason for abandoning “retain based on
a citation alone.”** After tightening, only section-number forms are accepted, and **having a
citation still does not mean the citation supports the assertions**—clause citations always stop at
“Pending Review.” (This sampled audit was abolished and removed in 0.6-19: as the test pool shrank
through retirement, 38 entries in the original sample no longer existed, and `--check` always
reported “38 missing, 38 extra”; the conclusion is retained, but the audit artifact is no longer
maintained.)

Rationale for the file-header radius: this repository’s convention is to write “回归背景：issue #N”
in the file header rather than repeating it in each test’s comment. Looking only at each test’s own
comments would miss the incident history for the entire file (measured: 7 entries at the test-own
radius, 291 at the file-header radius).

### Categories Never Marked for Retirement

The following tests are **always retained** and never enter the retirement-candidate pool:

- Corpus `tests/yaoxiang/**` and project-form `tests/project-corpus/**` (behavioral-criteria
  surfaces);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and traversing process boundaries);
- Status-quo anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Meaning of Each Disposition

| Disposition          | Meaning                                                                                                                                                                                      |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Retain               | Only two grounds: the Incident Gate matched, or the Proof Gate applies (spec-shaped directory / status-quo anchor name)                                                                      |
| Pending Review       | A clause is cited (with a section number, or only an entire RFC without a section number), but the citation has not been manually confirmed—a human must verify it before it can be retained |
| Retirement Candidate | Both gates fail and the assertion surface contains only weak assertions (or none)—a candidate is not a verdict; the Proof Gate still requires manual review                                  |
| Pending Adjudication | Both gates fail, but strong assertions are present; the machine gives no conclusion                                                                                                          |

## Distribution (regenerated 2026-11-27; 4.9.9 narrowing round, 8.7 batch, 4.9.10-L0, init test rewrite, and D53 batch)

| Gate                                                                   | Count     |
| ---------------------------------------------------------------------- | --------- |
| Clause (with a verifiable entry)                                       | 1,037     |
| Incident                                                               | 565       |
| Proof (spec-shaped criteria surface / status-quo anchor)               | 571       |
| Pending Adjudication (both gates fail; Proof Gate awaits human review) | 522       |
| Fail All Gates (no assertions anywhere in the body)                    | 6         |
| **Total**                                                              | **2,701** |

| Disposition          | Count     | Compared with 0.6-19 (2026-11-07) |
| -------------------- | --------- | --------------------------------- |
| Retain               | **1,136** | 737 (+399)                        |
| Pending Review       | 1,284     | 1,054 (+230)                      |
| Pending Adjudication | 223       | 203 (+20)                         |
| Retirement Candidate | 58        | 61 (−3)                           |
| **Total**            | **2,701** | 2,055 (+646)                      |

Retention breakdown: 1,136 = Proof Gate 571 (spec-shaped criteria surface 518 + status-quo
anchors 53) + Incident Gate 565. File-level count: among 337 test files, the **Clause Gate** covers
**125**; using the first 25 lines of the file header (`src/**/tests*/` together with the parent
`mod.rs`), **169** files yield a verifiable section number (`§` + number).

Regeneration reconciliation: the previous committed table contained 2,690 entries, while the current
tree contains 2,701; 11 entries are in the current tree but not the old table (D53 batch additions:
parser 7 / sema 3 / formatter 1), no entries are in the old table but not the current tree, and 0
same-key rows changed on recomputation. Per-key records are stored in the TSV.

The latest full regeneration, after the test rewrite in `src/package/commands/tests/init.rs`, left
the row count unchanged (2,690). Only two assertion-summary rows were recalculated
(`test_init_here_fails_when_project_already_exists` and
`test_init_here_preserves_preexisting_files`, for their `assert_eq` / `unwrap` counts); the TSV
matches the current tree (`--check` exits with 0).

### Two Changes in Criteria at the 0.6-19 Snapshot

1. **Retention breakdown**: 737 = Proof Gate 424 (spec-shaped criteria surface 390 + status-quo
   anchors 34) + Incident Gate 313. Of the original 2,897 entries, the 2,121 that were “retained
   based on a single citation” now fall under Pending Review (after the sampled audit was abolished
   in 0.6-19, the 9 entries previously classified as retained because the audit had confirmed
   genuine support also moved to Pending Review—clause citations no longer have an “audit-confirmed”
   tier).
2. **Incident Gate 47 → 313**: these are not new incidents but a **relocation of labels**. In 0.6-9,
   an entry that matched both the Clause and Incident Gates was recorded under the Clause Gate.
   After tightening, a Clause Gate match no longer automatically entails retention, so such entries
   must be caught by the Incident Gate, and their counts are folded into that gate. The total number
   retained is therefore unaffected (this batch of tests was also retained in 0.6-9).

File-level count at the time: among 285 test files, **136** had a verifiable section number in the
file header (the old 0.6-9 criterion yielded 221).

## Reconciliation Between the 0.6-19 Snapshot and the 0.6-2 Survey

The 0.6-2 survey figures are the first comparison baseline. The reasons for each difference are
listed below:

| Item                           | 0.6-2 Survey                   | Current                                                                                                         | Reason for Difference                                                                                                                                                                                                                                                                   |
| ------------------------------ | ------------------------------ | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`      | 3,443                          | 2,055                                                                                                           | 2 entries were added after 0.6-2 (check-criteria regression tests for E3005 and E3022), then 21 assertion-free tests were removed under the first retirement list, 11 weak-assertion tests under the second, 2 under the third, 1 under the fourth, and 1,353 under the fifth; net −388 |
| Number of test files           | 296                            | 285                                                                                                             | 11 entire files retired in the fifth batch                                                                                                                                                                                                                                              |
| Clause Gate                    | 227 files (77%)                | 136 files after tightening; 808 entries by count                                                                | The criterion tightened from “the term appears” to “a section number `§<number>` is present,” nearly halving the file count; the fifth batch retired 902 entries, and abolishing the sampled audit in 0.6-19 moved another 9 entries from “Retain” to Pending Review                    |
| Incident signals               | 442 entries / 148 files        | 313 entries within the gate; three radii: test-own 7 / file-header 291 (38 files) / whole-file 1,218 (79 files) | The 148 files in the 0.6-2 survey covered a wider scope than the current whole-file radius of 79 files and used a broader vocabulary/scope; the current run strictly follows the four gate-criteria signals                                                                             |
| Characterization strong signal | 1,176 (34%) / strong 631 (18%) | Not measured                                                                                                    | Characterization is not one of the three gates; incorporating it into a retirement conclusion would exceed the gate’s authority                                                                                                                                                         |
| Weak assertions                | 473 (14%)                      | See the “Assertion Summary” column in the TSV                                                                   | The counting units differ: 0.6-2 counted patterns that assert only `is_ok`/`is_some`; the current run uses lexical counts and distinguishes strong, weak, and delegated assertions                                                                                                      |

## Summary of the 0.6-19 Conclusions (Historical)

1. **The Clause Gate had previously been significantly overstated**: under the old criterion, 2,403
   entries were classified as “Clause/Retain,” but the sampled audit measured a 50% miscitation rate
   (95% interval 35%–65%)—“citation” and “a citation that supports the assertions” are different
   things, and the old criterion treated the former as the latter. (This sampled audit was abolished
   and removed in 0.6-19; the conclusion is retained. See “Abolition of the Sampled Audit” below for
   the reason for removal.)
2. **After tightening, approximately 1,202 entries genuinely have clause support** (range
   846–1,557); the rest must either await review with supporting evidence or were already destined
   to be adjudicated through the Incident Gate/Proof Gate.
3. **The retirement surface is unrelated to the clause-miscitation surface**: the 136 candidates are
   unrelated to clauses (they have no assertions/only weak assertions). The first batch retired 21,
   the second retired 11, the generator correction added 1, the third removed 2, the 0.6-15
   criterion correction removed 3, the redundancy scan removed 1, the fourth removed 1, and the
   fifth removed 1,353 under the mechanical rule “for the same symbol and result category, retain
   only the strongest entry,” reducing the candidate count to 61. The fifth-batch retirement count
   by gate was: Clause 902 · Pending Adjudication 428 · Proof 23 · Incident 0 · Fail All 0—**not one
   entry from the Incident Gate or the zero-assertion line was removed**.
4. **The Incident Gate is the largest legitimate source of retention** (313 entries): it
   demonstrates that “pinning an actual incident” is more reliable than “copying a sentence from an
   RFC.”

## First Retirement Batch (Executed 2026-11-06)

**First batch: retire 21 and retain 5 of the 26 “no-assertion” tests in
`src/frontend/core/types/eval/tests/`** (a single directory cluster; criterion = all **Three Gates**
in Part Seven of coding-rules fail: no Clause Gate match, no Incident Gate match, and no assertions
anywhere in the body). The item-by-item decisions and “same-cell coverage” are provided by the
0.6-11a decision table (21 deleted / 5 retained). This page does not restate the reasons and records
only the execution result.

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
- **5 retained** (the decision table could not identify alternative coverage, so they were retained
  under the hard requirement): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Second Retirement Batch (Executed 2026-11-06)

**Second batch: retire 11 and retain 7 of the 18 “weak-assertion” tests in
`src/frontend/core/parser/pratt/tests/nud.rs`** (a single directory cluster = the Pratt
prefix-parsing module; criterion = all **Three Gates** in Part Seven of coding-rules fail: no Clause
Gate match, no Incident Gate match, and the body contains only weak assertions such as `is_some()` /
`is_none()`—neither binding power nor the parse result is pinned).

The difference between this batch and the first must be stated clearly: these tests **do contain
assertions** and are not empty tests. The reason for retirement is “all Three Gates fail + zero
incremental criterion,” not “no assertions”; every test selected for deletion identifies a
**unit-level alternative-coverage test name in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The alternative
  coverage for every entry follows the **same** `parse_expression → parse_prefix → prefix_info` path
  and makes stronger assertions (pinning the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, plus `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 retained**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the commit message (task 0.6, second test-retirement batch) for the
  individual reasons. There are two categories of retention reason: the **Proof Gate applies** (the
  first three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are the sole
  **unit-level** observation points for the corresponding facts, and no second unit test observes
  them), or the **Incident Gate applies** (`break_removed` / `continue_removed` name #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Generator Correction (0.6-13)

**`body_span` truncated function bodies when braces appeared inside literals—fixed.**

The old implementation found function bodies by balancing braces without distinguishing real code
from braces in strings/character literals or comments. Such code is common in Rust source, for
example `tokenize("{")`; the balance was therefore skewed and the function body was truncated. The
body of `test_prefix_info_lbrace` was once truncated to the end of the file, causing the “Assertion
Summary” to capture the remainder of the file (`matchesx11 assertx24 …`), which placed it in
“Pending Adjudication” rather than “Retirement Candidate”—a **misclassification**.

Fix: add `mask_noncode` to replace the contents of string literals, character literals, line
comments, block comments (including nested comments), and raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (newlines are preserved, so line numbers and line counts do not change), then
balance braces. **The mask is used only to locate function-body boundaries**; assertion statistics
and the Incident Gate still read the original source, and the criteria themselves have not been
relaxed. `--selftest` now includes 4 cases (paired and unpaired braces in ordinary strings, braces
in raw strings and block comments, and braces in character literals), each followed by a sentinel
test. The old implementation found the wrong boundary in all 4 cases; the new implementation finds
all of them correctly.

### 11 Tests Affected by the Same Bug

Counting method: compare the **function-body boundaries before and after the fix** for each test in
each file (old implementation balancing against literals versus new implementation balancing after
masking); a test is affected if the boundaries differ. Of 3,413 tests in the repository, the old
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

### Differences in Bucketing Before and After the Fix: 2 Entries

For the other 9 entries, **8 affected only the “Assertion Summary” column** (their bucket did not
change), while **1 did not change even the summary**
(`test_fstring_close_brace_escape_preserved_raw`: the old implementation omitted the closing `}`
line, which contained no assertion keyword). The eight changed summaries are listed individually in
the commit message.

| Test Name                   | File                                          | Old Bucket                                  | New Bucket                                      | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| --------------------------- | --------------------------------------------- | ------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending Adjudication / Pending Adjudication | Pending Adjudication / **Retirement Candidate** | The real function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; neither the file header nor the body contains a `§` section number or issue/regression/defect/incident ⇒ both the Clause and Incident Gates fail, and the body contains only a weak assertion such as `is_some()` ⇒ Retirement Candidate (a candidate is not a verdict; the Proof Gate still requires manual review)                                 |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident / Retain                           | Clause / **Pending Review**                     | The old implementation truncated the body to the end of the file and swallowed text containing “regression / defect / incident,” falsely matching the Incident Gate; the real function body contains none of these terms ⇒ the Incident Gate no longer applies; the file header contains verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the Clause Gate matches, but the sampled audit did not confirm it ⇒ Pending Review |

The distribution changes accordingly: Clause 1,713 → 1,714 · Incident 314 → 313 · Retain 776 → 775 ·
Pending Review 2,121 → 2,122 · Pending Adjudication 412 → 411 · Retirement Candidate 104 → 105.
**The repository-wide `#[test]` count is unchanged (3,413)**—this round changes only the
implementation of the criteria, not any tests.

## Third Retirement Batch (Executed 2026-11-07)

**Third batch: adjudicate the 5 Retirement Candidates in `src/middle/passes/codegen/tests/`,
retiring 2 and retaining 3.** (A single directory cluster = one module under test,
`middle::passes::codegen`; criterion = all **Three Gates** in Part Seven of coding-rules fail.)

This batch continues the **retention criterion** established by the previous batch: a unit-level
observation point may be replaced only by another **unit test**; corpus/integration tests belong to
the process-boundary layer and cannot replace a unit-level observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—its body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (only the function
    name differs after comments are removed): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its documentation comment claims to verify that “the
    `execute_module` entry path returns non-Void,” but the body never observes `execute_module`’s
    return value (`run_fixture` discards it with `.expect(...)`)— the claimed intent is not
    implemented, and the body is a pure duplicate ⇒ zero incremental criterion. Alternative coverage
    = `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file covers it strictly:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)`, identical in shape to the inline `Symbol` in this
    entry). “A name that was never inserted cannot be found” and “it cannot be found after being
    popped” follow the same path in `SymbolScopeManager::lookup` (walk `scope.get(name)` layer by
    layer; return `None` if not found) ⇒ this entry adds no coverage. Alternative coverage =
    `test_scope_nesting`.
- **3 retained**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that
    “executes directly without going through serialization” (the serialized variant is another
    path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit-level observation point for the
    `id > 255` rejection branch of `OperandResolver::to_reg` (the bytecode layer has separate
    overflow checks that do not cover this function; no other case in the repository causes `to_reg`
    to go out of range). `is_err()` is a discriminating assertion (it would fail if the guard were
    removed), not merely proof that nothing panicked.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

### A Census Blind Spot Found in This Batch (**Fixed in 0.6-15**; see “Generator Criterion Corrections” below)

The census’s recognition of “delegated assertions,” `\b(assert|check)[A-Za-z_]*\s*\(`, **could not
match helpers whose names contain digits**: `[A-Za-z_]*` does not include digits, so a call such as
`assert_slot_is_42(&interp)` was not recognized as an assertion. Two of the five candidates in this
batch were consequently misbucketed as “no assertions anywhere in the body.”

After reviewing all 15 repository entries in the “Fail All Gates” bucket individually (expanding
helper names to include digits and additionally recognizing the `match` + `panic!` form), **4
actually contained assertions**: 3 used the numeric helper above (`assert_slot_is_42`), and 1 was
`src/backends/interpreter/tests/weak.rs`’s `test_weak_from_arc` (the `match` + `panic!` form;
test-specification rule 6.1 requires `assert!(matches!(...))`, but it does assert). The remaining 11
genuinely contained no assertions.

**Taking these 4 entries directly from the machine’s “Fail All Gates” bucket would have wrongly
deleted tests with criteria**—they were not deleted because this batch adjudicated each one
manually; the generator-side fix is described in “Generator Criterion Corrections (0.6-15)” below.

## Generator Criterion Corrections (0.6-15)

**Two criterion blind spots were fixed: helper names containing digits, and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3,411); it
changes only how the generator recognizes whether a body contains assertions.

### What Was Fixed

1. **Digits were allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing such calls were
   treated as “having no assertions anywhere in the body.”
2. **The `match` + `panic!` form was added**: `match <expr> { … panic!(…) … }`—the assertion is
   written in an arm, so the lexer cannot see an `assert!` macro (test-specification rule 6.1
   requires `assert!(matches!(...))`, but existing code contains this form). Recognition method:
   scan the expression after the `match` keyword up to the first `{`, determine the block boundary
   by **brace balancing**, and count it as an assertion only if `panic!(` appears inside the block.
   **The decision is made on masked lines**— braces in strings and comments do not participate in
   balancing, and `panic!` in a comment is not treated as an assertion.

### Bucket Differences: 3 Entries (the gate and disposition changed)

| Test Name                                           | File                                           | Old Bucket                            | New Bucket                                  | Basis                                                                                                                                                                                                                                                                 |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------- | ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Fail All Gates / Retirement Candidate | Pending Adjudication / Pending Adjudication | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine makes no judgment about its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Fail All Gates / Retirement Candidate | Pending Adjudication / Pending Adjudication | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit and the old pattern could not match it; it is now recognized as an assertion                                                                                              |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Fail All Gates / Retirement Candidate | Pending Adjudication / Pending Adjudication | Same as above (the serialization round-trip entry)                                                                                                                                                                                                                    |

A further **77 entries changed only in the “Assertion Summary” column** (the gate and disposition
did not change): most summaries gained `match_panicxN` (the body contains the `match` + `panic!`
form), and 4 gained `helperxN` (3 in `src/driver/tests/manifest_error.rs` and 1 in
`src/package/tests/cache.rs`—helper names containing digits).

**The relaxation changes only the “Fail All Gates” tier**: this tier is the machine’s **confident
verdict** (no assertions anywhere in the body), and its criterion is a hard requirement; “Retirement
Candidate” is only a pool awaiting manual review, so the size of that pool is not a verdict.
Therefore, all other buckets, including the weak-assertion pool, retain their original criteria.
Result: gates Fail All Gates 14 → 11 · Pending Adjudication 923 → 926; dispositions Retirement
Candidate 103 → 100 · Pending Adjudication 411 → 414.

### Existing “Fail All Gates” Entries: 11: Individual Review Conclusions

After reading the original source of all 11 repository entries in the “Fail All Gates” bucket, **all
11 genuinely contain no assertions** (no `assert*` / `check*` helper, no `match` + `panic!` form,
and no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                     | Review Conclusion                                          |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | Five `.expect(...)` calls, merely ensuring it does not crash                                    | Genuinely no assertions (only proof that it did not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                  | Genuinely no assertions                                    |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them              | Genuinely no assertions                                    |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Genuinely no assertions                                    |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Genuinely no assertions                                    |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                              | Genuinely no assertions                                    |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` with invalid content and discards the result                      | Genuinely no assertions (only proof that it did not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Genuinely no assertions (only proof that it did not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Genuinely no assertions (only proof that it did not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                            | Genuinely no assertions                                    |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertions                                    |

Five of these (`test_normalizer_context` and the four in reducer.rs) were retained in the 0.6-11a
decision table because it **could not identify alternative coverage**; the conclusions of this
review agree.

### Reverse Sampling: Did the Relaxation Go Too Far?

Five entries were selected with a fixed seed (20261107) from the 88 entries “newly recognized as
assertions,” and their original source was read to confirm that they really assert:

| Sampled Entry                                               | File                                                 | Assertions in the Original Source                                                                                                                  |
| ----------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | The matched arm contains two `assert_eq!` calls; the unmatched arm uses `panic!("Expected AssocType")`                                             |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | The matched arm contains `assert!(msg.contains("src/evil.yx"))`; the unmatched arm uses `panic!("expected InvalidPackage, got …")`                 |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | The matched arm uses `assert_eq!` to compare the constrained type; the unmatched arm uses `panic!("不是 Dynamic 变体…")`                           |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | The matched arm contains `assert!(!delta.edits.is_empty())`; the unmatched arm uses `panic!("Expected TokensDelta variant")`                       |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | The matched arm uses three `assert_eq!` / `assert!` calls to verify the error code and message; the unmatched arm uses `panic!("不是 Disproved…")` |

All 5 assert concrete values or concrete variants (replacing the implementation with another correct
path would make them fail)—**the relaxation did not go too far**. Reverse controls were also added
to `--selftest`: a `match` without `panic!`, and a `panic!` appearing only in a comment; neither may
be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

When the generator is reverted to the old implementation (`HELPER_RE` without digits and `match` +
`panic!` recognition disabled), all the newly added `--selftest` cases **fail**, and the exit code
is 1; the new implementation passes them all.

### Rollback

The generator and census table are in the same commit; `git revert <sha>` restores them as a whole.

## Fourth Retirement Batch (Executed 2026-11-07)

**Fourth batch: retire 1 and retain 9 of the 10 “Retirement Candidates” in the
`src/backends/interpreter/` directory cluster** (the interpreter backend’s test directories; a
single directory cluster, with all three files read together: 4 in `tests/bytecode_load.rs` · 2 in
`tests/weak.rs` · 4 in `executor/tests/debug.rs`). Criterion = the **Three Gates** in Part Seven of
coding-rules; retention criterion = “a unit-level observation point may be replaced only by another
unit test; corpus/integration tests belong to the process-boundary layer”; **a deletion must
identify a concrete replacement-coverage test name; if one cannot be identified or understood, the
entry is retained**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, one step, then obtain
  `capture_stack`), and its assertions are **strictly stronger**: it asserts that the stack is
  non-empty and that `stack[0].function_name == "main"` (pinning the top-of-stack position), whereas
  the listed entry asserts only `stack.iter().any(|f| f.function_name == "main")` (any position is
  sufficient) —the former being true necessarily implies the latter ⇒ zero incremental criterion.
- **9 retained**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each one in the
  original source, each is the **sole unit-level observation point** for some property (the load
  magic-number/version branches differ, the YXBC routing layer, both sides of Weak Some/None,
  stepping with no frame, step_out popping the stack, and running to completion with an empty
  stack). No replacement unit test could be identified ⇒ retained under the hard requirement. See
  the commit message for the individual bases (the decision table is not included on this page).
- **Figures**: repository-wide `#[test]` 3,409 → 3,408; 1 deletion = `git diff` counts one
  `-#[test]` line and zero `+#[test]` lines; the dominating test
  `test_capture_stack_during_execution` passes on its own. Census: Retirement Candidate 99 → 98 ·
  Pending Adjudication 925 → 924 · Clause 1,713 · Incident 313 · Proof 447 · Fail All Gates 11 ·
  Pending Review 2,121 · Retain 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Fifth Retirement Batch (Executed 2026-11-07)

**Fifth batch: mechanically retire 1,353 unit tests under the rule “for the same symbol and result
category, retain only the strongest entry”** (only unit tests under `src/**/tests/**` were changed;
not a single entry on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (executed mechanically, without case-by-case argument)**: for the same symbol under test
  and the same assertion-result category, retain only the entry with the strongest assertions and
  delete the rest.
  - Symbol under test = a repository symbol called in the test body (including one level of
    propagation through a helper in the same file); when multiple symbols are touched, classify by
    the **primary symbol** (the first repository call). Symbols are qualified by their repository
    path and are not merged across modules by bare name.
  - Result category = a concrete value / a specific error code (E code) / a specific variant match /
    only ok / only err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when entries are equally strong, retain the
    first one to appear in the file.
  - **Retain** a test that asserts multiple categories (when the primary category cannot be
    determined); do not guess.
- **Three exceptions (not one entry was omitted)**: ① the name or comment names `issue #` /
  regression / defect / incident (Incident Gate); ② a symbol has only one test (deleting it would
  leave the symbol without a unit-level observation point); ③ `#[should_panic]` is the only
  should_panic case for that symbol. **Zero** entries from the 313 Incident Gate entries or the 11
  Fail All Gates entries were retired.
- **Figures**: repository-wide `#[test]` 3,408 → 2,055; 1,353 deletions = `git diff` counts 1,353
  `-#[test]` lines and zero `+#[test]` lines. Census by gate, using the current values after the
  sampled audit was abolished in 0.6-19: Clause 808 · Incident 313 · Proof 424 · Pending
  Adjudication 499 · Fail All Gates 11. Census by disposition, current values: Retain 737 · Pending
  Review 1,054 · Pending Adjudication 203 · Retirement Candidate 61. The retirement count for this
  batch by gate was: Clause 902 · Pending Adjudication 428 · Proof 23 (0.6-19 reclassified 9 entries
  that the audit had “confirmed as genuinely supported” as Pending Review, so the current
  gate/disposition values differ from those when the fifth batch was closed).
- **Cleanup alongside the deletions**: helper functions used only by deleted tests, imports that
  became invalid, and struct fields that became unused were removed as well
  (`cargo clippy --all --all-features --all-targets -- -D warnings` reported 0 warnings).
- **Two out-of-bounds deletions that must be recorded**: 29 entries originally classified as
  “Retain” were still deleted by this batch’s rule—23 status-quo anchors (names containing
  unchanged/anchor/guard/preserve/baseline, many of them false matches on **domain terms** such as
  “guard” or “preserves,” for example `test_while_guard_scope_balanced`) and 6 entries with
  audit-confirmed clause support. This batch followed the closed list of three exceptions on the
  gate surface and did not provide additional protection for these two categories; to restore them,
  fill them back in using this page’s retain/delete comparison table.
- **Failure and abolition of the sampled audit**: the 40-entry sample was drawn from the test pool
  with a fixed seed. After the fifth batch’s retirement, 38 entries in the original sample no longer
  existed, so the audit’s `--check` always reported “38 missing, 38 extra.” The sampled audit was
  therefore abolished and removed in 0.6-19 (the script, data file, and generator dependency were
  all removed); see “Abolition of the Sampled Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Sixth Retirement Batch (Executed 2026-11-26)

**Sixth batch: the mechanism rollback of the 4.9.9 narrowing round—retire 28 tests deleted along
with the mechanism, plus 4 prohibition criteria that were never committed to the inventory.** This
batch does not use the Three Gates to decide whether something “should be retained”; instead, the
mechanism under test was **rolled back as a whole**: the criteria written for the first six rounds
of the 4.9.9 downstream-ambiguity mechanism (candidate-provider priority / candidate-signature
comparison / owner-compatibility reasoning / IR-side named-argument fallback / import-name
resolution-point E5006 prohibition) were retired along with the mechanism. With the mechanism
absent, the tests have no object under test. These deletions are registered as a “batch” under
coding-rules 7.3 and do not enter the retirement-candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 entries (28 deleted: 27
    downstream-mechanism criteria from the first six rounds
    - 1 alias-import named-argument criterion). The 2 retained entries pin the minimal fix for a
      bare-name import.
  - `src/driver/tests/type_import_ambiguity.rs`: all 4 E5006 prohibition criteria were retired as an
    entire file—the file existed only in an intermediate 4.9.9 commit, and neither the parent commit
    nor that version’s inventory included it; after the prohibition was withdrawn, there was no
    object under test, so this batch does not decrement the inventory.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the candidate-injection surface for alias imports was rolled back, so the fixture’s
    behavior no longer holds); the same-named corpus-difference baseline entry was deleted in the
    same batch. The baseline counts are stated separately: **relative to direct parent commit
    `ac810e97`, 405 → 404 lines** (the intermediate state had contained this fixture); **relative to
    the pre-4.9.9 baseline `08ccacc5`, 403 → 404 lines** (+1 basic fixture
    `named-args-cross-unit-method`; the retired `alias-overload` never entered the final history).
- **Which gate applies**: the 2 retained entries = **Incident Gate** (they pin a real defect
  registered in 4.9.9: a cross-unit named argument could not obtain the declared parameter name,
  leaving `E1014 available` empty); most of the 28 retired entries originally matched the Incident
  Gate, but the reason for deleting them in this batch is **not** the Three Gates (they were not
  judged worthless); rather, the mechanism under test was rolled back.
- **Replacement coverage**: cross-unit named arguments with a bare-name import = the 2 retained
  driver-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity surface
  no longer has rejection logic (the requirement was rolled back), so there is no replacement
  criterion; the 4.9.8 dispatch-attribution criteria (10 driver + 5 integration) remain unchanged.
- **Restoration**: `src/middle/lower/tests/query.rs`’s
  `test_method_param_info_empty_name_table_returns_none` was restored in the same batch (the
  empty-name-table guard for `method_param_info` is retained; this form is unreachable under the
  public syntax and serves as a defensive guard, so the unit test prevents accidental deletion).
- **Figures** (after this batch’s regeneration): repository-wide `#[test]` 2,680 across 334 files;
  the previous committed table contained 2,688 entries, while the current tree contains 2,680 (−8);
  1 entry is in the current tree but not the old table, 9 are in the old table but not the current
  tree, and 0 same-key rows changed on recomputation.
- **Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit); `git revert <sha>` reverts it as a whole.

## Abolition of the Sampled Audit (0.6-19)

**Criterion**: the sampled audit (0.6-10) measured the “true support rate of clause citations,” but
after 0.6-18 the retention rule no longer examines citations (it changed to “for the same symbol and
result category, retain only the strongest entry,” reducing the test pool from 3,408 to 2,055). Of
the original 40 sample entries, 38 no longer exist, so the audit’s `--check` always reported “38
missing, 38 extra”—keeping it would only produce a continuous alarm. It was therefore **abolished
and removed**.

**What was removed**: the sampled-audit script under `scripts/dev/` (the one that reproduced the
40-entry sample with a fixed seed) and its JSON data file (the 40 item-by-item decisions and their
bases).

**Generator dependency removed in parallel**: the generator previously read that JSON and classified
“audit-confirmed as genuinely supported” as “Clause Gate + Retain”; that branch has been
removed—**clause citations no longer have an “audit-confirmed” tier** and always fall under “Clause
/ Pending Review.”

**Distribution change** (all 2,055 repository-wide `#[test]` entries are **unchanged**, and the TSV
was regenerated): Retain 746 → 737 (−9: the 9 entries previously confirmed by the audit as genuinely
supported moved back to Pending Review); Pending Review 1,045 → 1,054; by gate: Clause 811 → 808,
Pending Adjudication 496 → 499 (the others are unchanged: Incident 313 · Proof 424 · Fail All Gates
11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` reverts it as a whole.

## Reruns and Self-Checks

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (both runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs only lexical determination, and its boundaries are stated honestly:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as having “no assertions”—the 26 entries in the first batch (21 retired / 5
  retained) and the 18 entries in the second batch (11 retired / 7 retained) were all manually
  reviewed at the function-body level.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator extracts a function body, it
  balances braces; braces in strings/character literals and comments can skew that balance and
  truncate the function body—`test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was
  once truncated to the end of the file, causing the “Assertion Summary” to capture the remainder of
  the file and placing the entry under “Pending Adjudication” rather than “Retirement Candidate.”
  The generator now masks non-code regions (strings / character literals / line comments / block
  comments / raw strings) before balancing, as described in “Generator Correction” below.
- **Recognition of “delegated assertions” (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not include digits in `[A-Za-z_]*`, so a call such as
  `assert_slot_is_42(&interp)` was not recognized as an assertion—a test containing such a call
  would be misclassified as having “no assertions anywhere in the body.” At the time, 4 of the
  repository’s 15 “Fail All Gates” entries actually contained assertions (3 numeric helpers + 1
  `match` + `panic!` form). The name portion has now been widened to `[A-Za-z0-9_]*`, and
  recognition of the `match` + `panic!` form has been added; see “Generator Criterion Corrections”
  below.
- **The Proof Gate is not adjudicated mechanically**: every test for which “both gates fail” falls
  only under “Pending Adjudication” or “Retirement Candidate”; the generator never declares that it
  “should be deleted.”
- **No cross-file deduplication**: completely duplicate tests are not identified in this table
  (0.6-2 measured them at only 1%, so they do not constitute a retirement surface).
- **Clause citations always remain pending review**: a verifiable section number in the file header
  only means that the test maps to an identifiable clause; it does not mean that the citation
  supports the assertions. The machine does not determine whether it does, so a Clause Gate match
  still stops at “Pending Review” and must be verified by a human before retention.
