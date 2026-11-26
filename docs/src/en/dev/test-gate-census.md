---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement candidate list plus retirement records for each batch'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **three gates (clause / incident history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a repeatable per-test decision table. A
40-item sampling audit of the clause gate once measured the proportion of tests that “cited a clause
but could not support their assertions” and thereby **tightened the criteria**. That audit was
deprecated and deleted in 0.6-19. **This page and the generator itself only read tests and do not
modify them.** Test retirement is performed in batches (one commit per batch, revertible as a whole
with `git revert`); execution results are recorded in the retirement subsections below and in the
commit messages.

Generated artifacts:

| Artifact                            | Purpose                                                                          |
| ----------------------------------- | -------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Per-test decision table (2712 rows + header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                      |

## Criteria

### Scan Scope

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Scope                                                                    | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 2108                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2712** (335 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                        | Meaning of a hit                                           |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | A **verifiable entry** appears in the first 25 lines of the file header: section-number form `§<数字>` (including `RFC-<数字> §<数字>` and `SPEC §<数字>`); files under `src/**/tests/` are evaluated together with their parent `mod.rs`                                                                                                                                                                                                                | The test maps to an identifiable clause                    |
| Incident-history gate             | The test function body, function documentation comment, or **file header** (`//! 回归背景：issue #226`, for example) names a real failure: `issue #N` / 回归 / 缺陷 / 事故                                                                                                                                                                                                                                                                               | The test pins an incident that has actually occurred       |
| Proof gate                        | **Cannot be machine-judged** (“If it were replaced with another correct implementation, would it turn red?” requires human judgment). Therefore, every test that fails the first two gates is classified as “pending judgment” rather than automatically as “neither passes.” The sole exception is a body with **not a single assertion** (including no delegation to `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only “pending judgment” or “no assertions” can be assigned |

Rationale for tightening: The 0.6-9 clause gate required only that the file header contain the token
`§`/`RFC`/`SPEC`, resulting in 2403 tests being classified as “keep.” A 40-item stratified sample
audit in 0.6-10 measured that **half of the citations could not support their assertions** (50%
miscitation rate, 95% interval 35%–65%). This measurement was the **direct reason for abandoning the
rule of keeping a test based on a single citation**. After tightening, only the section-number form
is recognized, and **having a citation still does not mean that the citation supports the
assertion**—all clause citations stop at “pending verification.” (This sampling audit was deprecated
and deleted in 0.6-19: as the test pool shrank through retirement, 38 of the original samples no
longer existed, and `--check` consistently reported “38 missing, 38 extra.” The conclusion is
retained, but the audit artifacts are no longer maintained.)

Rationale for the file-header radius: Repository convention places statements such as “regression
context: issue #N” in the file header rather than repeating them in each test comment. Looking only
at a test’s own comments would miss the incident history of the entire file (measured: 7 tests
within the test itself, 291 within the file header).

### Categories That Are Never Retired

The following tests are **always classified as “keep”** and never enter the retirement candidate
pool:

- Corpus `tests/yaoxiang/**` and project-shape `tests/project-corpus/**` (the behavioral-criterion
  surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and crossing the process boundary);
- Status anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` / `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                       |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Assigned for only two cases: the incident-history gate is hit, or the proof gate applies (spec-shaped directories / status-anchor names)                                                      |
| Pending verification | Cites a clause (with a section number, or only an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a human before it can be kept |
| Retirement candidate | Fails both gates and the assertion surface contains only weak assertions (or no assertions)—a candidate is not a verdict; the proof gate still requires manual review                         |
| Pending judgment     | Fails both gates but has strong assertions; the machine reaches no conclusion                                                                                                                 |

## Distribution (Regenerated 2026-11-26)

| Gate                                                               | Count    |
| ------------------------------------------------------------------ | -------- |
| Clause (has a verifiable entry)                                    | 1017     |
| Incident history                                                   | 584      |
| Proof (spec-shaped criterion surface / status anchors)             | 571      |
| Pending judgment (fails both gates; proof requires human judgment) | 534      |
| Neither passes (no assertions anywhere in the body)                | 6        |
| **Total**                                                          | **2712** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1155** | 737 (+418)                        |
| Pending verification | 1271     | 1054 (+217)                       |
| Pending judgment     | 227      | 203 (+24)                         |
| Retirement candidate | 59       | 61 (−2)                           |
| **Total**            | **2712** | 2055 (+657)                       |

Structure of “keep”: 1155 = proof gate 571 (spec-shaped criterion surface 518 + status anchors 53) +
incident-history gate 584. File-level metric: among 335 test files, the **clause gate** covers
**122**; using the first 25 lines of the file header (`src/**/tests*/` together with the parent
`mod.rs`), a verifiable section number can be detected in **166** files.

Regeneration reconciliation: The previous committed table contained 2688 entries, while the current
tree contains 2712. The current tree has 25 entries absent from the old table, the old table has 1
entry absent from the current tree, and 2 entries with the same key had their contents recomputed.
Every key-level record is in the TSV; the destinations of stale keys are recorded in the TSV and
commit message.

### Two Criteria Changes at the 0.6-19 Tightening Point

1. **Structure of “keep”**: 737 = proof gate 424 (spec-shaped criterion surface 390 + status
   anchors 34) + incident-history gate 313. Of the original 2897 tests, the 2121 that had been “kept
   based on a single citation” now fall under pending verification. After the sampling audit was
   deprecated in 0.6-19, the 9 tests previously classified as “audit confirms genuine support” and
   kept also fall under pending verification—clause citations no longer have an “audit-confirmed”
   tier.
2. **Incident-history gate 47 → 313**: These are not new incidents, but **label reassignments**. In
   0.6-9, a test that hit both the clause and incident-history gates was recorded under the clause
   gate. After tightening, a clause hit no longer automatically meant “keep,” so these tests had to
   be captured by the incident-history gate, and their counts were folded into that gate. The total
   number kept therefore remained unchanged (this batch of tests was also kept in 0.6-9).

File-level criteria at that time: among 285 test files, **136** had a verifiable section number in
the file header (the old 0.6-9 criterion covered 221 files).

## Reconciliation Between the 0.6-19 Snapshot and the 0.6-2 Survey

The 0.6-2 survey figures are the first comparison baseline. The reasons for each difference are
listed below:

| Item                            | 0.6-2 survey                  | Current                                                                                                                                        | Reason for the difference                                                                                                                                                                                                                                                                |
| ------------------------------- | ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                                                           | 2 tests were added after 0.6-2 (check-criterion regression tests for E3005 and E3022), followed by deletion of 21 no-assertion tests in the first batch, 11 weak-assertion tests in the second batch, 2 in the third batch, 1 in the fourth batch, and 1353 in the fifth batch; net −388 |
| Number of test files            | 296                           | 285                                                                                                                                            | The fifth batch removed 11 entire files                                                                                                                                                                                                                                                  |
| Clause gate                     | 227 files (77%)               | 136 files after tightening; 808 by entry                                                                                                       | The criterion was tightened from “the token appears” to “a section number `§<数字>` is present,” nearly halving the file count; the fifth batch retired 902 tests, and the sampling audit deprecated in 0.6-19 moved another 9 from “keep” to pending verification                       |
| Incident-history signals        | 442 entries / 148 files       | 313 entries in the gate; three radii: 7 within the test itself / 291 within the file header (38 files) / 1218 across the whole file (79 files) | The 148 files in the 0.6-2 survey covered a wider scope than the current whole-file radius of 79 files because its vocabulary and range were broader; the current run strictly uses the four surface-level signals                                                                       |
| Strong characterization signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                                                   | Characterization is not one of the three gates; mixing it into retirement conclusions would exceed the gate’s authority                                                                                                                                                                  |
| Weak assertions                 | 473 (14%)                     | See the TSV “assertion summary” column                                                                                                         | The counting unit differs: 0.6-2 counted patterns that only assert `is_ok`/`is_some`; the current run uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                                                     |

## 0.6-19 Conclusion Summary (Historical)

1. **The clause gate had previously been significantly overstated**: under the old criterion, 2403
   tests were classified as “clause/keep,” and the sample audit measured a 50% miscitation rate (95%
   interval 35%–65%). A “citation” and a “citation that supports the assertion” are different
   things, but the old criteria treated the former as the latter. (This sampling audit was
   deprecated and deleted in 0.6-19; its conclusion is retained. See “Sampling Audit Deprecated”
   below for the reason.)
2. **After tightening, approximately 1202 tests genuinely had clause support** (range 846–1557). The
   rest either require pending-verification evidence or should have been judged under the
   incident-history or proof gate from the outset.
3. **The retirement surface is unrelated to clause miscitations**: the 136 candidates were unrelated
   to clauses (no assertions/weak assertions). The first batch retired 21 tests, the second batch
   retired 11, the generator correction added 1, the third batch removed 2, the 0.6-15 criteria
   correction removed 3, the redundancy scan removed 1, the fourth batch removed 1, and the fifth
   batch removed 1353 under the mechanical rule “within the same symbol and result category, keep
   only the strongest test,” reducing the candidates to 61. The fifth batch’s retirement count by
   gate was: clause 902 · pending judgment 428 · proof 23 · incident history 0 · neither passes
   0—**not a single test was removed from either the incident-history gate or the zero-assertion
   line**.
4. **The incident-history gate is the largest legitimate source of “keep”** (313 tests): it
   demonstrates that “pinning a real incident” is more reliable than “copying a sentence from an
   RFC.”

## First Retirement Batch (Executed 2026-11-06)

**First batch: 21 of the 26 “no assertions” tests in `src/frontend/core/types/eval/tests/` were
retired and 5 were kept** (a single directory cluster; criterion = all **three gates fail** under
Part Seven of coding-rules: no clause gate, no incident-history gate, and not a single assertion in
the body). The per-test decisions and “same-cell coverage” are provided by the 0.6-11a decision
table (21 removed / 5 kept). This page does not repeat the reasons and records only the execution
result.

- **21 tests retired** (2 in `normalizer.rs` + 19 in `reducer.rs`): `test_normalizer_with_config`,
  `test_normalizer_evaluator`, `test_type_computer_register_alias`,
  `test_type_computer_compute_primitive`, `test_type_computer_set_context`,
  `test_type_computer_compute_type_var`, `test_type_computer_compute_list`,
  `test_type_computer_compute_tuple`, `test_type_computer_compute_fn`,
  `test_reducer_reduce_with_limit`, `test_unifier_different_types`,
  `test_unifier_substitution_after_unify`, `test_unifier_unify_option_same`,
  `test_unifier_unify_result_same`, `test_unifier_unify_dict_same`, `test_unifier_unify_set_same`,
  `test_unifier_unify_range_same`, `test_unifier_unify_arc_same`, `test_unifier_unify_weak_same`,
  `test_unifier_unify_enum_same`, `test_unifier_unify_struct_same`.
- **5 tests kept** (the decision table could not identify replacement coverage, so they were kept
  under the hard requirement): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Second Retirement Batch (Executed 2026-11-06)

**Second batch: 11 of the 18 “weak assertion” tests in `src/frontend/core/parser/pratt/tests/nud.rs`
were retired and 7 were kept** (a single directory cluster = the Pratt prefix parser module;
criterion = all **three gates fail** under Part Seven of coding-rules: no clause gate, no
incident-history gate, and the body contains only weak assertions such as `is_some()` /
`is_none()`—it neither pins binding power nor the parse result).

The difference from the first batch must be stated clearly: these tests **do contain assertions**
and are not empty tests. The reason for retirement is “all three gates fail + zero criterion
increment,” not “no assertions.” Every test selected for deletion identifies a **unit-level
replacement test in the same module**.

- **11 tests retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Every replacement
  follows the **same** `parse_expression → parse_prefix → prefix_info` path and makes a stronger
  assertion (pinning the AST shape rather than merely checking `is_some`): `test_parse_int_literal`
  / `test_parse_float_literal` / `test_parse_string_literal` / `test_parse_bool_true` /
  `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` / `test_parse_return_expr`
  in the same file, as well as `test_grouped_expr` / `test_tuple` / `test_list_literal` /
  `test_list_empty` / `test_ref_expr` in `src/frontend/core/parser/tests/expressions.rs`.
- **7 tests kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—the reason for each is recorded in the commit message (Task 0.6, second
  test-retirement batch). The reasons fall into two categories: **the proof gate applies** (the
  first three, along with `test_prefix_info_at` / `test_prefix_info_eof`, are the sole unit-level
  observation points for their corresponding facts, and the repository contains no second unit test
  that observes them), and **the incident-history gate applies** (`break_removed` /
  `continue_removed` explicitly mention #314).
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Generator Correction (0.6-13)

**`body_span` truncated function bodies when it encountered braces inside literals—fixed.**

The old implementation obtained a function body by balancing braces without distinguishing real code
from braces in string/character literals or comments. Forms such as `tokenize("{")` are common in
Rust source, so the balance was thrown off and the function body was truncated. The body of
`test_prefix_info_lbrace` was once truncated to the end of the file, causing the “assertion summary”
to consume the remainder of the file (`matchesx11 assertx24 …`) and the test to be bucketed as
“pending judgment” rather than “retirement candidate”—a **bucketing error**.

Fix: add `mask_noncode` and first replace the contents of string literals / character literals /
line comments / block comments (including nested ones) / raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces, preserving newlines exactly so line numbers and line counts remain
unchanged, and only then balance the braces. **The mask is used only to locate function body
boundaries**; assertion statistics and the incident-history gate still read the original source, and
the criteria themselves have not been relaxed. `--selftest` gained four cases (paired and unpaired
braces in ordinary strings, braces in raw strings and block comments, and braces in character
literals), each followed by a sentinel test. The old implementation obtained the wrong boundary in
all four cases; the new implementation obtains the correct boundary in all four.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function body boundaries before and after the fix** test by test and
file by file (old implementation balancing against literals versus the new implementation balancing
after masking). Any test with different boundaries was affected. The old implementation obtained the
wrong boundary for **11** of the 3413 repository-wide tests:

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

Of the other 9 tests, **8 affected only the “assertion summary” column** (the bucket did not
change), while **1 did not change even the summary**
(`test_fstring_close_brace_escape_preserved_raw`: the old implementation omitted the closing `}`
line, and that line contains no assertion token). The 8 summary changes are recorded individually in
the commit message.

| Test name                   | File                                          | Old bucket                          | New bucket                                  | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --------------------------- | --------------------------------------------- | ----------------------------------- | ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending judgment / Pending judgment | Pending judgment / **Retirement candidate** | The actual function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; neither the file header nor the body contains a `§` section number or issue/regression/defect/incident ⇒ neither the clause gate nor the incident-history gate applies, and the body contains only a weak assertion such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires manual review)                                                |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident history / Keep             | Clause / **Pending verification**           | The old implementation truncated the body to the end of the file and swallowed text containing “regression/defect/incident,” incorrectly causing the incident-history gate to match. The actual body contains none of these terms ⇒ the incident-history gate no longer applies. The file header contains verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate matches, but the citation was not confirmed by the sampling audit ⇒ pending verification |

The distribution changed accordingly: clause 1713 → 1714 · incident history 314 → 313 · keep 776 →
775 · pending verification 2121 → 2122 · pending judgment 412 → 411 · retirement candidate 104
→ 105. **The repository-wide `#[test]` count remains unchanged (3413)**—this round changed only the
implementation of the criteria, not any test.

## Third Retirement Batch (Executed 2026-11-07)

**Third batch: 2 of the 5 retirement candidates in `src/middle/passes/codegen/tests/` were retired
and 3 were kept.** (A single directory cluster = the `middle::passes::codegen` module under test;
criterion = all **three gates fail** under Part Seven of coding-rules.)

This batch continues the **retention criterion** established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus and integration tests
operate at the process-boundary layer and cannot replace a unit-level observation point.

- **2 tests retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—its function body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file after comments are
    removed (only the function name differs): both consist of `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its documentation comment claims to verify that the
    `execute_module` entry path returns non-Void, but the body never observes the return value from
    `execute_module` (`run_fixture` discards it with `.expect(...)`)— the claimed intent is not
    implemented, and the function body is a pure duplicate ⇒ zero criterion increment. Replacement
    coverage = `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file covers it strictly:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` has the same shape as the inline `Symbol` in this
    test). “A name that was never inserted cannot be found” and “it cannot be found after being
    popped” follow the same path in `SymbolScopeManager::lookup` (layer by layer through
    `scope.get(name)`, returning `None` when not found) ⇒ zero increment. Replacement coverage =
    `test_scope_nesting`.
- **3 tests kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that
    “executes directly without going through serialization” (the serialized variant is a different
    path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **only** unit-level observation point for the
    `id > 255` rejection branch of `OperandResolver::to_reg` (the bytecode layer has its own
    independent overflow check and does not cover this function; no second test in the repository
    causes `to_reg` to go out of range). `is_err()` is a discriminative assertion (removing the
    guard would make the test fail), not merely a check that it did not panic.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

### Census Blind Spot Found in This Batch (**Fixed in 0.6-15**; see “Generator Criteria Correction” below)

The census’s recognition of “delegated assertions” uses `\b(assert|check)[A-Za-z_]*\s*\(` and
**cannot match helper names containing digits**: `[A-Za-z_]*` does not include digits, so a call
such as `assert_slot_is_42(&interp)` is not recognized as an assertion. As a result, 2 of the 5
candidates in this batch were incorrectly bucketed as “no assertions anywhere in the body.”

After all 15 “neither passes” tests were reviewed individually (expanding helper names to include
digits and additionally recognizing the `match` + `panic!` form), **4 actually contained
assertions**—3 used the numeric helper above, and 1 was `test_weak_from_arc` in
`src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule 6.1
requires `assert!(matches!(...))`, but it does assert). The other 11 truly contained no assertion at
all.

**Taking these 4 tests directly from the machine’s “neither passes” category would wrongly delete
tests that carry criteria.** The item-by-item human review in this batch prevented that; the
generator-side fix is described under “Generator Criteria Correction (0.6-15)” below.

## Generator Criteria Correction (0.6-15)

**Two criteria blind spots were fixed: helper names containing digits, and the `match` + `panic!`
form.** This round **does not delete any test** (the repository-wide `#[test]` count remains 3411);
it changes only how the generator recognizes whether a body contains assertions.

### What Was Fixed

1. **Digits were allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so a test containing such a call was
   treated as “not a single assertion anywhere in the body.”
2. **Added the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the assertion is written
   in one of the match arms, so lexically there is no `assert!` macro (test-specification rule 6.1
   requires `assert!(matches!(...))`, but existing code contains this form). Recognition method:
   scan the expression from the `match` keyword to the first `{`, then determine the block
   boundaries by **brace balancing**; the block counts only if it contains `panic!(`. **The decision
   is made on masked lines**— braces in strings and comments do not participate in balancing, and a
   `panic!` in a comment is not treated as an assertion.

### Bucket Differences: 3 (the gate and disposition changed)

| Test name                                           | File                                           | Old bucket                            | New bucket                          | Basis                                                                                                                                                                                                                                                        |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither passes / Retirement candidate | Pending judgment / Pending judgment | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending judgment / Pending judgment | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern could not match; it is now recognized as an assertion                                                                                     |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending judgment / Pending judgment | The same applies (the serialization round-trip test)                                                                                                                                                                                                         |

Another **77 tests changed only the “assertion summary” column** (the gate and disposition remained
unchanged): most gained `match_panicxN` in the summary because the body contains the `match` +
`panic!` form, and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs` has 3,
`src/package/tests/cache.rs` has 1—helper names containing digits).

**The relaxation changes only the “neither passes” tier**: that tier is the machine’s **confident
verdict** (not a single assertion anywhere in the body), and its criterion is a hard requirement.
The “retirement candidate” pool is only a pool awaiting manual review; its size is not a verdict.
All other buckets, including the weak-assertion pool, retain the original criteria. Result: gate
Neither passes 14 → 11 · Pending judgment 923 → 926; disposition Retirement candidate 103 → 100 ·
Pending judgment 411 → 414.

### Review of the Existing 11 “Neither Passes” Tests

After reading the original source for all 11 repository-wide “neither passes” tests, **all 11 truly
contain no assertion at all** (including no `assert*` / `check*` helpers, no `match` + `panic!`
form, and no `#[should_panic]`):

| Test name                                       | File                                             | What the body actually does                                                                   | Review conclusion                                       |
| ----------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 calls to `.expect(...)`, guaranteeing only that it does not crash                           | Truly no assertion (proves only that it does not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                | Truly no assertion                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them            | Truly no assertion                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                           | Truly no assertion                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                     | Truly no assertion                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                            | Truly no assertion                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                      | Truly no assertion (proves only that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                              | Truly no assertion (proves only that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                        | Truly no assertion (proves only that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                          | Truly no assertion                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Truly no assertion                                      |

The 5 tests consisting of `test_normalizer_context` and the 4 tests in reducer.rs were the tests
that the 0.6-11a decision table **could not identify replacement coverage for** and therefore kept.
The conclusion of this review is consistent with that result.

### Reverse Spot Check: Was the Relaxation Too Broad?

Five tests were selected with a fixed seed (20261107) from the 88 newly recognized assertions, and
their source was read to confirm that they really assert:

| Spot-checked test                                           | File                                                 | Assertion in the source                                                                                                               |
| ----------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matched arm, with `panic!("Expected AssocType")` otherwise                                              |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matched arm, with `panic!("expected InvalidPackage, got …")` otherwise                  |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | `assert_eq!` compares the constraint type in the matched arm, with `panic!("不是 Dynamic 变体…")` otherwise                           |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matched arm, with `panic!("Expected TokensDelta variant")` otherwise                        |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matched arm verify the error code and message, with `panic!("不是 Disproved…")` otherwise |

All 5 assert concrete values or concrete variants (replacing the implementation with another correct
path would make them fail)—**the relaxation did not go too far**. The reverse controls were also
added to `--selftest`: a `match` without `panic!`, and a `panic!` appearing only in a comment;
neither may be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

After rolling the generator back to the old implementation (`HELPER_RE` without digits and `match` +
`panic!` recognition disabled), all the newly added `--selftest` cases **fail**, with exit code 1;
the new implementation passes them all.

### Rollback

The generator and the census table are in the same commit, so `git revert <sha>` restores everything
as a whole.

## Fourth Retirement Batch (Executed 2026-11-07)

**Fourth batch: 1 of the 10 “retirement candidates” in the `src/backends/interpreter/` directory
cluster (the interpreter backend’s test directories) was retired and 9 were kept** (a single
directory cluster; all three files were read before deciding: `tests/bytecode_load.rs` with 4 tests
· `tests/weak.rs` with 2 tests · `executor/tests/debug.rs` with 4 tests). Criterion = the **three
gates** in Part Seven of coding-rules; retention criterion = “a unit-level observation point can
only be replaced by another unit test; corpus/integration tests operate at the process-boundary
layer.” **Deletion requires identifying a specific replacement test; if none can be identified or
the test cannot be understood, retain it.**

- **1 test retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, followed by one step
  and then `capture_stack`), and its assertion is **strictly stronger**: it asserts that the stack
  is nonempty and that `stack[0].function_name == "main"` (pinning the top-of-stack position),
  whereas the listed test asserts only `stack.iter().any(|f| f.function_name == "main")` (any
  position is sufficient) —the former being true necessarily implies the latter ⇒ zero criterion
  increment.
- **9 tests kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each source, every one
  is the **sole unit-level observation point** for some property (the two distinct branches for load
  magic/version, the YXBC routing layer, the two sides of Weak’s Some/None, stepping with no frame,
  step_out popping the stack, and running to completion leaving the stack empty). No unit test that
  could replace them could be identified ⇒ retained under the hard requirement. The basis for each
  is recorded in the commit message (the decision table is not included on this page).
- **Numbers**: Repository-wide `#[test]` 3409 → 3408; 1 deletion = `git diff` counts `-#[test]` on 1
  line and `+#[test]` on 0 lines; the dominant replacement, `test_capture_stack_during_execution`,
  passes independently. Census: retirement candidate 99 → 98 · pending judgment 925 → 924 · clause
  1713 · incident history 313 · proof 447 · neither passes 11 · pending verification 2121 ·
  keep 775.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Fifth Retirement Batch (Executed 2026-11-07)

**Fifth batch: 1353 unit tests were retired in bulk under the mechanical rule “within the same
symbol and result category, keep only the strongest test”** (only unit tests under `src/**/tests/**`
were changed; not a single test was touched in the E2E surfaces `tests/yaoxiang/**`,
`tests/integration/**`, `tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or
`tests/project-corpus/**`).

- **Rule (executed mechanically, without arguing each test individually)**: for the same tested
  symbol and the same assertion-result category, keep only the test with the strongest assertion and
  delete the others.
  - Tested symbol = a repository-internal symbol called in the test body (including one level of
    propagation through a helper function in the same file). When multiple symbols are reached,
    classify by the **primary symbol** (the first repository-internal call). Symbols are qualified
    by their path within the repository; bare names are not merged across modules.
  - Result category = a concrete value / a specific error code (E code) / a specific variant match /
    only asserting ok / only asserting err / only `should_panic`.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or `match` + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when equally strong, keep the first one to
    appear in the file.
  - **Keep** a test that asserts multiple categories simultaneously (when the primary category
    cannot be determined); do not guess.
- **Three exceptions (not one test fewer)**: ① the name or comment mentions `issue #` / regression /
  defect / incident (incident-history gate); ② a symbol has only one test (removing it would leave
  the symbol without a unit-level observation point); ③ `#[should_panic]` is the symbol’s only
  `should_panic` use. **Zero tests were retired** from the 313 incident-history tests or the 11
  “neither passes” tests.
- **Numbers**: Repository-wide `#[test]` 3408 → 2055; 1353 deletions = `git diff` counts `-#[test]`
  on 1353 lines and `+#[test]` on 0 lines. Census by gate (current values after the sampling audit
  was deprecated in 0.6-19): clause 808 · incident history 313 · proof 424 · pending judgment 499 ·
  neither passes 11. Census by disposition (current values): keep 737 · pending verification 1054 ·
  pending judgment 203 · retirement candidate 61. This batch’s retirement count by gate was: clause
  902 · pending judgment 428 · proof 23 (because 0.6-19 reclassified 9 entries previously marked as
  “audit confirms genuine support,” the current gate/disposition values differ from those when the
  fifth batch was closed).
- **Cleanup accompanying the deletions**: helper functions used only by deleted tests, imports that
  became invalid, and struct fields that became invalid were all removed
  (`cargo clippy --all --all-features --all-targets -- -D warnings` produced 0 warnings).
- **Two out-of-bounds deletions that must be recorded**: 29 tests that the census originally
  classified as “keep” were still deleted by this batch’s rule—23 status anchors (names containing
  unchanged/anchor/guard/preserve/baseline, many of which are **domain-term false positives**, such
  as `test_while_guard_scope_balanced`) and 6 clause entries whose genuine support had been
  confirmed by the audit. This batch followed the closed list of three exceptions on the gate
  surface and did not additionally protect these two categories; to restore them, fill them back in
  using the keep/delete comparison table on this page.
- **Failure and deprecation of the sampling audit**: The 40-item sample was drawn from the test pool
  with a fixed seed. After the fifth batch’s retirement, 38 of the original samples no longer
  existed, so the audit’s `--check` consistently reported “38 missing, 38 extra.” This sampling
  audit was deprecated and deleted in 0.6-19 (the script, data file, and the generator’s dependency
  on it were all removed). See “Sampling Audit Deprecated” below.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Sampling Audit Deprecated (0.6-19)

**Criterion**: The sampling audit (0.6-10) measured the “actual support rate of clause citations,”
but the retention rules after 0.6-18 no longer inspect citations (they changed to “within the same
symbol and result category, keep only the strongest test,” reducing the test pool from 3408 to
2055). Of the original 40 samples, 38 no longer existed, so the audit’s `--check` consistently
reported “38 missing, 38 extra”—retaining it would only produce a continuous alarm. Therefore, it
was **deprecated and deleted**.

**What was deleted**: The sampling audit script under `scripts/dev/` (the one that reproduced the
40-item sample with a fixed seed) and its JSON data file (the per-item decisions and basis for all
40 samples).

**Generator dependency removed in sync**: The generator previously read this JSON and classified
“audit confirms genuine support” as “clause gate + keep.” That branch was removed—**clause citations
no longer have an “audit-confirmed” tier** and uniformly fall under “clause / pending verification.”

**Distribution change** (not one of the 2055 repository-wide `#[test]` tests was changed; the TSV
was regenerated): keep 746 → 737 (−9: the 9 previously confirmed by the audit as genuinely supported
fell back to pending verification); pending verification 1045 → 1054; by gate: clause 811 → 808,
pending judgment 496 → 499 (the others remain unchanged: incident history 313 · proof 424 · neither
passes 11).

**Rollback**: The entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit), so `git revert <sha>` rolls it back as a whole.

## Rerun and Self-Test

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (byte-for-byte identical across two runs)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs only lexical decisions, and the boundaries are stated honestly:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name lacks these two prefixes, it will be
  misclassified as “no assertions”—the first batch of 26 tests (21 retired / 5 kept) and the second
  batch of 18 tests (11 retired / 7 kept) were each manually reviewed at the function-body level.
- **Function body boundaries (fixed in 0.6-13)**: When obtaining a function body, the generator
  balances braces; braces in strings / character literals and comments throw off the balance and
  truncate the body—`test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was once
  truncated to the end of the file, causing the “assertion summary” to consume the remainder of the
  file and the test to be bucketed as “pending judgment” rather than “retirement candidate.” The
  generator now masks non-code regions (strings / character literals / line comments / block
  comments / raw strings) before balancing them; see “Generator Correction” below.
- **“Delegated assertion” recognition (fixed in 0.6-15)**: The old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` has no digits in `[A-Za-z_]*`, so a call such as
  `assert_slot_is_42(&interp)` is not recognized as an assertion—a test containing such a call is
  misclassified as “not a single assertion anywhere in the body.” At the time, 4 of the 15
  repository-wide “neither passes” tests actually contained assertions (3 numeric helpers and 1
  `match` + `panic!` form). The name portion has now been expanded to `[A-Za-z0-9_]*`, and
  recognition of the `match` + `panic!` form has been added; see “Generator Criteria Correction”
  below.
- **The proof gate is not judged**: every test that fails both gates falls under either “pending
  judgment” or “retirement candidate”; the generator does not declare that it “should be deleted.”
- **No cross-file deduplication**: completely duplicate tests are not identified in this table
  (0.6-2 measured only 1%, which does not constitute a retirement surface).
- **Clause citations uniformly require verification**: a verifiable section number in the file
  header only means that the test “maps to an identifiable clause”; it does not mean that the
  citation supports the assertion. The machine does not judge whether it does, so even a clause hit
  stops at “pending verification” and must be verified by a human before it can be kept.
