---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Great Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement-candidate list plus retirement records for each batch'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Great Cleanup)

Prerequisite material for WBS 0.6 “Great Cleanup”: mechanically classify every repository-wide
`#[test]` according to the **Three Gates (Clause / History / Proof)** in
[Part Seven of coding-rules](./coding-rules.md), producing a rerunnable per-test decision table. The
Clause Gate sample audit (40 cases) once measured the proportion of cases that “cited a clause but
could not support the assertions” and accordingly **tightened the criteria**; that audit was retired
and deleted at 0.6-19. **This page and the generator itself are read-only and do not modify tests**;
test retirement is performed in batches (one commit per batch, revertible as a whole with
`git revert`), and the execution results are recorded in the batch-retirement subsections below and
in commit messages.

Generated artifacts:

| Artifact                            | Purpose                                                                              |
| ----------------------------------- | ------------------------------------------------------------------------------------ |
| `docs/src/dev/test-gate-census.tsv` | Per-test decision table (2,736 rows plus header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                          |

## Scope and Definitions

### Scan Surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 2132                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2748** (346 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine Criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Meaning When Matched                                      |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| Clause Gate (tightened at 0.6-10) | The first 25 lines of the file contain a **verifiable clause**: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); files under `src/**/tests/` are examined together with their parent `mod.rs`                                                                                                                                                                                                                                                   | The test maps to an individually identifiable clause      |
| History Gate                      | The test function body, function doc comment, or **file header** (for example, `//! Regression context: issue #226`) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                                    | The test pins down an incident that has actually occurred |
| Proof Gate                        | **Cannot be determined mechanically** (“If it were replaced with another correct implementation, would it turn red?” requires human judgment). Therefore, everything that fails both preceding gates falls into “Pending Judgment,” rather than automatically being classified as “Neither Passes”; the sole exception is **not a single assertion in the body** (including no assertion delegated to an `assert*`/`check*` helper, no `match` + `panic!` form, and no `#[should_panic]`) | Can only assign “Pending Judgment” or “No Assertions”     |

Rationale for tightening: the Clause Gate at 0.6-9 only required the file header to **contain the
words `§`/`RFC`/`SPEC`**, so 2,403 cases were classified as “Keep.” The 40-case stratified sample
measured at 0.6-10 found that **half of the citations could not support the assertions**
(mis-citation rate: 50%; 95% interval: 35%–65%)—this measurement was the **direct reason for
abandoning “keep based on a single citation.”** After tightening, only section-number forms are
recognized, and **having a citation still does not mean that it supports the assertions**—all clause
citations are classified only as “Needs Review.” (The sample audit was retired and deleted at
0.6-19: as the audit pool shrank with test retirement, 38 cases from the original sample no longer
existed, and `--check` consistently reported “38 missing, 38 extra.” The conclusion is retained, but
the audit artifacts are no longer maintained.)

Rationale for the file-header radius: this repository’s convention is to write “Regression context:
issue #N” in the file header rather than repeat it in each test comment. Looking only at each test’s
own comment would miss the history of an entire file (measured: 7 cases within the test itself
versus 291 within the file header).

### Categories That Are Never Retired

The following tests are **always classified as “Keep”** and do not enter the retirement-candidate
pool:

- Corpus `tests/yaoxiang/**` and project-form `tests/project-corpus/**` (behavioral-criteria
  surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and crossing a process boundary);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                               |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | There are only two paths: a History Gate match, or the Proof Gate (spec-shaped criteria surface / current-state anchor name)                                                                          |
| Needs Review         | Cites a clause (with a section number, or only citing an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a person before it can be kept |
| Retirement Candidate | Fails both gates and the assertion surface consists only of weak assertions (or no assertions)—a candidate is not a verdict; the Proof Gate still requires manual review                              |
| Pending Judgment     | Fails both gates but has strong assertions; the machine gives no verdict                                                                                                                              |

## Distribution (Regenerated 2026-11-28; 4.9.9 narrowing round, batch 8.7, 4.9.10-L0, init test rewrite, D53 batch, L3-1b S1, batch 7e, #593 criteria and L3-1 identity switch)

| Gate                                                                 | Count    |
| -------------------------------------------------------------------- | -------- |
| Clause (has a verifiable clause)                                     | 1048     |
| History                                                              | 568      |
| Proof (spec-shaped criteria surface / current-state anchor)          | 574      |
| Pending Judgment (fails both gates; Proof Gate awaits manual review) | 552      |
| Neither Passes (no assertions in the body)                           | 6        |
| **Total**                                                            | **2736** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1142** | 737 (+405)                        |
| Needs Review         | 1320     | 1054 (+266)                       |
| Pending Judgment     | 227      | 203 (+24)                         |
| Retirement Candidate | 59       | 61 (−2)                           |
| **Total**            | **2748** | 2055 (+693)                       |

Structure of “Keep”: 1142 = Proof Gate 574 (spec-shaped criteria surface 518 + current-state
anchors 56) + History Gate 568. File-level count: among 346 test files, the **Clause Gate** covers
**128**.

Regeneration reconciliation: this regeneration agrees with the checked-in table key for key (0
additions, 0 deletions, 0 content recomputations), for 2,748 rows. Key-by-key records are in the
TSV.

The latest full regeneration (after checking in the L3-1 identity switch and lower cleanup, and
merging upstream #591/#593/opcode convergence) produced 2,748 rows, consistent with the current tree
key for key (`--check` exits with status 0).

### Two Changes in the Criteria at 0.6-19

1. **Structure of “Keep”**: 737 = Proof Gate 424 (spec-shaped criteria surface 390 + current-state
   anchors 34) + History Gate 313. Of the original 2,897 cases, the 2,121 that had been “kept based
   on a single citation” now fall under “Needs Review” (after the sample audit was retired at
   0.6-19, the 9 cases previously kept because the audit had confirmed genuine support also moved to
   “Needs Review”—clause citations no longer have an “audit-confirmed” tier).
2. **History Gate 47 → 313**: this was not an increase in history cases, but a **relocation of
   labels**—in 0.6-9, cases that matched both the Clause and History Gates were recorded under the
   Clause Gate. After tightening, a Clause Gate match no longer automatically resulted in “Keep,” so
   these cases had to be caught by the History Gate and their counts folded into it. The total
   number kept was therefore unaffected (these tests were also classified as “Keep” in 0.6-9).

The file-level criterion at the time: among 285 test files, **136** had a verifiable section number
in the file header (the old 0.6-9 criterion yielded 221).

## Reconciliation Between 0.6-19 and the 0.6-2 Survey

The figures from the 0.6-2 survey form the first comparison baseline. The reasons for each
difference are listed below:

| Item                            | 0.6-2 Survey                  | This Run                                                                                                                                           | Reason for Difference                                                                                                                                                                                                                                                               |
| ------------------------------- | ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                                                               | After 0.6-2, 2 cases were added (check-criteria regression tests for E3005 and E3022); then 21 no-assertion tests were deleted in the first batch, 11 weak-assertion tests in the second batch, 2 in the third batch, 1 in the fourth batch, and 1,353 in the fifth batch; net −388 |
| Number of test files            | 296                           | 285                                                                                                                                                | The fifth batch retired 11 entire files                                                                                                                                                                                                                                             |
| Clause Gate                     | 227 files (77%)               | 136 files after tightening; 808 by item                                                                                                            | The criterion was tightened from “the word appears” to “it has a section number `§<number>`,” nearly halving the file count; the fifth batch retired 902 cases, and after the sample audit was retired at 0.6-19, another 9 moved from “Keep” to “Needs Review”                     |
| History signals                 | 442 cases / 148 files         | 313 cases within the gate; three radii: 7 within the test itself / 291 within the file header (38 files) / 1,218 across the entire file (79 files) | The 148 files in the 0.6-2 survey were broader than the current whole-file radius of 79 files, and its vocabulary and scope were larger; the current run strictly uses the four gate-surface signals                                                                                |
| Characterization strong signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                                                       | Characterization is not one of the three gates; mixing it into a retirement conclusion would exceed the machine’s authority                                                                                                                                                         |
| Weak assertions                 | 473 (14%)                     | See the “assertion summary” column in the TSV                                                                                                      | The counting unit differs: 0.6-2 counted the pattern that only asserts `is_ok`/`is_some`; the current run uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                                            |

## 0.6-19 Conclusion Summary (Historical)

1. **The Clause Gate had previously been significantly inflated**: under the old criterion, 2,403
   cases were classified as “Clause/Keep.” The sample measurement found a 50% mis-citation rate (95%
   interval: 35%–65%)—“having a citation” and “the citation supporting the assertions” are different
   things, and the old criterion treated the former as the latter. (The sample audit was retired and
   deleted at 0.6-19; its conclusion is retained. See “Retirement of the Sample Audit” below for the
   reason for deletion.)
2. **Approximately 1,202 cases genuinely had clause support after tightening** (range: 846–1,557);
   the remainder either need a reviewed ticket under “Needs Review” or should have been classified
   through the History Gate or Proof Gate from the outset.
3. **The retirement surface was unrelated to mis-cited clauses**: the 136 candidates were unrelated
   to clauses (no assertions/weak assertions); 21 cases were retired in the first batch and 11 in
   the second; the generator correction contributed +1, the third batch −2, the 0.6-15 criteria
   correction −3, the redundancy scan −1, the fourth batch −1, and the fifth batch −1,353 (using the
   mechanical rule “for the same symbol and result category, keep only the strongest case”), leaving
   61 candidates. The fifth-batch retirement count by gate was: Clause 902 · Pending Judgment 428 ·
   Proof 23 · History 0 · Neither Passes 0—**not one History Gate case or zero-assertion case was
   removed**.
4. **The History Gate was the largest legitimate source of “Keep”** (313 cases): it demonstrates
   that “pinning down a real incident” is more reliable than “copying a sentence from an RFC.”

## First Retirement Batch (Executed 2026-11-06)

**First batch: retire 21 and keep 5 of the 26 “no assertion” tests under
`src/frontend/core/types/eval/tests/`** (single directory cluster; criterion = all **Three Gates**
in Part Seven of coding-rules fail: no Clause Gate, no History Gate, and no assertions in the body).
The per-item decisions and “same-cell coverage” are provided by the 0.6-11a decision table (21
delete / 5 keep). This page does not repeat the reasons; it records only the execution result.

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
- **5 kept** (the decision table could not identify replacement coverage, so they were kept as
  required): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Second Retirement Batch (Executed 2026-11-06)

**Second batch: retire 11 and keep 7 of the 18 “weak assertion” tests in
`src/frontend/core/parser/pratt/tests/nud.rs`** (single directory cluster = the Pratt prefix-parsing
module; criterion = all **Three Gates** in Part Seven of coding-rules fail: no Clause Gate, no
History Gate, and the body contains only weak assertions such as `is_some()` / `is_none()`—neither
binding power nor the parse result is pinned down).

The difference between this batch and the first must be made clear: these tests **do contain
assertions** and are not empty tests. The reason for retirement is “all Three Gates fail + zero
criterion increment,” not “no assertions”—each case selected for deletion names a **unit-level
replacement coverage test in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Every replacement
  follows the **same** `parse_expression → parse_prefix → prefix_info` path and makes stronger
  assertions (pinning down the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, plus `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—the reason for each is recorded in the commit message (task 0.6, second
  test-retirement batch). There are two categories of reason: the **Proof Gate applies** (the first
  three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are the sole **unit-level**
  observation points for their respective facts, and the repository contains no second unit test
  that observes them), and the **History Gate applies** (`break_removed` / `continue_removed` name
  #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Generator Correction (0.6-13)

**`body_span` truncated function bodies at braces inside literals — fixed.**

The old implementation balanced braces without distinguishing real code from string/character
literals or comments. Such code is common in Rust—for example, `tokenize("{")`—so the balance became
skewed and the function body was truncated: the body of `test_prefix_info_lbrace` was once truncated
to the end of the file, causing the “assertion summary” to capture the rest of the file
(`matchesx11 assertx24 …`) and the case to be binned as “Pending Judgment” rather than “Retirement
Candidate”—**the wrong bin**.

Fix: add `mask_noncode`, first replacing the contents of string literals, character literals, line
comments, block comments (including nested ones), and raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (preserving newlines as-is, so line numbers and line counts remain
unchanged), and only then balance braces. **The mask is used solely to locate function-body
boundaries**; assertion statistics and the History Gate still read the original source, and the
criteria themselves have not been relaxed. `--selftest` gained 4 cases (paired and unpaired braces
in ordinary strings, braces in raw strings and block comments, and braces in a character literal),
each followed by a sentinel test. The old implementation found the wrong boundary in all 4 cases;
the new implementation finds all of them correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** for each test in
each file (the old implementation balanced raw braces; the new implementation balances braces after
masking). A test is affected if its boundaries differ. The old implementation found the wrong
boundary for **11** of the repository’s 3,413 tests:

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

Among the other 9 cases, **8 affected only the “assertion summary” column** (their bins did not
change), and **1 did not change even the summary** (`test_fstring_close_brace_escape_preserved_raw`:
the old implementation omitted the closing `}` line, which contained no assertion keyword). The 8
summary changes are recorded individually in the commit message.

| Test Name                   | File                                          | Old Bin                             | New Bin                                     | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --------------------------- | --------------------------------------------- | ----------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending Judgment / Pending Judgment | Pending Judgment / **Retirement Candidate** | The real function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; neither the file header nor the function body contains a § section number or issue/regression/defect/incident ⇒ both the Clause Gate and History Gate fail, and the body contains only a weak assertion such as `is_some()` ⇒ Retirement Candidate (a candidate is not a verdict; the Proof Gate still requires manual review)                                                |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | History / Keep                      | Clause / **Needs Review**                   | The old implementation truncated the function body to the end of the file and swallowed text containing “regression / defect / incident,” causing a false History Gate match; the real function body contains none of those words ⇒ the History Gate no longer applies; the file header contains verifiable clauses (`§1.6.1` / `§1.6.2` / `1.6.3` / `§1.6.4` / RFC-012) ⇒ the Clause Gate matches, but the reference has not been confirmed by the sample audit ⇒ Needs Review |

The distribution changes accordingly: Clause 1713 → 1714 · History 314 → 313 · Keep 776 → 775 ·
Needs Review 2121 → 2122 · Pending Judgment 412 → 411 · Retirement Candidate 104 → 105. The
repository-wide `#[test]` count is unchanged (3413)—this round changes only the implementation of
the criteria, not any test.

## Third Retirement Batch (Executed 2026-11-07)

**Third batch: review the 5 Retirement Candidates under `src/middle/passes/codegen/tests/` one by
one; retire 2 and keep 3.** (single directory cluster = the `middle::passes::codegen` module under
test; criterion = all **Three Gates** in Part Seven of coding-rules fail.)

This batch follows the **keep criterion** established by the previous batch: a unit-level
observation point may only be replaced by another **unit test**; corpus and integration tests
operate at the process-boundary layer and cannot replace a unit observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`) — its function body is **verbatim
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (apart from the
    function name, after comments are removed): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that “the `execute_module` entry
    path returns non-Void,” but the body never observes the return value from `execute_module`
    (`run_fixture` discards it with `.expect(...)`)— the claimed intent is not implemented, and the
    function body is a pure duplicate ⇒ zero criterion increment. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`) — `test_scope_nesting` in the same file covers it
    strictly: positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`),
    the `scope_level` count, and the inserted `Symbol` shape
    (`symbol("a", Int(64), Local(0), false, 0)` has the same shape as the inline `Symbol` in this
    test). “A name that was never inserted cannot be found” and “a name cannot be found after being
    popped” follow the same path in `SymbolScopeManager::lookup` (walk each `scope.get(name)`,
    returning `None` when not found) ⇒ this test contributes zero increment. Replacement coverage =
    `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip` — the sole observation point for the path that does
    “not execute through a direct serialization connection” (the serialized variant is another
    path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip` — the sole observation point for a
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`) — the **sole** unit observation point for the rejection
    branch where `id > 255` in `OperandResolver::to_reg` (the bytecode layer has independent
    overflow checks and does not cover this function; no second case in the repository drives
    `to_reg` out of range). `is_err()` is a discriminating assertion (removing the guard would make
    the test fail), not merely a check that nothing panicked.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

### Census Blind Spot Found in This Batch (**Fixed at 0.6-15**, see “Generator Criteria Correction” below)

The census’s recognition of “delegated assertions” used `\b(assert|check)[A-Za-z_]*\s*\(` and
**could not match helper names containing digits**: `[A-Za-z_]*` does not include digits, so a call
such as `assert_slot_is_42(&interp)` was **not recognized as an assertion**— and 2 of the 5
candidates in this batch were consequently mis-binned as “no assertions in the body.”

After reviewing all 15 “Neither Passes” cases individually (by allowing digits in helper names and
additionally recognizing the `match` + `panic!` form), **4 actually contained assertions**: 3 used
the numeric helper above, and 1 was `test_weak_from_arc` in `src/backends/interpreter/tests/weak.rs`
(the `match` + `panic!` form; test-specification rule 6.1 requires `assert!(matches!(...))`, but it
does assert). The remaining 11 genuinely contain no assertions.

**Directly removing these 4 under the machine’s “Neither Passes” classification would have deleted
tests that did contain criteria**—the manual review in this batch prevented that; the generator-side
fix is described in “Generator Criteria Correction (0.6-15)” below.

## Generator Criteria Correction (0.6-15)

**Two criteria blind spots were fixed: helper names containing digits, and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411); it
changes only how the generator recognizes whether a body contains assertions.

### What Was Fixed

1. **Digits were allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so a test containing such a call was
   treated as “not a single assertion in the body.”
2. **Added recognition of the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the
   assertion is written in one of the match arms, so `assert!` is not visible lexically
   (test-specification rule 6.1 requires `assert!(matches!(...))`, but existing code contains this
   form). Recognition method: scan the expression after the `match` keyword to the first `{`, use
   **brace balancing** to determine the block boundary, and count it only if `panic!(` occurs inside
   the block. **The determination is made on masked lines**— braces in strings and comments do not
   participate in balancing, and `panic!` in a comment is not treated as an assertion.

### Bin Differences: 3 Cases (the gate and disposition changed)

| Test Name                                           | File                                           | Old Bin                               | New Bin                             | Basis                                                                                                                                                                                                                                                      |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------- | ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither Passes / Retirement Candidate | Pending Judgment / Pending Judgment | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts a variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither Passes / Retirement Candidate | Pending Judgment / Pending Judgment | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit and was not matched by the old pattern; it is now recognized as an assertion                                                                                   |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither Passes / Retirement Candidate | Pending Judgment / Pending Judgment | The same applies (the serialization round-trip case)                                                                                                                                                                                                       |

A further **77 cases changed only the “assertion summary” column** (their gate and disposition did
not change): most gained `match_panicxN` in the summary (the body contains the `match` + `panic!`
form), and 4 gained `helperxN` (3 in `src/driver/tests/manifest_error.rs` and 1 in
`src/package/tests/cache.rs`—helper names containing digits).

**The relaxation changes only the “Neither Passes” tier**: this tier is the machine’s **confident
verdict** (not a single assertion in the body), and its criterion is a hard requirement; a
“Retirement Candidate” is only a pool awaiting manual review, and the pool size is not a verdict.
Therefore, the weak-assertion pool and all other bins continue to follow the previous criteria. As a
result: Gate Neither Passes 14 → 11 · Pending Judgment 923 → 926; Disposition Retirement Candidate
103 → 100 · Pending Judgment 411 → 414.

### Review of the Existing 11 “Neither Passes” Cases

After reading all 11 repository-wide “Neither Passes” cases in the original source, **all 11
genuinely contain no assertions** (including no `assert*` / `check*` helpers, no `match` + `panic!`
form, and no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                     | Review Conclusion                                           |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 calls to `.expect(...)`, merely ensuring that nothing crashes                                 | Genuinely no assertions (only proves that nothing panicked) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains a `context()` handle and discards it                                                    | Genuinely no assertions                                     |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` values, obtains the context, and discards it                      | Genuinely no assertions                                     |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Genuinely no assertions                                     |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Genuinely no assertions                                     |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                              | Genuinely no assertions                                     |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                        | Genuinely no assertions (only proves that nothing panicked) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Genuinely no assertions (only proves that nothing panicked) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Genuinely no assertions (only proves that nothing panicked) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs the lsp / repl and discards them (within the wasm32 cfg gate)                        | Genuinely no assertions                                     |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertions                                     |

Five of these (`test_normalizer_context` and the 4 cases in reducer.rs) were kept in the 0.6-11a
decision table because **no replacement coverage could be identified**; the conclusion of this
review agrees with that result.

### Reverse Spot Check: Did the Relaxation Go Too Far?

Five cases were selected with a fixed seed (20261107) from the 88 cases newly recognized as
assertions, and their original source was read to confirm that they really assert:

| Spot-Check Case                                             | File                                                 | Assertion in the Original Source                                                                                                          |
| ----------------------------------------------------------- | ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching arm, with `panic!("Expected AssocType")` otherwise                                                 |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm, with `panic!("expected InvalidPackage, got …")` otherwise                     |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | An `assert_eq!` comparing constraint types in the matching arm, with `panic!("不是 Dynamic 变体…")` otherwise                             |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm, with `panic!("Expected TokensDelta variant")` otherwise                           |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls verifying the error code and message in the matching arm, with `panic!("不是 Disproved…")` otherwise |

All 5 assert concrete values or concrete variants (replacing the implementation with another correct
path would make them fail)—**the relaxation did not go too far**. Reverse controls were also added
to `--selftest`: a `match` with no `panic!`, and a `panic!` that appears only in a comment; neither
may be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

When the generator is reverted to the old implementation (`HELPER_RE` without digits + recognition
of `match` + `panic!` disabled), all newly added `--selftest` cases **turn red**, with exit code 1;
the new implementation passes them all.

### Rollback

The generator and census table are in the same commit; `git revert <sha>` restores everything as a
whole.

## Fourth Retirement Batch (Executed 2026-11-07)

**Fourth batch: retire 1 and keep 9 of the 10 “Retirement Candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter-backend test directories)** (single directory cluster;
three files were read together before making a decision: 4 cases in `tests/bytecode_load.rs` · 2 in
`tests/weak.rs` · 4 in `executor/tests/debug.rs`). Criterion = the **Three Gates** in Part Seven of
coding-rules; keep criterion = “a unit-level observation point may only be replaced by another unit
test; corpus/integration tests operate at the process-boundary layer”; **a deletion must name a
specific replacement coverage test, and if one cannot be identified or understood, the case is
kept**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`) — `test_capture_stack_during_execution` in
  the same file follows the same path (`make_module` + `load_module_for_stepping`, one step, then
  obtain `capture_stack`), and its assertions are **strictly stronger**: it asserts that the stack
  is nonempty and that `stack[0].function_name == "main"` (pinning down the top-of-stack position),
  whereas the listed test asserts only `stack.iter().any(|f| f.function_name == "main")` (any
  position is sufficient) —the former being true necessarily implies the latter ⇒ zero criterion
  increment.
- **9 kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each in the original
  source, each is the **sole unit observation point** for a particular property (the distinct load
  magic-number/version branches, the YXBC routing layer, both sides of Weak Some/None, stepping with
  no frames, popping the stack in step_out, and running to an empty final stack); no unit test
  capable of replacing them could be identified ⇒ they are kept as required. The basis for each case
  is recorded in the commit message (the decision table is not included in this page).
- **Numbers**: repository-wide `#[test]` 3409 → 3408; 1 deletion = `git diff` counts `-#[test]` on 1
  line and `+#[test]` on 0 lines; the dominating test `test_capture_stack_during_execution` passes
  on its own. Census: Retirement Candidate 99 → 98 · Pending Judgment 925 → 924 · Clause 1713 ·
  History 313 · Proof 447 · Neither Passes 11 · Needs Review 2121 · Keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Fifth Retirement Batch (Executed 2026-11-07)

**Fifth batch: mechanically retire 1,353 unit tests using the rule “for the same symbol and result
category, keep only the strongest case.”** (Only unit tests under `src/**/tests/**` were changed;
not a single case on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (mechanically applied, without individual justification)**: for the same system-under-test
  symbol and the same assertion result category, keep only the test with the strongest assertion and
  delete the rest.
  - System-under-test symbol = a repository-internal symbol called by the test body (including one
    level of propagation through a helper function in the same file); when multiple symbols are
    reached, classify by the **primary symbol** (the first repository-internal call). Symbols are
    qualified by repository-internal path and are not merged across modules by bare name.
  - Result category = concrete value / a specific error code (E code) / a specific variant match /
    only checking ok / only checking err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when equally strong within a group, keep the
    first occurrence in the file.
  - If a test asserts multiple categories at once (making the primary category indeterminable),
    **keep it** rather than guessing.
- **Three exceptions (not one case removed)**: ① the name or comment names `issue #` / regression /
  defect / incident (History Gate); ② a symbol has only one test (deleting it would leave the symbol
  without a unit observation point); ③ `#[should_panic]` is the symbol’s only should_panic case. **0
  cases** were retired from the 313 History Gate cases or the 11 “Neither Passes” cases.
- **Numbers**: repository-wide `#[test]` 3408 → 2055; 1,353 deletions = `git diff` counts `-#[test]`
  on 1,353 lines and `+#[test]` on 0 lines. Census by gate (current values after the 0.6-19
  retirement of the sample audit): Clause 808 · History 313 · Proof 424 · Pending Judgment 499 ·
  Neither Passes 11. Census by disposition (current values): Keep 737 · Needs Review 1054 · Pending
  Judgment 203 · Retirement Candidate 61. The retirement count for this batch by gate was: Clause
  902 · Pending Judgment 428 · Proof 23 (at 0.6-19, 9 cases previously classified as
  “audit-confirmed genuine support” were reclassified as Needs Review, so the current
  gate/disposition values differ from those at the close of the fifth batch).
- **Cleanup accompanying the deletions**: helper functions used only by deleted tests, imports that
  became unused, and structure fields that became obsolete were removed
  (`cargo clippy --all --all-features --all-targets -- -D warnings` produced 0 warnings).
- **Two out-of-scope deletions that must be recorded**: 29 cases originally classified as “Keep”
  were nevertheless deleted by this batch’s rules—23 current-state anchors (names containing
  unchanged/anchor/guard/preserve/baseline, many of which are false matches on **domain terms** such
  as “guard” or “preserves,” for example `test_while_guard_scope_balanced`) and 6 cases with
  audit-confirmed genuine clause support. This batch followed the closed three-exception list on the
  ruleset surface and did not add protection for these two categories; to restore them, fill them
  back according to the keep/delete comparison table on this page.
- **Failure and retirement of the sample audit**: the 40-case sample was drawn from the test pool
  using a fixed seed. After the fifth batch’s retirement, 38 cases from the original sample no
  longer existed, so the audit’s `--check` consistently reported “38 missing, 38 extra.” The sample
  audit was retired and deleted at 0.6-19 (the script, data file, and generator dependency on it
  were all removed); see “Retirement of the Sample Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Sixth Retirement Batch (Executed 2026-11-26)

**Sixth batch: retire the 28 tests deleted along with the mechanism during the 4.9.9 narrowing
round’s mechanism rollback; the 4 prohibition criteria were never checked in.** This batch does not
use the Three Gates to decide what “should be kept”; instead, the system under test was **rolled
back as a whole**: the criteria written for the downstream ambiguity mechanisms in the first six
4.9.9 rounds (candidate-provider priority / candidate-signature comparison / owner-compatibility
inference / IR-side named-argument fallback / imported-name resolution point E5006 prohibition) were
retired along with the mechanisms themselves—the mechanism no longer exists, so the tests have no
system under test. These deletions are recorded as a “batch” under coding-rules 7.3 and do not enter
the retirement-candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 cases (28 deleted: 27
    downstream-mechanism criteria from the first six rounds
    - 1 alias-import named-argument criterion). The 2 retained cases pin down the minimal fix for
      bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: all 4 E5006 prohibition criteria retired—the file
    existed only in an intermediate 4.9.9 commit and was included in neither the parent commit nor
    that version’s checked-in table; after the prohibition was withdrawn, there was no system under
    test, so this batch does not reduce the checked-in table.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the alias-import candidate-injection surface was rolled back, and the fixture’s
    behavior no longer holds); the same-named corpus differential-baseline entry was deleted in the
    same batch. The baseline is recorded separately: **405 → 404 lines relative to the direct parent
    commit `ac810e97`** (the intermediate state once included this fixture); **403 → 404 lines
    relative to the pre-4.9.9 baseline `08ccacc5`** (+1 base fixture `named-args-cross-unit-method`;
    the retired `alias-overload` never entered the final history).
- **Which gate applies**: the 2 retained cases = **History Gate** (they pin down a real defect
  registered in 4.9.9: cross-unit named arguments could not obtain declared parameter names, leaving
  `E1014 available` empty). Most of the 28 retired cases from the first six rounds were originally
  under the History Gate, but the reason for their deletion in this batch **is not** one of the
  Three Gates (they were not judged worthless); it is that the system under test was rolled back.
- **Replacement coverage**: cross-unit named arguments using bare-name imports = the 2 retained
  driver-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity surface
  no longer has rejection logic (the requirement was rolled back), so there is no replacement
  criterion; the 4.9.8 dispatch-attribution criteria (10 driver + 5 integration cases) are retained
  unchanged.
- **Restoration**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the empty-name-table guard for
  `method_param_info` is retained; this form is unreachable in the public syntax and is a defensive
  guard whose unit test merely prevents accidental deletion).
- **Numbers** (after this batch’s regeneration): repository-wide `#[test]` 2680 cases / 334 files;
  the previous checked-in table had 2688 cases → the current tree has 2680 cases (−8); the current
  tree has 1 case absent from the old table, and the old table has 9 cases absent from the current
  tree; 0 rows have recomputed content under the same key.
- **Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit); `git revert <sha>` reverts it as a whole.

## Retirement of the Sample Audit (0.6-19)

**Criterion**: the sample audit (0.6-10) measured the “true support rate” of clause citations, but
the keep rule after 0.6-18 no longer examines citations (it changed to “for the same symbol and
result category, keep only the strongest case,” and the test pool shrank from 3,408 to 2,055). Of
the original 40 samples, 38 no longer exist, so the audit’s `--check` consistently reported “38
missing, 38 extra”—keeping it would only produce a permanent alarm. Therefore, it was **retired and
deleted**.

**What was deleted**: the sample-audit script under `scripts/dev/` (the one that reproduced the
40-case sample using a fixed seed) and its JSON data file (40 per-case decisions and bases).

**Generator dependency removed in sync**: the generator originally read that JSON and classified
“audit-confirmed genuine support” as “Clause Gate + Keep”; that branch has been removed—**clause
citations no longer have an “audit-confirmed” tier** and always fall under “Clause / Needs Review.”

**Distribution changes** (not one of the repository’s 2,055 `#[test]` cases was changed; the TSV was
regenerated): Keep 746 → 737 (−9: the 9 cases previously confirmed by the audit as genuinely
supported moved back to Needs Review); Needs Review 1045 → 1054; by gate: Clause 811 → 808, Pending
Judgment 496 → 499 (the others unchanged: History 313 · Proof 424 · Neither Passes 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` reverts it as a whole.

## Rerun and Self-Test

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical classification only, and its boundaries are stated explicitly:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as having “no assertions”—the first batch of 26 cases (21 retired / 5 kept) and the
  second batch of 18 cases (11 retired / 7 kept) were all manually reviewed function body by
  function body.
- **Function-body boundaries (fixed at 0.6-13)**: when the generator obtains a function body, it
  balances braces, so braces in string/character literals and comments can skew the balance and
  truncate the function body—`test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was
  once truncated to the end of the file, causing the “assertion summary” to capture the rest of the
  file and the case to be binned as “Pending Judgment” rather than “Retirement Candidate.” The
  generator now masks non-code regions (string/character literals, line comments, block comments,
  and raw strings) before balancing; see “Generator Correction” below.
- **Delegated-assertion recognition (fixed at 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not include digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions, and tests containing such calls
  would be misclassified as having “not a single assertion in the body.” At the time, 4 of the
  repository’s 15 “Neither Passes” cases actually contained assertions (3 numeric helpers + 1
  `match` + `panic!` form). The name portion has now been widened to `[A-Za-z0-9_]*`, and
  recognition of the `match` + `panic!` form has been added; see “Generator Criteria Correction”
  below.
- **The Proof Gate is not judged**: every test that fails both gates is classified only as “Pending
  Judgment” or “Retirement Candidate”; the generator never declares that it “should be deleted.”
- **No cross-file deduplication**: completely duplicated tests are not identified in this table
  (0.6-2 measured them at only 1%, which does not constitute a retirement surface).
- **Clause citations always require review**: a verifiable section number in the file header only
  means that the test “maps to an individually identifiable clause”; it does not mean that the
  citation supports the assertions. The machine does not judge whether it does, so even a Clause
  Gate match only reaches “Needs Review” and must be verified by a person before it can be kept.
