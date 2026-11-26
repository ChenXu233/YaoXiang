---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement-candidate list and retirement records for each batch'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **Three Gates (Clause / Incident History / Proof)** in
[Part Seven of coding-rules](./coding-rules.md), producing a rerunnable row-by-row decision table.
The sampling audit of the clause gate (40 cases) once measured the proportion of cases that “cited a
clause but whose assertions were not supported by it” and thereby **tightened the criteria**; this
audit was repealed and deleted in 0.6-19. **This page and the generator itself only read tests and
never modify them**; test retirement is carried out in batches (one commit per batch, revertible as
a whole with `git revert`), and execution results are recorded in the per-batch retirement sections
below and in commit messages.

Generated artifacts:

| Artifact                            | Purpose                                                                            |
| ----------------------------------- | ---------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Row-by-row decision table (2688 rows + header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                        |

## Criteria

### Scan Surface

Every `#[test]` in all `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 2084                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2688** (335 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine Criterion                                                                                                                                                                                                                                                                                                                                                                                                       | Meaning of a Hit                                           |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | A **verifiable clause** appears within the first 25 lines of the file header: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); for files under `src/**/tests/`, inspect the parent `mod.rs` together                                                                                                                                                                          | The test maps to an identifiable clause                    |
| Incident-history gate             | The test function body, function doc comment, or **file header** (such as `//! Regression context: issue #226`) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                       | The test pins an incident that has actually occurred       |
| Proof gate                        | **Cannot be machine-adjudicated** (“Would it fail with another correct alternative implementation?” requires human judgment). Therefore, every test that fails the first two gates automatically falls into “Pending adjudication,” not “Neither”; the only exception is a body with **no assertions at all** (including no delegated `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Can only produce “Pending adjudication” or “No assertions” |

Rationale for tightening: The 0.6-9 clause gate only required the file header to **contain
`§`/`RFC`/`SPEC`**, so 2403 entries were classified as “Keep.” The 40-case stratified sample in
0.6-10 found that **half of the citations did not support the assertions** (mis-citation rate: 50%,
95% interval: 35%–65%)—this measurement was the **direct reason for abandoning “Keep based on a
citation alone.”** After tightening, only section-number forms are accepted, and **having a citation
still does not mean that the citation supports the assertions**—clause citations always stop at
“Pending verification.” (This sampling audit was repealed and deleted in 0.6-19: as the
test-retirement pool shrank, 38 of the original samples no longer existed, so `--check` always
reported “38 missing, 38 extra”; the conclusion is retained, but the audit artifact is no longer
maintained.)

Rationale for the file-header radius: This repository’s convention is to put “Regression context:
issue #N” in the file header and not repeat it in individual test comments; looking only at each
test’s own comments would miss the history of entire files (measured: 7 entries at the test’s own
radius, 291 entries at the file-header radius).

### Categories That Are Never Retired

The following tests are **always classified as “Keep”** and do not enter the retirement-candidate
pool:

- The corpus `tests/yaoxiang/**` and project-form `tests/project-corpus/**` (behavior-criterion
  surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and crossing the process boundary);
- Current-state anchors in test names containing `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                               |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Only two cases: the incident-history gate is hit, or the proof gate applies (spec-shaped directory / current-state anchor name)                                                                       |
| Pending verification | Cites a clause (with a section number, or citing an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a human before the test can be kept |
| Retirement candidate | Both gates fail and the assertion surface contains only weak assertions (or no assertions)—a candidate is not a verdict; the proof gate still requires manual review                                  |
| Pending adjudication | Both gates fail, but strong assertions are present; the machine reaches no conclusion                                                                                                                 |

## Distribution (Regenerated 2026-11-27; 4.9.9 Narrowing Round, Batch 8.7, and 4.9.10-L0 Re-verification)

| Gate                                                               | Count    |
| ------------------------------------------------------------------ | -------- |
| Clause (has a verifiable clause)                                   | 1030     |
| Incident history                                                   | 559      |
| Proof (spec-shaped criterion surface / current-state anchor)       | 571      |
| Pending adjudication (both gates fail; proof pending human review) | 522      |
| Neither (no assertions in the body)                                | 6        |
| **Total**                                                          | **2688** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1130** | 737 (+393)                        |
| Pending verification | 1277     | 1054 (+223)                       |
| Pending adjudication | 223      | 203 (+20)                         |
| Retirement candidate | 58       | 61 (−3)                           |
| **Total**            | **2688** | 2055 (+633)                       |

Keep breakdown: 1130 = proof gate 571 (spec-shaped criterion surface 518 + current-state
anchors 53) + incident-history gate 559. File-level metric: Among 335 test files, the **clause
gate** covers **123**; **166** files have a detectable verifiable section number within the first 25
lines of the file header (`src/**/tests*/`, including the parent `mod.rs`).

Regeneration reconciliation: The previous committed table had 2682 entries, while the current tree
has 2688. The current tree has 13 entries absent from the old table (6 `DefPath` projection criteria
from 4.9.10-L0 + 7 tests moved out of inline tests in `symbol.rs` and renamed), while the old table
has 7 entries absent from the current tree (from the same migration; see `module/tests/symbol.rs`).
No same-key rows were recomputed with changed content. L0 also had one entry retired before it was
added to the table (`test_def_path_disambiguator_reserved_and_not_projected`): under coding-rules
7.1/7.3, it characterizes the projection choice frozen for this round and will inevitably fail once
L1 introduces a disambiguating suffix, so this table does not contain that row. Per-key records are
in the TSV; the destinations of stale keys are in this batch’s commit message.

### Two Criteria Changes at 0.6-19

1. **Keep breakdown**: 737 = proof gate 424 (spec-shaped criterion surface 390 + current-state
   anchors 34) + incident-history gate 313. Of the original 2897 entries, the 2121 previously
   classified as “Keep based on a citation alone” now fall under pending verification (after the
   sampling audit was repealed in 0.6-19, the 9 previously classified as “audit confirmed true
   support” also fall under pending verification—clause citations no longer have an
   “audit-confirmed” tier).
2. **Incident-history gate 47 → 313**: This was not an increase in incident history, but
   **relabeling**—in 0.6-9, entries where both the clause and incident-history gates were hit were
   recorded under the clause gate. After tightening, a clause hit no longer automatically means
   “Keep,” so the incident-history gate had to catch these entries and absorb their counts. The
   total number kept is therefore unchanged (those tests were also classified as “Keep” in 0.6-9).

At that time, the file-level metric was: Of 285 test files, **136** had a verifiable section number
in the file header (the old 0.6-9 criterion yielded 221).

## Reconciliation Between 0.6-19 and the 0.6-2 Survey

The 0.6-2 survey figures are the first comparison baseline. The differences and their causes are
itemized below:

| Item                           | 0.6-2 Survey                  | Current                                                                                                   | Reason for the Difference                                                                                                                                                                                                                                                   |
| ------------------------------ | ----------------------------- | --------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`      | 3443                          | 2055                                                                                                      | After 0.6-2, 2 were added (check-criterion regression tests for E3005 and E3022), then 21 no-assertion tests were deleted per the first retirement list, 11 weak-assertion tests per the second, 2 per the third, 1 per the fourth, and 1353 per the fifth; net −388        |
| Number of test files           | 296                           | 285                                                                                                       | The fifth batch retired 11 entire files                                                                                                                                                                                                                                     |
| Clause gate                    | 227 files (77%)               | 136 files after tightening; 808 entries                                                                   | The criterion was tightened from “contains a keyword” to “has a section number `§<number>`,” nearly halving the file count; the fifth batch retired 902 entries, and the repeal of the sampling audit in 0.6-19 moved another 9 entries from “Keep” to pending verification |
| Incident-history signals       | 442 entries / 148 files       | 313 within the gate; three radii: test’s own 7 / file header 291 (38 files) / entire file 1218 (79 files) | The 148 files in the 0.6-2 survey were broader than the current entire-file radius of 79 files because its vocabulary and scope were larger; the current scan strictly follows the four card-surface signals                                                                |
| Characterization strong signal | 1176 (34%) / strong 631 (18%) | Not tested                                                                                                | Characterization is not one of the three gates; mixing it into a retirement conclusion would exceed the gate’s authority                                                                                                                                                    |
| Weak assertions                | 473 (14%)                     | See the TSV “assertion summary” column                                                                    | The counting unit differs: 0.6-2 counted patterns that only asserted `is_ok`/`is_some`; the current scan uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                                     |

## 0.6-19 Conclusion Summary (Historical)

1. **The clause gate had previously been significantly overstated**: Under the old criterion, 2403
   entries were classified as “Clause/Keep.” The sampled measurement found a 50% mis-citation rate
   (95% interval 35%–65%)—“citing a rule” and “the citation supporting the assertions” are different
   things, and the old criterion treated the former as the latter. (This sampling audit was repealed
   and deleted in 0.6-19; its conclusion is retained, and the reason for deletion is given in
   “Sampling Audit Repeal” below.)
2. **After tightening, approximately 1202 entries genuinely had clause support** (range 846–1557);
   the rest either await supporting evidence through pending verification or should have been
   adjudicated through the incident-history/proof gate.
3. **The retirement surface was independent of clause mis-citations**: The 136 candidates had no
   clause basis (no assertions/weak assertions); the first batch retired 21, the second retired 11,
   the generator correction added 1, the third removed 2, the 0.6-15 criterion correction removed 3,
   the redundancy scan removed 1, the fourth removed 1, and the fifth removed 1353 (using the
   mechanical rule “for the same symbol and result category, retain only the strongest test”),
   reducing the candidates to 61. The fifth batch’s retirement distribution by gate was: clause 902
   · pending adjudication 428 · proof 23 · incident history 0 · neither 0—**not one entry from the
   incident-history gate or the zero-assertion line was removed**.
4. **The incident-history gate was the largest legitimate source of “Keep”** (313 entries): it
   demonstrates that “pinning a real incident” is more reliable than “copying a sentence from an
   RFC.”

## First Retirement Batch (Executed, 2026-11-06)

**The first batch retired 21 and retained 5 of the 26 “no assertions” tests under
`src/frontend/core/types/eval/tests/`** (a single directory cluster; criterion = all **Three Gates**
in Part Seven of coding-rules fail: no clause gate, no incident-history gate, and no assertions in
the body). The item-by-item decisions and “same-cell coverage” are given by the 0.6-11a decision
table (delete 21 / keep 5). This page does not repeat the reasons; it records only the execution
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
- **5 retained** (the decision table could not identify replacement coverage, so they were kept as
  required): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Second Retirement Batch (Executed, 2026-11-06)

**The second batch retired 11 and retained 7 of the 18 “weak assertions” tests in
`src/frontend/core/parser/pratt/tests/nud.rs`** (a single directory cluster = the Pratt
prefix-parsing module; criterion = all **Three Gates** in Part Seven of coding-rules fail: no clause
gate, no incident-history gate, and the body contains only weak assertions such as `is_some()` /
`is_none()`—it does not pin binding power or the parse result).

The difference from the first batch must be made explicit: These tests **do contain assertions** and
are not empty. The reason for retirement is “all Three Gates fail + zero criterion increment,” not
“no assertions.” For every test selected for deletion, a **unit-level replacement-coverage test name
in the same module** was identified.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Every replacement
  traverses the **same** `parse_expression → parse_prefix → prefix_info` path and makes a stronger
  assertion (pinning the AST shape rather than merely checking `is_some`): `test_parse_int_literal`
  / `test_parse_float_literal` / `test_parse_string_literal` / `test_parse_bool_true` /
  `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` / `test_parse_return_expr`
  in the same file, plus `test_grouped_expr` / `test_tuple` / `test_list_literal` /
  `test_list_empty` / `test_ref_expr` in `src/frontend/core/parser/tests/expressions.rs`.
- **7 retained**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—the item-by-item reasons are in the commit message (task 0.6, second
  test-retirement batch). The reasons fall into two categories: **the proof gate applies** (the
  first three, together with `test_prefix_info_at` and `test_prefix_info_eof`, are the **only
  unit-level probes** of their corresponding facts, and the repository contains no second unit test
  that observes them), and **the incident-history gate applies** (`break_removed` /
  `continue_removed` explicitly name #314).
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Generator Correction (0.6-13)

**`body_span` truncates a function body when braces occur in literals—fixed.**

The old implementation located a function body by balancing braces without distinguishing real code
from string/character literals or braces in comments. Forms such as `tokenize("{")` are common in
Rust source, so the balance was thrown off and the function body was truncated: the body of
`test_prefix_info_lbrace` was previously truncated at the end of the file, causing the “assertion
summary” to consume the remainder of the file (`matchesx11 assertx24 …`) and the entry to be
classified as “Pending adjudication” rather than “Retirement candidate”—**a misclassification**.

Fix: Add `mask_noncode` to first replace the contents of string literals / character literals / line
comments / block comments (nestable) / raw strings (`r"…"` / `r#"…"#` / `br"…"` / `cr#"…"#`) with
spaces (preserving newlines exactly, so line numbers and line counts do not change), then balance
the braces. **The mask is used only to locate function-body boundaries**; assertion counting and the
incident-history gate still read the original source, and the criteria themselves have not been
relaxed. `--selftest` adds 4 cases (paired and unpaired braces in ordinary strings, braces in raw
strings and block comments, and braces in character literals), each followed by a sentinel test—the
old implementation got all 4 boundaries wrong, while the new implementation gets all of them right.

### Tests Affected by the Same Bug: 11

Counting method: Compare the **function-body boundaries before and after the fix** test by test and
file by file (old implementation balancing literals directly vs. new implementation balancing after
masking); any differing boundary is affected. Of 3413 tests in the repository, the old
implementation got the boundary wrong for **11**:

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

For the other 9, **8 affected only the “assertion summary” column** (the bucket did not change), and
**1 did not even change its summary** (`test_fstring_close_brace_escape_preserved_raw`: the old
implementation omitted the closing `}` line, and that line contains no assertion keywords). The 8
summary changes are itemized in the commit message.

| Test Name                   | File                                          | Old Bucket                                  | New Bucket                                      | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| --------------------------- | --------------------------------------------- | ------------------------------------------- | ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending adjudication / Pending adjudication | Pending adjudication / **Retirement candidate** | The real function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; the file header and function body contain no § section number, issue/regression/defect/incident ⇒ both the clause gate and incident-history gate fail, and the body contains only a weak assertion such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires manual review)                                          |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident history / Keep                     | Clause / **Pending verification**               | The old implementation truncated the body to the end of the file and swallowed text containing “regression / defect / incident,” falsely hitting the incident-history gate; the real function body contains none of these terms ⇒ the incident-history gate no longer applies; the file header has verifiable clauses (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate is hit, but without sampling-audit confirmation ⇒ Pending verification |

The distribution changed accordingly: clause 1713 → 1714 · incident history 314 → 313 · Keep 776 →
775 · pending verification 2121 → 2122 · pending adjudication 412 → 411 · retirement candidate 104
→ 105. **The repository-wide `#[test]` count remains unchanged (3413)**—this round changed only the
implementation of the criteria, not any test.

## Third Retirement Batch (Executed, 2026-11-07)

**The third batch adjudicated the 5 retirement candidates under `src/middle/passes/codegen/tests/`
one by one, retiring 2 and retaining 3.** (A single directory cluster = the
`middle::passes::codegen` module under test; criterion = all **Three Gates** in Part Seven of
coding-rules fail.)

This batch follows the **retention criterion** established by the previous batch: A unit-level
observation point can only be replaced by another **unit test**; corpus/integration tests are at the
process-boundary layer and cannot replace a unit-level observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—its function body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (apart from
    comments and the function name): both are `run_fixture()` + `assert_slot_is_42(&interp)`. Its
    doc comment claims to verify that the `execute_module` entry path returns a non-Void value, but
    the body never observes the return value of `execute_module` (`run_fixture` discards it with
    `.expect(...)`)— the claimed intent was not implemented, and the function body is a pure
    duplicate ⇒ zero criterion increment. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file covers it strictly:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` has the same shape as the inline `Symbol` in this
    test). “A name that was never inserted cannot be found” and “it cannot be found after being
    popped” traverse the same path in `SymbolScopeManager::lookup` (walk the layers with
    `scope.get(name)`, returning `None` when not found) ⇒ this test adds zero increment. Replacement
    coverage = `test_scope_nesting`.
- **3 retained**:
  - `test_global_slot_store_then_load_roundtrip`—the **only observation point** for the path that
    avoids serialization and executes directly (the serialized variant is a different path), and
    `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the **only observation point** for the
    serialization round trip (`write_to` → `read_from` → execute).
  - `test_register_overflow` (`operand.rs`)—the **only unit observation point** for the `id > 255`
    rejection branch of `OperandResolver::to_reg` (the bytecode layer has its own independent
    overflow checks and does not cover this function; no second test in the repository pushes
    `to_reg` out of range). `is_err()` is a discriminating assertion (removing the guard would make
    the test fail), not merely proof that nothing panicked.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

### One Census Blind Spot Found in This Batch (**Fixed in 0.6-15**, See “Generator Criterion Correction” Below)

The census’s recognition of “delegated assertions” uses `\b(assert|check)[A-Za-z_]*\s*\(` and
**cannot match helper names containing digits**: `[A-Za-z_]*` does not include digits, so a call
such as `assert_slot_is_42(&interp)` is **not recognized as an assertion**— as a result, 2 of the 5
candidates in this batch were incorrectly classified as “no assertions in the body.”

After reviewing all 15 repository entries classified as “Neither” one by one (broadening helper
names to include digits and additionally recognizing the `match` + `panic!` form), **4 actually did
contain assertions**—3 used the numeric helpers above, and 1 was
`src/backends/interpreter/tests/weak.rs`’s `test_weak_from_arc` (the `match` + `panic!` form;
test-specification rule 6.1 requires `assert!(matches!(...))`, but it does assert). The remaining 11
truly contained no assertions.

**Directly removing the 4 entries that the machine classified as “Neither” would have wrongly
deleted tests with criteria**—the manual adjudication in this batch prevented that; the
generator-side fix is described in “Generator Criterion Correction (0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots were fixed: helper names containing digits, and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411); it only
changes how the generator recognizes whether a body contains assertions.

### What Was Fixed

1. **Digits are now allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so any test containing such a call
   was treated as “no assertions in the body.”
2. **Added recognition of the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the
   assertion is written in one arm, but `assert!` is not lexically visible. Recognition method: scan
   the expression after the `match` keyword up to the first `{` to open the block, determine the
   block boundary by **brace balancing**, and require a `panic!(` within that block. **The decision
   is made on masked lines**— braces in strings and comments do not participate in balancing, and
   `panic!` in a comment is not treated as an assertion.

### Bucket Differences: 3 (Gate and Disposition Changed)

| Test Name                                           | File                                           | Old Bucket                     | New Bucket                                  | Basis                                                                                                                                                                                                                                                                 |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------ | ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither / Retirement candidate | Pending adjudication / Pending adjudication | The body contains `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts a variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not adjudicate its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither / Retirement candidate | Pending adjudication / Pending adjudication | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern could not match; it is now recognized as an assertion                                                                                              |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither / Retirement candidate | Pending adjudication / Pending adjudication | Same as above (the serialization-round-trip entry)                                                                                                                                                                                                                    |

Another **77 entries changed only the “assertion summary” column** (the gate and disposition did not
change): Most gained `match_panicxN` in the summary (the body contains the `match` + `panic!` form),
and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3; `src/package/tests/cache.rs`:
1—helper names containing digits).

**The relaxation changes only the “Neither” tier**: This tier is the machine’s **confident verdict**
(not a single assertion in the body), and its criterion is a hard requirement; “Retirement
candidate” is only a pool awaiting manual review, so the pool size is not a verdict. All other
buckets continue to use the previous criteria. Result: gates—Neither 14 → 11 · pending adjudication
923 → 926; dispositions—retirement candidate 103 → 100 · pending adjudication 411 → 414.

### Review of the Existing 11 “Neither” Entries

After reviewing the source of all 11 repository entries classified as “Neither,” **all 11 truly
contain no assertions** (no `assert*` / `check*` helpers, no `match` + `panic!` form, and no
`#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                     | Review Conclusion                                        |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | Five `.expect(...)` calls; only guarantees that it does not crash                               | Truly no assertions (only proves that it does not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                  | Truly no assertions                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them              | Truly no assertions                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Truly no assertions                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Truly no assertions                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                              | Truly no assertions                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                        | Truly no assertions (only proves that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Truly no assertions (only proves that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Truly no assertions (only proves that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs the lsp / repl and discards them (inside the wasm32 cfg gate)                        | Truly no assertions                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Truly no assertions                                      |

Five of these (`test_normalizer_context` and the 4 entries in reducer.rs) were classified as “Keep”
in the 0.6-11a decision table because **no replacement coverage could be identified**; the
conclusions of this review are consistent with that result.

### Reverse Sample: Did the Relaxation Go Too Far?

Five entries were selected with a fixed seed (20261107) from the 88 newly recognized assertions and
their source was read to confirm that they really assert:

| Sampled Entry                                               | File                                                 | Assertion in the Source                                                                                                             |
| ----------------------------------------------------------- | ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching `match` arm; otherwise `panic!("Expected AssocType")`                                        |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm; otherwise `panic!("expected InvalidPackage, got …")`                    |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | `assert_eq!` compares the constrained type in the matching arm; otherwise `panic!("不是 Dynamic 变体…")`                            |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm; otherwise `panic!("Expected TokensDelta variant")`                          |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm validate the error code and message; otherwise `panic!("不是 Disproved…")` |

All five assert concrete values or concrete variants (replacing the implementation with another
correct path would make them fail)—**the relaxation did not go too far**. The reverse controls were
also added to `--selftest`: A `match` without `panic!`, and `panic!` appearing only in a comment;
neither may be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

After reverting the generator to the old implementation (`HELPER_RE` without digits + `match` +
`panic!` recognition disabled), the newly added `--selftest` cases **all fail**, with exit code 1;
the new implementation passes them all.

### Rollback

The generator and the census table are in the same commit; `git revert <sha>` restores everything as
a whole.

## Fourth Retirement Batch (Executed, 2026-11-07)

**The fourth batch retired 1 and retained 9 of the 10 “retirement candidates” in the directory
cluster `src/backends/interpreter/` (the interpreter backend’s test directories)** (a single
directory cluster; all three files were read in full before adjudication: `tests/bytecode_load.rs`:
4 · `tests/weak.rs`: 2 · `executor/tests/debug.rs`: 4). Criterion = the **Three Gates** in Part
Seven of coding-rules; retention criterion = “A unit-level observation point can only be replaced by
another unit test; corpus/integration tests are at the process-boundary layer”; **deletion must
identify a specific replacement-coverage test name, and if none can be identified or understood, the
test must be retained**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file traverses the same path (`make_module` + `load_module_for_stepping`, followed by one
  step before obtaining `capture_stack`), and its assertion is **strictly stronger**: it asserts
  that the stack is nonempty and that `stack[0].function_name == "main"` (pinning the top-of-stack
  position), whereas the listed test asserts only `stack.iter().any(|f| f.function_name == "main")`
  (any position is sufficient) —the former being true necessarily implies the latter ⇒ zero
  criterion increment.
- **9 retained**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each source, they are
  each the **only unit observation point** for some property (the load magic-number/version branches
  are distinct, the YXBC routing layer, both the Some/None sides of Weak, stepping with no frames,
  popping the frame on step_out, and running to completion with an empty stack), and no unit test
  capable of replacing them could be identified ⇒ they must be retained as required. The
  item-by-item basis is in the commit message (the decision table is not included on this page).
- **Numbers**: Repository-wide `#[test]` 3409 → 3408; 1 deletion = `git diff` counts `-#[test]` on 1
  line and `+#[test]` on 0 lines; the dominator `test_capture_stack_during_execution` passes
  independently. Census: retirement candidate 99 → 98 · pending adjudication 925 → 924 · clause 1713
  · incident history 313 · proof 447 · neither 11 · pending verification 2121 · Keep 775.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Fifth Retirement Batch (Executed, 2026-11-07)

**The fifth batch mechanically retired 1353 unit tests using the rule “for the same symbol and
result category, retain only the strongest test”** (only unit tests under `src/**/tests/**` were
changed; not one test on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (executed mechanically, without item-by-item argument)**: For the same symbol under test +
  the same assertion-result category, retain only the strongest assertion and delete the rest.
  - Symbol under test = a repository-internal symbol called in the test body (including one level of
    propagation through a helper function in the same file); when multiple symbols are reached,
    classify by the **primary symbol** (the first repository-internal call). Symbols are qualified
    by repository path and are not merged across modules by bare name.
  - Result category = a concrete value / a specific error code (E code) / a specific variant match /
    only `ok` / only `err` / only `should_panic`.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or `match` + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when strengths are equal, retain the first
    occurrence in the file.
  - If a test asserts multiple categories (making the primary category indeterminate), **retain
    it**; do not guess.
- **Three exceptions (not one was missed)**: ① The name or comment names `issue #` / regression /
  defect / incident (incident-history gate); ② A symbol has only one test (removing it would leave
  the symbol without a unit observation point); ③ `#[should_panic]` is the symbol’s only
  should_panic case. None of the 313 incident-history-gate entries or 11 “Neither” entries was
  retired.
- **Numbers**: Repository-wide `#[test]` 3408 → 2055; 1353 deletions = `git diff` counts `-#[test]`
  on 1353 lines and `+#[test]` on 0 lines. Census (by gate, current values after the repeal of the
  0.6-19 sampling audit): clause 808 · incident history 313 · proof 424 · pending adjudication 499 ·
  neither 11. Census (by disposition, current values): Keep 737 · pending verification 1054 ·
  pending adjudication 203 · retirement candidate 61. The fifth batch’s retirement count by gate
  was: clause 902 · pending adjudication 428 · proof 23 (in 0.6-19, 9 entries previously classified
  as “audit confirmed true support” were changed to pending verification, so the current
  gate/disposition values differ from those when the fifth batch was closed).
- **Cleanup accompanying the deletions**: Helper functions used only by deleted tests, imports that
  became invalid, and struct fields that became unused were also removed
  (`cargo clippy --all --all-features --all-targets -- -D warnings` reports 0 warnings).
- **Two categories of out-of-boundary deletions requiring a record**: The census originally
  classified 29 entries as “Keep,” but this batch’s rule still deleted them—23 current-state anchors
  (names containing unchanged/anchor/guard/preserve/baseline, including many **domain-term false
  positives** such as `test_while_guard_scope_balanced`) and 6 entries whose clause support had been
  confirmed by the audit. This batch followed the closed list of three exceptions on the card
  surface and did not additionally protect these two categories; to restore them, fill them back
  using the Keep/Delete comparison table on this page.
- **Failure and repeal of the sampling audit**: The 40-case sample was drawn from the test pool with
  a fixed seed. After the fifth retirement batch, 38 of the original samples no longer existed, so
  the audit’s `--check` always reported “38 missing, 38 extra.” This sampling audit was repealed and
  deleted in 0.6-19 (the script, data file, and the generator’s dependency on it were all removed);
  see “Sampling Audit Repeal” below.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` reverts it as a whole.

## Sixth Retirement Batch (Executed, 2026-11-26)

**The sixth batch was the mechanism rollback from the 4.9.9 narrowing round: it retired 28 tests
deleted along with the mechanism, while 4 prohibition criteria had never been added to the table.**
This batch does not use the Three Gates to decide whether a test should be kept. Instead, the
mechanism under test was **rolled back wholesale**: The criteria written for the downstream
ambiguity mechanism during the first six rounds before 4.9.9 (candidate-provider priority /
candidate-signature comparison / owner compatibility reasoning / IR-side named-argument fallback /
import-name resolution-point E5006 prohibition) were retired along with the mechanism—the mechanism
no longer exists, so the tests have no subject under test. Such deletions are registered under the
“batch” provision of coding-rules 7.3 and do not enter the retirement-candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 (28 deleted: 27 downstream-mechanism
    criteria from the first six rounds
    - 1 alias-import named-argument criterion). The 2 retained tests pin the minimal fix for
      bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: All 4 E5006 prohibition criteria were retired as an
    entire file—the file existed only in an intermediate 4.9.9 commit, and neither the parent commit
    nor that version’s stored table included it; after the prohibition was withdrawn, there was no
    subject under test, so this batch does not decrement a row in the stored table.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the candidate-injection surface for alias imports was rolled back, so the fixture
    behavior no longer holds); the corpus differential-baseline entry with the same name was deleted
    in the same batch. The baseline metric is stated separately: **relative to the immediate parent
    `ac810e97`, 405 → 404 lines** (the intermediate state had included the fixture); **relative to
    the pre-4.9.9 baseline `08ccacc5`, 403 → 404 lines** (+1 base fixture
    `named-args-cross-unit-method`; the retired `alias-overload` never entered the final history).
- **Which gate applies**: The 2 retained entries = **incident-history gate** (they pin a real defect
  registered in 4.9.9: a cross-unit named argument could not obtain the declaration-time parameter
  name, and `E1014 available` was empty); among the 28 retired entries, the mechanism criteria added
  during the first six rounds were mostly incident-history-gate entries, but the reason for deletion
  in this batch is **not** the Three Gates (it is not a judgment that they were worthless); it is
  that the mechanism was rolled back.
- **Replacement coverage**: Cross-unit named arguments with bare-name imports = the 2 retained
  driver-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity surface
  no longer has rejection logic (the requirement was rolled back), so there is no replacement
  criterion; the 4.9.8 dispatch-attribution criteria (10 driver + 5 integration) are retained
  unchanged.
- **Restoration**: `src/middle/lower/tests/query.rs`’s
  `test_method_param_info_empty_name_table_returns_none` was restored in the same batch (the
  empty-name-table guard for `method_param_info` is retained; this form is unreachable in the public
  syntax, but it is a defensive guard that the unit test prevents from being accidentally removed).
- **Numbers** (after this batch was regenerated): Repository-wide `#[test]` 2680 / 334 files; the
  previous stored table had 2688 entries → the current tree has 2680 (−8); the current tree has 1
  entry absent from the old table, the old table has 9 entries absent from the current tree, and no
  same-key rows were recomputed with changed content.
- **Rollback**: The entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit); `git revert <sha>` reverts it as a whole.

## Sampling Audit Repeal (0.6-19)

**Criterion**: The sampling audit (0.6-10) measured the “true support rate” of clause citations, but
the retention rule after 0.6-18 no longer looks at citations (it changed to “for the same symbol and
result category, retain only the strongest test,” reducing the test pool from 3408 to 2055). Of the
original 40 samples, 38 no longer existed, so the audit’s `--check` always reported “38 missing, 38
extra”—keeping it would only produce a continuing alarm. It was therefore **repealed and deleted**.

**What was deleted**: Under `scripts/dev/`, the sampling-audit script (the one that reproduces the
40-case sample with a fixed seed) and its JSON data file (the item-by-item decisions and basis for
all 40 cases).

**Generator dependency removed**: The generator previously read that JSON and classified “audit
confirmed true support” as “Clause gate + Keep”; that branch has been deleted—**clause citations no
longer have an “audit-confirmed” tier** and always fall under “Clause / Pending verification.”

**Distribution changes** (not one of the repository-wide 2055 `#[test]` entries was changed; the TSV
was regenerated): Keep 746 → 737 (−9: the 9 entries previously confirmed by the audit to have true
support fell back to pending verification); pending verification 1045 → 1054; by gate: clause 811 →
808, pending adjudication 496 → 499 (all others unchanged: incident history 313 · proof 424 ·
neither 11).

**Rollback**: The entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` reverts it as a whole.

## Rerun and Self-check

```bash
python scripts/dev/test-gate-census.py            # Recalculate and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical adjudication only, and states its boundaries honestly:

- **Delegated assertions are identified only by name** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name does not begin with either of these prefixes, it
  will be misclassified as “No assertions”—the first batch’s 26 entries (21 retired / 5 kept) and
  the second batch’s 18 entries (11 retired / 7 kept) were all manually reviewed function by
  function.
- **Function-body boundaries (fixed in 0.6-13)**: When locating a function body, the generator
  balances braces; braces in string/character literals and comments can throw the balance off and
  truncate the body—`test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was previously
  truncated at the end of the file, so the “assertion summary” consumed the remainder of the file
  and classified it as “Pending adjudication” rather than “Retirement candidate.” It now masks
  non-code regions (strings / character literals / line comments / block comments / raw strings)
  before balancing, as described in “Generator Correction.”
- **“Delegated assertion” recognition (fixed in 0.6-15)**: The old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not allow digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions—tests containing such calls were
  misclassified as “no assertions in the body.” At that time, 4 of the repository’s 15 “Neither”
  entries actually contained assertions (3 numeric helpers + 1 `match` + `panic!` form). The name
  portion has now been broadened to `[A-Za-z0-9_]*`, and recognition of the `match` + `panic!` form
  has been added; see “Generator Criterion Correction.”
- **The proof gate is not adjudicated**: Every test that fails the first two gates falls only under
  “Pending adjudication” or “Retirement candidate”; the generator never declares that a test “should
  be deleted.”
- **No cross-file deduplication**: Completely duplicate tests are not identified in this table (the
  0.6-2 measurement found only 1%, which does not constitute a retirement surface).
- **Clause citations always require verification**: A verifiable section number in the file header
  only means that “the test maps to an identifiable clause”; it does not mean that the citation
  supports the assertions. The machine does not adjudicate whether it does, so a clause hit still
  stops at “Pending verification” and must be verified by a human before the test can be kept.
