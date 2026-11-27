---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement candidate list and retirement records for each batch'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the [Part Seven of coding-rules](./coding-rules.md) **three gates (Clause /
History / Proof)**, and produce a rerunnable item-by-item adjudication table. A sample audit of the
Clause Gate (40 cases) measured the proportion of tests that “cited a clause but whose citations did
not support the assertions” and tightened the criterion accordingly; that audit was deprecated and
deleted in 0.6-19. **This page and the generator itself are read-only and do not modify tests**;
test retirement is performed in batches (one commit per batch, revertible as a whole), and the
results are recorded in the per-batch retirement sections below and in commit messages.

Generated artifacts:

| Artifact                            | Purpose                                                                                  |
| ----------------------------------- | ---------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Item-by-item adjudication table (2706 rows + header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                              |

## Criteria

### Scan Scope

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Area                                                                     | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module's `tests/` subdirectory) | 2102                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2706** (338 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                         | Meaning of a hit                                         |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| Clause Gate (tightened in 0.6-10) | The first 25 lines of the file contain a **verifiable citation**: section-number form `§<数字>` (including `RFC-<数字>` and `SPEC §<数字>`); files under `src/**/tests/` are considered together with their parent `mod.rs`                                                                                                                                                                                                               | The test maps to an identifiable clause                  |
| History Gate                      | The test body, function doc comment, or **file header** (such as `//! 回归背景：issue #226`) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                            | The test pins an incident that has actually occurred     |
| Proof Gate                        | **The machine cannot decide this** (“If it were replaced with another correct implementation, would it fail?” requires human judgment). Therefore, anything that fails the first two gates is classified as “Pending Review,” not automatically as “Fails All Gates”; the sole exception is a body with **no assertions at all** (including no delegated `assert*`/`check*` helper, no `match` + `panic!` form, and no `#[should_panic]`) | Only “Pending Review” or “No Assertions” can be assigned |

Rationale for tightening: the Clause Gate in 0.6-9 only required the file header to **contain
`§`/`RFC`/`SPEC`**, so 2403 entries were classified as “Keep.” The stratified sample of 40 entries
measured in 0.6-10 found that **half of the citations could not support the assertions**
(mis-citation rate 50%, 95% interval 35%–65%)—this measurement was the **direct reason for
abandoning the practice of keeping a test based on a single citation**. After tightening, only
section-number forms are accepted, and **having a citation still does not mean that the citation
supports the assertions**—all clause citations stop at “Pending Verification.” (This sample audit
was deprecated and deleted in 0.6-19: as the test-retirement pool shrank, 38 entries from the
original sample no longer existed, and `--check` consistently reported “missing 38, extra 38”; the
conclusion is retained, but the audit artifact is no longer maintained.)

Rationale for the file-header radius: this repository's convention is to put “regression background:
issue #N” in the file header rather than repeat it in each test comment. Looking only at a test's
own comments would miss the history of the entire file (measured: 7 entries within the test itself,
291 within the file header).

### Categories That Are Never Classified for Retirement

The following tests are **always classified as “Keep”** and are not included among retirement
candidates:

- Corpus `tests/yaoxiang/**` and project-form corpus `tests/project-corpus/**` (behavior-criterion
  surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and traversing a process boundary);
- Current-behavior anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                                       |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Only two cases: the History Gate is hit, or the Proof Gate applies (spec-shaped area / status-anchor name)                                                                                                    |
| Pending Verification | A clause is cited (with a section number, or only citing an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a human before the test can be kept |
| Retirement Candidate | Fails both gates and the assertion surface contains only weak assertions (or none)—a candidate is not a verdict; the Proof Gate still requires manual review                                                  |
| Pending Review       | Fails both gates but has strong assertions; the machine draws no conclusion                                                                                                                                   |

## Distribution (Regenerated 2026-11-27; 4.9.9 narrowing round, 8.7 batch, 4.9.10-L0, init test rewrite, and D53 batch)

| Gate                                                               | Count    |
| ------------------------------------------------------------------ | -------- |
| Clause (with a verifiable citation)                                | 1037     |
| History                                                            | 565      |
| Proof (spec-shaped criterion surface / status anchors)             | 573      |
| Pending Review (fails both gates; Proof Gate awaits manual review) | 525      |
| Fails All Gates (no assertions in the body)                        | 6        |
| **Total**                                                          | **2706** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1138** | 737 (+401)                        |
| Pending Verification | 1284     | 1054 (+230)                       |
| Pending Review       | 226      | 203 (+23)                         |
| Retirement Candidate | 58       | 61 (−3)                           |
| **Total**            | **2706** | 2055 (+651)                       |

Keep breakdown: 1138 = Proof Gate 573 (spec-shaped criterion surface 518 + status anchors 55) +
History Gate 565. File-level count: among 338 test files, the Clause Gate covers **125**; a
verifiable section number (`§` + number) can be detected within the first 25 lines (`src/**/tests*/`
together with the parent `mod.rs`) in **147**.

Regeneration reconciliation: the previous stored table had 2701 entries, while the current tree has
2706; 5 entries are present in the current tree but absent from the old table (4 equivalent tests in
`src/frontend/core/types/tests/identity.rs` under L3-0 + 1 MonoType Debug golden sample under
L3-1a), 0 entries are present in the old table but absent from the current tree, and 0 same-key rows
have changed content. Per-key records are stored in the TSV.

The latest full regeneration, after the test rewrite in `src/package/commands/tests/init.rs`, left
the row count unchanged (2690) and recalculated only 2 assertion summaries (the `assert_eq` /
`unwrap` counts for `test_init_here_fails_when_project_already_exists` and
`test_init_here_preserves_preexisting_files`); the TSV is consistent with the current tree
(`--check` exits with 0).

### Two Changes in Criteria at the 0.6-19 Tightening Point

1. **Keep breakdown**: 737 = Proof Gate 424 (spec-shaped criterion surface 390 + status
   anchors 34) + History Gate 313. Of the original 2897 entries, the 2121 that had been kept “on the
   strength of a single citation” now fall under Pending Verification (after the sample audit was
   deprecated in 0.6-19, the 9 entries previously classified as “audit confirms genuine support”
   also fell under Pending Verification—there is no longer an “audit-confirmed” tier for clause
   citations).
2. **History Gate 47 → 313**: these are not new history cases but **label realignment**—in 0.6-9,
   entries that hit both the Clause Gate and the History Gate were recorded as Clause cases. After
   tightening, a Clause Gate hit no longer automatically implies Keep, so the History Gate must
   catch these entries, and their count is folded into History. The total number kept is therefore
   unaffected (those tests were also kept in 0.6-9).

File-level count at the time: among 285 test files, **136** had a verifiable section number in the
file header (the old criterion in 0.6-9 identified 221).

## Reconciliation Between the 0.6-19 Snapshot and the 0.6-2 Survey

The 0.6-2 survey numbers are the first comparison baseline. The reasons for each difference are
listed below:

| Item                            | 0.6-2 Survey                  | This Run                                                                                                                                    | Reason for the Difference                                                                                                                                                                                                                                                                                 |
| ------------------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                                                        | 2 entries were added after 0.6-2 (check-criterion regression tests for E3005 and E3022), followed by the removal of 21 assertion-free tests from the first batch, 11 weak-assertion tests from the second batch, 2 from the third batch, 1 from the fourth batch, and 1353 from the fifth batch; net −388 |
| Number of test files            | 296                           | 285                                                                                                                                         | 11 entire files retired in the fifth batch                                                                                                                                                                                                                                                                |
| Clause Gate                     | 227 files (77%)               | 136 files after tightening; 808 by entry                                                                                                    | The criterion was tightened from “the token appears” to “a section number `§<数字>` is present,” nearly halving the file count; the fifth batch retired 902 entries, and after the sample audit was deprecated in 0.6-19, another 9 moved from “Keep” to Pending Verification                             |
| History signals                 | 442 entries / 148 files       | 313 within the gate; three radii: 7 within the test itself / 291 within the file header (38 files) / 1218 across the entire file (79 files) | The 148 files in the 0.6-2 survey represent a broader scope than the current whole-file radius of 79 files because its vocabulary and scope were broader; this run strictly follows the four signals specified by the criterion card                                                                      |
| Characterization strong signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                                                | Characterization is not one of the three gates; mixing it into a retirement conclusion would exceed the criterion's authority                                                                                                                                                                             |
| Weak assertions                 | 473 (14%)                     | See the “Assertion Summary” column in the TSV                                                                                               | The counting unit differs: 0.6-2 counted patterns that assert only `is_ok`/`is_some`; this run uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                                                                             |

## Summary of the 0.6-19 Conclusions (Historical)

1. **The Clause Gate had previously been substantially inflated**: under the old criterion, 2403
   entries were classified as “Clause/Keep,” and the sample audit measured a 50% mis-citation rate
   (95% interval 35%–65%)—“citing a rule” and “having the rule support the assertion” are different
   things, but the old criterion treated them as the same. (This sample audit was deprecated and
   deleted in 0.6-19; the conclusion is retained. See “Deprecation of the Sample Audit” below for
   the reason.)
2. **After tightening, approximately 1202 entries genuinely have clause support** (range 846–1557);
   the rest either need a Pending Verification ticket or should have been decided through the
   History Gate or Proof Gate.
3. **The retirement surface is unrelated to the clause mis-citation surface**: the 136 candidates
   were unrelated to clauses (they had no assertions or only weak assertions); 21 were retired in
   the first batch and 11 in the second batch; the generator correction changed the count by +1, the
   third batch by −2, the 0.6-15 criterion correction by −3, the redundancy scan by −1, the fourth
   batch by −1, and the fifth batch by −1353 (using the mechanical rule “for the same symbol and
   result category, keep only the strongest entry”), reducing the candidates to 61. Fifth-batch
   retirements by gate: Clause 902 · Pending Review 428 · Proof 23 · History 0 · Fails All Gates
   0—**not a single History Gate or zero-assertion entry was removed**.
4. **The History Gate is the largest legitimate source of retention** (313 entries): it demonstrates
   that “pinning an actual incident” is more reliable than “copying a line from an RFC.”

## First Retirement Batch (Executed 2026-11-06)

**First batch: of the 26 assertion-free tests in `src/frontend/core/types/eval/tests/`, 21 were
retired and 5 were kept** (a single directory cluster; criterion = all **three gates fail** under
Part Seven of coding-rules: no Clause Gate hit, no History Gate hit, and no assertions in the body).
The item-by-item adjudication and “same-cell coverage” are provided by the 0.6-11a adjudication
table (21 deleted / 5 kept); this page does not repeat the reasons and only records the execution
result.

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
- **5 kept** (the adjudication table could not identify replacement coverage, so they were kept as
  required): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Second Retirement Batch (Executed 2026-11-06)

**Second batch: of the 18 weak-assertion tests in `src/frontend/core/parser/pratt/tests/nud.rs`, 11
were retired and 7 were kept** (a single directory cluster = the Pratt prefix-parsing module;
criterion = all **three gates fail** under Part Seven of coding-rules: no Clause Gate hit, no
History Gate hit, and the body contains only weak assertions such as `is_some()` /
`is_none()`—neither binding power nor the parse result is pinned).

The difference from the first batch must be stated clearly: these tests **do contain assertions**
and are not empty tests. The reason for retirement is “all three gates fail + zero criterion
increment,” not “no assertions”—every entry selected for deletion identifies a **unit-level
replacement test in the same module** by name.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Every replacement
  follows **the same** `parse_expression → parse_prefix → prefix_info` path and makes a stronger
  assertion (pinning the AST shape rather than merely checking `is_some`): `test_parse_int_literal`
  / `test_parse_float_literal` / `test_parse_string_literal` / `test_parse_bool_true` /
  `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` / `test_parse_return_expr`
  in the same file, plus `test_grouped_expr` / `test_tuple` / `test_list_literal` /
  `test_list_empty` / `test_ref_expr` in `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—the reasons for each are recorded in the commit message (Task 0.6, second
  test-retirement batch). The reasons fall into two categories: **the Proof Gate applies** (the
  first three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are the only unit-level
  walkthrough points for their respective facts, and the repository contains no second unit test
  that observes them), and **the History Gate applies** (`break_removed` / `continue_removed` name
  #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Generator Correction (0.6-13)

**`body_span` truncates function bodies at braces inside literals — fixed.**

The old implementation located function bodies by balancing braces and did not distinguish real code
from braces inside string or character literals or comments. Forms such as `tokenize("{")` are
common in Rust source, so the balancing became skewed and the function body was truncated: the body
of `test_prefix_info_lbrace` was once truncated to the end of the file, and the “Assertion Summary”
captured the remainder of the file (`matchesx11 assertx24 …`), causing it to be binned as “Pending
Review” rather than “Retirement Candidate”—**the wrong bin**.

Fix: add `mask_noncode`, which first replaces the contents of string literals / character literals /
line comments / block comments (which may be nested) / raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (newlines are preserved exactly, so line and row counts do not change), and
then balances braces. **The mask is used only to locate function-body boundaries**; assertion
counting and the History Gate still read the original source, and the criterion itself was not
relaxed. `--selftest` added 4 cases (paired and unpaired braces in ordinary strings, braces in raw
strings and block comments, and braces in character literals), each followed by a sentinel test—the
old implementation found the wrong boundary in all 4 cases, while the new implementation finds all
of them correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare **the function-body boundaries before and after the fix** file by file and
test by test (the old implementation balanced literals, whereas the new implementation balances
after masking); an entry is affected when the boundaries differ. Of 3413 tests across the
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

### Differences in Bins Before and After the Fix: 2

Of the other 9, **8 affected only the “Assertion Summary” column** (their bins did not change), and
**1 did not even change its summary** (`test_fstring_close_brace_escape_preserved_raw`: the old
implementation omitted the closing `}` line, which contained no assertion keywords). The 8 summary
changes are itemized in the commit message.

| Test Name                   | File                                          | Old Bin                         | New Bin                                   | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| --------------------------- | --------------------------------------------- | ------------------------------- | ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending Review / Pending Review | Pending Review / **Retirement Candidate** | The actual function body contains only one weak assertion, `assert!(state.prefix_info().is_some())`; neither the file header nor the body contains a § section number or issue/regression/defect/incident ⇒ both the Clause Gate and the History Gate fail; the body contains only a weak assertion such as `is_some()` ⇒ Retirement Candidate (a candidate is not a verdict; the Proof Gate still requires manual review)                   |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | History / Keep                  | Clause / **Pending Verification**         | The old implementation truncated the body to the end of the file and swallowed text containing “regression / defect / incident,” falsely triggering the History Gate; the actual body contains no such text ⇒ the History Gate no longer applies; the file header contains verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the Clause Gate is hit, but the sample audit did not confirm it ⇒ Pending Verification |

The distribution changes accordingly: Clause 1713 → 1714 · History 314 → 313 · Keep 776 → 775 ·
Pending Verification 2121 → 2122 · Pending Review 412 → 411 · Retirement Candidate 104 → 105. **The
repository-wide `#[test]` count does not change (3413)**—this round changes only the implementation
of the criterion, not any tests.

## Third Retirement Batch (Executed 2026-11-07)

**Third batch: of the 5 retirement candidates in `src/middle/passes/codegen/tests/`, 2 were retired
and 3 were kept.** (A single directory cluster = the `middle::passes::codegen` module under test;
criterion = all **three gates fail** under Part Seven of coding-rules.)

This batch follows the **retention criterion** established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus and integration tests
operate at the process-boundary layer and cannot replace a unit-level observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—its body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (after removing
    comments, only the function name differs): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that the `execute_module` entry
    path returns a non-Void value, but the body never observes the return value from
    `execute_module` (`run_fixture` discards it with `.expect(...)`)— the claimed intent was not
    implemented, and the body is a pure duplicate ⇒ zero criterion increment. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file covers it strictly:
    a positive lookup (`lookup("a").is_some()`), a negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)`, identical to the inline `Symbol` in this entry).
    Looking up a name that was never inserted and looking up a name after it was popped follow the
    same path in `SymbolScopeManager::lookup` (searching each layer with `scope.get(name)` and
    returning `None` when not found) ⇒ this entry adds zero coverage. Replacement coverage =
    `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip`—the only observation point for the path that “does
    not go through serialization to connect directly to execution” (the serialized variant is a
    different path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the only observation point for a
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **only** unit-level observation point for the
    `id > 255` rejection branch of `OperandResolver::to_reg` (the bytecode layer has independent
    overflow checks that do not cover this function; no second test in the repository makes `to_reg`
    overflow). `is_err()` is a discriminating assertion (removing the guard would make the test
    fail), not merely proof that no panic occurred.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

### A Census Blind Spot Found in This Batch (**Fixed in 0.6-15**; see “Generator Criterion Correction” below)

The census's recognition of “delegated assertions,” `\b(assert|check)[A-Za-z_]*\s*\(`, **could not
match helper names containing digits**: because `[A-Za-z_]*` does not include digits, a call such as
`assert_slot_is_42(&interp)` was not recognized as an assertion— so 2 of the 5 candidates in this
batch were incorrectly binned as “no assertions in the body.”

After all 15 “Fails All Gates” entries were reviewed one by one (broadening helper names to include
digits and additionally recognizing the `match` + `panic!` form), **4 actually contained
assertions**—3 used the numeric helper above, and 1 was `test_weak_from_arc` in
`src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule 6.1
requires `assert!(matches!(...))`, but it does assert). The remaining 11 genuinely had no
assertions.

**Taking these 4 directly based on the machine's “Fails All Gates” result would have wrongly deleted
tests with criteria**—the manual review in this batch prevented that; the generator-side fix is
described in “Generator Criterion Correction (0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots were fixed: helper names containing digits, and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411) and
changes only how the generator detects whether a body contains assertions.

### What Was Fixed

1. **Digits were allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing them were
   treated as having “no assertions in the body.”
2. **Added recognition of the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the
   assertion is written in one of the arms, but the lexer cannot see an `assert!` macro
   (test-specification rule 6.1 requires `assert!(matches!(...))`, but this form exists in the
   existing code). Detection method: scan the expression from the `match` keyword to the first `{`,
   determine the block boundary by **brace balancing**, and count it only if `panic!(` appears
   inside the block. **The decision is made on masked lines**— braces in strings and comments do not
   participate in balancing, and `panic!` in comments is not recognized as an assertion.

### Differences in Bins: 3 (the gate and disposition changed)

| Test Name                                           | File                                           | Old Bin                                | New Bin                         | Basis                                                                                                                                                                                                                                                         |
| --------------------------------------------------- | ---------------------------------------------- | -------------------------------------- | ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Fails All Gates / Retirement Candidate | Pending Review / Pending Review | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not decide its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Fails All Gates / Retirement Candidate | Pending Review / Pending Review | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern could not match; it is now recognized as an assertion                                                                                      |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Fails All Gates / Retirement Candidate | Pending Review / Pending Review | Same as above (the serialization round-trip entry)                                                                                                                                                                                                            |

Another **77 entries changed only the “Assertion Summary” column** (their gates and dispositions did
not change): most gained `match_panicxN` in the summary (the body contains the `match` + `panic!`
form), and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3,
`src/package/tests/cache.rs`: 1—helper names containing digits).

**The relaxation changes only the “Fails All Gates” tier**: this tier is the machine's **confident
verdict** (there is not a single assertion in the body), and the criterion is a hard requirement;
“Retirement Candidate” is only a pool awaiting manual review, so the size of that pool is not a
verdict. All other bins, including the weak-assertion pool, continue to use the original criterion.
Result: gate Fails All Gates 14 → 11 · Pending Review 923 → 926; disposition Retirement Candidate
103 → 100 · Pending Review 411 → 414.

### Review of the Remaining 11 “Fails All Gates” Entries

After all 11 repository-wide “Fails All Gates” entries were reviewed against the original source,
**all 11 genuinely contain no assertions** (including no `assert*` / `check*` helper, no `match` +
`panic!` form, and no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                     | Review Conclusion                                           |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 calls to `.expect(...)`, merely guaranteeing that it does not crash                           | Genuinely no assertion (only proves that no panic occurred) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                  | Genuinely no assertion                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them              | Genuinely no assertion                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Genuinely no assertion                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Genuinely no assertion                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                              | Genuinely no assertion                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                        | Genuinely no assertion (only proves that no panic occurred) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Genuinely no assertion (only proves that no panic occurred) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Genuinely no assertion (only proves that no panic occurred) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                            | Genuinely no assertion                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertion                                      |

Five of these (`test_normalizer_context` and the 4 entries in reducer.rs) were kept by the 0.6-11a
adjudication table because **no replacement coverage could be identified**; this review reaches the
same conclusion.

### Reverse Sample: Did the Relaxation Go Too Far?

Five entries were read from the 88 newly recognized assertions using a fixed seed (20261107),
confirming that they genuinely assert:

| Sampled Entry                                               | File                                                 | Assertion in the Source                                                                                                                              |
| ----------------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching `match` arm, with `panic!("Expected AssocType")` in the non-matching arm                                      |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm, with `panic!("expected InvalidPackage, got …")` in the non-matching arm                  |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | An `assert_eq!` comparing constraint types in the matching arm, with `panic!("不是 Dynamic 变体…")` in the non-matching arm                          |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm, with `panic!("Expected TokensDelta variant")` in the non-matching arm                        |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm verify the error code and message, with `panic!("不是 Disproved…")` in the non-matching arm |

All 5 assert concrete values or concrete variants (replacing the implementation with another correct
path would make them fail)—**the relaxation did not go too far**. The reverse controls were also
added to `--selftest`: a `match` without `panic!`, and a `panic!` that appears only in a comment;
neither may be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

When the generator was rolled back to the old implementation (`HELPER_RE` without digits + `match` +
`panic!` recognition disabled), the newly added `--selftest` cases **all failed**, with exit code 1;
the new implementation passes them all.

### Rollback

The generator and the census table are in the same commit, so `git revert <sha>` restores them as a
whole.

## Fourth Retirement Batch (Executed 2026-11-07)

**Fourth batch: of the 10 retirement candidates in the directory cluster `src/backends/interpreter/`
(the interpreter backend's test directory), 1 was retired and 9 were kept** (a single directory
cluster, with three files read together: `tests/bytecode_load.rs` 4 entries · `tests/weak.rs` 2
entries · `executor/tests/debug.rs` 4 entries). Criterion = the **three gates** under Part Seven of
coding-rules; retention criterion = “a unit-level observation point can only be replaced by another
unit test; corpus/integration tests operate at the process-boundary layer”; **deletion must identify
a concrete replacement test by name; if it cannot be identified or understood, the entry is kept**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, followed by one step
  and then `capture_stack`), and its assertion is **strictly stronger**: it asserts that the stack
  is nonempty and that `stack[0].function_name == "main"` (pinning the top-of-stack position),
  whereas the listed entry only asserts `stack.iter().any(|f| f.function_name == "main")` (any
  position is sufficient) —the former being true implies the latter ⇒ zero criterion increment.
- **9 kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each source entry, each
  is the **only unit-level observation point** for some property (the load magic/version branches
  are distinct, the YXBC routing layer, both sides of Weak's Some/None, stepping with no frames,
  step_out popping the stack, and running to completion with an empty stack). No unit test that
  could replace them could be identified ⇒ they are kept as required. The basis for each entry is
  recorded in the commit message (the adjudication table is not included in this page).
- **Numbers**: repository-wide `#[test]` 3409 → 3408; 1 deleted = `git diff` counts 1 `-#[test]`
  line and 0 `+#[test]` lines; the dominating test `test_capture_stack_during_execution` passes in
  isolation. Census: Retirement Candidate 99 → 98 · Pending Review 925 → 924 · Clause 1713 · History
  313 · Proof 447 · Fails All Gates 11 · Pending Verification 2121 · Keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Fifth Retirement Batch (Executed 2026-11-07)

**Fifth batch: 1353 unit tests were retired in bulk using the mechanical rule “for the same symbol
and result category, keep only the strongest entry”** (only unit tests under `src/**/tests/**` were
modified; not a single test in the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (executed mechanically, without item-by-item argument)**:
  - Symbol under test = a repository symbol invoked in the test body (including one level of
    propagation through a helper function in the same file); when multiple symbols are reached,
    classify by the **primary symbol** (the first repository-internal call). Symbols are qualified
    by their repository path and are not merged across modules by bare name.
  - Result category = a concrete value / a specific error code (E code) / a specific variant match /
    only ok / only err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when entries in the same group are equally
    strong, keep the first one in the file.
  - When a test asserts multiple categories (and the primary category cannot be determined), **keep
    it; do not guess**.
- **Three exceptions (not a single entry was removed)**: ① the name or comment names `issue #` /
  regression / defect / incident (History Gate); ② a symbol has only one test (removing it would
  leave the symbol without a unit-level observation point); ③ `#[should_panic]` is the symbol's only
  should_panic case. History Gate 313 and Fails All Gates 11 had **zero retirements**.
- **Numbers**: repository-wide `#[test]` 3408 → 2055; 1353 deleted = `git diff` counts 1353
  `-#[test]` lines and 0 `+#[test]` lines. Census by gate (current values after the sample audit was
  deprecated in 0.6-19): Clause 808 · History 313 · Proof 424 · Pending Review 499 · Fails All
  Gates 11. Census by disposition (current values): Keep 737 · Pending Verification 1054 · Pending
  Review 203 · Retirement Candidate 61. Fifth-batch retirements by gate: Clause 902 · Pending Review
  428 · Proof 23 (because 0.6-19 reclassified 9 entries previously considered “audit-confirmed to
  have genuine support” as Pending Verification, the current values by gate and disposition differ
  from those at the fifth batch's close).
- **Cleanup accompanying the deletions**: helper functions used only by deleted tests, imports that
  became invalid, and struct fields that were no longer used were removed as well
  (`cargo clippy --all --all-features --all-targets -- -D warnings` reports 0 warnings).
- **Two out-of-scope deletions that require a record**: 29 entries originally classified as “Keep”
  were nevertheless deleted by this batch's rule—23 status anchors (names containing
  unchanged/anchor/guard/preserve/baseline, many of which are false positives on **domain terms**
  such as “guard” and “preserves,” for example `test_while_guard_scope_balanced`) and 6 entries with
  clause support confirmed by the audit. This batch followed the closed list of three exceptions on
  the criterion card and did not additionally protect these two categories; to restore them,
  backfill the keep/delete correspondence table in this page.
- **Invalidation and deprecation of the sample audit**: the 40-entry sample was drawn from the test
  pool using a fixed seed; after the fifth batch's retirements, 38 entries in the original sample no
  longer existed, so the audit's `--check` consistently reported “missing 38, extra 38.” This sample
  audit was deprecated and deleted in 0.6-19 (the script, data file, and the generator's dependency
  on it were removed together); see “Deprecation of the Sample Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Sixth Retirement Batch (Executed 2026-11-26)

**Sixth batch: 28 tests were retired along with the mechanisms they covered in the 4.9.9
narrowing-round rollback, while 4 prohibition criteria had never been stored.** This batch does not
use the three gates to decide “whether a test should be kept”; instead, the mechanism under test was
**rolled back as a whole**: the criteria written for the downstream ambiguity mechanisms in the
first six rounds of 4.9.9 (candidate-provider priority / candidate-signature comparison / owner
compatibility inference / named-argument fallback on the IR side / import-name resolution point
E5006 prohibition) were retired along with the mechanisms themselves—the mechanism no longer exists,
so the tests have no subject under test. These deletions are recorded as a “batch” under
coding-rules 7.3 and do not enter the retirement-candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 entries (28 deleted: 27
    downstream-mechanism criteria from the first six rounds
    - 1 alias-import named-argument criterion). The 2 retained entries pin the minimal fix for
      bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: all 4 E5006 prohibition criteria were retired—the
    file existed only in an intermediate 4.9.9 commit, and neither the parent commit nor that
    version's stored table included it; after the prohibition was withdrawn, there was no subject
    under test, so this batch does not reduce the stored table.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the candidate-injection surface for alias imports was rolled back, so the fixture's
    behavior no longer holds); the corresponding corpus-difference baseline entry was deleted in the
    same batch. Baseline accounting is stated separately: **relative to the direct parent commit
    `ac810e97`, 405 → 404 lines** (the intermediate state had included the fixture); **relative to
    the pre-4.9.9 baseline `08ccacc5`, 403 → 404 lines** (+1 base fixture
    `named-args-cross-unit-method`; the retired `alias-overload` never entered the final history).
- **Which gate applies**: the 2 retained entries = **History Gate** (they pin a real defect
  registered in 4.9.9: cross-unit named arguments could not obtain the declaration-time parameter
  name, leaving `E1014 available` empty); most of the newly added mechanism criteria among the 28
  retired entries were originally History Gate cases, but the reason for their deletion **is not**
  the three gates (they were not judged worthless); the mechanism under test was rolled back.
- **Replacement coverage**: cross-unit named arguments using bare-name imports = the 2 retained
  driver-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity side no
  longer has rejection logic (the requirement was rolled back), so there is no replacement
  criterion; the 4.9.8 dispatch-attribution criteria (10 driver entries + 5 integration entries) are
  retained unchanged.
- **Recovery**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the empty-name-table guard for
  `method_param_info` is retained; this form is unreachable through the public syntax and is a
  defensive guard that the unit test protects against accidental deletion).
- **Numbers** (after this batch was regenerated): repository-wide `#[test]` 2680 entries / 334
  files; the previous stored table had 2688 entries → the current tree has 2680 (−8); 1 entry is
  present in the current tree but absent from the old table, 9 entries are present in the old table
  but absent from the current tree, and 0 same-key rows have changed content.
- **Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit), so `git revert <sha>` rolls it back as a whole.

## Deprecation of the Sample Audit (0.6-19)

**Criterion**: the sample audit (0.6-10) measured the “true support rate” of clause citations, but
the retention rule after 0.6-18 no longer considers citations (it changed to “for the same symbol
and result category, keep only the strongest entry,” reducing the test pool from 3408 to 2055). Of
the original 40 samples, 38 no longer existed, and the audit's `--check` consistently reported
“missing 38, extra 38”—keeping it would only produce a continuing alarm. Therefore, it was
**deprecated and deleted**.

**What was deleted**: the sample-audit script under `scripts/dev/` (the one that reproduced the 40
samples using a fixed seed) and its JSON data file (the item-by-item decisions and bases for all 40
samples).

**Synchronizing the generator**: the generator previously read that JSON and classified “audit
confirms genuine support” as “Clause Gate + Keep”; that branch was removed—**there is no longer an
“audit-confirmed” tier for clause citations**, and all entries fall under “Clause / Pending
Verification.”

**Distribution change** (all 2055 repository-wide `#[test]` entries were **left unchanged**, and the
TSV was regenerated): Keep 746 → 737 (−9: the 9 entries previously confirmed by the audit to have
genuine support fell back to Pending Verification); Pending Verification 1045 → 1054; by gate:
Clause 811 → 808, Pending Review 496 → 499 (all others unchanged: History 313 · Proof 424 · Fails
All Gates 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit), so `git revert <sha>` rolls it back as a whole.

## Rerunning and Self-Checks

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (byte-for-byte identical across two runs)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical classification only, and states its boundaries honestly:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test's assertion is hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as “no assertions”—the first batch of 26 entries (21 retired / 5 kept) and the
  second batch of 18 entries (11 retired / 7 kept) had their function bodies manually reviewed one
  by one.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator obtains a function body, it
  balances braces; braces in string or character literals and comments skew the balance, truncating
  the body—the body of `test_prefix_info_lbrace` (which contains `tokenize("{")`) was once truncated
  to the end of the file, and the “Assertion Summary” captured the remainder of the file, causing it
  to be binned as “Pending Review” rather than “Retirement Candidate.” It now masks non-code regions
  (strings / character literals / line comments / block comments / raw strings) before balancing
  them; see “Generator Correction” below.
- **Delegated-assertion recognition (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not include digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions—tests containing such calls were
  misclassified as having “no assertions in the body.” At the time, 4 of the repository-wide 15
  “Fails All Gates” entries actually contained assertions (3 numeric helpers + 1 `match` + `panic!`
  form). The name portion has now been broadened to `[A-Za-z0-9_]*`, and recognition of the
  `match` + `panic!` form has been added; see “Generator Criterion Correction” below.
- **The Proof Gate is not decided automatically**: any test that “fails both gates” is classified
  only as “Pending Review” or “Retirement Candidate”; the generator never declares that it “should
  be deleted.”
- **No cross-file deduplication**: exactly duplicated tests are not identified in this table (0.6-2
  measured them at only 1%, which does not constitute a retirement surface).
- **Clause citations always require verification**: a verifiable section number in the file header
  only means that the test maps to an identifiable clause; it does not mean that the citation
  supports the assertion. The machine does not decide whether the citation supports the assertion,
  so even a Clause Gate hit stops at “Pending Verification” and must be verified by a human before
  the test can be kept.
