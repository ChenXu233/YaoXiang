---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement candidate list plus retirement records for each batch'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **Three Gates (clause / failure history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a reproducible, item-by-item
adjudication table. A sampling audit of the clause gate (40 entries) once measured the proportion of
tests that “cited a clause but whose assertions were not supported by it” and thereby **tightened
the criteria**. That audit was abolished and deleted in 0.6-19. **This page and the generator itself
are read-only and do not modify tests**; test retirement is performed in batches (one commit per
batch, with the whole batch eligible for `git revert`), and the execution results are recorded in
the batch retirement sections below and in the commit messages.

Artifacts:

| Artifact                            | Purpose                                                                                         |
| ----------------------------------- | ----------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Item-by-item adjudication table (2680 rows plus the header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                                     |

## Criteria

### Scanning surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 2076                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2680** (334 files) |

### Machine criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Meaning of a hit                                                |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | A **verifiable reference** appears within the first 25 lines of the file: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); for files under `src/**/tests/`, the parent `mod.rs` is included in the search                                                                                                                                                                                                                                  | The test corresponds to a clause that can be identified by name |
| Failure-history gate              | The test function body, function doc comment, or **file header** (such as `//! Regression context: issue #226`) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                                    | The test pins down an incident that has actually occurred       |
| Proof gate                        | **Cannot be determined mechanically** (“If it were replaced with another correct implementation, would it fail?” requires human judgment). Therefore, anything that fails both of the first two gates is classified as “Pending adjudication” rather than automatically as “Fails all gates”; the sole exception is a test with **not a single assertion in its body** (including no delegation to `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only “Pending adjudication” or “No assertions” can be assigned  |

Rationale for tightening: the clause gate in 0.6-9 only required the file header to **contain the
text `§`/`RFC`/`SPEC`**, so 2403 entries were classified as “Clause/Keep”. The 40-entry stratified
sample measured in 0.6-10 showed that **half of the citations did not support the assertions**
(misquotation rate: 50%; 95% interval: 35%–65%)—this measurement was the **direct reason for
abandoning the practice of keeping a test on the strength of a single citation**. After tightening,
only section-number forms are recognized, and **having a citation still does not mean that the
citation supports the assertions**—all clause citations stop at “Pending verification”. (This
sampling audit was abolished and deleted in 0.6-19: as the test population shrank through
retirement, 38 entries of the original sample no longer existed, so `--check` consistently reported
“38 missing, 38 extra”; the conclusion is retained, but the audit artifacts are no longer
maintained.)

Rationale for the file-header radius: this repository’s convention is to write “Regression context:
issue #N” in the file header and not repeat it in each test’s comment. Looking only at a test’s own
comment would miss the failure history of the entire file (measured: 7 entries at the test’s own
radius versus 291 entries at the file-header radius).

### Categories that are never retired

The following tests are **always “Keep”** and do not enter the retirement candidate pool:

- Corpus `tests/yaoxiang/**` and project-shape `tests/project-corpus/**` (behavior-criteria
  surfaces);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and cross a process boundary);
- Tests whose names contain `unchanged` / `anchor` / `guard` / `preserv` / `baseline`, which serve
  as status quo anchors.

### Disposition semantics

| Disposition          | Meaning                                                                                                                                                                                         |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Only two possibilities: a failure-history gate hit, or a proof gate (spec-shaped surface / status quo anchor name)                                                                              |
| Pending verification | Cites a clause (with a section number, or citing an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a human before it can be kept |
| Retirement candidate | Both gates fail and the assertion surface contains only weak assertions (or no assertions)—a candidate is not a verdict; the proof gate still requires manual review                            |
| Pending adjudication | Both gates fail, but there are strong assertions; the machine reaches no conclusion                                                                                                             |

## Distribution (regenerated on 2026-11-26; rechecked in the 4.9.9 narrowing round)

| Gate                                                                     | Count    |
| ------------------------------------------------------------------------ | -------- |
| Clause (with a verifiable reference)                                     | 1017     |
| Failure history                                                          | 557      |
| Proof (spec-shaped criteria surface / status quo anchor)                 | 571      |
| Pending adjudication (both gates fail; proof gate awaits human judgment) | 529      |
| Fails all gates (no assertions in the body)                              | 6        |
| **Total**                                                                | **2680** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1128** | 737 (+391)                        |
| Pending verification | 1271     | 1054 (+217)                       |
| Pending adjudication | 223      | 203 (+20)                         |
| Retirement candidate | 58       | 61 (−3)                           |
| **Total**            | **2680** | 2055 (+625)                       |

Breakdown of Keep: 1128 = 571 proof-gate entries (518 on spec-shaped criteria surfaces + 53 status
quo anchors) + 557 failure-history-gate entries. File-level criteria: the **clause gate** covers
**122 of the 334** test files; within the first 25 lines of a file (`src/**/tests*/` including the
parent `mod.rs`), a verifiable section number can be detected in **166** files.

Reconciliation: the previous committed table had 2688 entries, while the current tree has 2680. The
current tree has one entry absent from the old table (the new deterministic criterion
`all_method_bindings`), while the old table has nine entries absent from the current tree (all
downstream-mechanism criteria from the first six rounds of 4.9.9; see “Sixth Retirement Batch”). No
rows with the same key were recalculated. Each key is recorded in the TSV; the fate of stale keys is
recorded in this batch’s commit message.

### Two policy changes at the 0.6-19 point in time

1. **Keep breakdown**: 737 = 424 proof-gate entries (390 on spec-shaped criteria surfaces + 34
   status quo anchors) + 313 failure-history-gate entries. Of the 2121 entries among the original
   2897 that had been “kept on the strength of a single citation,” all are now pending verification
   (after the 0.6-19 sampling audit was abolished, the nine entries previously classified as
   “confirmed by the audit as genuinely supporting” were also moved to pending verification—there is
   no longer an “audit-confirmed” tier for clause citations).
2. **Failure-history gate 47 → 313**: these are not newly discovered failure histories, but **label
   reassignments**. In 0.6-9, an entry that hit both the clause and failure-history gates was
   recorded under the clause gate. After tightening, a clause hit no longer automatically resulted
   in Keep, so the failure-history gate had to catch these entries; their counts are therefore
   folded into the failure-history total. The overall Keep total is unaffected (these tests were
   also kept in 0.6-9).

File-level criteria at the time: of 285 test files, **136** had a verifiable section number in the
file header (the old 0.6-9 criterion found 221 files).

## Reconciliation between the 0.6-19 point in time and the 0.6-2 survey

The figures from the 0.6-2 survey are the baseline for the first comparison. The reasons for each
difference are listed below:

| Item                            | 0.6-2 survey                  | Current                                                                                                                                                  | Reason for the difference                                                                                                                                                                                                                                                                                   |
| ------------------------------- | ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                                                                     | Two entries were added after 0.6-2 (check-criterion regression tests for E3005 and E3022), followed by the removal of 21 assertion-free tests from the first batch, 11 weak-assertion tests from the second batch, 2 from the third batch, 1 from the fourth batch, and 1353 from the fifth batch; net −388 |
| Number of test files            | 296                           | 285                                                                                                                                                      | Eleven entire files were retired in the fifth batch                                                                                                                                                                                                                                                         |
| Clause gate                     | 227 files (77%)               | 136 files after tightening; 808 entries                                                                                                                  | The criterion was tightened from “the text appears” to “there is a section number `§<number>`,” nearly halving the file count; the fifth batch retired 902 entries, and another nine moved from “Keep” to pending verification after the 0.6-19 sampling audit was abolished                                |
| Failure-history signals         | 442 entries / 148 files       | 313 entries within the gate; three radii: 7 at the test’s own radius / 291 at the file-header radius (38 files) / 1218 across the entire file (79 files) | The 148 files in the 0.6-2 survey covered more than the 79 files found at the current whole-file radius because its vocabulary and scope were broader; the current survey strictly applies the four signals on the card                                                                                     |
| Strong characterization signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                                                             | Characterization is not one of the Three Gates; incorporating it into a retirement conclusion would exceed the mandate                                                                                                                                                                                      |
| Weak assertions                 | 473 (14%)                     | See the “Assertion summary” column in the TSV                                                                                                            | The counting units differ: 0.6-2 counted patterns that only assert `is_ok`/`is_some`; the current survey uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                                                                     |

## 0.6-19 conclusion summary (historical)

1. **The clause gate had previously been substantially inflated**: under the old criterion, 2403
   entries were classified as “Clause/Keep,” and the measured misquotation rate in the sample was
   50% (95% interval: 35%–65%)—“having a citation” and “the citation supporting the assertions” are
   two different things, and the old criteria treated the former as the latter. (This sampling audit
   was abolished and deleted in 0.6-19; the conclusion is retained. See “Abolition of the Sampling
   Audit” below for the reason for deletion.)
2. **After tightening, approximately 1202 entries genuinely have clause support** (range: 846–1557);
   the remainder must either await verification or be adjudicated through the failure-history/proof
   gates.
3. **The retirement surface is unrelated to misquoted clauses**: the 136 candidates had no clauses
   (no assertions/weak assertions). The first batch retired 21 entries, the second batch retired 11,
   the generator correction added 1, the third batch removed 2, the 0.6-15 criterion correction
   removed 3, the redundancy scan removed 1, the fourth batch removed 1, and the fifth batch removed
   1353 (using the mechanical rule “for the same symbol and result category, keep only the strongest
   entry”), reducing the candidates to 61. Fifth-batch retirement by gate: Clause 902 · Pending
   adjudication 428 · Proof 23 · Failure history 0 · Fails all gates 0—**not a single
   failure-history-gate or zero-assertion entry was removed**.
4. **The failure-history gate is the largest legitimate source of Keep entries** (313 entries): it
   demonstrates that “pinning down an actual incident” is more reliable than “copying a sentence
   from an RFC”.

## First Retirement Batch (executed on 2026-11-06)

**First batch: among the 26 assertion-free tests in `src/frontend/core/types/eval/tests/`, retire 21
and keep 5** (a single directory cluster; criterion = all **Three Gates fail** according to Part
Seven of coding-rules: no clause-gate hit, no failure-history-gate hit, and not a single assertion
in the body). The item-by-item adjudications and “same-slot coverage” are provided by the 0.6-11a
adjudication table (21 deleted / 5 kept); this page does not repeat the reasons and only records the
execution results.

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
- **5 kept** (the adjudication table could not identify substitute coverage, so they must be kept
  under the hard requirement): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Second Retirement Batch (executed on 2026-11-06)

**Second batch: among the 18 “weak assertion” tests in
`src/frontend/core/parser/pratt/tests/nud.rs`, retire 11 and keep 7** (a single directory cluster =
the Pratt prefix-parsing module; criterion = all **Three Gates fail** according to Part Seven of
coding-rules: no clause-gate hit, no failure-history-gate hit, and the body contains only weak
assertions such as `is_some()` / `is_none()`—neither binding power nor the parsing result is pinned
down).

The difference between this batch and the first batch must be stated explicitly: these tests **do
contain assertions** and are not empty tests. The reason for retirement is “all Three Gates fail +
zero criterion delta,” not “no assertions”—each entry slated for deletion identifies a **substitute
unit test name in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Every substitute
  follows the **same** `parse_expression → parse_prefix → prefix_info` path and makes stronger
  assertions (pinning down the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, plus `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—the reasons for each are recorded in the commit message (Task 0.6, second
  test retirement batch). There are two categories of reason for keeping them: the **proof gate
  holds** (the first three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are the
  sole unit-level observation points for their corresponding facts, and no second unit test in the
  repository observes them), and the **failure-history gate holds** (`break_removed` /
  `continue_removed` name #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Generator Correction (0.6-13)

**`body_span` truncates a function body when it encounters braces in literals—fixed.**

The old implementation located function bodies by balancing braces without distinguishing real code
from braces in strings/character literals or comments. Such forms are common in Rust source, e.g.
`tokenize("{")`, so the balancing was thrown off and the function body was truncated: the body of
`test_prefix_info_lbrace` once ran to the end of the file, and the “Assertion summary” captured the
remainder of the file (`matchesx11 assertx24 …`), causing it to be binned as “Pending adjudication”
rather than “Retirement candidate”—**a binning error**.

The fix adds `mask_noncode`, which first replaces the contents of string literals, character
literals, line comments, block comments (including nested ones), and raw strings (`r"…"` / `r#"…"#`
/ `br"…"` / `cr#"…"#`) with spaces (preserving newlines, line numbers, and line counts), then
balances the braces. **The mask is used only to locate function-body boundaries**; assertion
statistics and the failure-history gate still read the original text, and the criteria themselves
have not been relaxed. `--selftest` gained four cases (paired and unpaired braces in ordinary
strings, braces in raw strings and block comments, and braces in character literals), each followed
by a sentinel test—the old implementation found the wrong boundary in all four cases, while the new
implementation finds the correct boundary in every case.

### Tests affected by the same bug: 11 entries

Counting method: compare the **function-body boundaries before and after the fix** file by file and
test by test (old implementation balancing literals directly vs. new implementation balancing after
masking); any test with different boundaries is affected. The old implementation found the wrong
boundary for **11 of 3413** tests in the repository:

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

### Binning differences before and after the fix: 2 entries

In the other nine entries, **eight affect only the “Assertion summary” column** (the bin is
unchanged), and **one does not change even the summary**
(`test_fstring_close_brace_escape_preserved_raw`: the old implementation omitted the closing `}`
line, which contains no assertion keyword). The eight summary changes are listed individually in the
commit message.

| Test name                   | File                                          | Old bin                                     | New bin                                         | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| --------------------------- | --------------------------------------------- | ------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending adjudication / Pending adjudication | Pending adjudication / **Retirement candidate** | The real function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; neither the file header nor the function body has a § section number, issue/regression/defect/incident, so both the clause gate and failure-history gate fail; the body contains only a weak assertion such as `is_some()`, and therefore it is a retirement candidate (a candidate is not a verdict; the proof gate still requires manual review)                                                         |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Failure history / Keep                      | Clause / **Pending verification**               | The old implementation truncated the function body to the end of the file, swallowing text containing “regression / defect / incident” and incorrectly satisfying the failure-history gate; the real function body contains no such text, so the failure-history gate no longer holds; the file header has verifiable references (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012), so the clause gate is hit but has not been confirmed by the sampling audit, and therefore it is pending verification |

The distribution changes accordingly: Clause 1713 → 1714 · Failure history 314 → 313 · Keep 776 →
775 · Pending verification 2121 → 2122 · Pending adjudication 412 → 411 · Retirement candidate 104
→ 105. **The repository-wide `#[test]` count is unchanged (3413)**—this round changes only the
implementation of the criteria, not the tests.

## Third Retirement Batch (executed on 2026-11-07)

**Third batch: adjudicate the five retirement candidates in `src/middle/passes/codegen/tests/`
individually; retire two and keep three.** (a single directory cluster = the
`middle::passes::codegen` module under test; criterion = the **Three Gates** according to Part Seven
of coding-rules.)

This batch follows the **keep criteria** established in the previous batch: a unit-level observation
point can only be replaced by another **unit test**; corpus/integration tests are at the
process-boundary layer and cannot replace a unit observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—its function body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (apart from
    comments and the function name): both are `run_fixture()` + `assert_slot_is_42(&interp)`. Its
    doc comment claims to verify that the `execute_module` entry path returns non-Void, but the body
    never observes the return value of `execute_module` (`run_fixture` discards it with
    `.expect(...)`)— the claimed intent is not implemented, and the function body is a pure
    duplicate ⇒ zero criterion delta. Substitute coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file provides strict
    coverage: a positive lookup (`lookup("a").is_some()`), a negative lookup
    (`lookup("b").is_none()`), the `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)`, the same shape as the inline `Symbol` in this
    entry). “A name that was never inserted cannot be found” and “it cannot be found after being
    popped” follow the same path in `SymbolScopeManager::lookup` (walking `scope.get(name)` layer by
    layer and returning `None` when not found) ⇒ this entry adds zero coverage. Substitute coverage
    = `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that
    bypasses direct serialization and executes the value (the serialization variant is a different
    path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for a
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit observation point for the `id > 255`
    rejection branch of `OperandResolver::to_reg` (the bytecode layer has its own independent
    overflow checks and does not cover this function; no other test in the repository takes `to_reg`
    out of range). `is_err()` is a discriminating assertion (removing the guard would make it fail),
    not merely a check that it did not panic.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

### A census blind spot found in this batch (**fixed in 0.6-15**; see “Generator Criterion Correction” below)

The census’s recognition of “delegated assertions” uses `\b(assert|check)[A-Za-z_]*\s*\(` and
therefore **cannot match helper names containing digits**: because `[A-Za-z_]*` does not include
digits, a call such as `assert_slot_is_42(&interp)` is not recognized as an assertion, and two of
the five candidates in this batch were consequently mis-binned as “no assertions in the body”.

After individually reviewing all 15 repository-wide “Fails all gates” entries (by allowing digits in
helper names and additionally recognizing the `match` + `panic!` form), **four did in fact contain
assertions**: three were the numeric helpers above (`assert_slot_is_42`), and one was
`test_weak_from_arc` in `src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form;
test-specification rule 6.1 requires `assert!(matches!(...))`, but it does assert). The remaining 11
truly contain no assertions.

**Taking these four entries away based on the machine’s “Fails all gates” result would have wrongly
deleted tests that contain criteria**—the manual adjudication in this batch prevented that; the
generator-side fix is described in “Generator Criterion Correction (0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots have been fixed: digits in helper names, and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411); it only
changes how the generator recognizes whether a test body contains assertions.

### What was fixed

1. **Digits are now allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing them were
   treated as “not a single assertion in the body”.
2. **The `match` + `panic!` form was added**: `match <expr> { … panic!(…) … }`—the assertion is
   written in one of the arms, so the macro `assert!` is not visible lexically (test-specification
   rule 6.1 requires `assert!(matches!(...))`, but existing code contains this form). Recognition
   method: scan the expression after the `match` keyword up to the first `{`, then determine the
   block boundary by **balancing braces**; only a `panic!(` within that block counts. **The decision
   is made on masked lines**— braces in strings and comments do not participate in balancing, and
   `panic!` in comments is not treated as an assertion.

### Binning differences: 3 entries (the gate and disposition changed)

| Test name                                           | File                                           | Old bin                                | New bin                                     | Basis                                                                                                                                                                                                                                                        |
| --------------------------------------------------- | ---------------------------------------------- | -------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Fails all gates / Retirement candidate | Pending adjudication / Pending adjudication | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts a variant, but the form was not previously recognized; it is now recognized as an assertion, but the machine does not adjudicate its coverage power |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Fails all gates / Retirement candidate | Pending adjudication / Pending adjudication | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, so the old pattern did not match; it is now recognized as an assertion                                                                                          |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Fails all gates / Retirement candidate | Pending adjudication / Pending adjudication | Same as above (the serialization round-trip entry)                                                                                                                                                                                                           |

Another **77 entries changed only in the “Assertion summary” column** (the gate and disposition are
unchanged): most summaries gained `match_panicxN` (the body contains the `match` + `panic!` form),
and four gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3 entries;
`src/package/tests/cache.rs`: 1 entry—helper names containing digits).

**The relaxation changes only the “Fails all gates” tier**: that tier is the machine’s **confident
verdict** (not a single assertion in the body), and its criterion is a hard requirement; “Retirement
candidate” is merely a pool awaiting manual review, and the size of that pool is not a verdict. All
other bins, including the weak-assertion pool, continue to use the original criteria. Result:
Gates—Fails all gates 14 → 11 · Pending adjudication 923 → 926; Dispositions—Retirement candidate
103 → 100 · Pending adjudication 411 → 414.

### Item-by-item review of the 11 remaining “Fails all gates” entries

After individually reading the original text of all 11 repository-wide “Fails all gates” entries,
**all 11 truly contain no assertions** (including no `assert*` / `check*` helper, no `match` +
`panic!` form, and no `#[should_panic]`):

| Test name                                       | File                                             | What the body actually does                                                                   | Review conclusion                                         |
| ----------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | Five `.expect(...)` calls, guaranteeing only that it does not crash                           | Truly assertion-free (proves only that it does not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                | Truly assertion-free                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them            | Truly assertion-free                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards it                                                   | Truly assertion-free                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards it                                             | Truly assertion-free                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards it                                    | Truly assertion-free                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards it                              | Truly assertion-free (proves only that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                              | Truly assertion-free (proves only that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                        | Truly assertion-free (proves only that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                          | Truly assertion-free                                      |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                          | Truly assertion-free                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Truly assertion-free                                      |

Five of these (`test_normalizer_context` and the four entries in reducer.rs) were kept in the
0.6-11a adjudication table because **no substitute coverage could be identified**; the conclusion of
this review agrees with that table.

### Reverse spot check: did the relaxation go too far?

Five entries were selected with a fixed seed (20261107) from the 88 entries newly recognized as
assertions, and their original text was read to confirm that they really do assert:

| Spot-check entry                                            | File                                                 | Assertion in the original text                                                                                                               |
| ----------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching arm; `panic!("Expected AssocType")` if it does not match                                              |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm; `panic!("expected InvalidPackage, got …")` if it does not match                  |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | An `assert_eq!` in the matching arm compares the constraint type; `panic!("不是 Dynamic 变体…")` if it does not match                        |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm; `panic!("Expected TokensDelta variant")` if it does not match                        |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm verify the error code and message; `panic!("不是 Disproved…")` if it does not match |

All five assert concrete values or concrete variants (replacing the implementation with another
correct path would make them fail)—**the relaxation did not go too far**. Reverse controls were also
added to `--selftest`: a `match` without `panic!`, and `panic!` appearing only in a comment, must
**not** be recognized as assertions.

### The criterion has teeth (proof by development)

After reverting the generator to the old implementation (`HELPER_RE` without digits + recognition of
the `match` + `panic!` form disabled), the newly added `--selftest` cases **all fail**, with exit
code 1; the new implementation passes them all.

### Rollback

The generator and census table are in the same commit; `git revert <sha>` restores the whole thing.

## Fourth Retirement Batch (executed on 2026-11-07)

**Fourth batch: among the 10 “retirement candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend’s test directories), retire one and keep nine**
(a single directory cluster; all three files are read together before adjudication: 4 in
`tests/bytecode_load.rs` · 2 in `tests/weak.rs` · 4 in `executor/tests/debug.rs`). Criterion = the
**Three Gates** according to Part Seven of coding-rules; keep criteria = “a unit-level observation
point can only be replaced by another unit test; corpus/integration tests are at the
process-boundary layer”; **deletion requires identifying a specific substitute coverage test name;
if it cannot be identified or understood, the entry is kept**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping` plus one step, then
  obtaining `capture_stack`), and its assertions are **strictly stronger**: it asserts that the
  stack is nonempty and that `stack[0].function_name == "main"` (pinning down the top-of-stack
  position), whereas the listed entry asserts only `stack.iter().any(|f| f.function_name == "main")`
  (any position is sufficient) —the former being true necessarily implies the latter ⇒ zero
  criterion delta.
- **9 kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each one in full, each
  is the **sole unit observation point** for some property (the distinct magic-number/version
  branches for load, the YXBC routing layer, both sides of Weak’s Some/None, stepping with no
  frames, step_out popping the stack, and running to completion with an empty stack), and no
  substitute unit test could be identified ⇒ they are kept under the hard requirement. The basis for
  each entry is recorded in the commit message (the adjudication table is not included in this
  page).
- **Numbers**: repository-wide `#[test]` 3409 → 3408; 1 deleted = `git diff` counts `-#[test]` on 1
  line and `+#[test]` on 0 lines; the dominating test `test_capture_stack_during_execution` passes
  in isolation. Census: Retirement candidate 99 → 98 · Pending adjudication 925 → 924 · Clause 1713
  · Failure history 313 · Proof 447 · Fails all gates 11 · Pending verification 2121 · Keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Fifth Retirement Batch (executed on 2026-11-07)

**Fifth batch: retire 1353 unit tests in bulk using the mechanical rule “for the same symbol and
result category, keep only the strongest entry”** (only unit tests under `src/**/tests/**` were
changed; not a single test was touched on the E2E surfaces `tests/yaoxiang/**`,
`tests/integration/**`, `tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or
`tests/project-corpus/**`).

- **Rule (mechanically applied, without arguing each entry individually)**: for the same symbol
  under test + the same assertion-result category, keep only the strongest assertion and delete the
  rest.
  - Symbol under test = a repository symbol called in the test body (including one level of
    propagation through a helper in the same file); when multiple symbols are reached, classify by
    the **primary symbol** (the first repository call). Symbols are qualified by their repository
    path and are not merged across modules by bare name.
  - Result category = concrete value / a specific error code (E code) / a specific variant match /
    only ok / only err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when entries in a group are equally strong,
    keep the first one to appear in the file.
  - When a test asserts multiple categories (making the primary category indeterminate), **keep it**
    and do not guess.
- **Three exceptions (not one was removed)**: ① the name or comment names `issue #` / regression /
  defect / incident (failure-history gate); ② a symbol has only one test (deleting it would leave
  that symbol without a unit observation point); ③ `#[should_panic]` is the symbol’s only
  should_panic case. Of the 313 failure-history-gate entries and the 11 “Fails all gates” entries,
  **zero were retired**.
- **Numbers**: repository-wide `#[test]` 3408 → 2055; 1353 deleted = `git diff` counts `-#[test]` on
  1353 lines and `+#[test]` on 0 lines. Census by gate (current values after the 0.6-19 sampling
  audit was abolished): Clause 808 · Failure history 313 · Proof 424 · Pending adjudication 499 ·
  Fails all gates 11. Census by disposition (current values): Keep 737 · Pending verification 1054 ·
  Pending adjudication 203 · Retirement candidate 61. The fifth batch’s retirement count by gate:
  Clause 902 · Pending adjudication 428 · Proof 23 (0.6-19 reclassified nine entries previously
  “confirmed by the audit as genuinely supporting” as pending verification, so the current
  gate/disposition values differ from those when the fifth batch was closed).
- **Cleanup accompanying the deletions**: helper functions used only by deleted tests, imports that
  became invalid, and struct fields that became unused were also removed
  (`cargo clippy --all --all-features --all-targets -- -D warnings` reports 0 warnings).
- **Two out-of-bounds deletions that must be recorded**: 29 entries originally classified as “Keep”
  were still deleted by this batch’s rule—23 status quo anchors (names containing
  unchanged/anchor/guard/preserve/baseline, many of which were false positives on **domain words**
  such as “guard” or “preserves,” e.g. `test_while_guard_scope_balanced`) and six entries whose
  clause support had been confirmed by the audit. This batch followed the closed list of three
  exceptions on the criteria card and did not additionally protect these two categories; restoring
  them only requires backfilling according to the keep/delete cross-reference on this page.
- **Failure and abolition of the sampling audit**: the 40-entry sample was drawn from the test pool
  with a fixed seed; after the fifth batch retired tests, 38 entries of the original sample no
  longer existed, and the audit’s `--check` consequently consistently reported “38 missing, 38
  extra”. This sampling audit was abolished and deleted in 0.6-19 (the script, data file, and the
  generator’s dependency on it were all removed); see “Abolition of the Sampling Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Sixth Retirement Batch (executed on 2026-11-26)

**Sixth batch: mechanism rollback in the 4.9.9 narrowing round—retire 28 tests deleted along with
their mechanism, while four prohibition criteria were never committed to the repository.** This
batch does not use the Three Gates to decide whether tests “should be kept”; instead, the mechanism
under test was **rolled back as a whole**. The criteria written in the first six rounds of 4.9.9 for
downstream ambiguity mechanisms (candidate-provider priority / candidate-signature comparison /
owner-compatibility inference / IR-side named-argument fallback / imported-name resolution-point
E5006 prohibition) were retired along with the mechanism—there is no mechanism, so the tests have no
subject under test. Such deletions are recorded as a “batch” according to coding-rules 7.3 and do
not enter the retirement candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 (28 deleted: 27 downstream-mechanism
    criteria from the first six rounds
    - 1 alias-import named-argument criterion). The two retained entries pin down the minimal fix
      for bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: all four E5006 prohibition criteria were retired
    with the file—this file existed only in an intermediate 4.9.9 commit, and neither the parent
    commit nor that version’s committed table included it; after the prohibition was withdrawn there
    was no subject under test, so this batch does not decrement the committed table.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the candidate-injection surface for alias imports was rolled back, so the fixture’s
    behavior no longer holds); the differential-baseline entry with the same name was deleted in the
    same batch. The baseline criteria are stated separately: **relative to the direct parent commit
    `ac810e97`, 405 → 404 lines** (the intermediate state had contained the fixture); **relative to
    the pre-4.9.9 baseline `08ccacc5`, 403 → 404 lines** (+1 base fixture
    `named-args-cross-unit-method`; the retired `alias-overload` never entered the final history).
- **Which gate applies**: the two retained entries satisfy the **failure-history gate** (they pin
  down a real defect registered in 4.9.9: cross-unit named arguments could not obtain
  declaration-time parameter names, leaving `E1014 available` empty). Among the 28 deleted entries,
  the mechanism criteria added in the first six rounds were mostly failure-history-gate entries, but
  the reason for deletion in this batch **is not** the Three Gates (they were not judged worthless);
  rather, the mechanism under test was rolled back.
- **Substitute coverage**: cross-unit named arguments with bare-name imports = the two retained
  driver-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity surface
  no longer has rejection logic (the requirement was rolled back), so there is no substitute
  criterion; the 4.9.8 dispatch-attribution criteria (10 driver + 5 integration) remain unchanged.
- **Restoration**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the empty-name-table guard for
  `method_param_info` is retained; this form is unreachable under the public syntax and is a
  defensive guard, so the unit test merely prevents accidental deletion).
- **Numbers** (after this batch was regenerated): repository-wide `#[test]` 2680 / 334 files;
  previous committed table 2688 → current tree 2680 (−8); the current tree has one entry absent from
  the old table, the old table has nine entries absent from the current tree, and no rows with the
  same key were recalculated.
- **Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit); `git revert <sha>` rolls back the whole batch.

## Abolition of the Sampling Audit (0.6-19)

**Criterion**: the sampling audit (0.6-10) measured the “true support rate of clause citations,”
whereas the retention rules after 0.6-18 no longer examine citations (they changed to “for the same
symbol and result category, keep only the strongest entry,” reducing the test pool from 3408 to
2055). Of the original 40-entry sample, 38 no longer exist, so the audit’s `--check` consistently
reported “38 missing, 38 extra”—keeping it would only produce continuous alarms. It was therefore
**abolished and deleted**.

**What was deleted**: the sampling audit script under `scripts/dev/` (the one that reproduced the
40-entry sample with a fixed seed) and its JSON data file (the 40 item-by-item adjudications and
bases).

**The generator was synchronized to remove the dependency**: the generator previously read that JSON
and classified “confirmed by the audit as genuinely supporting” as “Clause gate + Keep”; that branch
has been deleted—**clause citations no longer have an “audit-confirmed” tier** and all fall under
“Clause / Pending verification”.

**Distribution change** (not one of the repository-wide 2055 `#[test]` entries changed; the TSV was
regenerated): Keep 746 → 737 (−9: the nine entries confirmed by the audit as genuinely supporting
fell back to pending verification); Pending verification 1045 → 1054; by gate: Clause 811 → 808,
Pending adjudication 496 → 499 (all others unchanged: Failure history 313 · Proof 424 · Fails all
gates 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` rolls back the whole batch.

## Rerunning and Self-Checks

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (the two outputs are byte-for-byte identical)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical adjudication only, and states its boundaries honestly:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name lacks these prefixes, it is misclassified as “no
  assertions”—the first batch’s 26 entries (21 retired / 5 kept) and the second batch’s 18 entries
  (11 retired / 7 kept) were all manually reviewed function body by function body.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator obtains a function body, it
  balances braces; braces in strings/character literals and comments throw off the balance and
  truncate the function body. `test_prefix_info_lbrace` (whose body contains `tokenize("{")`) once
  ran to the end of the file, and the “Assertion summary” captured the remainder of the file,
  causing it to be binned as “Pending adjudication” rather than “Retirement candidate”. It now masks
  non-code regions (strings/character literals/line comments/block comments/raw strings) before
  balancing the braces; see “Generator Correction” below.
- **Recognition of “delegated assertions” (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not include digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions, and tests containing such calls
  were misclassified as “not a single assertion in the body”. At the time, 4 of the repository-wide
  15 “Fails all gates” entries in fact contained assertions (3 numeric helpers + 1 `match` +
  `panic!` form). The name portion has now been relaxed to `[A-Za-z0-9_]*`, and recognition of the
  `match` + `panic!` form has been added; see “Generator Criterion Correction” below.
- **The proof gate does not adjudicate**: every test that “fails both gates” falls under “Pending
  adjudication” or “Retirement candidate”; the generator never declares that it “should be deleted”.
- **No cross-file deduplication**: exact duplicate tests are not identified in this table (0.6-2
  measured only 1%, which does not constitute a retirement surface).
- **Clause citations are always pending verification**: a verifiable section number in the file
  header only means that the test “corresponds to a clause that can be identified by name”; it does
  not mean that the citation supports the assertions. The machine does not adjudicate whether it
  does, so even a clause-gate hit stops at “Pending verification” and must be verified by a human
  before the test can be kept.
