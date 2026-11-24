---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement candidate list plus per-batch retirement records'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **three gates (clause / history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a rerunnable, row-by-row adjudication
table. A sample audit of the clause gate (40 cases) once measured the proportion of cases that
“cited a clause but could not support the assertion” and **tightened the criteria** accordingly;
that audit was repealed and deleted in 0.6-19. **This page and the generator itself only read tests
and never modify them**; test retirement is executed in batches (one commit per batch, with the
whole batch revertible via `git revert`), and execution results are recorded in the batch retirement
subsections and commit messages below.

Generated artifacts:

| Artifact                            | Purpose                                                                                       |
| ----------------------------------- | --------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Row-by-row adjudication table (2646 rows plus the header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                                   |

## Criteria

### Scanning Surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module's `tests/` subdirectory) | 2044                 |
| `tests/integration`                                                      | 461                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2646** (328 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Meaning of a hit                                             |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| Clause gate (tightened in 0.6-10) | The first 25 lines of the file contain a **verifiable reference**: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); files under `src/**/tests/` are inspected together with the parent `mod.rs`                                                                                                                                                                                                                                              | The test maps to an identifiable clause                      |
| History gate                      | The test body, function doc comment, or **file header** (such as `//! 回归背景：issue #226`) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                                                         | The test pins down an incident that actually occurred        |
| Proof gate                        | **Cannot be determined by machine** (“If it were replaced with another correct implementation, would it turn red?” requires human judgment). Therefore, anything that fails the first two gates is classified as “pending judgment” and is never automatically classified as “neither gate passes”; the sole exception is a test with **not a single assertion in its body** (including no delegated `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | The result can only be “pending judgment” or “no assertions” |

Reason for tightening: the 0.6-9 clause gate only required the file header to **contain the strings
`§`/`RFC`/`SPEC`**, so 2403 entries were adjudicated as “Keep”. The 40-case stratified sample
measured in 0.6-10 found that **half of the citations could not support the assertions**
(mis-citation rate: 50%, 95% interval: 35%–65%)—this measurement was the **direct reason for
abandoning the practice of “keeping a test based on a single citation.”** After tightening, only the
section-number form is recognized, and **having a citation still does not mean that the citation
supports the assertion**—all clause references are classified only as “pending verification.” (This
sample audit was repealed and deleted in 0.6-19: as the test pool shrank through retirement, 38
entries from the original sample no longer existed, and `--check` always reported “38 missing, 38
extra”; the conclusion is retained, but the audit artifact is no longer maintained.)

Reason for the file-header radius: this repository's convention is to write “regression background:
issue #N” in the file header rather than repeat it in each test comment. Looking only at each test's
own comments would miss the history of the entire file (measured: 7 entries within the test's own
radius, 291 within the file-header radius).

### Categories That Are Never Retired

The following tests are **always “Keep”** and do not enter the retirement candidate pool:

- Corpus tests under `tests/yaoxiang/**` and project-form tests under `tests/project-corpus/**`
  (behavioral-criteria surfaces);
- Tests under `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs`
  (spec-shaped and operating across process boundaries);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Meaning of Each Disposition

| Disposition          | Meaning                                                                                                                                                                                                 |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Only two reasons are given: a history-gate hit, or the proof gate (spec-shaped directories / current-state anchor names)                                                                                |
| Pending verification | A clause is cited (either with a section number or only an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a person before it can be kept |
| Retirement candidate | Both gates fail and the assertion surface contains only weak assertions (or no assertions)—a candidate is not a verdict; the proof gate still requires manual review                                    |
| Pending judgment     | Both gates fail, but strong assertions are present, so the machine reaches no conclusion                                                                                                                |

## Distribution (Regenerated 2026-11-24)

| Gate                                                                        | Count    |
| --------------------------------------------------------------------------- | -------- |
| Clause (has a verifiable reference)                                         | 997      |
| History                                                                     | 539      |
| Proof (spec-shaped criteria surface / current-state anchor)                 | 568      |
| Pending judgment (both initial gates fail; proof gate awaits manual review) | 536      |
| Neither gate passes (no assertions in the body)                             | 6        |
| **Total**                                                                   | **2646** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1107** | 737 (+370)                        |
| Pending verification | 1259     | 1054 (+205)                       |
| Pending judgment     | 222      | 203 (+19)                         |
| Retirement candidate | 58       | 61 (−3)                           |
| **Total**            | **2646** | 2055 (+591)                       |

Keep structure: 1107 = 568 proof-gate entries (516 spec-shaped criteria surfaces + 52 current-state
anchors) + 539 history-gate entries. File-level count: among the 328 test files, **160** have a
verifiable section number in the file header.

Reconciliation: the old table had 2055 entries, while the current tree has 2646; there are 639 keys
present in the current tree but absent from the old table, including 25 added in the current round
under 8.8, 606 pre-existing entries that had not previously been regenerated, and 8 new
`grammar_probe` entries from the 8.3 go/no-go probe; there are 48 keys present in the old table but
absent from the current tree. The per-key records for the 25 new tests, 606 pre-existing entries,
and 8 probes are all in the TSV; the per-entry destinations of the 48 stale keys under “deleted /
renamed / relocated” are documented in
[09a §P8 8.3 batch-entry prerequisite](./architecture/09a-wbs-history.md). Another 46 entries with
identical keys had their row contents recomputed against the current source; these are counted as
neither additions nor deletions.

### Two Criteria Changes at the 0.6-19 Tightening Point

1. **Keep structure**: 737 = 424 proof-gate entries (390 spec-shaped criteria surfaces + 34
   current-state anchors) + 313 history-gate entries. Of the original 2897 entries, the 2121 that
   were “kept based on a single citation” are now pending verification. After the sample audit was
   repealed in 0.6-19, another 9 entries that had previously been classified as “Keep because the
   audit confirmed genuine support” were also moved to pending verification—clause references no
   longer have an “audit-confirmed” category.
2. **History gate 47 → 313**: these are not newly discovered histories but **relabelled entries**.
   In 0.6-9, an entry that matched both the clause and history gates was recorded under the clause
   gate. After tightening, a clause hit no longer automatically resulted in “Keep,” so the history
   gate had to catch these entries; their counts were therefore folded into the history total. This
   did not affect the total number kept (this batch of tests was also kept in 0.6-9).

The file-level criteria at that time: among 285 test files, **136** had a verifiable section number
in the file header (the old 0.6-9 criterion counted 221).

## Reconciliation Between the 0.6-19 Snapshot and the 0.6-2 Survey

The 0.6-2 survey numbers are the first-version comparison baseline. The reasons for each difference
are listed below:

| Item                            | 0.6-2 Survey                  | This Census                                                                                                       | Reason for the difference                                                                                                                                                                                                                                                                                    |
| ------------------------------- | ----------------------------- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                              | 2 entries were added after 0.6-2 (the check-criterion regression tests for E3005 and E3022); 21 no-assertion tests were then deleted from the first-batch list, 11 weak-assertion tests from the second-batch list, 2 from the third batch, 1 from the fourth batch, and 1353 from the fifth batch; net −388 |
| Number of test files            | 296                           | 285                                                                                                               | 11 entire files were retired in the fifth batch                                                                                                                                                                                                                                                              |
| Clause gate                     | 227 files (77%)               | After tightening: 136 files; 808 entries                                                                          | The criterion was tightened from “the string appears” to “there is a section number `§<number>`,” nearly halving the file count; the fifth batch retired 902 entries, and another 9 moved from “Keep” to pending verification after the sample audit was repealed in 0.6-19                                  |
| History signals                 | 442 entries / 148 files       | 313 entries within the gate; three radii: test itself 7 / file header 291 (38 files) / whole file 1218 (79 files) | The 148 files in the 0.6-2 survey were broader than the current whole-file radius of 79 files, and its vocabulary and scope were larger; this census strictly uses the four signals in the gate definition                                                                                                   |
| Strong characterization signals | 1176 (34%) / strong 631 (18%) | Not evaluated                                                                                                     | Characterization is not one of the three gates; mixing it into retirement conclusions would exceed the gate's authority                                                                                                                                                                                      |
| Weak assertions                 | 473 (14%)                     | See the TSV “assertion summary” column                                                                            | The counting units differ: 0.6-2 counted patterns that only asserted `is_ok`/`is_some`; this census uses lexical counts and distinguishes strong, weak, and delegated assertions                                                                                                                             |

## Summary of the 0.6-19 Conclusions (Historical)

1. **The clause gate had previously been substantially overstated**: under the old criterion, 2403
   entries were classified as “clause/Keep,” while the sample measured a 50% mis-citation rate (95%
   interval 35%–65%)—“citing a rule” and “having a citation that supports the assertion” are
   different things, and the old criterion treated the former as the latter. (This sample audit was
   repealed and deleted in 0.6-19; its conclusion is retained. See “Repeal of the Sample Audit”
   below for the reason for deletion.)
2. **After tightening, approximately 1202 entries genuinely had clause support** (range 846–1557);
   the remainder must either obtain pending verification or should have been adjudicated through the
   history gate / proof gate.
3. **The retirement surface was unrelated to the clause mis-citation surface**: the 136 candidates
   were unrelated to clauses (no assertions / weak assertions); the first batch retired 21 entries,
   the second batch retired 11, the generator correction was +1, the third batch was −2, the 0.6-15
   criterion correction was −3, the redundancy scan was −1, the fourth batch was −1, and the fifth
   batch was −1353 (bulk retirement under the mechanical rule “retain only the strongest entry for
   the same symbol and result category”), reducing the candidates to 61. The fifth batch's
   retirement count by gate was: Clause 902 · Pending judgment 428 · Proof 23 · History 0 · Neither
   gate passes 0—**not one history-gate or zero-assertion-line test was removed**.
4. **The history gate was the largest legitimate source of “Keep”** (313 entries): it demonstrates
   that “pinning down a real incident” is more reliable than “copying a sentence from an RFC.”

## First Retirement Batch (Executed 2026-11-06)

**First batch: of the 26 “no assertions” tests under `src/frontend/core/types/eval/tests/`, 21 were
retired and 5 were kept** (a single directory cluster; criterion = **all three gates in Part Seven
of coding-rules fail**: no clause gate, no history gate, and not a single assertion in the body).
The row-by-row adjudication and “same-cell coverage” are provided by the 0.6-11a adjudication table
(21 deleted / 5 kept); this page does not repeat the reasons and records only the execution result.

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
- **5 kept** (the adjudication table could not identify replacement coverage, so they were kept
  under the hard requirement): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` reverts the whole batch.

## Second Retirement Batch (Executed 2026-11-06)

**Second batch: of the 18 “weak assertions” tests in `src/frontend/core/parser/pratt/tests/nud.rs`,
11 were retired and 7 were kept** (a single directory cluster = the Pratt prefix-parsing module;
criterion = **all three gates in Part Seven of coding-rules fail**: no clause gate, no history gate,
and the body contains only weak assertions such as `is_some()` / `is_none()`—neither binding power
nor the parse result is pinned down).

The difference between this batch and the first must be made explicit: these tests **do contain
assertions** and are not empty tests. The reason for retirement is “all three gates fail + zero
criterion increment,” not “no assertions”—every test slated for deletion identifies a **unit-level
replacement coverage test in the same module** by name.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The replacement
  coverage for every entry follows the **same** `parse_expression → parse_prefix → prefix_info` path
  and makes stronger assertions (pinning the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, plus `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the commit message for the per-test reasons (task 0.6, second
  test-retirement batch). The reasons fall into two categories: **the proof gate holds** (the first
  three, plus `test_prefix_info_at` and `test_prefix_info_eof`, are the sole unit-level walkthrough
  points for their corresponding facts, and no second unit test in the repository observes them),
  and **the history gate holds** (`break_removed` / `continue_removed` name #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` reverts the whole batch.

## Generator Correction (0.6-13)

**`body_span` truncates the function body when it encounters braces inside literals — fixed.**

The old implementation found function bodies by balancing braces without distinguishing real code
from braces in string / character literals or comments. Forms such as `tokenize("{")` are common in
Rust source, so the balance was thrown off and the function body was truncated. The body of
`test_prefix_info_lbrace` was previously truncated all the way to the end of the file, and the
“assertion summary” picked up the rest of the file (`matchesx11 assertx24 …`), causing it to be
classified as “pending judgment” rather than “retirement candidate”—**the wrong bucket**.

Fix: added `mask_noncode`, which first replaces the contents of string literals / character literals
/ line comments / block comments (which may be nested) / raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (newlines are preserved exactly, so line numbers and line counts do not
change), and then balances the braces. **The mask is used only to locate function-body boundaries**;
assertion statistics and the history gate still read the original source, and the criteria
themselves have not been relaxed. `--selftest` gained 4 cases (paired / unpaired braces in ordinary
strings, braces in raw strings and block comments, and braces in character literals), each followed
by a sentinel test—the old implementation found the wrong boundary in all 4 cases, while the new
implementation finds them all correctly.

### Tests Affected by the Same Bug: 11 Entries

Counting method: compare the **function-body boundaries before and after the fix** for every test in
every file (the old implementation balanced literals directly, while the new implementation balances
after masking); any entry with different boundaries is affected. Of the 3413 tests in the
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

### Bucket Differences Before and After the Fix: 2 Entries

Of the other 9 entries, **8 affected only the “assertion summary” column** (the bucket did not
change), and **1 did not change even the summary** (`test_fstring_close_brace_escape_preserved_raw`:
the old implementation omitted the closing `}` line, and that line contained no assertion keyword).
The 8 summary changes are documented individually in the commit message.

| Test Name                   | File                                          | Old Bucket                          | New Bucket                                  | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| --------------------------- | --------------------------------------------- | ----------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending judgment / Pending judgment | Pending judgment / **Retirement candidate** | The real function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; the file header and body contain no section number and no issue/regression/defect/incident ⇒ both the clause gate and history gate fail, and the body contains only a weak assertion such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires manual review)                                                                        |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | History / Keep                      | Clause / **Pending verification**           | The old implementation truncated the function body to the end of the file and swallowed text containing “regression / defect / incident,” causing a false history-gate hit; the real function body contains none of these terms ⇒ the history gate no longer holds; the file header contains verifiable references (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate hits, but the reference has not been confirmed by the sample audit ⇒ pending verification |

The distribution changes accordingly: Clause 1713 → 1714 · History 314 → 313 · Keep 776 → 775 ·
Pending verification 2121 → 2122 · Pending judgment 412 → 411 · Retirement candidate 104 → 105.
**The repository-wide `#[test]` count is unchanged (3413)**—this round changed only the
implementation of the criteria, not any test.

## Third Retirement Batch (Executed 2026-11-07)

**Third batch: 5 retirement candidates under `src/middle/passes/codegen/tests/` were adjudicated
individually; 2 were retired and 3 were kept.** (a single directory cluster = one module under test,
`middle::passes::codegen`; criterion = **all three gates in Part Seven of coding-rules fail**.)

This batch retains the **keep criteria** established by the previous batch: a unit-level observation
point may only be replaced by another **unit test**; corpus / integration tests operate at the
process-boundary layer and cannot replace a unit-level observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is **verbatim
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (apart from
    comments, only the function name differs): both use `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that the `execute_module` entry
    path returns a non-Void value, but the body never observes the return value from
    `execute_module` (`run_fixture` discards it via `.expect(...)`)— the claimed intent was not
    implemented, and the body is a pure duplicate ⇒ zero criterion increment. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file strictly covers it:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)`, identical in shape to the inline `Symbol` in this
    entry). “A name that was never inserted cannot be found” and “it cannot be found after being
    popped” follow the same path in `SymbolScopeManager::lookup` (walk each scope with
    `scope.get(name)`, returning `None` when not found) ⇒ this entry adds zero increment.
    Replacement coverage = `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that
    “executes directly without serialization” (the serialized variant is a different path), and
    `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for a
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit-level observation point for the
    `id > 255` rejection branch of `OperandResolver::to_reg` (the bytecode layer has independent
    overflow checks that do not cover this function; no second test in the repository pushes
    `to_reg` out of range). `is_err()` is a discriminating assertion (removing the guard would make
    the test fail), not merely a “proof that it did not panic”.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` reverts the whole batch.

### Census Blind Spot Found in This Batch (**Fixed in 0.6-15**, see “Generator Criterion Correction” below)

The census's “delegated assertion” recognizer, `\b(assert|check)[A-Za-z_]*\s*\(`, **could not match
helper names containing digits**: `[A-Za-z_]*` does not include digits, so calls such as
`assert_slot_is_42(&interp)` were not recognized as assertions— and 2 of the 5 candidates in this
batch were consequently misclassified as “not a single assertion in the body”.

After all 15 repository-wide “Neither gate passes” entries were reviewed individually (by allowing
digits in helper names and additionally recognizing the `match` + `panic!` form), **4 did in fact
contain assertions**—3 used the numeric helper above (`assert_slot_is_42`), and 1 was
`test_weak_from_arc` in `src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form;
test-specification rule 6.1 requires `assert!(matches!(...))`, but this test does assert). The other
11 genuinely contained no assertions at all.

**If these 4 had been directly removed under the machine's “Neither gate passes” classification,
tests with criteria would have been deleted by mistake**—individual human adjudication in this batch
prevented that; the generator-side fix is described under “Generator Criterion Correction (0.6-15)”
below.

## Generator Criterion Correction (0.6-15)

**Two criteria blind spots were fixed: helper names containing digits, and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411) and
changes only how the generator determines whether a body contains assertions.

### What Was Fixed

1. **Digits were allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so any test containing one was
   treated as “not a single assertion in the body”.
2. **Added recognition of the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the
   assertion is written in one arm, so `assert!` is not visible lexically (test-specification rule
   6.1 requires `assert!(matches!(...))`, but existing code contains this form). Recognition method:
   scan the expression after the `match` keyword to the first `{` to identify the block it opens,
   determine the block boundary by **brace balancing**, and count it as an assertion only if
   `panic!(` occurs inside. **The judgment is performed on masked lines**— braces in strings and
   comments do not participate in balancing, and a `panic!` in a comment is not treated as an
   assertion.

### Bucket Differences: 3 Entries (Gate and Disposition Changed)

| Test Name                                           | File                                           | Old Bucket                                 | New Bucket                          | Basis                                                                                                                                                                                                                                                         |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------------ | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither gate passes / Retirement candidate | Pending judgment / Pending judgment | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but this form was not previously recognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither gate passes / Retirement candidate | Pending judgment / Pending judgment | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit and the old pattern could not match it; it is now recognized as an assertion                                                                                      |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither gate passes / Retirement candidate | Pending judgment / Pending judgment | Same as above (the serialization-round-trip entry)                                                                                                                                                                                                            |

Another **77 entries changed only the “assertion summary” column** (the gate and disposition
remained unchanged): most gained `match_panicxN` in the summary (the body contains a `match` +
`panic!` form), and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3 entries;
`src/package/tests/cache.rs`: 1 entry—helper names containing digits).

**The relaxation changes only the “Neither gate passes” category**: that category is the machine's
**confident verdict** (not a single assertion in the body), and its criterion is a hard requirement;
a “retirement candidate” is only a pool awaiting manual review, so the size of that pool is not a
verdict. All other buckets, including the weak-assertion pool, retain their original criteria.
Result: Gate—Neither gate passes 14 → 11 · Pending judgment 923 → 926; Disposition—Retirement
candidate 103 → 100 · Pending judgment 411 → 414.

### Existing “Neither Gate Passes” Entries: Individual Review Conclusions

After all 11 repository-wide “Neither gate passes” entries were read and reviewed, **all 11
genuinely contained no assertions at all** (including no `assert*` / `check*` helpers, no `match` +
`panic!` form, and no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                     | Review Conclusion                                            |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 calls to `.expect(...)`, which only guarantee that it does not crash                          | Genuinely no assertions (only proves that it does not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                  | Genuinely no assertions                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them              | Genuinely no assertions                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Genuinely no assertions                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained alias `reduce` and discards the result                                       | Genuinely no assertions                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential alias `reduce` and discards the result                              | Genuinely no assertions                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` with invalid content and discards the result                      | Genuinely no assertions (only proves that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Genuinely no assertions (only proves that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Genuinely no assertions (only proves that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                            | Genuinely no assertions                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertions                                      |

Of these, 5 (`test_normalizer_context` and the 4 entries in reducer.rs) were classified as **Keep**
in the 0.6-11a adjudication table because **no replacement coverage could be identified**; the
conclusions of this review agree with that table.

### Reverse Spot Check: Did the Relaxation Go Too Far?

Five entries were sampled with a fixed seed (20261107) from the 88 entries “newly recognized as
assertions” and read in their original form to confirm that they really assert:

| Spot-Check Entry                                            | File                                                 | Assertion in the Original Source                                                                                                            |
| ----------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching `match` arm; `panic!("Expected AssocType")` when it does not match                                   |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm; `panic!("expected InvalidPackage, got …")` when it does not match               |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | An `assert_eq!` comparing constraint types in the matching arm; `panic!("不是 Dynamic 变体…")` when it does not match                       |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm; `panic!("Expected TokensDelta variant")` when it does not match                     |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` checks of the error code and message in the matching arm; `panic!("不是 Disproved…")` when it does not match |

All 5 assert concrete values or concrete variants (replacing the implementation with another correct
path would make them fail)—**the relaxation did not go too far**. The reverse controls were also
added to `--selftest`: a `match` without `panic!`, and a `panic!` that appears only in a comment;
neither may be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

When the generator was reverted to the old implementation (`HELPER_RE` did not allow digits +
recognition of `match` + `panic!` was disabled), all the newly added `--selftest` cases **failed**,
with exit code 1; the new implementation passes them all.

### Rollback

The generator and census table are in the same commit, so `git revert <sha>` restores the whole
state.

## Fourth Retirement Batch (Executed 2026-11-07)

**Fourth batch: of the 10 “retirement candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend's test directories), 1 was retired and 9 were
kept** (a single directory cluster; all three files were read together before adjudication:
`tests/bytecode_load.rs`: 4 entries · `tests/weak.rs`: 2 entries · `executor/tests/debug.rs`: 4
entries). Criterion = the **three gates** in Part Seven of coding-rules; keep criteria = “a
unit-level observation point may only be replaced by another unit test; corpus/integration tests
operate at the process-boundary layer”; **deletion must identify a concrete replacement coverage
test by name; if none can be identified or the test cannot be understood, keep it**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping` plus one step before
  obtaining `capture_stack`), and its assertions are **strictly stronger**: it asserts that the
  stack is non-empty and that `stack[0].function_name == "main"` (pinning the top-of-stack
  position), whereas the listed test only asserts `stack.iter().any(|f| f.function_name == "main")`
  (any position is sufficient) —the former being true implies the latter ⇒ zero criterion increment.
- **9 kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each in its original
  form, each is the **sole unit-level observation point** for some property (the two distinct
  branches for load magic / version, the YXBC routing layer, both sides of Weak Some/None, step with
  no frames, step_out popping the stack, and run-to-completion leaving the stack empty), and no
  replacement unit test could be identified ⇒ keep under the hard requirement. See the commit
  message for the per-entry basis (the adjudication table is not included on this page).
- **Numbers**: repository-wide `#[test]` 3409 → 3408; 1 deleted = `git diff` counted `-#[test]` on 1
  line and `+#[test]` on 0 lines; the dominant test `test_capture_stack_during_execution` passed in
  isolation. Census: retirement candidate 99 → 98 · pending judgment 925 → 924 · Clause 1713 ·
  History 313 · Proof 447 · Neither gate passes 11 · Pending verification 2121 · Keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` reverts the whole batch.

## Fifth Retirement Batch (Executed 2026-11-07)

**Fifth batch: 1353 unit tests were retired in bulk under the mechanical rule “for the same symbol
and result category, retain only the strongest entry”** (only unit tests under `src/**/tests/**`
were changed; not a single test on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (mechanically executed, without individual argument)**: for the same symbol under test +
  the same assertion-result category, retain only the entry with the strongest assertion and delete
  the rest.
  - Symbol under test = a repository symbol invoked in the test body (including one level of
    delegation through a helper function in the same file); when multiple symbols are reached,
    classify by the **primary symbol** (the first repository call). Symbols are qualified by
    repository path and are not merged across modules by bare name.
  - Result category = a concrete value / a particular error code (E code) / a particular variant
    match / only `ok` / only `err` / only `should_panic`.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or `match` + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when strength is equal within a group,
    retain the first entry in the file.
  - If a test asserts multiple categories (making the primary category indeterminate), **retain it**
    and do not guess.
- **Three exceptions (not one was removed)**: ① the name or comment names `issue #` / regression /
  defect / incident (history gate); ② a symbol has only one test (deleting it would leave the symbol
  without a unit-level observation point); ③ `#[should_panic]` is the symbol's only should_panic
  test. History-gate entries 313 and “Neither gate passes” entries 11 had **0 retirements**.
- **Numbers**: repository-wide `#[test]` 3408 → 2055; 1353 deleted = `git diff` counted `-#[test]`
  on 1353 lines and `+#[test]` on 0 lines. Census (by gate, current values after the sample audit
  was repealed in 0.6-19): Clause 808 · History 313 · Proof 424 · Pending judgment 499 · Neither
  gate passes 11. Census (by disposition, current values): Keep 737 · Pending verification 1054 ·
  Pending judgment 203 · Retirement candidate 61. The retirement count for this batch by gate was:
  Clause 902 · Pending judgment 428 · Proof 23 (in 0.6-19, 9 entries that had been “confirmed by the
  audit as genuinely supported” were reclassified as pending verification, so the current
  gate/disposition values differ from those at the time the fifth batch closed).
- **Cleanup alongside deletion**: helper functions used only by deleted tests, imports that became
  unused, and fields that became obsolete were removed as well
  (`cargo clippy --all --all-features --all-targets -- -D warnings` produced 0 warnings).
- **Two out-of-bounds deletions requiring a record**: there were 29 entries that the census
  originally classified as “Keep” but that this batch's rule still deleted—23 current-state anchors
  (names containing unchanged/anchor/guard/preserve/baseline, many of which were false positives on
  **domain words** such as “guard” or “preserves,” for example `test_while_guard_scope_balanced`)
  and 6 entries whose clauses were confirmed by the audit as genuinely supported. This batch
  followed the closed list of three exceptions and did not provide additional protection for these
  two categories; to restore them, fill them back in using the keep/delete correspondence table on
  this page.
- **Failure and repeal of the sample audit**: the 40-case sample was drawn from the test pool with a
  fixed seed, and after the fifth batch 38 entries from the original sample no longer existed, so
  the audit's `--check` always reported “38 missing, 38 extra.” The sample audit was repealed and
  deleted in 0.6-19 (the script, data file, and the generator's dependency on them were all
  removed); see “Repeal of the Sample Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` reverts the whole batch.

## Repeal of the Sample Audit (0.6-19)

**Criterion**: the sample audit (0.6-10) measured the “true support rate of clause references”, but
after 0.6-18 the retention rule no longer examines references (it changed to “for the same symbol
and result category, retain only the strongest entry”, reducing the test pool from 3408 to 2055). Of
the original 40 samples, 38 no longer existed, and the audit's `--check` always reported “38
missing, 38 extra”—keeping it would only produce continuous alarms. Therefore, it was **repealed and
deleted**.

**What was deleted**: the sample-audit script under `scripts/dev/` (the one that reproduced the
40-case sample with a fixed seed) and its JSON data file (40 per-entry decisions and bases).

**The generator's dependency was removed in sync**: the generator previously read that JSON and
classified “audit-confirmed genuine support” as “clause gate + Keep”; that branch has been
deleted—**clause references no longer have an “audit-confirmed” category** and are always classified
as “Clause / Pending verification”.

**Distribution changes** (not one of the repository-wide 2055 `#[test]` entries changed; the TSV was
regenerated): Keep 746 → 737 (−9: the 9 entries confirmed by the audit as genuinely supported moved
back to pending verification); Pending verification 1045 → 1054; by gate: Clause 811 → 808, Pending
judgment 496 → 499 (the others remained unchanged: History 313 · Proof 424 · Neither gate passes
11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit), so `git revert <sha>` reverts the whole batch.

## Rerun and Self-Test

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical judgments only, and its boundaries are stated faithfully:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test's assertion is hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as “no assertions”—the first batch of 26 entries (21 retired / 5 kept) and the
  second batch of 18 entries (11 retired / 7 kept) were all manually reviewed function body by
  function body.
- **Function-body boundaries (fixed in 0.6-13)**: the generator locates function bodies by balancing
  braces, and braces in string / character literals or comments can throw off the balance,
  truncating the function body. The body of `test_prefix_info_lbrace` (which contains
  `tokenize("{")`) was previously truncated to the end of the file, causing the “assertion summary”
  to pick up the rest of the file and the entry to be classified as “pending judgment” rather than
  “retirement candidate”. It now masks non-code regions (strings / character literals / line
  comments / block comments / raw strings) before balancing; see “Generator Correction” below.
- **“Delegated assertion” recognition (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not allow digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions, and any test containing such a
  call was misclassified as “not a single assertion in the body”. At the time, 4 of the repository's
  15 “Neither gate passes” entries actually contained assertions (3 numeric helpers + 1 `match` +
  `panic!` form). The name portion has now been relaxed to `[A-Za-z0-9_]*`, and recognition of the
  `match` + `panic!` form has been added; see “Generator Criterion Correction” below.
- **The proof gate is not adjudicated**: every test that fails the first two gates is classified
  only as “pending judgment” or “retirement candidate”; the generator never declares that a test
  “should be deleted”.
- **No cross-file deduplication**: exactly duplicate tests are not identified in this table (the
  0.6-2 survey measured only 1%, which does not constitute a retirement surface).
- **All clause references require verification**: a verifiable section number in the file header
  only means that the test “maps to an identifiable clause”; it does not mean that the citation
  supports the assertion. The machine does not judge whether it does, so even a clause hit is
  classified only as “pending verification” and must be verified by a person before the test can be
  kept.
