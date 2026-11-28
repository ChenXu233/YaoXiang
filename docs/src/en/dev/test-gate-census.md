---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement-candidate list plus retirement records for each batch'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)

Pre-work for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the repository
according to the **three gates (clause / history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a rerunnable per-test decision table. A
sample audit of the clause gate (40 entries) once measured the proportion of tests that “cited a
clause but could not support their assertions,” and thereby **tightened the criteria**. That audit
was deprecated and removed in 0.6-19. **This page and the generator itself only read the tests and
do not modify them**; test retirement is performed in batches (one commit per batch, each
individually revertible with `git revert`), and the results are recorded in the retirement
subsections below and in the commit messages.

Generated artifacts:

| Artifact                            | Purpose                                                                                 |
| ----------------------------------- | --------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Per-test decision table (2710 rows plus the header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                             |

## Criteria

### Scan Scope

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Scope                                                                    | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module's `tests/` subdirectory) | 2106                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2710** (340 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Meaning of a hit                                            |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | The first 25 lines of the file contain an **identifiable clause**: a section-number form such as `§<数字>` (including `RFC-<数字> §<数字>` and `SPEC §<数字>`); files under `src/**/tests/` are considered together with their parent `mod.rs`                                                                                                                                                                                                                          | The test maps to an identifiable clause                     |
| History gate                      | The test body, function doc comment, or **file header** (such as `//! Regression context: issue #226`) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                                | The test pins an incident that has actually occurred        |
| Proof gate                        | **The machine cannot determine this** (“if it were replaced with another correct implementation, would it turn red?” requires human judgment). Therefore, anything that misses both preceding gates falls into “pending determination” and is never automatically classified as “neither gate”; the sole exception is **no assertion at all in the body** (including no delegation to `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Can only produce “pending determination” or “no assertions” |

Reason for the tightening: the 0.6-9 clause gate merely required the file header to contain the
strings `§`/`RFC`/`SPEC`, so 2403 tests were classified as “Keep.” The stratified sample of 40
entries in 0.6-10 measured that **half of the citations could not support their assertions** (50%
miscitation rate, 95% interval 35%–65%)—this was the **direct reason for abandoning the rule “keep
based on a single citation.”** After tightening, only section-number forms are recognized, and
**having a citation still does not mean the citation supports the assertion**—clause citations
always advance only to “pending verification.”  
(The sample audit was deprecated and removed in 0.6-19: as the test pool shrank through retirement,
38 of the original sample entries no longer existed, and `--check` consistently reported “missing
38, extra 38.” The conclusion is retained, but the audit artifacts are no longer maintained.)

Reason for the file-header radius: this repository's convention is to put “Regression context: issue
#N” in the file header and not repeat it in each test comment. Looking only at each test's own
comments would miss the history of an entire file (measured: 7 entries in the test's own radius, 291
in the file-header radius).

### Categories That Are Never Retired

The following tests are **always classified as “Keep”** and do not enter the retirement-candidate
pool:

- Corpus `tests/yaoxiang/**` and project-shaped `tests/project-corpus/**` (behavioral criterion
  surfaces);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and operating across process boundaries);
- Tests whose names contain `unchanged` / `anchor` / `guard` / `preserv` / `baseline`, which serve
  as status anchors.

### Disposition Semantics

| Disposition           | Meaning                                                                                                                                                                                          |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Keep                  | Only two cases: the history gate is hit, or the proof gate applies (spec-shaped directories / status-anchor names)                                                                               |
| Pending verification  | A clause is cited (with a section number, or by citing an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified before the test can be kept |
| Retirement candidate  | Both gates miss and the assertion surface contains only weak assertions (or none). A candidate is not a verdict; the proof gate still requires manual review                                     |
| Pending determination | Both gates miss, but strong assertions are present; the machine gives no conclusion                                                                                                              |

## Distribution (regenerated 2026-11-28; 4.9.9 narrowing round, batch 8.7, 4.9.10-L0, init-test rewrite, batch D53, and L3-1b S1)

| Gate                                                                     | Count    |
| ------------------------------------------------------------------------ | -------- |
| Clause (has an identifiable clause)                                      | 1037     |
| History                                                                  | 565      |
| Proof (spec-shaped criterion surfaces / status anchors)                  | 573      |
| Pending determination (both gates miss; proof gate awaits manual review) | 529      |
| Neither gate (no assertions in the body)                                 | 6        |
| **Total**                                                                | **2710** |

| Disposition           | Count    | Compared with 0.6-19 (2026-11-07) |
| --------------------- | -------- | --------------------------------- |
| Keep                  | **1138** | 737 (+401)                        |
| Pending verification  | 1287     | 1054 (+230)                       |
| Pending determination | 226      | 203 (+23)                         |
| Retirement candidate  | 59       | 61 (−3)                           |
| **Total**             | **2710** | 2055 (+651)                       |

Breakdown of “Keep”: 1138 = 573 proof-gate tests (518 spec-shaped criterion surfaces + 55 status
anchors) + 565 history-gate tests. At the file level, the **clause gate** covers **125** of the 340
test files; **147** files contain an identifiable section number (`§` followed by a number) within
the first 25 lines (`src/**/tests*/`, including the parent `mod.rs`).

Regeneration reconciliation: the previous inventory table contained 2709 entries, while the current
tree contains 2710; there is 1 entry in the current tree but not the old table (the project-helper
smoke test in `src/driver/tests/project.rs`) and 0 entries in the old table but not the current
tree. No same-key rows required recalculation. Each key is recorded in the TSV.

Latest full regeneration (after the project-helper smoke test from L3-1b S2 entered the table): 2710
rows (+1), with the newly added `test_project_helper_runs_multifile_project` in
`src/driver/tests/project.rs`; the TSV matches the current tree (`--check` exits with 0).

### Two Changes to the Criteria at the 0.6-19 Tightening Point

1. **Breakdown of “Keep”**: 737 = 424 proof-gate tests (390 spec-shaped criterion surfaces + 34
   status anchors) + 313 history-gate tests. Of the original 2897 tests, the 2121 that were “kept
   based on a single citation” now fall into pending verification. After the sample audit was
   deprecated in 0.6-19, the 9 tests previously classified as “Keep” because the audit had confirmed
   genuine support also fall into pending verification—there is no longer an “audit-confirmed” tier
   for clause citations.
2. **History gate 47 → 313**: this does not represent new history; it is a **label
   reclassification**. In 0.6-9, tests that hit both the clause and history gates were counted as
   clause hits. After tightening, a clause hit no longer automatically means “Keep,” so these tests
   must be caught by the history gate and their counts folded into it. The total number kept is
   therefore unaffected (this batch was also classified as “Keep” in 0.6-9).

At that time, the file-level criterion was: of 285 test files, **136** had an identifiable section
number in the file header (the old 0.6-9 criterion covered 221).

## Reconciliation Between 0.6-19 and the 0.6-2 Survey

The 0.6-2 survey figures form the baseline for the first comparison. The reasons for each difference
are listed below:

| Item                            | 0.6-2 survey                  | Current                                                                                                                               | Reason for the difference                                                                                                                                                                                                                                                           |
| ------------------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                                                  | 2 tests were added after 0.6-2 (the check-criterion regression tests for E3005 and E3022); 21 assertion-free tests were removed in the first batch, 11 weak-assertion tests in the second batch, 2 in the third batch, 1 in the fourth batch, and 1353 in the fifth batch; net −388 |
| Number of test files            | 296                           | 285                                                                                                                                   | 11 entire files were removed in the fifth batch                                                                                                                                                                                                                                     |
| Clause gate                     | 227 files (77%)               | 136 files after tightening; 808 tests                                                                                                 | The criterion was tightened from “the string appears” to “a section number `§<数字>` is present,” nearly halving the file count; the fifth batch retired 902 tests, and 9 more fell from “Keep” to pending verification after the sample audit was deprecated in 0.6-19             |
| History signals                 | 442 tests / 148 files         | 313 tests within the gate; three radii: 7 in the test itself / 291 in the file header (38 files) / 1218 in the entire file (79 files) | The 148 files in the 0.6-2 survey covered a wider range than the 79 files in the current whole-file radius; the current survey strictly uses the four signals on the gate card                                                                                                      |
| Characterization strong signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                                          | Characterization is not one of the three gates; mixing it into a retirement conclusion would exceed the gate's authority                                                                                                                                                            |
| Weak assertions                 | 473 (14%)                     | See the “assertion summary” column in the TSV                                                                                         | The counting units differ: 0.6-2 counted patterns that asserted only `is_ok`/`is_some`; the current census uses lexical counts and distinguishes strong, weak, and delegated assertions                                                                                             |

## 0.6-19 Conclusion Summary (Historical)

1. **The clause gate had previously been significantly inflated**: under the old criterion, 2403
   tests were classified as “clause/Keep,” and the sample audit measured a 50% miscitation rate (95%
   interval 35%–65%)—a “citation” and a “citation that supports the assertion” are different things,
   but the old criterion treated the former as the latter. (The sample audit was deprecated and
   removed in 0.6-19; its conclusion is retained. See “Sample Audit Deprecation” below for the
   reason.)
2. **After tightening, approximately 1202 tests genuinely have clause support** (range 846–1557);
   the remainder must either obtain pending verification or were already destined to be classified
   through the history or proof gate.
3. **The retirement surface is unrelated to the clause-miscitation surface**: the 136 candidates
   were unrelated to clauses (no assertions / weak assertions). The first batch retired 21 tests,
   the second retired 11, the generator correction added 1, the third batch removed 2, the 0.6-15
   criterion correction removed 3, the redundancy scan removed 1, the fourth batch removed 1, and
   the fifth batch removed 1353 under the mechanical rule “for the same symbol and result category,
   keep only the strongest test,” reducing the candidates to 61. The fifth batch's retirement count
   by gate was: clause 902 · pending determination 428 · proof 23 · history 0 · neither gate 0—**not
   a single history-gate test or zero-assertion test was retired**.
4. **The history gate is the largest legitimate source of “Keep”** (313 tests): it demonstrates that
   “pinning an actual incident” is more reliable than “copying a sentence from an RFC.”

## First Retirement Batch (Executed, 2026-11-06)

**First batch: 21 of the 26 assertion-free tests in `src/frontend/core/types/eval/tests/` were
retired, and 5 were kept** (a single directory cluster; criterion = all **three gates** in Part
Seven of coding-rules miss: no clause gate, no history gate, and no assertions in the body). The
per-test decisions and “same-cell coverage” are provided by the 0.6-11a decision table (21 deleted /
5 kept). This page does not repeat the reasons and only records the execution result.

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
- **5 tests kept** (the decision table could not identify replacement coverage, so they were kept as
  required): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Second Retirement Batch (Executed, 2026-11-06)

**Second batch: 11 of the 18 weak-assertion tests in `src/frontend/core/parser/pratt/tests/nud.rs`
were retired, and 7 were kept** (a single directory cluster = the Pratt prefix-parsing module;
criterion = all **three gates** in Part Seven of coding-rules miss: no clause gate, no history gate,
and the body contains only weak assertions such as `is_some()` / `is_none()`—neither binding power
nor the parsing result is pinned).

The difference from the first batch must be stated explicitly: these tests **do contain assertions**
and are not empty tests. The reason for retirement is “all three gates miss + zero criterion delta,”
not “there are no assertions”—every test selected for deletion identifies a **unit-level
replacement-coverage test name in the same module**.

- **11 tests retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Every replacement
  traverses the **same** `parse_expression → parse_prefix → prefix_info` path and makes stronger
  assertions (pinning the AST shape rather than merely checking `is_some`): `test_parse_int_literal`
  / `test_parse_float_literal` / `test_parse_string_literal` / `test_parse_bool_true` /
  `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` / `test_parse_return_expr`
  in the same file, plus `test_grouped_expr` / `test_tuple` / `test_list_literal` /
  `test_list_empty` / `test_ref_expr` in `src/frontend/core/parser/tests/expressions.rs`.
- **7 tests kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—the reasons for each are recorded in the commit message (Task 0.6, second
  test-retirement batch). There are two categories of reason for keeping them: **the proof gate
  applies** (the first three, together with `test_prefix_info_at` and `test_prefix_info_eof`, are
  the sole **unit-level** observation points for the corresponding facts, with no second unit test
  in the repository observing them), and **the history gate applies** (`break_removed` /
  `continue_removed` name #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Generator Correction (0.6-13)

**`body_span` truncates function bodies when it encounters braces inside literals—fixed.**

The old implementation obtained a function body by balancing braces without distinguishing real code
from braces in string/character literals or comments. Rust source commonly contains expressions such
as `tokenize("{")`; this skewed the balance, truncated the function body, and caused the “assertion
summary” to capture the rest of the file (`matchesx11 assertx24 …`). The test was consequently
classified as “pending determination” rather than “retirement candidate”—**the classification was
wrong**.

Fix: add `mask_noncode` to replace the contents of string literals, character literals, line
comments, block comments (which may be nested), and raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (preserving newlines, line numbers, and line counts), and only then balance
the braces. **The mask is used only to locate function-body boundaries**; assertion statistics and
the history gate still read the original source, and the criteria themselves have not been relaxed.
`--selftest` gained four cases (paired and unpaired braces in an ordinary string, braces in raw
strings and block comments, and braces in a character literal), each followed by a sentinel test.
The old implementation obtained the wrong boundary in all four cases; the new implementation obtains
all four correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** test by test and
file by file (the old implementation balanced braces directly, while the new implementation balances
them after masking). A test is affected if its boundaries differ. Of 3413 repository-wide tests, the
old implementation obtained the wrong boundary for **11**:

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

For the other 9, **8 affected only the “assertion summary” column** (their classifications did not
change), and **1 did not change even the summary** (`test_fstring_close_brace_escape_preserved_raw`:
the old implementation omitted the closing `}` line, which contained no assertion keywords). The 8
summary changes are recorded individually in the commit message.

| Test name                   | File                                          | Old classification                            | New classification                               | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| --------------------------- | --------------------------------------------- | --------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending determination / Pending determination | Pending determination / **Retirement candidate** | The real function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; there is no § section number and no issue/regression/defect/incident in the file header or body ⇒ both the clause and history gates miss, and the body contains only a weak assertion such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires manual review)                                                                                 |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | History / Keep                                | Clause / **Pending verification**                | The old implementation truncated the function body to the end of the file and swallowed text containing “regression / defect / incident,” incorrectly causing the history gate to match; those words do not occur in the real function body ⇒ the history gate no longer applies; the file header contains identifiable clauses (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate matches, but the citation was not confirmed by the sample audit ⇒ pending verification |

The distribution consequently changed as follows: clause 1713 → 1714 · history 314 → 313 · Keep 776
→ 775 · pending verification 2121 → 2122 · pending determination 412 → 411 · retirement candidate
104 → 105. **The repository-wide `#[test]` count did not change (3413)**—this round changed only the
implementation of the criteria, not any test.

## Third Retirement Batch (Executed, 2026-11-07)

**Third batch: 5 retirement candidates in `src/middle/passes/codegen/tests/` were evaluated
individually; 2 were retired and 3 were kept.** (A single directory cluster = the
`middle::passes::codegen` module under test; criterion = all **three gates** in Part Seven of
coding-rules miss.)

This batch retains the **keep criterion** established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus and integration tests
operate at the process-boundary layer and cannot replace a unit-level observation point.

- **2 tests retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is **byte-for-byte
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (only the function
    name differs after removing comments): both are `run_fixture()` + `assert_slot_is_42(&interp)`.
    Its doc comment claims to verify that the `execute_module` entry path returns a non-Void value,
    but the body never observes the return value from `execute_module` (`run_fixture` discards it
    with `.expect(...)`)—the stated intent was not implemented, and the body is a pure duplicate ⇒
    zero criterion delta. Replacement coverage = `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file strictly covers it:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` has the same shape as the inline `Symbol` in this
    test). “A name that was never inserted cannot be found” and “a name cannot be found after being
    popped” follow the same path in `SymbolScopeManager::lookup` (walking through `scope.get(name)`
    layer by layer and returning `None` when not found) ⇒ this test adds zero criterion. Replacement
    coverage = `test_scope_nesting`.
- **3 tests kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that does
    **not** use serialization to execute directly (the serialized variant is a different path), and
    `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **only** unit-level observation point for the
    rejection branch where `OperandResolver::to_reg` receives `id > 255` (the bytecode layer has
    separate overflow checks and does not cover this function; no other repository-wide test causes
    `to_reg` to go out of range). `is_err()` is a discriminating assertion (removing the guard would
    turn the test red), not merely a check that nothing panicked.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

### Census Blind Spot Found in This Batch (**Fixed in 0.6-15**; see “Generator Criterion Correction” below)

The census's recognition of “delegated assertions,” `\b(assert|check)[A-Za-z_]*\s*\(`, **could not
match helper names containing digits**: `[A-Za-z_]*` does not include digits, so a call such as
`assert_slot_is_42(&interp)` was not recognized as an assertion. Consequently, 2 of the 5 candidates
in this batch were incorrectly classified as “no assertions in the body.”

After reviewing all 15 repository-wide “neither gate” entries individually (broadening helper names
to include digits and additionally recognizing the `match` + `panic!` form), **4 actually contained
assertions**: 3 used the numeric helpers described above, and 1 was `test_weak_from_arc` in
`src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule 6.1
requires `assert!(matches!(...))`, but this test does in fact assert). The other 11 genuinely
contain no assertions.

**Taking these 4 at face value from the machine's “neither gate” classification would have deleted
tests with criteria**—manual per-test review in this batch prevented that. The generator-side fix is
described under “Generator Criterion Correction (0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots have been fixed: helper names containing digits, and the `match` +
`panic!` form.** This round **does not delete any test** (the repository-wide `#[test]` count
remains 3411); it changes only how the generator recognizes whether a body contains assertions.

### What Was Fixed

1. **Digits are now allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing such calls were
   treated as having “no assertions in the body.”
2. **The `match` + `panic!` form was added**: `match <expr> { … panic!(…) … }`—the assertion is
   written in an arm, so the `assert!` macro is not visible lexically (test-specification rule 6.1
   requires `assert!(matches!(...))`, but the existing code contains this form). Recognition works
   by scanning the expression from the `match` keyword to the first `{`, determining the block
   boundary through **brace balancing**, and then requiring a `panic!(` within the block. **The
   decision is made on masked lines**—braces in strings and comments do not affect the balance, and
   `panic!` in a comment is not treated as an assertion.

### Classification Differences: 3 (the gate and disposition changed)

| Test name                                           | File                                           | Old classification                  | New classification                            | Basis                                                                                                                                                                                                                                                        |
| --------------------------------------------------- | ---------------------------------------------- | ----------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither gate / Retirement candidate | Pending determination / Pending determination | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts a variant, but that shape was not previously recognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither gate / Retirement candidate | Pending determination / Pending determination | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern could not match; it is now recognized as an assertion                                                                                     |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither gate / Retirement candidate | Pending determination / Pending determination | The same applies to the serialization-round-trip test                                                                                                                                                                                                        |

Another **77 entries changed only the “assertion summary” column** (the gate and disposition did not
change): most summaries gained `match_panicxN` because the body contains the `match` + `panic!`
form, and 4 gained `helperxN` because their helper names contain digits (3 in
`src/driver/tests/manifest_error.rs` and 1 in `src/package/tests/cache.rs`).

**The relaxation changes only the “neither gate” tier**: that tier is the machine's **confident
verdict** (there is not a single assertion in the body), and its criterion is a hard requirement. A
“retirement candidate” is only a pool awaiting manual review, and the size of that pool is not a
verdict. All other classifications, including the weak-assertion pool, continue to use the original
criteria. Results: gates—neither gate 14 → 11 · pending determination 923 → 926;
dispositions—retirement candidate 103 → 100 · pending determination 411 → 414.

### Existing “Neither Gate” Entries: 11 Reviewed Individually

After individually reading the source of all 11 repository-wide “neither gate” entries, **all 11
genuinely contain no assertions** (including no `assert*` / `check*` helper, no `match` + `panic!`
form, and no `#[should_panic]`):

| Test name                                       | File                                             | What the body actually does                                                                     | Review conclusion                                            |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | Five `.expect(...)` calls, guaranteeing only that it does not crash                             | Genuinely no assertions (proves only that it does not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                  | Genuinely no assertions                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them              | Genuinely no assertions                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Genuinely no assertions                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Genuinely no assertions                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                              | Genuinely no assertions                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` with invalid content and discards the result                      | Genuinely no assertions (proves only that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Genuinely no assertions (proves only that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Genuinely no assertions (proves only that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (within the wasm32 cfg gate)                            | Genuinely no assertions                                      |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (within the wasm32 cfg gate)                            | Genuinely no assertions                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertions                                      |

The 5 tests consisting of `test_normalizer_context` and the 4 tests in reducer.rs were classified as
“Keep” in the 0.6-11a decision table because **no replacement coverage could be identified**; the
conclusion of this review is consistent with that table.

### Reverse Spot Check: Did the Relaxation Go Too Far?

Five entries were selected from the 88 entries “newly recognized as assertions” using a fixed seed
(20261107), and their source was read to confirm that they genuinely assert:

| Spot-check entry                                            | File                                                 | Assertion in the source                                                                                                                |
| ----------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching arm of a `match`, with `panic!("Expected AssocType")` otherwise                                 |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm, with `panic!("expected InvalidPackage, got …")` otherwise                  |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | `assert_eq!` compares the constraint type in the matching arm, with `panic!("不是 Dynamic 变体…")` otherwise                           |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm, with `panic!("Expected TokensDelta variant")` otherwise                        |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm verify the error code and message, with `panic!("不是 Disproved…")` otherwise |

All 5 assert a concrete value or a concrete variant (replacing the implementation with another
correct path would turn them red)—**the relaxation did not go too far**. Reverse controls were also
added to `--selftest`: a `match` without `panic!`, and a `panic!` that occurs only in a comment;
neither may be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

After reverting the generator to the old implementation (`HELPER_RE` without digits + `match` +
`panic!` recognition disabled), all the newly added `--selftest` cases **failed**, with exit code 1;
the new implementation passes all of them.

### Rollback

The generator and census table are in the same commit, so `git revert <sha>` restores everything.

## Fourth Retirement Batch (Executed, 2026-11-07)

**Fourth batch: 1 of the 10 “retirement candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend's test directories) was retired, and 9 were
kept** (a single directory cluster; all three files were read before making the decision:
`tests/bytecode_load.rs` 4 entries · `tests/weak.rs` 2 entries · `executor/tests/debug.rs` 4
entries). Criterion = the **three gates** in Part Seven of coding-rules; keep criterion = “a
unit-level observation point can only be replaced by another unit test; corpus/integration tests
operate at the process-boundary layer”; **deletion must identify a specific replacement-coverage
test name, and a test is kept if no replacement can be identified or its behavior cannot be
understood**.

- **1 test retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file traverses the same path (`make_module` + `load_module_for_stepping`, one step, then
  obtain `capture_stack`) and makes a **strictly stronger** assertion: it asserts that the stack is
  non-empty and that `stack[0].function_name == "main"` (pinning the top-of-stack position), whereas
  the listed test only asserts `stack.iter().any(|f| f.function_name == "main")` (any position is
  sufficient)——the former being true necessarily implies the latter ⇒ zero criterion delta.
- **9 tests kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each one individually,
  each is the sole unit-level observation point for some property (the two distinct load branches
  for the magic number/version, the YXBC routing layer, the Some/None sides of Weak, stepping with
  no frames, `step_out` popping the stack, and running to completion leaving the stack empty). No
  unit test capable of replacing them could be identified ⇒ they are kept as required. The basis for
  each is recorded in the commit message (the decision table is not included on this page).
- **Numbers**: repository-wide `#[test]` 3409 → 3408; 1 deletion = `git diff` counts `-#[test]` on 1
  line and `+#[test]` on 0 lines; the dominating test `test_capture_stack_during_execution` passes
  when run on its own. Census: retirement candidate 99 → 98 · pending determination 925 → 924 ·
  clause 1713 · history 313 · proof 447 · neither gate 11 · pending verification 2121 · Keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Fifth Retirement Batch (Executed, 2026-11-07)

**Fifth batch: 1353 unit tests were retired in bulk under the mechanical rule “for the same symbol
and result category, keep only the strongest test”** (only unit tests under `src/**/tests/**` were
changed; not a single test in the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (executed mechanically, without individual justification)**: for the same symbol under test
  and the same assertion-result category, keep only the test with the strongest assertion and delete
  the others.
  - Symbol under test = a repository-internal symbol called in the test body (including one level of
    propagation through a helper function in the same file). When multiple symbols are touched,
    classify by the **primary symbol** (the first repository-internal call). Symbols are qualified
    by their repository path and are not merged across modules by bare name.
  - Result category = a concrete value / a specific error code (an E code) / a specific variant
    match / only `ok` / only `err` / only `should_panic`.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or `match` + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when tests in the same group are equally
    strong, keep the one that appears first in the file.
  - **Keep** a test that asserts multiple categories simultaneously (when the primary category
    cannot be determined); do not guess.
- **Three exceptions (not one test was exempt)**: ① the name or comment names `issue #` / regression
  / defect / incident (history gate); ② a symbol has only one test (deleting it would leave the
  symbol without a unit-level observation point); ③ `#[should_panic]` is the symbol's only
  `should_panic` test. **Zero** history-gate tests and **zero** “neither gate” tests were retired.
- **Numbers**: repository-wide `#[test]` 3408 → 2055; 1353 deletions = `git diff` counts `-#[test]`
  on 1353 lines and `+#[test]` on 0 lines. Census by gate (current values after the 0.6-19 sample
  audit was deprecated): clause 808 · history 313 · proof 424 · pending determination 499 · neither
  gate 11. Census by disposition (current values): Keep 737 · pending verification 1054 · pending
  determination 203 · retirement candidate 61. The retirement count in this batch by gate was:
  clause 902 · pending determination 428 · proof 23 (0.6-19 changed 9 entries previously classified
  as “audit-confirmed genuine clause support” to pending verification, so the current values by gate
  and disposition differ from those when the fifth batch was closed).
- **Cleanup accompanying the deletions**: helper functions used only by deleted tests, imports that
  became unused, and struct fields that became obsolete were removed
  (`cargo clippy --all --all-features --all-targets -- -D warnings` reported 0 warnings).
- **Two kinds of out-of-bounds deletion that must be recorded**: 29 tests that the census originally
  classified as “Keep” were nevertheless deleted by this batch's rule—23 status anchors (names
  containing unchanged/anchor/guard/preserve/baseline, many of which were false matches on **domain
  terms** such as “guard” or “preserves,” for example `test_while_guard_scope_balanced`) and 6
  entries with audit-confirmed genuine clause support. This batch followed the closed list of three
  exceptions on the gate card and did not additionally protect these two categories; to restore
  them, simply refill them using this page's keep/delete comparison table.
- **Failure and deprecation of the sample audit**: the 40-entry sample was drawn from the test pool
  using a fixed seed. After the fifth batch retired the tests, 38 entries in the original sample no
  longer existed, so the audit's `--check` consistently reported “missing 38, extra 38.” The sample
  audit was deprecated and removed in 0.6-19 (the script, data file, and the generator's dependency
  on it were all removed); see “Sample Audit Deprecation” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Sixth Retirement Batch (Executed, 2026-11-26)

**Sixth batch: 28 tests were retired along with the mechanism they covered in the mechanism rollback
from the 4.9.9 narrowing round; another 4 prohibition criteria were never included in the
inventory.** This batch does not use the three gates to decide whether tests “should be kept.”
Instead, the mechanisms under test were **rolled back as a whole**: the criteria written during the
first six rounds of 4.9.9 for downstream ambiguity mechanisms (candidate provider priority /
candidate signature comparison / owner compatibility inference / IR-side named-argument fallback /
the E5006 import-name resolution-point prohibition) were retired along with the mechanisms
themselves—with no mechanism, there is no subject for the tests to exercise. These deletions are
recorded as a “batch” under coding-rules 7.3 and do not enter the retirement-candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 entries → 2 entries (28 deleted: 27
    downstream-mechanism criteria from the first six rounds + 1 alias-import named-argument
    criterion). The 2 retained entries pin the minimal fix for bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: all 4 E5006 prohibition criteria were retired with
    the entire file—the file existed only in an intermediate 4.9.9 commit and appeared in neither
    the parent commit nor that version's inventory table; once the prohibition was withdrawn, there
    was no subject under test, so this batch does not reduce the inventory table.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the alias-import candidate-injection surface was rolled back, so the fixture's behavior
    no longer holds); the same-named corpus-diff baseline entry was deleted in the same batch. The
    baselines are stated separately: **relative to the direct parent commit `ac810e97`, 405 → 404
    lines** (the intermediate state had contained the fixture); **relative to the pre-4.9.9 baseline
    `08ccacc5`, 403 → 404 lines** (+1 base fixture `named-args-cross-unit-method`; the retired
    `alias-overload` never entered the final history).
- **Which gate applies**: the 2 retained entries = **history gate** (they pin a real defect
  registered in 4.9.9: cross-unit named arguments could not obtain the declared parameter name,
  leaving `E1014 available` empty). Among the 28 retired entries, the mechanism criteria added
  during the first six rounds were mostly history-gate criteria, but the reason for deleting them in
  this batch is **not** the three gates (they were not judged unworthy); rather, the mechanisms
  under test were rolled back.
- **Replacement coverage**: cross-unit named arguments for bare-name imports = the 2 retained
  driver-level criteria
  - the `named-args-cross-unit-method` project fixture; the ambiguity surface no longer has
    rejection logic (the requirement was rolled back), so there is no replacement criterion; the
    4.9.8 dispatch-attribution criteria (10 driver tests + 5 integration tests) are retained
    unchanged.
- **Restoration**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the empty-name-table guard for
  `method_param_info` is retained; this shape is unreachable under the public syntax and is a
  defensive guard, with a unit test solely to prevent accidental deletion).
- **Numbers** (after this batch was regenerated): repository-wide `#[test]` 2680 entries / 334
  files; the previous inventory table contained 2688 entries, while the current tree contains 2680
  (−8); there is 1 entry in the current tree but not the old table and 9 entries in the old table
  but not the current tree, with 0 same-key rows requiring recalculation.
- **Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit), so `git revert <sha>` rolls it back as a whole.

## Sample Audit Deprecation (0.6-19)

**Criterion**: the sample audit (0.6-10) measured the “actual support rate of clause citations,” but
the keep rule after 0.6-18 no longer considers citations (it changed to “for the same symbol and
result category, keep only the strongest test”), reducing the test pool from 3408 to 2055. Of the
original 40 sample entries, 38 no longer exist, so the audit's `--check` consistently reports
“missing 38, extra 38”—keeping it would only produce continuous alerts. It was therefore
**deprecated and removed**.

**What was deleted**: the sample-audit script under `scripts/dev/` (the one that reproduced the
40-entry sample using a fixed seed) and its JSON data file (the per-entry decisions and bases for
all 40 entries).

**The generator's dependency was removed in sync**: the generator previously read that JSON and
classified “audit-confirmed genuine support” as “clause gate + Keep.” That branch was
removed—**there is no longer an “audit-confirmed” tier for clause citations**; all such citations
fall into “clause / pending verification.”

**Distribution change** (all 2055 repository-wide `#[test]` entries remained unchanged, and the TSV
was regenerated): Keep 746 → 737 (−9: the 9 entries previously confirmed by the audit as genuinely
supported fell back to pending verification); pending verification 1045 → 1054; by gate: clause 811
→ 808, pending determination 496 → 499 (all other values unchanged: history 313 · proof 424 ·
neither gate 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit), so `git revert <sha>` rolls it back as a whole.

## Rerunning and Self-Checking

```bash
python scripts/dev/test-gate-census.py            # Recalculate and overwrite the TSV (both runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical classification only, and states its boundaries accurately:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test hides its assertions in a helper whose name lacks these two prefixes, it will be
  misclassified as “no assertions”—the first batch of 26 tests (21 retired / 5 kept) and second
  batch of 18 tests (11 retired / 7 kept) were each manually reviewed at the function-body level.
- **Function-body boundaries (fixed in 0.6-13)**: when obtaining a function body, the generator
  balances braces; braces in string/character literals or comments can skew the balance and truncate
  the function body. `test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was once
  truncated to the end of the file, causing the “assertion summary” to capture the rest of the file
  and the test to be classified as “pending determination” rather than “retirement candidate.” The
  generator now masks non-code regions (string / character literals / line comments / block comments
  / raw strings) before balancing the braces; see “Generator Correction” below.
- **“Delegated assertion” recognition (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not include digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions, and tests containing them were
  misclassified as having “no assertions in the body.” At the time, 4 of the 15 repository-wide
  “neither gate” entries actually contained assertions (3 numeric helpers + 1 `match` + `panic!`
  form). The name portion has now been broadened to `[A-Za-z0-9_]*`, and the `match` + `panic!` form
  has been added; see “Generator Criterion Correction” below.
- **The proof gate is not judged**: any test that “misses both gates” is classified only as “pending
  determination” or “retirement candidate”; the generator never declares that a test “should be
  deleted.”
- **No cross-file deduplication**: completely duplicated tests are not identified in this table (the
  0.6-2 survey measured only 1%, which does not constitute a retirement surface).
- **Clause citations are always pending verification**: an identifiable section number in the file
  header only means that the test “maps to an identifiable clause”; it does not mean that the
  citation supports the assertion. The machine does not judge whether it does, so even a clause hit
  only advances to “pending verification” and must be manually verified before the test can be kept.
