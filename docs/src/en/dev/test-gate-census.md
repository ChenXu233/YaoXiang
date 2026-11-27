---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement candidate list and retirement records for each batch'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every repository-wide
`#[test]` according to the **three gates (clauses / incident history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), producing a rerunnable per-test adjudication table.
A 40-entry stratified sample audited for the clause gate once measured the proportion of tests that
“cited a clause but whose assertions were not supported by it” and thereby **tightened the
criteria**; that audit was abolished and deleted in 0.6-19. **This page and the generator itself are
read-only and do not modify tests**; test retirements are executed in batches (one commit per batch,
with the entire batch revertible via `git revert`), and the results are recorded in the batch
retirement sections and commit messages below.

Generated artifacts:

| Artifact                            | Purpose                                                                                     |
| ----------------------------------- | ------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Per-test adjudication table (2690 rows plus the header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                                 |

## Criteria

### Scan Surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module's `tests/` subdirectory) | 2086                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2690** (336 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Meaning of a hit                                               |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | A **verifiable reference** appears in the first 25 lines of the file: section-number form `§<数字>` (including `RFC-<数字> §<数字>` and `SPEC §<数字>`); files under `src/**/tests/` are examined together with their parent `mod.rs`                                                                                                                                                                                                                                  | The test maps to a specific, identifiable clause               |
| Incident-history gate             | The test function body, function doc comment, or **file header** (such as `//! 回归背景：issue #226`) identifies an actual failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                        | The test pins an incident that actually occurred               |
| Proof gate                        | **Cannot be determined mechanically** (“If replaced with another correct alternative implementation, would it fail?” requires human judgment). Therefore, anything that fails both preceding gates is classified as “pending adjudication,” not automatically as “neither passed”; the sole exception is a test with **not one assertion in its body** (including no delegation to an `assert*`/`check*` helper, no `match` + `panic!` form, and no `#[should_panic]`) | Only “pending adjudication” or “no assertions” can be assigned |

Rationale for the tightening: the 0.6-9 clause gate only required the file header to **contain the
characters `§`/`RFC`/`SPEC`**, so 2403 tests were classified as “keep.” The 40-entry stratified
sample measured in 0.6-10 found that **half of the citations could not support their assertions**
(50% mis-citation rate, 95% interval 35%–65%)—this measurement was the **direct reason for
abandoning the rule “keep based on a citation alone.”** After the tightening, only section-number
forms are recognized, and **having a citation still does not mean the citation supports the
assertion**—clause references always remain “pending verification.” (The sample audit was abolished
and deleted in 0.6-19: as the test pool shrank through retirements, 38 entries from the original
sample no longer existed, and `--check` consistently reported “38 missing, 38 extra”; the conclusion
is retained, but the audit artifact is no longer maintained.)

Rationale for the file-header radius: this repository conventionally writes “regression context:
issue #N” in the file header and does not repeat it in the comment for each test; looking only at
the test's own comment would miss the incident history for the entire file (measured: 7 entries
within the test's own radius versus 291 within the file-header radius).

### Categories That Are Never Classified for Retirement

The following tests are **always classified as “keep”** and do not enter the retirement candidate
pool:

- Corpus tests under `tests/yaoxiang/**` and project-form tests under `tests/project-corpus/**` (the
  behavioral criterion surface);
- Tests under `tests/integration/**`, in `tests/formatter_integration.rs`, and in
  `tests/yx_runner*.rs` (spec-shaped and crossing a process boundary);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                               |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Only two sources: a hit on the incident-history gate, or the proof gate (spec-shaped criterion surface / current-state anchor names)                                                                  |
| Pending verification | Cites a clause (with a section number, or citing an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a human before the test can be kept |
| Retirement candidate | Fails both gates and has only weak assertions (or no assertions) in its assertion surface—a candidate is not a verdict; the proof gate still requires human review                                    |
| Pending adjudication | Fails both gates but has strong assertions; the machine reaches no conclusion                                                                                                                         |

## Distribution (regenerated on 2026-11-27; 4.9.9 narrowing round, 8.7 batch, 4.9.10-L0, and init test rewrite)

| Gate                                                              | Count    |
| ----------------------------------------------------------------- | -------- |
| Clause (with a verifiable reference)                              | 1033     |
| Incident history                                                  | 558      |
| Proof (spec-shaped criterion surface / current-state anchors)     | 571      |
| Pending adjudication (both gates fail; proof awaits human review) | 522      |
| Neither passed (no assertions in the body)                        | 6        |
| **Total**                                                         | **2690** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1129** | 737 (+392)                        |
| Pending verification | 1280     | 1054 (+226)                       |
| Pending adjudication | 223      | 203 (+20)                         |
| Retirement candidate | 58       | 61 (−3)                           |
| **Total**            | **2690** | 2055 (+635)                       |

Composition of “keep”: 1129 = proof gate 571 (518 spec-shaped criterion-surface tests + 53
current-state anchors) + incident-history gate 558. File-level scope: among 336 test files, the
**clause gate** covers **124**; using the first 25 lines of the file (`src/**/tests*/` together with
the parent `mod.rs`), a verifiable section number (`§` plus a number) can be detected in **146**.

Regeneration reconciliation (at 4.9.10-L0): the previous checked-in table had 2682 entries, while
the tree at that time had 2688; the current tree contains 13 entries absent from the old table (6
DefPath projection criteria from 4.9.10-L0 and 7 tests moved out of inline tests in `symbol.rs` and
renamed), while the old table contains 7 entries absent from the current tree (from the same move;
see `module/tests/symbol.rs`); 0 rows with the same key were recomputed. L0 also included one test
that was retired before being checked in (`test_def_path_disambiguator_reserved_and_not_projected`:
under coding-rules 7.1/7.3, it characterizes this round's frozen projection choice and must fail
once L1 introduces a disambiguating suffix), so this table does not contain that row. Per-key
records are in the TSV; the fate of stale keys is recorded in that batch's commit message.

Latest full regeneration (after the tests in `src/package/commands/tests/init.rs` were rewritten):
the row count is unchanged (2690), and only 2 assertion summaries were recalculated (the `assert_eq`
/ `unwrap` counts for `test_init_here_fails_when_project_already_exists` and
`test_init_here_preserves_preexisting_files`); the TSV is consistent with the current tree
(`--check` exits with 0).

### Two Changes in Criteria at the 0.6-19 Tightening Point

1. **Composition of “keep”**: 737 = proof gate 424 (390 spec-shaped criterion-surface tests + 34
   current-state anchors) + incident-history gate 313. Of the original 2897, the 2121 tests that had
   been “kept based on a single citation” now fall into pending verification (after the sample audit
   was abolished, the 9 tests previously classified as “audit confirmed actual support” also moved
   to pending verification—clause references no longer have an “audit confirmed” tier).
2. **Incident-history gate 47 → 313**: this does not represent newly discovered incident history; it
   is a **label correction**. In 0.6-9, tests that hit both the clause and incident-history gates
   were recorded under the clause gate. After the tightening, a clause hit no longer automatically
   meant “keep,” so these tests had to be caught by the incident-history gate, and their counts were
   folded into that gate. The total number kept was therefore unaffected (these tests were also
   classified as “keep” in 0.6-9).

File-level scope at that time: among 285 test files, **136** had a verifiable section number in the
file header (the old 0.6-9 criterion found 221).

## Reconciliation Between the 0.6-19 Snapshot and the 0.6-2 Survey

The 0.6-2 survey figures form the baseline for the first comparison. The reasons for each difference
are listed below:

| Item                            | 0.6-2 survey                  | Current                                                                                                         | Reason for the difference                                                                                                                                                                                                                                                                                  |
| ------------------------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                            | 2 tests were added after 0.6-2 (check-criterion regression tests for E3005 and E3022), and 21 no-assertion tests were deleted under the first batch list, 11 weak-assertion tests under the second batch list, 2 under the third batch, 1 under the fourth batch, and 1353 under the fifth batch; net −388 |
| Number of test files            | 296                           | 285                                                                                                             | 11 entire files were retired in the fifth batch                                                                                                                                                                                                                                                            |
| Clause gate                     | 227 files (77%)               | 136 files after tightening; 808 by test                                                                         | The criterion was tightened from “the word appears” to “there is a section number `§<number>`,” so the file count was nearly halved; the fifth batch retired 902 tests, and another 9 moved from “keep” to pending verification after the sample audit was abolished                                       |
| Incident-history signals        | 442 tests / 148 files         | 313 tests within the gate; three radii: test's own 7 / file header 291 (38 files) / entire file 1218 (79 files) | The 148 files in the 0.6-2 survey were broader than the current entire-file radius of 79 files, and its vocabulary/range was broader; the current implementation strictly uses the four card-level signals                                                                                                 |
| Strong characterization signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                    | Characterization is not one of the three gates; incorporating it into a retirement conclusion would exceed the gate's authority                                                                                                                                                                            |
| Weak assertions                 | 473 (14%)                     | See the TSV's “assertion summary” column                                                                        | The counting units differ: 0.6-2 counted the pattern “only asserts is_ok/is_some,” whereas the current census uses lexical counts and distinguishes strong, weak, and delegated assertions                                                                                                                 |

## 0.6-19 Conclusion Summary (Historical)

1. **The clause gate had previously been significantly overstated**: under the old criterion, 2403
   tests were classified as “clause/keep”; the sample measured a 50% mis-citation rate (95% interval
   35%–65%)—a “citation” and “a citation that supports the assertion” are different things, and the
   old criterion treated the former as the latter. (The sample audit was abolished and deleted in
   0.6-19; the conclusion is retained, and the reason for deletion appears in the “Sample Audit
   Abolition” section below.)
2. **After the tightening, approximately 1202 tests genuinely had clause support** (range 846–1557);
   the rest either require pending verification or should already have been adjudicated through the
   incident-history gate/proof gate.
3. **The retirement surface was unrelated to clause mis-citations**: the 136 candidates were
   unrelated to clauses (no assertions/weak assertions); the first batch retired 21 tests, the
   second retired 11, the generator correction adjusted the count by +1, the third batch by −2, the
   0.6-15 criterion correction by −3, the redundancy scan by −1, the fourth batch by −1, and the
   fifth batch by −1353 (using the mechanical rule “for the same symbol and result category, keep
   only the strongest test”), leaving 61 candidates. The fifth batch's retirements by gate were:
   clause 902 · pending adjudication 428 · proof 23 · incident history 0 · neither passed 0—**not
   one test on the incident-history gate or zero-assertion line was removed**.
4. **The incident-history gate was the largest legitimate source of “keep”** (313 tests): it
   demonstrates that “pinning an actual incident” is more reliable than “copying a sentence from an
   RFC.”

## First Retirement Batch (Executed, 2026-11-06)

**First batch: retire 21 and keep 5 of the 26 “no assertion” tests under
`src/frontend/core/types/eval/tests/`** (a single directory cluster; criterion = **all three gates
fail** under Part Seven of coding-rules: no clause gate, no incident-history gate, and not one
assertion in the body). The 0.6-11a adjudication table provides the per-test decisions and
“same-cell coverage” (21 deleted / 5 kept); this page does not repeat the rationale and records only
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
- **5 kept** (the adjudication table could not identify replacement coverage, so they were kept
  under the hard requirement): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the entire batch.

## Second Retirement Batch (Executed, 2026-11-06)

**Second batch: retire 11 and keep 7 of the 18 “weak assertion” tests in
`src/frontend/core/parser/pratt/tests/nud.rs`** (a single directory cluster = the Pratt
prefix-parsing module; criterion = **all three gates fail** under Part Seven of coding-rules: no
clause gate, no incident-history gate, and only weak assertions such as `is_some()` / `is_none()` in
the body—neither binding strength nor the parse result is pinned down).

The difference from the first batch must be stated explicitly: these tests **do contain
assertions**; they are not empty tests. The reason for retirement is “all three gates fail + zero
criterion increment,” not “there are no assertions”—each deleted test identifies a **unit-level
replacement coverage test in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The replacement
  coverage for every entry follows the **same** `parse_expression → parse_prefix → prefix_info` path
  and makes stronger assertions (pinning the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, and `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—the per-test rationale appears in the commit message (task 0.6, second test
  retirement batch). The reasons for keeping them fall into two categories: **the proof gate holds**
  (the first three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are the sole
  **unit-level** observation points for their corresponding facts, and the repository contains no
  second unit test that observes them), and **the incident-history gate holds** (`break_removed` /
  `continue_removed` identify #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the entire batch.

## Generator Correction (0.6-13)

**`body_span` truncated the function body when it encountered braces inside literals — fixed.**

The old implementation located the function body by balancing braces and did not distinguish real
code from braces in string/character literals or comments. Forms such as `tokenize("{")` are common
in Rust source, so the balance was thrown off and the function body was truncated: the body of
`test_prefix_info_lbrace` was formerly truncated to the end of the file, causing the “assertion
summary” to take in the rest of the file (`matchesx11 assertx24 …`) and the test to be bucketed as
“pending adjudication” rather than “retirement candidate”—**misbucketed**.

Fix: add `mask_noncode` to replace the contents of string literals / character literals / line
comments / block comments (nestable) / raw strings (`r"…"` / `r#"…"#` / `br"…"` / `cr#"…"#`) with
spaces (preserving line breaks and therefore line numbers and line counts), then balance the braces.
**The mask is used only to locate function-body boundaries**; assertion statistics and the
incident-history gate still read the original text, and the criteria themselves were not relaxed.
`--selftest` now includes 4 cases (paired and unpaired braces in ordinary strings, braces in raw
strings and block comments, and braces in character literals), each followed by a sentinel test—the
old implementation found the wrong boundary in all 4 cases, while the new implementation finds all
of them correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** test by test and
file by file (the old implementation balanced raw literals, while the new implementation balances
after masking); entries with different boundaries are affected. Of 3413 tests repository-wide, the
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

### Bucket Differences Before and After the Fix: 2

For the other 9, **8 affected only the “assertion summary” column** (the bucket was unchanged),
while **1 did not change even the summary** (`test_fstring_close_brace_escape_preserved_raw`: the
old implementation omitted the closing `}` line, which contained no assertion keyword). The 8
changed summaries are listed individually in the commit message.

| Test name                   | File                                          | Old bucket                                  | New bucket                                      | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --------------------------- | --------------------------------------------- | ------------------------------------------- | ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending adjudication / pending adjudication | Pending adjudication / **retirement candidate** | The actual function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; the file header and function body contain no § section number and no issue/regression/defect/incident ⇒ the clause and incident-history gates both fail, and the body contains only a weak assertion such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires human review)                                    |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident history / keep                     | Clause / **pending verification**               | The old implementation truncated the body to the end of the file and swallowed text containing “regression / defect / incident,” causing a false incident-history gate hit; the actual function body contains no such wording ⇒ the incident-history gate no longer holds; the file header has verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate is hit but was not confirmed by the sample audit ⇒ pending verification |

The distribution consequently changed: clause 1713 → 1714 · incident history 314 → 313 · keep 776 →
775 · pending verification 2121 → 2122 · pending adjudication 412 → 411 · retirement candidate 104
→ 105. **The repository-wide `#[test]` count is unchanged (3413)**—this round changed only the
implementation of the criteria, not any tests.

## Third Retirement Batch (Executed, 2026-11-07)

**Third batch: adjudicate the 5 retirement candidates under `src/middle/passes/codegen/tests/`,
retiring 2 and keeping 3.** (a single directory cluster = the `middle::passes::codegen` module under
test; criterion = **all three gates fail** under Part Seven of coding-rules.)

This batch continues the **keep criterion** established by the previous batch: a unit-level
observation point may only be replaced by another **unit test**; corpus and integration tests are at
the process-boundary layer and cannot replace a unit-level observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)——the function body is **verbatim
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (apart from
    comments, only the function name differs): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that the `execute_module` entry
    path returns a non-Void value, but the body never observes the return value from
    `execute_module` (`run_fixture` discards it with `.expect(...)`)— the stated intent was not
    implemented, and the function body is a pure duplicate ⇒ zero criterion increment. Replacement
    coverage = `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)——`test_scope_nesting` in the same file covers it
    strictly: a positive lookup (`lookup("a").is_some()`), a negative lookup
    (`lookup("b").is_none()`), the `scope_level` count, and the inserted `Symbol` shape
    (`symbol("a", Int(64), Local(0), false, 0)`, identical to the inline `Symbol` in this test). “A
    name that was never inserted cannot be found” and “it cannot be found after being popped” follow
    the same path in `SymbolScopeManager::lookup` (walking `scope.get(name)` layer by layer and
    returning `None` when it is not found) ⇒ this test adds zero increment. Replacement coverage =
    `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip`——the sole observation point for the path that “does
    not go through serialization to execute directly” (the serialization variant is a different
    path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`——the sole observation point for the
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)——the **sole** unit-level observation point for the
    `OperandResolver::to_reg` rejection branch where `id > 255` (the bytecode layer has independent
    overflow checks that do not cover this function; no second test in the repository drives
    `to_reg` out of range). `is_err()` is a discriminating assertion (removing the guard would make
    the test fail), not merely proof that the code did not panic.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the entire batch.

### Census Blind Spot Found in This Batch (**Fixed in 0.6-15**, See “Generator Criterion Correction” Below)

The census's “delegated assertion” recognition, `\b(assert|check)[A-Za-z_]*\s*\(`, **could not match
helper names containing digits**: because `[A-Za-z_]*` does not include digits, a call such as
`assert_slot_is_42(&interp)` was not recognized as an assertion— so 2 of the 5 candidates in this
batch were incorrectly bucketed as “not one assertion in the body.”

After all 15 repository-wide “neither passed” entries were reviewed individually (with helper names
broadened to include digits and the `match` + `panic!` form recognized separately), **4 did in fact
contain assertions**—3 used the numeric helper above, and 1 was
`src/backends/interpreter/tests/weak.rs`'s `test_weak_from_arc` (the `match` + `panic!` form; Rule
6.1 of test-specification requires `assert!(matches!(...))`, but it was in fact asserting). The
remaining 11 genuinely contained no assertions whatsoever.

**Taking these 4 directly from the machine's “neither passed” bucket would have deleted tests that
did have criteria**—manual per-test adjudication in this batch prevented that; the generator-side
fix appears below in “Generator Criterion Correction (0.6-15).”

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots were fixed: helper names containing digits, and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411); it
changes only how the generator recognizes whether a test body contains assertions.

### What Was Fixed

1. **Digits were allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing such calls were
   treated as “not one assertion in the body.”
2. **Added the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the assertion is written
   in one of the arms, and no `assert!` macro is visible lexically (Rule 6.1 of test-specification
   requires `assert!(matches!(...))`, but this form exists in the existing code). Detection method:
   scan from the `match` keyword to the first `{` (the expression), then determine the block
   boundary by **brace balancing**; the form is recognized only if `panic!(` occurs inside the
   block. **The decision is made on masked lines**— braces in strings and comments do not
   participate in balancing, and `panic!` in a comment is not treated as an assertion.

### Bucket Differences: 3 (the gate and disposition changed)

| Test name                                           | File                                           | Old bucket                            | New bucket                                  | Rationale                                                                                                                                                                                                                                                         |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------- | ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither passed / retirement candidate | Pending adjudication / pending adjudication | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not adjudicate its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither passed / retirement candidate | Pending adjudication / pending adjudication | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, so the old pattern could not match it; it is now recognized as an assertion                                                                                          |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither passed / retirement candidate | Pending adjudication / pending adjudication | Same as above (the serialization round-trip test)                                                                                                                                                                                                                 |

Another **77 entries changed only the “assertion summary” column** (the gate and disposition were
unchanged): most gained `match_panicxN` (the body contains the `match` + `panic!` form), and 4
gained `helperxN` (3 in `src/driver/tests/manifest_error.rs` and 1 in
`src/package/tests/cache.rs`—helper names containing digits).

**The relaxation changes only the “neither passed” bucket**: that bucket is the machine's
**confident verdict** (not one assertion in the body), and its criterion is a hard requirement;
“retirement candidate” is only a pool awaiting manual review, and the size of that pool is not a
verdict. All other buckets, including the weak-assertion pool, continue to use the original
criteria. Result: gate neither passed 14 → 11 · pending adjudication 923 → 926; disposition
retirement candidate 103 → 100 · pending adjudication 411 → 414.

### Existing 11 “Neither Passed” Entries: Individual Review Conclusions

After all 11 repository-wide “neither passed” entries were read and reviewed individually, **all 11
genuinely contained not one assertion** (including no `assert*` / `check*` helpers, no `match` +
`panic!` form, and no `#[should_panic]`):

| Test name                                       | File                                             | What the body actually does                                                                       | Review conclusion                                          |
| ----------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 calls to `.expect(...)`, guaranteeing only that it does not crash                               | Genuinely no assertions (proof only that it did not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                    | Genuinely no assertions                                    |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them                | Genuinely no assertions                                    |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                               | Genuinely no assertions                                    |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                         | Genuinely no assertions                                    |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                                | Genuinely no assertions                                    |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` with invalid content and discards the result                        | Genuinely no assertions (proof only that it did not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                  | Genuinely no assertions (proof only that it did not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                            | Genuinely no assertions (proof only that it did not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs the lsp / repl and discards them (inside the wasm32 cfg gate)                          | Genuinely no assertions                                    |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is merely a type annotation | Genuinely no assertions                                    |

Five of these (`test_normalizer_context` and the 4 in reducer.rs) were classified as “keep” in the
0.6-11a adjudication table because **no replacement coverage could be identified**; the conclusions
of this review agree.

### Reverse Spot Check: Did the Relaxation Go Too Far?

From the 88 entries newly recognized as assertions, 5 were read from the source using a fixed seed
(20261107) to confirm that they really assert:

| Spot-checked entry                                          | File                                                 | Assertion in the source                                                                                                                      |
| ----------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching `match` arm; `panic!("Expected AssocType")` if it does not match                                      |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm; `panic!("expected InvalidPackage, got …")` if it does not match                  |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | An `assert_eq!` comparing the constraint type in the matching arm; `panic!("不是 Dynamic 变体…")` if it does not match                       |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm; `panic!("Expected TokensDelta variant")` if it does not match                        |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm verify the error code and message; `panic!("不是 Disproved…")` if it does not match |

All 5 assert a concrete value or concrete variant (replacing the implementation with another correct
path would make them fail)—**the relaxation did not go too far**. Reverse controls were also added
to `--selftest`: a `match` without `panic!`, and a `panic!` that occurs only in a comment; neither
may be recognized as an assertion.

### The Criteria Have Teeth (a Developmental Proof)

When the generator was reverted to the old implementation (`HELPER_RE` without digits and with
`match` + `panic!` recognition disabled), all of the new `--selftest` cases **failed**, with exit
code 1; the new implementation passes all of them.

### Rollback

The generator and census table are in the same commit; `git revert <sha>` restores the entire state.

## Fourth Retirement Batch (Executed, 2026-11-07)

**Fourth batch: retire 1 and keep 9 of the 10 “retirement candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend's test directory)** (a single directory
cluster, with all three files read before adjudication: `tests/bytecode_load.rs` 4 entries ·
`tests/weak.rs` 2 entries · `executor/tests/debug.rs` 4 entries). Criterion = the **three gates**
under Part Seven of coding-rules; keep criterion = “a unit-level observation point may only be
replaced by another unit test, while corpus/integration tests are at the process-boundary layer”;
**a deletion must identify a specific replacement coverage test name; if none can be identified or
the code cannot be understood, classify the test as “keep.”**

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)——`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, one step, then obtain
  `capture_stack`), and its assertions are **strictly stronger**: it asserts that the stack is
  nonempty and that `stack[0].function_name == "main"` (pinning the top-of-stack position), whereas
  the listed test asserts only `stack.iter().any(|f| f.function_name == "main")` (any position is
  sufficient)—— the former being true necessarily implies the latter ⇒ zero criterion increment.
- **9 kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading the source
  individually, each is the **sole unit-level observation point** for some property (the load
  magic-number and version branches are distinct, the YXBC routing layer, both sides of Weak's
  Some/None, stepping with no frames, popping the frame on step_out, and reaching the end with an
  empty stack), and no replacement unit test could be identified ⇒ classified as “keep” under the
  hard requirement. The per-test rationale appears in the commit message (the adjudication table is
  not included in this page).
- **Figures**: repository-wide `#[test]` 3409 → 3408; 1 deletion = `git diff` counts `-#[test]` 1
  line and `+#[test]` 0 lines; the dominating test `test_capture_stack_during_execution` passes when
  run on its own. Census: retirement candidate 99 → 98 · pending adjudication 925 → 924 · clause
  1713 · incident history 313 · proof 447 · neither passed 11 · pending verification 2121 ·
  keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the entire batch.

## Fifth Retirement Batch (Executed, 2026-11-07)

**Fifth batch: bulk retirement of 1353 unit tests under the mechanical rule “for the same symbol and
result category, keep only the strongest test”** (only unit tests under `src/**/tests/**` were
changed; not a single test on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (executed mechanically, without per-test argument)**: for the same symbol under test + the
  same assertion-result category, keep only the test with the strongest assertions and delete the
  rest.
  - Symbol under test = a repository symbol invoked in the test body (including one level of
    propagation through a helper function in the same file); when multiple symbols are reached,
    classify by the **primary symbol** (the first repository invocation). Symbols are qualified by
    repository path and are not merged across modules by bare name.
  - Result category = concrete value / a specific error code (E code) / a specific variant match /
    only checks ok / only checks err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when tests in the same group are equally
    strong, keep the first one to appear in the file.
  - When a test asserts multiple categories simultaneously (making the primary category
    indeterminable), **keep it**; do not guess.
- **Three exceptions (not one test was spared)**: ① the name or comment identifies `issue #` /
  regression / defect / incident (the incident-history gate); ② a symbol has only one test (deleting
  it would leave the symbol without a unit-level observation point); ③ `#[should_panic]` and the
  test is the symbol's only should_panic case. The 313 incident-history tests and 11 “neither
  passed” tests had **0 retirements**.
- **Figures**: repository-wide `#[test]` 3408 → 2055; 1353 deletions = `git diff` counts `-#[test]`
  1353 lines and `+#[test]` 0 lines. Census (by gate, current values after the sample audit was
  abolished in 0.6-19): clause 808 · incident history 313 · proof 424 · pending adjudication 499 ·
  neither passed 11. Census (by disposition, current values): keep 737 · pending verification 1054 ·
  pending adjudication 203 · retirement candidate 61. The batch's retirements by gate were: clause
  902 · pending adjudication 428 · proof 23 (0.6-19 moved 9 entries previously classified as “audit
  confirmed actual support” to pending verification, so the current values by gate/disposition
  differ from those at the fifth batch's closing point).
- **Cleanup alongside deletions**: helper functions used only by deleted tests, imports that became
  invalid, and structure fields that became unused were removed as well
  (`cargo clippy --all --all-features --all-targets -- -D warnings` reported 0 warnings).
- **Two out-of-scope deletions that must be recorded**: 29 tests originally classified as “keep”
  were still deleted under this batch's rule—23 current-state anchors (names containing
  unchanged/anchor/guard/preserve/baseline, many of which were false positives caused by **domain
  terms** such as “guard” or “preserves,” for example `test_while_guard_scope_balanced`) and 6 tests
  whose clause support had been confirmed by the audit. This batch followed the closed list of three
  exceptions on the card and did not add protection for these two categories; to restore them,
  backfill them using the keep/delete correspondence table in this page.
- **Failure and abolition of the sample audit**: the 40-entry sample was drawn from the test pool
  using a fixed seed; after the fifth batch, 38 entries from the original sample no longer existed,
  so the audit's `--check` consistently reported “38 missing, 38 extra.” The sample audit was
  therefore abolished and deleted in 0.6-19 (the script, data file, and the generator's dependencies
  on them were all removed); see “Sample Audit Abolition” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the entire batch.

## Sixth Retirement Batch (Executed, 2026-11-26)

**Sixth batch: the mechanism rollback from the 4.9.9 narrowing round—retire 28 tests deleted along
with the mechanism; the 4 prohibition criteria were never checked in.** This batch does not use the
three gates to decide whether the tests “should be kept”; instead, the mechanism under test was
**rolled back in its entirety**: the criteria written for the downstream ambiguity mechanisms from
the first six rounds before 4.9.9 (candidate-provider priority / candidate-signature comparison /
owner compatibility reasoning / IR-side named-argument fallback / import-name resolution-point E5006
prohibition) were retired along with the mechanism—the mechanism no longer exists, so the tests have
no object under test. These deletions are registered as a **batch** under coding-rules 7.3 and do
not enter the retirement candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 entries (28 deletions: 27
    downstream-mechanism criteria from the first six rounds
    - 1 alias-import named-argument criterion). The 2 retained entries pin the minimal fix for
      bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: all 4 E5006 prohibition criteria were retired—the
    file existed only in an intermediate 4.9.9 commit and appeared in neither the parent commit nor
    that version's checked-in table; after the prohibition was withdrawn, there was no object under
    test, so this batch does not decrement the checked-in table.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the candidate-injection surface for alias imports was rolled back, so the fixture's
    behavior no longer holds); the corpus-difference baseline entry with the same name was deleted
    in the same batch. The baseline scope is stated separately: **relative to direct parent
    `ac810e97`, 405 → 404 lines** (the intermediate state had included the fixture); **relative to
    the pre-4.9.9 baseline `08ccacc5`, 403 → 404 lines** (+1 base fixture
    `named-args-cross-unit-method`; the retired `alias-overload` never entered the final history).
- **Which gate applies**: the 2 retained entries = **incident-history gate** (they pin an actual
  defect registered in 4.9.9: a cross-unit named argument could not obtain the declaring parameter
  name, leaving `E1014 available` empty); among the 28 retired entries, the mechanism criteria added
  during the first six rounds were mostly on the incident-history gate, but the reason for this
  batch's deletion is **not** the three gates (they are not being judged worthless); the mechanism
  under test was rolled back.
- **Replacement coverage**: cross-unit named arguments for bare-name imports = the 2 retained
  driver-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity surface
  no longer has rejection logic (the requirement was rolled back), so there is no replacement
  criterion; the 4.9.8 dispatch-attribution criteria (10 driver tests + 5 integration tests) are
  retained unchanged.
- **Restoration**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the `method_param_info`
  empty-name-table guard was retained; this form is unreachable under the public syntax and is a
  defensive guard, whose unit test prevents accidental deletion).
- **Figures** (after this batch was regenerated): repository-wide `#[test]` 2680 entries / 334
  files; previous checked-in table 2688 entries → current tree 2680 entries (−8); the current tree
  contains 1 entry absent from the old table, the old table contains 9 entries absent from the
  current tree, and 0 rows with the same key were recomputed.
- **Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit); `git revert <sha>` rolls back the entire batch.

## Sample Audit Abolition (0.6-19)

**Criterion**: the sample audit (0.6-10) measured the “actual support rate” of clause references,
but after 0.6-18 the retention rule no longer examines citations (changed to “for the same symbol
and result category, keep only the strongest test,” reducing the test pool from 3408 to 2055). Of
the original 40 samples, 38 no longer existed, and the audit's `--check` consistently reported “38
missing, 38 extra”—keeping it would only produce a continuous alarm. It was therefore **abolished
and deleted**.

**What was deleted**: the sample-audit script under `scripts/dev/` (the one that reproduced the 40
samples using a fixed seed) and its JSON data file (40 per-test decisions and rationales).

**The generator's dependency was removed at the same time**: the generator previously read that JSON
and classified “audit confirmed actual support” as “clause gate + keep”; that branch was
removed—**clause references no longer have an “audit confirmed” tier** and always fall into “clause
/ pending verification.”

**Distribution changes** (all 2055 repository-wide `#[test]` entries remained unchanged, and the TSV
was regenerated): keep 746 → 737 (−9: the 9 entries previously confirmed by the audit as genuinely
supported fell back to pending verification); pending verification 1045 → 1054; by gate: clause 811
→ 808, pending adjudication 496 → 499 (the others are unchanged: incident history 313 · proof 424 ·
neither passed 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` rolls back the entire batch.

## Rerunning and Self-Checking

```bash
python scripts/dev/test-gate-census.py            # Recalculate and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV is consistent with the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical classification only, and states its boundaries honestly:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test's assertion is hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as having “no assertions”—the first batch of 26 tests (21 retired / 5 kept) and the
  second batch of 18 tests (11 retired / 7 kept) were manually reviewed function by function.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator extracts a function body, it
  balances braces; braces in string/character literals and comments can throw the balance off and
  truncate the body, so the body of `test_prefix_info_lbrace` (which contains `tokenize("{")`) was
  formerly truncated to the end of the file, causing the “assertion summary” to take in the rest of
  the file and the test to be bucketed as “pending adjudication” rather than “retirement candidate.”
  The generator now masks non-code regions (strings / character literals / line comments / block
  comments / raw strings) before balancing; see “Generator Correction” below.
- **Delegated-assertion recognition (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not include digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions, and tests containing such calls
  were misclassified as having “not one assertion in the body.” At the time, 4 of the repository's
  15 “neither passed” entries actually contained assertions (3 numeric helpers + 1 `match` +
  `panic!` form). The name portion has now been broadened to `[A-Za-z0-9_]*`, and the `match` +
  `panic!` form is recognized; see “Generator Criterion Correction” below.
- **The proof gate is not adjudicated**: every test that fails both preceding gates is classified
  only as “pending adjudication” or “retirement candidate”; the generator never declares that the
  test “should be deleted.”
- **No cross-file deduplication**: completely duplicate tests are not identified in this table
  (0.6-2 measured only 1%, which does not constitute a retirement surface).
- **Clause references are always pending verification**: a verifiable section number in the file
  header only shows that the test “maps to a specific, identifiable clause”; it does not mean that
  the reference supports the assertion. The machine does not determine whether it does, so even a
  clause hit stops at “pending verification” and must be verified by a human before the test can be
  kept.
