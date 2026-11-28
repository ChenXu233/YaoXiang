---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the Three Gates in Part Seven of
  coding-rules, and provide a retirement-candidate list and retirement records for each batch'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6, “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **Three Gates (Clause / History / Proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a reproducible, per-test adjudication
table. A sample audit of the Clause Gate (40 entries) once measured the proportion of tests that
“cited a clause but whose assertions were not supported by it” and thereby **tightened the
criteria**. That audit was repealed and removed in 0.6-19. **This page and the generator itself only
read tests and do not modify them**; test retirement is executed in batches (one commit per batch,
revertible as a whole with `git revert`), and the results are recorded in the batch-retirement
subsections below and in the commit messages.

Generated artifacts:

| Artifact                            | Purpose                                                                                     |
| ----------------------------------- | ------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Per-test adjudication table (2732 rows plus the header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                                 |

## Criteria

### Scan Surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 2128                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2732** (343 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine Criterion                                                                                                                                                                                                                                                                                                                                                                                                                                          | Meaning of a Match                                        |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| Clause Gate (tightened in 0.6-10) | A **verifiable entry** appears within the first 25 lines of the file header: a section-number form such as `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); files under `src/**/tests/` are inspected together with their parent `mod.rs`                                                                                                                                                                                            | The test maps to a specific, citable clause               |
| History Gate                      | The test function body, function doc comment, or **file header** (for example, `//! Regression context: issue #226`) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                     | The test pins down an incident that has actually occurred |
| Proof Gate                        | **The machine cannot decide this** (“Would it fail if replaced with another correct implementation?” requires human judgment). Therefore, everything that misses the first two gates is classified as “Undecided” and is never automatically classified as “Neither Gate Passes”; the sole exception is a test with **no assertions at all** (including no delegation to `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only “Undecided” or “No Assertions” can be assigned       |

Rationale for tightening: the Clause Gate criterion in 0.6-9 only required the file header to
**contain the characters `§`/`RFC`/`SPEC`**, resulting in 2403 tests being classified as “Keep”. A
40-entry stratified sample in 0.6-10 found that **half of the citations did not support the
assertions** (mis-citation rate: 50%; 95% interval: 35%–65%)—this finding was the **direct reason
for abandoning “keep based on a single citation.”** After tightening, only section-number forms are
recognized, and **having a citation still does not mean that the citation supports the
assertion**—all Clause Gate citations remain “Pending Verification”. (The sampling audit was
repealed and removed in 0.6-19: as the test pool shrank through retirement, 38 of the original
sample entries no longer existed, and `--check` consistently reported “missing 38, extra 38.” The
conclusion is retained, but the audit artifacts are no longer maintained.)

Rationale for the file-header radius: this repository’s convention is to put “Regression context:
issue #N” in the file header rather than repeat it in each test comment. Looking only at the test’s
own comment would miss the history of the entire file (measured: 7 entries at the test’s own radius,
291 entries at the file-header radius).

### Categories That Are Never Retired

The following tests are **always classified as “Keep”** and never enter the retirement-candidate
pool:

- Corpus `tests/yaoxiang/**` and project-form `tests/project-corpus/**` (behavior-criterion
  surfaces);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and crossing the process boundary);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                                |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Keep                 | Only two cases: the History Gate matches, or the Proof Gate applies (spec-shaped directories / current-state anchor names)                                                                             |
| Pending Verification | A clause is cited (with a section number, or only an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a human before the test can be kept |
| Retirement Candidate | Both gates fail and the assertion surface contains only weak assertions (or no assertions)—a candidate is not a verdict; the Proof Gate still requires manual review                                   |
| Undecided            | Both gates fail, but strong assertions are present; the machine reaches no conclusion                                                                                                                  |

## Distribution (regenerated on 2026-11-28; 4.9.9 narrowing round, 8.7 batch, 4.9.10-L0, init-test rewrite, D53 batch, L3-1b S1, 7e batch, and the #593 criterion)

| Gate                                                           | Count    |
| -------------------------------------------------------------- | -------- |
| Clause (has a verifiable entry)                                | 1041     |
| History                                                        | 568      |
| Proof (spec-shaped criterion surfaces / current-state anchors) | 574      |
| Undecided (both gates fail; Proof Gate awaits manual review)   | 543      |
| Neither Gate Passes (no assertions in the body)                | 6        |
| **Total**                                                      | **2732** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1142** | 737 (+405)                        |
| Pending Verification | 1301     | 1054 (+245)                       |
| Undecided            | 230      | 203 (+27)                         |
| Retirement Candidate | 59       | 61 (−2)                           |
| **Total**            | **2732** | 2055 (+655)                       |

Structure of “Keep”: 1142 = Proof Gate 574 (518 spec-shaped criterion surfaces + 56 current-state
anchors) + History Gate 568. File-level scope: the Clause Gate covers **126 of the 343** test files.

Regeneration reconciliation: the previous checked-in table contained 2728 entries, while the current
tree contains 2732; the current tree has 4 entries absent from the old table (the 4 direct-driver
cases in 7d.2), the old table has 0 entries absent from the current tree, and 0 same-key rows
changed content. Every key is recorded in the TSV.

The latest full regeneration (after the 7d batch and the upstream #593 criterion were merged)
contains 2732 rows and matches the current tree key by key (`--check` exits with 0).

### Two Criteria Changes at the 0.6-19 Tightening Point

1. **Structure of “Keep”**: 737 = Proof Gate 424 (390 spec-shaped criterion surfaces + 34
   current-state anchors) + History Gate 313. Of the original 2897 entries, the 2121 that were “kept
   based on a single citation” are now Pending Verification (after the sampling audit was repealed
   in 0.6-19, the 9 entries that had been classified as “audit-confirmed as genuinely supporting the
   assertions” also became Pending Verification—Clause citations no longer have an “audit-confirmed”
   tier).
2. **History Gate 47 → 313**: this does not represent new history, but rather **relabeling**—in
   0.6-9, cases where the Clause and History Gates both matched were recorded as Clause matches.
   After tightening, a Clause Gate match no longer automatically resulted in “Keep”, so these cases
   had to be caught by the History Gate and were included in its count. The total number kept was
   therefore unaffected (this batch was also classified as “Keep” in 0.6-9).

File-level scope at the time: of 285 test files, **136** had a verifiable section number in their
file headers (the old 0.6-9 criterion applied to 221 files).

## Reconciliation Between the 0.6-19 Snapshot and the 0.6-2 Survey

The figures from the 0.6-2 survey are the baseline for the first comparison. The reasons for each
difference are listed below:

| Item                           | 0.6-2 Survey                  | Current                                                                                                           | Reason for the Difference                                                                                                                                                                                                                                                               |
| ------------------------------ | ----------------------------- | ----------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`      | 3443                          | 2055                                                                                                              | 2 were added after 0.6-2 (check-criterion regression tests for E3005 and E3022); 21 assertion-free tests were removed in the first batch, 11 weak-assertion tests in the second batch, 2 in the third batch, 1 in the fourth batch, and 1353 in the fifth batch; net −388               |
| Number of test files           | 296                           | 285                                                                                                               | 11 entire files were retired in the fifth batch                                                                                                                                                                                                                                         |
| Clause Gate                    | 227 files (77%)               | 136 files after tightening; 808 entries                                                                           | The criterion was tightened from “the characters appear” to “a section number `§<number>` is present,” nearly halving the file count; the fifth batch retired 902 entries, and the repeal of the sampling audit in 0.6-19 moved another 9 entries from “Keep” to “Pending Verification” |
| History signal                 | 442 entries / 148 files       | 313 entries within the gate; three radii: test’s own 7 / file header 291 (38 files) / entire file 1218 (79 files) | The 148 files in the 0.6-2 survey covered a broader scope than the current 79 files at the entire-file radius, because its vocabulary and scope were larger; the current calculation strictly uses the four signals listed above                                                        |
| Characterization strong signal | 1176 (34%) / strong 631 (18%) | Not tested                                                                                                        | Characterization is not one of the Three Gates; including it in a retirement conclusion would exceed the gate’s authority                                                                                                                                                               |
| Weak assertions                | 473 (14%)                     | See the “Assertion Summary” column in the TSV                                                                     | The counting units differ: 0.6-2 counted patterns that asserted only `is_ok`/`is_some`; the current calculation uses lexical counts and distinguishes strong, weak, and delegated assertions                                                                                            |

## Summary of the 0.6-19 Conclusions (Historical)

1. **The Clause Gate had previously been substantially overstated**: under the old criterion, 2403
   tests were classified as “Clause/Keep”. The sample measured a 50% mis-citation rate (95%
   interval: 35%–65%)—a “citation” and “a citation that supports the assertion” are different
   things, but the old criterion treated the former as the latter. (The sampling audit was repealed
   and removed in 0.6-19; its conclusion is retained. See “Repeal of the Sampling Audit” below for
   the reason for removal.)
2. **After tightening, approximately 1202 tests genuinely have Clause support** (range: 846–1557);
   the rest either await verification or should be adjudicated through the History or Proof Gates.
3. **The retirement surface is unrelated to Clause mis-citations**: the 136 candidates were
   unrelated to clauses (no assertions / weak assertions); 21 were retired in the first batch and 11
   in the second; the generator fix contributed +1, the third batch −2, the 0.6-15 criterion
   correction −3, the redundancy scan −1, the fourth batch −1, and the fifth batch −1353
   (mechanically retiring tests under the rule “for the same symbol and result category, keep only
   the strongest one”), reducing the candidate count to 61. The fifth batch’s retirement count by
   gate was: Clause 902 · Undecided 428 · Proof 23 · History 0 · Neither Gate Passes 0—**not one
   History Gate or zero-assertion test was removed**.
4. **The History Gate is the largest legitimate source of “Keep”** (313 entries): it demonstrates
   that “pinning down a real incident” is more reliable than “copying a sentence from an RFC”.

## First Retirement Batch (Executed, 2026-11-06)

**First batch: of the 26 assertion-free tests under `src/frontend/core/types/eval/tests/`, 21 were
retired and 5 were retained** (a single directory cluster; criterion = all **Three Gates fail**
according to Part Seven of coding-rules: no Clause Gate match, no History Gate match, and no
assertions in the body). The per-test adjudications and “same-cell coverage” are provided by the
0.6-11a adjudication table (21 removed / 5 retained); this page does not repeat the reasons and
records only the execution results.

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
- **5 retained** (the adjudication table could not identify replacement coverage, so they were kept
  as required): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`, and
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls it back as a whole.

## Second Retirement Batch (Executed, 2026-11-06)

**Second batch: of the 18 weak-assertion tests in `src/frontend/core/parser/pratt/tests/nud.rs`, 11
were retired and 7 were retained** (a single directory cluster = the Pratt prefix-parsing module;
criterion = all **Three Gates fail** according to Part Seven of coding-rules: no Clause Gate match,
no History Gate match, and the body contains only weak assertions such as `is_some()` /
`is_none()`—neither binding power nor the parsing result is pinned down).

The difference between this batch and the first must be made clear: these tests **do contain
assertions**; they are not empty tests. The reason for retirement is “all Three Gates fail + zero
incremental criterion coverage,” not “there are no assertions.” Every test classified for removal
names a **unit-level replacement-coverage test in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Each replacement
  follows the **same** `parse_expression → parse_prefix → prefix_info` path and makes stronger
  assertions (pinning down the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, plus `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 retained**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`, and
  `test_prefix_info_eof`—the per-test reasons are in the commit message (Task 0.6, second
  test-retirement batch). There are two categories of reason for retention: the **Proof Gate
  applies** (the first three, together with `test_prefix_info_at` and `test_prefix_info_eof`, are
  the **only unit-level** walk-through points for the corresponding facts, and the repository
  contains no second unit test that observes them), or the **History Gate applies** (`break_removed`
  / `continue_removed` name #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls it back as a whole.

## Generator Fix (0.6-13)

**`body_span` truncated function bodies when it encountered braces inside literals—this has been
fixed.**

The old implementation found function bodies by balancing braces and did not distinguish real code
from braces in strings/character literals or comments. Code such as `tokenize("{")` is common in
Rust, so the balance became skewed and the function body was truncated: the body of
`test_prefix_info_lbrace` once ran to the end of the file, and the “Assertion Summary” picked up the
rest of the file (`matchesx11 assertx24 …`), causing it to be classified as “Undecided” rather than
“Retirement Candidate”—**an incorrect classification**.

Fix: add `mask_noncode`, first replacing the contents of string literals / character literals / line
comments / block comments (which may be nested) / raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (preserving newlines, line numbers, and line counts), and then balance the
braces. **The mask is used only to locate function-body boundaries**; assertion statistics and the
History Gate still read the original text, and the criteria themselves have not been relaxed.
`--selftest` now includes 4 cases (paired and unpaired braces in ordinary strings, braces in raw
strings and block comments, and braces in character literals), each followed by a sentinel test—the
old implementation found the wrong boundary in all 4 cases, while the new implementation finds all
of them correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** test by test and
file by file (the old implementation balanced literals directly; the new implementation balances
them after masking). A test is affected if its boundaries differ. The old implementation selected
incorrect boundaries for **11 of the 3413** tests in the repository:

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

### Classification Differences Before and After the Fix: 2

In the other 9, **8 affected only the “Assertion Summary” column** (the classification did not
change), and **1 did not change even the summary** (`test_fstring_close_brace_escape_preserved_raw`:
the old implementation omitted the closing `}` line, which contained no assertion words). The
changes to the 8 summaries are listed individually in the commit message.

| Test Name                   | File                                          | Old Classification    | New Classification                   | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --------------------------- | --------------------------------------------- | --------------------- | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Undecided / Undecided | Undecided / **Retirement Candidate** | The actual function body contains only one weak assertion, `assert!(state.prefix_info().is_some())`; the file header and function body contain no `§` section number and no issue/regression/defect/incident ⇒ both the Clause and History Gates fail, and the body contains only a weak assertion such as `is_some()` ⇒ Retirement Candidate (a candidate is not a verdict; the Proof Gate still requires manual review)                                                       |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | History / Keep        | Clause / **Pending Verification**    | The old implementation truncated the function body to the end of the file and swallowed text containing “regression/defect/incident,” falsely classifying it under the History Gate; the actual function body contains no such wording ⇒ the History Gate no longer applies; the file header has verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the Clause Gate matches, but it has not been confirmed by the sampling audit ⇒ Pending Verification |

The distribution changes accordingly: Clause 1713 → 1714 · History 314 → 313 · Keep 776 → 775 ·
Pending Verification 2121 → 2122 · Undecided 412 → 411 · Retirement Candidate 104 → 105. **The
repository-wide `#[test]` count is unchanged (3413)**—this round changed only the implementation of
the criteria, not any test.

## Third Retirement Batch (Executed, 2026-11-07)

**Third batch: the 5 Retirement Candidates under `src/middle/passes/codegen/tests/` were adjudicated
test by test; 2 were retired and 3 were retained.** (A single directory cluster = the
`middle::passes::codegen` module under test; criterion = all **Three Gates fail** according to Part
Seven of coding-rules.)

This batch follows the **retention criterion** established by the previous batch: a unit-level
observation point may only be replaced by another **unit test**; corpus and integration tests belong
to the process-boundary layer and cannot replace a unit-level observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is **literally
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (apart from
    comments, only the function name differs): both use `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that the `execute_module` entry
    path returns a non-Void value, but the body never observes the return value of `execute_module`
    (`run_fixture` discards it with `.expect(...)`)—the claimed intent is not implemented, and the
    function body is a pure duplicate ⇒ zero incremental criterion coverage. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file strictly covers it:
    a positive lookup (`lookup("a").is_some()`), a negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` is identical in shape to the inline `Symbol` in this
    test). “A name that was never inserted cannot be found” and “it cannot be found after being
    popped” follow the same path in `SymbolScopeManager::lookup` (each layer executes
    `scope.get(name)`, returning `None` when there is no match) ⇒ this test adds zero coverage.
    Replacement coverage = `test_scope_nesting`.
- **3 retained**:
  - `test_global_slot_store_then_load_roundtrip`—the **only** observation point for the path that
    does not go through serialization to connect directly to execution (the serialization variant is
    a different path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the only observation point for the
    serialization roundtrip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **only** unit-level observation point for the
    `id > 255` rejection branch of `OperandResolver::to_reg` (the bytecode layer has its own
    independent overflow checks and does not cover this function; no second test in the repository
    drives `to_reg` out of bounds). `is_err()` is a discriminating assertion (removing the guard
    would make the test fail), not merely a check that it did not panic.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls it back as a whole.

### One Census Blind Spot Discovered in This Batch (**Fixed in 0.6-15**; see “Generator Criterion Correction” below)

The census’s recognition of “delegated assertions” uses `\b(assert|check)[A-Za-z_]*\s*\(` and
**cannot match helper names containing digits**: `[A-Za-z_]*` does not include digits, so a call
such as `assert_slot_is_42(&interp)` is not recognized as an assertion—2 of the 5 candidates in this
batch were consequently misclassified as “no assertions in the body”.

After reviewing all 15 repository-wide “Neither Gate Passes” entries (by allowing digits in helper
names and additionally recognizing the `match` + `panic!` form), **4 did in fact contain
assertions**—3 used the numbered helpers above (`assert_slot_is_42`), and 1 was `test_weak_from_arc`
in `src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule
6.1 requires `assert!(matches!(...))`, but it does assert). The remaining 11 genuinely had no
assertions.

**Applying the machine’s “Neither Gate Passes” classification directly to these 4 would have deleted
tests that contained criteria**—the batch’s test-by-test human review prevented that; the
generator-side fix is described in “Generator Criterion Correction (0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots have been fixed: digits in helper names and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411); it
changes only how the generator recognizes whether a test body contains assertions.

### What Was Fixed

1. **Digits are now allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing such calls were
   treated as having “no assertions in the body”.
2. **Added recognition of the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the
   assertion is written in one of the arms, and lexically there is no `assert!` macro
   (test-specification rule 6.1 requires `assert!(matches!(...))`, but existing code contains this
   form). Recognition method: scan the expression after the `match` keyword up to the first `{`,
   determine the block boundary by **balancing braces**, and only then count it as an assertion if
   `panic!(` appears in the block. **The decision is made on masked lines**—braces in strings and
   comments do not participate in balancing, and `panic!` in comments is not treated as an
   assertion.

### Classification Differences: 3 (the gate and disposition changed)

| Test Name                                           | File                                           | Old Classification                         | New Classification    | Basis                                                                                                                                                                                                                                                        |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------------ | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither Gate Passes / Retirement Candidate | Undecided / Undecided | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither Gate Passes / Retirement Candidate | Undecided / Undecided | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern could not match; it is now recognized as an assertion                                                                                     |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither Gate Passes / Retirement Candidate | Undecided / Undecided | Same as above (the serialization-roundtrip test)                                                                                                                                                                                                             |

Another **77 entries changed only the “Assertion Summary” column** (the gate and disposition did not
change): most gained `match_panicxN` in the summary (the body contains the `match` + `panic!` form),
and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3; `src/package/tests/cache.rs`:
1—helper names containing digits).

**The relaxation changes only the “Neither Gate Passes” tier**: that tier is the machine’s
**confident verdict** (no assertions in the body), and its criterion is a hard requirement;
“Retirement Candidate” is only a pool awaiting manual review, so the size of that pool is not a
verdict. All other classifications, including the weak-assertion pool, continue to use the original
criteria. Results: gate Neither Gate Passes 14 → 11 · Undecided 923 → 926; disposition Retirement
Candidate 103 → 100 · Undecided 411 → 414.

### Review of the Existing 11 “Neither Gate Passes” Entries

After reading the original source for all 11 repository-wide “Neither Gate Passes” entries, **all 11
genuinely contain no assertions** (no `assert*` / `check*` helpers, no `match` + `panic!` form, and
no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                   | Review Conclusion                                           |
| ----------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 calls to `.expect(...)`, which only guarantee that it does not crash                        | Genuinely no assertions (only proves that it did not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                | Genuinely no assertions                                     |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` values, obtains their contexts, and discards them               | Genuinely no assertions                                     |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                           | Genuinely no assertions                                     |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                     | Genuinely no assertions                                     |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                            | Genuinely no assertions                                     |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                      | Genuinely no assertions (only proves that it did not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                              | Genuinely no assertions (only proves that it did not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                        | Genuinely no assertions (only proves that it did not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg guard)                         | Genuinely no assertions                                     |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertions                                     |

Five of these (`test_normalizer_context` and the 4 in reducer.rs) were classified as “Keep” in the
0.6-11a adjudication table because **no replacement coverage could be identified**; the conclusion
of this review is consistent with that table.

### Reverse Spot Check: Did the Relaxation Go Too Far?

Five entries were selected with the fixed seed (20261107) from the 88 entries newly recognized as
assertions, and their original source was read to confirm that they really do assert:

| Spot-Check Entry                                            | File                                                 | Assertions in the Original Source                                                                                                                 |
| ----------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` assertions in the matching arm; `panic!("Expected AssocType")` if there is no match                                              |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm; `panic!("expected InvalidPackage, got …")` if there is no match                       |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | An `assert_eq!` comparing the constraint type in the matching arm; `panic!("不是 Dynamic 变体…")` if there is no match                            |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm; `panic!("Expected TokensDelta variant")` if there is no match                             |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` assertions in the matching arm verify the error code and message; `panic!("不是 Disproved…")` if there is no match |

All 5 assert concrete values or concrete variants (replacing the implementation with another correct
path would make them fail)—**the relaxation did not go too far**. Reverse controls were also added
to `--selftest`: a `match` without `panic!`, and `panic!` appearing only in a comment; neither may
be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

When the generator is reverted to the old implementation (`HELPER_RE` does not allow digits +
recognition of `match` + `panic!` is disabled), all the newly added `--selftest` cases **fail**, and
the exit code is 1; the new implementation passes them all.

### Rollback

The generator and census table are in the same commit; `git revert <sha>` restores everything as a
whole.

## Fourth Retirement Batch (Executed, 2026-11-07)

**Fourth batch: of the 10 “Retirement Candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend’s test directories), 1 was retired and 9 were
retained** (a single directory cluster; all three files were read before adjudication:
`tests/bytecode_load.rs`: 4 · `tests/weak.rs`: 2 · `executor/tests/debug.rs`: 4). Criterion = the
**Three Gates** in Part Seven of coding-rules; retention criterion = “a unit-level observation point
may only be replaced by another unit test; corpus and integration tests belong to the
process-boundary layer”; **retirement requires naming a specific replacement-coverage test; if one
cannot be identified or understood, the test is retained**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, followed by one step
  and then `capture_stack`) and makes **strictly stronger assertions**: it asserts that the stack is
  nonempty and that `stack[0].function_name == "main"` (pinning down the top-of-stack position),
  while the retired test asserts only `stack.iter().any(|f| f.function_name == "main")` (any
  position is sufficient)—if the former is true, the latter must also be true ⇒ zero incremental
  criterion coverage.
- **9 retained**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each one’s original
  source, each is the **only unit-level observation point** for a particular property (the separate
  magic-number and version branches of load, the YXBC routing layer, both the Some and None sides of
  Weak, stepping with no frames, step_out popping the stack, and running until the stack is empty).
  No unit test that could replace them could be identified ⇒ they are retained as required. The
  per-test basis is in the commit message (the adjudication table is not included on this page).
- **Figures**: repository-wide `#[test]` 3409 → 3408; 1 classified for removal = `git diff` counts 1
  `-#[test]` line and 0 `+#[test]` lines; the dominating test `test_capture_stack_during_execution`
  passes when run on its own. Census: Retirement Candidate 99 → 98 · Undecided 925 → 924 · Clause
  1713 · History 313 · Proof 447 · Neither Gate Passes 11 · Pending Verification 2121 · Keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls it back as a whole.

## Fifth Retirement Batch (Executed, 2026-11-07)

**Fifth batch: 1353 unit tests were retired in bulk under the mechanical rule “for the same symbol
and result category, keep only the strongest one”** (only unit tests under `src/**/tests/**` were
changed; not one test on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (executed mechanically, without a per-test argument)**:
  - Tested symbol = a repository symbol called in the test body (including one level of propagation
    through a helper function in the same file); when multiple symbols are reached, classify by the
    **primary symbol** (the first repository call). Symbols are qualified by repository path and are
    not merged across modules based on bare names.
  - Result category = a concrete value / a specific error code (E code) / a specific variant match /
    checking only Ok / checking only Err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when tests in the same group have equal
    strength, retain the one that appears first in the file.
  - When one test asserts multiple categories (making the primary category indeterminate), **retain
    it**; do not guess.
- **Three exceptions (not one test was removed)**: ① the name or comment names `issue #` /
  regression / defect / incident (History Gate); ② a symbol has only one test (removing it would
  leave that symbol without a unit-level observation point); ③ the `#[should_panic]` test is the
  symbol’s only should_panic use case. **0 of the 313 History Gate entries and 0 of the 11 “Neither
  Gate Passes” entries were retired**.
- **Figures**: repository-wide `#[test]` 3408 → 2055; 1353 classified for removal = `git diff`
  counts 1353 `-#[test]` lines and 0 `+#[test]` lines. Census (by gate, current values after the
  repeal of the sampling audit in 0.6-19): Clause 808 · History 313 · Proof 424 · Undecided 499 ·
  Neither Gate Passes 11. Census (by disposition, current values): Keep 737 · Pending Verification
  1054 · Undecided 203 · Retirement Candidate 61. The batch’s retirement count by gate was: Clause
  902 · Undecided 428 · Proof 23 (because 0.6-19 reclassified the 9 entries previously
  “audit-confirmed as genuinely supported by their citations” as Pending Verification, the current
  gate/disposition figures differ from those at the time the fifth batch was closed).
- **Cleanup accompanying the removals**: helper functions used only by removed tests, imports that
  became unused, and struct fields that became invalid were all removed
  (`cargo clippy --all --all-features --all-targets -- -D warnings` reports 0 warnings).
- **Two out-of-scope removals that must be recorded**: 29 tests that the census had classified as
  “Keep” were still removed by this batch’s rule—23 current-state anchors (names containing
  unchanged/anchor/guard/preserve/baseline, many of which are false matches on **domain terms** such
  as “guard” or “preserves”, for example `test_while_guard_scope_balanced`) and 6 Clauses whose
  support had been confirmed by the audit. This batch followed the closed list of three exceptions
  on the criterion card and did not add protection for these two categories; to restore them, use
  the keep/remove correspondence table on this page.
- **Failure and repeal of the sampling audit**: the 40-entry sample was drawn from the test pool
  with a fixed seed; after the fifth batch’s retirement, 38 entries in the original sample no longer
  existed, so the audit’s `--check` consistently reported “missing 38, extra 38.” The sampling audit
  was repealed and removed in 0.6-19 (the script, data file, and the generator’s dependency on it
  were all removed); see “Repeal of the Sampling Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls it back as a whole.

## Sixth Retirement Batch (Executed, 2026-11-26)

**Sixth batch: 28 tests were retired because their mechanism was removed during the rollback of the
4.9.9 narrowing round; the criteria for another 4 prohibitions were never checked in.** This batch
does not use the Three Gates to decide “whether a test should be kept”; instead, the mechanism under
test was **rolled back as a whole**: the criteria written during the first six rounds of 4.9.9 for
downstream ambiguity mechanisms (candidate-provider priority / candidate-signature comparison /
owner-compatibility reasoning / IR-side named-argument fallback / imported-name resolution point
E5006 prohibition) were retired along with the mechanism. With the mechanism absent, the tests had
no object under test. Such removals are registered as a “batch” according to coding-rules 7.3 and do
not enter the retirement-candidate pool.

- **What was removed**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 (28 removed: 27 downstream-mechanism
    criteria from the first six rounds + 1 alias-import named-argument criterion). The 2 retained
    tests pin down the minimal fix for bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: the entire file containing 4 E5006 prohibition
    criteria was retired—the file existed only in an intermediate 4.9.9 commit, and neither the
    parent commit nor that version’s checked-in table included it; after the prohibition was
    withdrawn, there was no object under test, so this batch does not reduce the table’s row count.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the candidate-injection surface for alias imports was rolled back, so the fixture’s
    behavior no longer holds); the same-named corpus differential-baseline entry was removed in the
    same batch. The baseline is stated separately: **relative to the direct parent commit
    `ac810e97`, 405 → 404 lines** (the intermediate state had contained the fixture); **relative to
    the pre-4.9.9 baseline `08ccacc5`, 403 → 404 lines** (+1 baseline fixture
    `named-args-cross-unit-method`; the retired `alias-overload` never entered the final history).
- **Which Gate Applies**: the 2 retained tests = **History Gate** (they pin down a real defect
  registered in 4.9.9: cross-unit named arguments could not obtain the declared parameter name,
  leaving `E1014 available` empty). Of the 28 retired tests, most had originally matched the History
  Gate, but the reason for removal in this batch **was not** the Three Gates (it was not a judgment
  that they lacked value); the mechanism under test was rolled back.
- **Replacement coverage**: for cross-unit named arguments using bare-name imports, the 2 retained
  driver-level criteria plus the project fixture `named-args-cross-unit-method` provide coverage;
  the ambiguity surface no longer has rejection logic (the requirement was rolled back), so there is
  no replacement criterion; the 4.9.8 dispatch-attribution criteria (10 driver tests + 5 integration
  tests) are retained unchanged.
- **Restoration**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the empty-name-table guard for
  `method_param_info` is retained; this form is unreachable in the public syntax and is a defensive
  guard, with a unit test to prevent accidental deletion).
- **Figures** (after this batch’s regeneration): repository-wide `#[test]` 2680 / 334 files;
  previous checked-in table 2688 → current tree 2680 (−8); the current tree has 1 entry absent from
  the old table, the old table has 9 entries absent from the current tree, and 0 same-key rows
  changed content.
- **Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit); `git revert <sha>` rolls it back as a whole.

## Repeal of the Sampling Audit (0.6-19)

**Rationale**: the sampling audit (0.6-10) measured “the true support rate of Clause citations,” but
the retention rule after 0.6-18 no longer examines citations (it changed to “for the same symbol and
result category, keep only the strongest one,” reducing the test pool from 3408 to 2055). Of the
original 40 samples, 38 no longer existed, so the audit’s `--check` consistently reported “missing
38, extra 38”; retaining it would cause continuous alarms. It was therefore **repealed and
removed**.

**What was removed**: the sampling-audit script under `scripts/dev/` (the one that reproduced the
40-entry sample with a fixed seed) and its JSON data file (the 40 per-entry adjudications and
bases).

**The generator’s dependency was removed in sync**: the generator previously read that JSON and
classified “audit-confirmed as genuinely supporting the assertion” as “Clause Gate + Keep”; that
branch has been removed—**Clause citations no longer have an “audit-confirmed” tier** and always
fall under “Clause / Pending Verification”.

**Distribution changes** (all 2055 repository-wide `#[test]` entries remain unchanged, and the TSV
was regenerated): Keep 746 → 737 (−9: the 9 entries previously audit-confirmed as genuinely
supporting their assertions are now Pending Verification); Pending Verification 1045 → 1054; by
gate: Clause 811 → 808, Undecided 496 → 499 (all others unchanged: History 313 · Proof 424 · Neither
Gate Passes 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` rolls it back as a whole.

## Rerun and Self-Check

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (both runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical classification only, and its boundaries are stated truthfully:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as having “no assertions”—the first batch’s 26 tests (21 retired / 5 retained) and
  the second batch’s 18 tests (11 retired / 7 retained) were all manually reviewed function by
  function.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator obtains a function body, it
  balances braces. Braces in strings / character literals and comments skew the balance, truncating
  the function body—`test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was once
  truncated to the end of the file, causing the “Assertion Summary” to pick up the rest of the file
  and the classification to become “Undecided” rather than “Retirement Candidate”. The generator now
  masks non-code regions first (strings / character literals / line comments / block comments / raw
  strings) and then balances braces; see “Generator Fix” below.
- **Delegated-assertion recognition (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not allow digits in `[A-Za-z_]*`, so a call such as
  `assert_slot_is_42(&interp)` was not recognized as an assertion, and a test containing such a call
  would be misclassified as having “no assertions in the body”. At the time, 4 of the
  repository-wide 15 “Neither Gate Passes” entries actually contained assertions (3 numbered
  helpers + 1 `match` + `panic!` form). The name portion has now been relaxed to `[A-Za-z0-9_]*`,
  and recognition of the `match` + `panic!` form has been added; see “Generator Criterion
  Correction” below.
- **The Proof Gate is not adjudicated**: every test that misses both gates is classified only as
  “Undecided” or “Retirement Candidate”; the generator never declares that a test “should be
  deleted”.
- **No cross-file deduplication**: completely duplicate tests are not identified in this table
  (0.6-2 measured only 1%, which does not constitute a retirement surface).
- **Clause citations always remain pending verification**: a verifiable section number in the file
  header only means that the test “maps to a specific, citable clause”; it does not mean that the
  citation supports the assertion. The machine does not judge whether it does, so even a Clause Gate
  match only reaches “Pending Verification” and must be verified by a human before the test can be
  kept.
