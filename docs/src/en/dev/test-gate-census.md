---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement candidate list and retirement records for each batch'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)

Prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every repository-wide
`#[test]` according to the **three gates (clause / incident history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), producing a rerunnable item-by-item decision table.
The sampling audit of the clause gate (40 cases) once measured the proportion of tests that “cited a
clause but whose citation could not support their assertions” and **tightened the criteria**
accordingly; that audit was repealed and deleted in 0.6-19. **This page and the generator itself are
read-only and do not modify tests**; tests are retired in batches (one commit per batch, with the
entire batch revertible via `git revert`), and execution results are recorded in the per-batch
retirement sections and commit messages below.

Generated artifacts:

| Artifact                            | Purpose                                                                                     |
| ----------------------------------- | ------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Item-by-item decision table (2712 rows plus the header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable; supports `--check` and `--selftest`)                                 |

## Criteria

### Scan Scope

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root-package lib (`src/`, including each module’s `tests/` subdirectory) | 2108                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2712** (341 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criteria                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Meaning of a hit                                           |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | A **verifiable entry** appears within the first 25 lines of the file: section-number form `§<数字>` (including `RFC-<数字> §<数字>` and `SPEC §<数字>`); files under `src/**/tests/` are examined together with their parent `mod.rs`                                                                                                                                                                                                                                          | The test maps to one clause that can be identified by name |
| Incident-history gate             | The test body, function doc comment, or **file header** (such as `//! 回归背景：issue #226`) names an actual failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                                              | The test pins down an incident that actually occurred      |
| Proof gate                        | **Cannot be determined by the machine** (“If it were replaced with another correct implementation, would it turn red?” requires human judgment). Therefore, anything that fails both preceding gates is classified as “pending judgment,” never automatically as “neither gate passes”; the sole exception is a body with **no assertions whatsoever** (including no assertions delegated to `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Can only yield “pending judgment” or “no assertions”       |

Rationale for tightening: the clause gate in 0.6-9 only required the file header to **contain
`§`/`RFC`/`SPEC`**, so 2403 tests were classified as “keep.” The 40-case stratified sample measured
in 0.6-10 found that **half of the citations could not support the assertions** (mis-citation rate:
50%, 95% interval: 35%–65%)—this measurement was the **direct reason for abandoning “keep based on a
single citation.”** After tightening, only section-number forms are recognized, and **having a
citation still does not mean the citation supports the assertions**—clause citations always stop at
“pending verification.” (This sampling audit was repealed and deleted in 0.6-19: as the audit pool
shrank with test retirement, 38 tests from the original sample no longer existed, and `--check`
always reported “38 missing, 38 extra”; the conclusion is retained, but the audit artifacts are no
longer maintained.)

Rationale for the file-header radius: this repository conventionally puts “Regression context: issue
#N” in the file header and does not repeat it in individual test comments; looking only at a test’s
own comment would miss the incident history of an entire file (measured: 7 tests within the test’s
own radius versus 291 within the file-header radius).

### Categories That Are Never Retired

The following tests are **always kept** and do not enter the retirement candidate pool:

- Corpus tests under `tests/yaoxiang/**` and project-form fixtures under `tests/project-corpus/**`
  (behavioral-criteria surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and operating across a process boundary);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                                |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Keep                 | Only two reasons: the incident-history gate is hit, or the proof gate applies (spec-shaped directories / current-state anchor names)                                                                   |
| Pending verification | A clause is cited (with a section number, or only an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a human before the test can be kept |
| Retirement candidate | Neither gate passes, and the assertion surface contains only weak assertions (or no assertions)—a candidate is not a verdict; the proof gate still requires manual review                              |
| Pending judgment     | Neither gate passes, but strong assertions are present; the machine reaches no conclusion                                                                                                              |

## Distribution (regenerated on 2026-11-28; 4.9.9 narrowing round, batch 8.7, 4.9.10-L0, init test rewrite, batch D53, and L3-1b S1)

| Gate                                                                           | Count    |
| ------------------------------------------------------------------------------ | -------- |
| Clause (has a verifiable entry)                                                | 1037     |
| Incident history                                                               | 565      |
| Proof (spec-shaped criteria surface / current-state anchors)                   | 573      |
| Pending judgment (neither of the first two gates; proof pending manual review) | 531      |
| Neither gate passes (no assertions in the body)                                | 6        |
| **Total**                                                                      | **2712** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1138** | 737 (+401)                        |
| Pending verification | 1287     | 1054 (+230)                       |
| Pending judgment     | 228      | 203 (+23)                         |
| Retirement candidate | 59       | 61 (−3)                           |
| **Total**            | **2712** | 2055 (+651)                       |

Composition of the kept tests: 1138 = 573 proof-gate tests (518 on the spec-shaped criteria
surface + 55 current-state anchors) + 565 incident-history-gate tests. File-level scope: the clause
gate covers **125** of the 341 test files; using the first 25 lines of the file (`src/**/tests*/`
together with the parent `mod.rs`), a verifiable section number (`§` + a number) can be detected in
**147** files.

Regeneration reconciliation: the previous committed table had 2710 entries, while the current tree
has 2712; there are 2 tests in the current tree but not in the old table (the identity-index guard
tests in `src/frontend/core/sema/environment.rs`), 0 tests in the old table but not in the current
tree, and 0 same-key rows whose contents changed. Per-key records are in the TSV.

Latest full regeneration (after the env.types derived-index guard tests from L3-1b S3 were
committed): 2712 rows (+2), adding the bypass-write guard and duplicate-identity tests for
`src/frontend/core/sema/environment.rs`; the TSV matches the current tree (`--check` exits with 0).

### Two Changes in the Criteria at the 0.6-19 Snapshot

1. **Composition of kept tests**: 737 = 424 proof-gate tests (390 on the spec-shaped criteria
   surface + 34 current-state anchors) + 313 incident-history-gate tests. Of the original 2897
   tests, the 2121 that were “kept based on a single citation” are now pending verification (after
   the sampling audit was repealed in 0.6-19, the 9 previously classified as “audit-confirmed as
   genuinely supporting” were also moved to pending verification—clause citations no longer have an
   “audit-confirmed” tier).
2. **Incident-history gate, 47 → 313**: these are not newly discovered incidents, but **label
   reassignments**—in 0.6-9, tests that hit both the clause and incident-history gates were recorded
   as clause hits; after tightening, the clause gate no longer automatically means “keep,” so these
   tests must be caught by the incident-history gate and their counts folded into that gate. The
   total number kept is therefore unaffected (this batch of tests was also kept in 0.6-9).

File-level scope at the time: of 285 test files, **136** had a verifiable section number in the file
header (the old 0.6-9 criterion found 221).

## Reconciliation Between the 0.6-19 Snapshot and the 0.6-2 Survey

The 0.6-2 survey figures form the first comparison baseline. The reasons for each difference are
listed below:

| Item                           | 0.6-2 Survey                  | This Review                                                                                                                       | Reason for the Difference                                                                                                                                                                                                                                                                   |
| ------------------------------ | ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`      | 3443                          | 2055                                                                                                                              | 2 tests were added after 0.6-2 (check-criteria regression tests for E3005 and E3022), followed by the deletion of 21 no-assertion tests in the first batch, 11 weak-assertion tests in the second batch, 2 in the third batch, 1 in the fourth batch, and 1353 in the fifth batch; net −388 |
| Number of test files           | 296                           | 285                                                                                                                               | 11 whole files were removed in the fifth batch                                                                                                                                                                                                                                              |
| Clause gate                    | 227 files (77%)               | 136 files after tightening; 808 by item                                                                                           | The criterion was tightened from “the token appears” to “a section number `§<number>` is present,” nearly halving the file count; the fifth batch retired 902 tests, and after the sampling audit was repealed in 0.6-19, another 9 moved from “keep” to “pending verification”             |
| Incident-history signal        | 442 tests / 148 files         | 313 within the gate; three radii: 7 within the test itself / 291 in file headers (38 files) / 1218 across entire files (79 files) | The 148 files in the 0.6-2 survey cover more than the 79 files found by the entire-file radius here; its vocabulary and scope are broader; this review strictly follows the card’s four signals                                                                                             |
| Characterization strong signal | 1176 (34%) / strong 631 (18%) | Not assessed                                                                                                                      | Characterization is not one of the three gates; mixing it into a retirement conclusion would overstep the gate’s authority                                                                                                                                                                  |
| Weak assertions                | 473 (14%)                     | See the “assertion summary” column in the TSV                                                                                     | The counting units differ: 0.6-2 used pattern matching for “only asserts is_ok/is_some,” whereas this review uses lexical counts and distinguishes strong, weak, and delegated assertions                                                                                                   |

## 0.6-19 Conclusion Summary (Historical)

1. **The clause gate had previously been significantly inflated**: under the old criterion, 2403
   tests were classified as “clause/keep.” The sample measured a 50% mis-citation rate (95%
   interval: 35%–65%)—“citation” and “a citation that supports the assertion” are different things,
   and the old criterion treated the former as the latter. (This sampling audit was repealed and
   deleted in 0.6-19; its conclusion is retained. See “Sampling Audit Repeal” below for the reason
   for deletion.)
2. **After tightening, approximately 1202 tests genuinely have clause support** (range: 846–1557);
   the rest must either obtain pending-verification evidence or were already destined for judgment
   under the incident-history or proof gate.
3. **The retirement surface is unrelated to the clause mis-citation surface**: the 136 candidates
   had no clause (no assertions/weak assertions); the first batch retired 21, the second retired 11,
   the generator correction added 1, the third batch removed 2, the 0.6-15 criterion correction
   removed 3, the redundancy scan removed 1, the fourth batch removed 1, and the fifth batch removed
   1353 under the mechanical rule “for the same symbol and result category, keep only the strongest
   test,” reducing the candidates to 61. The fifth batch’s retirement count by gate was: clause 902
   · pending judgment 428 · proof 23 · incident history 0 · neither gate 0—**not a single test from
   the incident-history gate or the zero-assertion line was removed**.
4. **The incident-history gate is the largest legitimate source of kept tests** (313 tests): it
   demonstrates that “pinning down an actual incident” is more reliable than “copying a sentence
   from an RFC.”

## First Retirement Batch (Executed on 2026-11-06)

**First batch: 21 of the 26 “no-assertion” tests under `src/frontend/core/types/eval/tests/` were
retired, and 5 were kept** (a single directory cluster; criterion = none of the **three gates in
Part Seven of coding-rules** passes: no clause gate, no incident-history gate, and no assertions in
the body). The item-by-item decisions and “same-cell coverage” were provided by the 0.6-11a decision
table (21 deleted / 5 kept); this page does not repeat the reasons and records only the execution
results.

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
- **5 kept** (the decision table could not identify replacement coverage, so they were kept under
  the hard requirement): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Second Retirement Batch (Executed on 2026-11-06)

**Second batch: 11 of the 18 “weak-assertion” tests in `src/frontend/core/parser/pratt/tests/nud.rs`
were retired, and 7 were kept** (a single directory cluster = the Pratt prefix-parsing module;
criterion = none of the **three gates in Part Seven of coding-rules** passes, and the body contains
only weak assertions such as `is_some()` / `is_none()`—it does not pin down binding power or the
parse result).

The difference between this batch and the first must be stated clearly: these tests **do contain
assertions**; they are not empty tests. The reason for retirement is “none of the three gates
passes + zero incremental criterion value,” not “no assertions”—every test selected for deletion
identifies a **unit-level replacement coverage test in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The replacement
  coverage for every test follows the **same** `parse_expression → parse_prefix → prefix_info` path
  and makes stronger assertions (pinning down the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, and `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the commit message for the item-by-item reasons (task 0.6, second
  test-retirement batch). The reasons fall into two categories: the **proof gate applies** (the
  first three, along with `test_prefix_info_at` and `test_prefix_info_eof`, are the sole unit-level
  inspection points for their corresponding facts, and the repository contains no second unit test
  that observes them), or the **incident-history gate applies** (`break_removed` /
  `continue_removed` explicitly name #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Generator Correction (0.6-13)

**`body_span` truncates a function body when it encounters braces inside literals — fixed.**

The old implementation extracted function bodies by balancing braces and did not distinguish real
code from braces in string/character literals or comments. Forms such as `tokenize("{")` are common
in Rust source, so balancing became skewed and the function body was truncated: the body of
`test_prefix_info_lbrace` was once truncated to the end of the file, causing the “assertion summary”
to include the remainder of the file (`matchesx11 assertx24 …`) and the test to be bucketed as
“pending judgment” rather than “retirement candidate”—**an incorrect bucket**.

Fix: add `mask_noncode`, replacing the contents of string literals, character literals, line
comments, block comments (which may be nested), and raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (newlines are preserved exactly, so line numbers and line counts do not
change), then balance the braces. **The mask is used only to locate function-body boundaries**;
assertion statistics and the incident-history gate still read the original source, and the criterion
itself has not been relaxed. `--selftest` gained 4 cases (paired and unpaired braces in ordinary
strings, braces in raw strings and block comments, and braces in character literals), each followed
by a sentinel test. The old implementation found the wrong boundary in all 4 cases; the new
implementation finds all of them correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** test by test and
file by file (old implementation: balance against literals; new implementation: balance after
masking). Any test whose boundaries differ is affected. Of the 3413 repository-wide tests, the old
implementation found incorrect boundaries for **11** tests:

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

### Bucket Differences Before and After the Fix: 2 Tests

For the other 9 tests, **8 affected only the “assertion summary” column** (the bucket did not
change), and **1 did not even change the summary** (`test_fstring_close_brace_escape_preserved_raw`:
the old implementation omitted the closing `}` line, which contained no assertion token). The 8
summary changes are listed individually in the commit message.

| Test Name                   | File                                          | Old Bucket                          | New Bucket                                  | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| --------------------------- | --------------------------------------------- | ----------------------------------- | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending judgment / Pending judgment | Pending judgment / **Retirement candidate** | The actual function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; the file header and function body contain no § section number and no issue/regression/defect/incident ⇒ neither the clause gate nor the incident-history gate passes, and the body contains only weak assertions such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires manual review)                                                          |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident history / Keep             | Clause / **Pending verification**           | The old implementation truncated the function body to the end of the file and swallowed text containing “regression/defect/incident,” falsely causing the incident-history gate to hit; those terms do not occur in the actual function body ⇒ the incident-history gate no longer applies; the file header has verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate hits but the citation has not been confirmed by the sampling audit ⇒ pending verification |

The distribution changes accordingly: clause 1713 → 1714 · incident history 314 → 313 · keep 776 →
775 · pending verification 2121 → 2122 · pending judgment 412 → 411 · retirement candidate 104
→ 105. **The repository-wide `#[test]` count remains unchanged (3413)**—this round changes only the
implementation of the criteria, not any tests.

## Third Retirement Batch (Executed on 2026-11-07)

**Third batch: of the 5 retirement candidates under `src/middle/passes/codegen/tests/`, 2 were
retired and 3 were kept after item-by-item review.** (A single directory cluster = the
`middle::passes::codegen` module under test; criterion = none of the **three gates in Part Seven of
coding-rules** passes.)

This batch follows the **keep criterion** established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus and integration tests belong
to the process-boundary layer and cannot replace a unit observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (apart from the
    function name, once comments are removed): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that the `execute_module` entry
    path returns non-Void, but the body never observes the return value from `execute_module`
    (`run_fixture` discards it with `.expect(...)`)— the stated intent is not implemented, and the
    function body is pure duplication ⇒ zero incremental criterion value. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file covers it strictly:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` is the same shape as the inline `Symbol` in this
    test). “A name that was never inserted cannot be found” and “it cannot be found after being
    popped” use the same path in `SymbolScopeManager::lookup` (walking the layers via
    `scope.get(name)`, returning `None` when not found) ⇒ this test contributes no incremental
    criterion value. Replacement coverage = `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that “does
    not go through serialization to connect directly to execution” (the serialized variant is
    another path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for a
    serialization roundtrip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit observation point for the `id > 255`
    rejection branch of `OperandResolver::to_reg` (the bytecode layer has its own separate overflow
    checks and does not cover this function; no second repository-wide test causes `to_reg` to go
    out of range). `is_err()` is a discriminating assertion (removing the guard would turn the test
    red), not merely proof that nothing panicked.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

### One Census Blind Spot Discovered in This Batch (**Fixed in 0.6-15**; see “Generator Criterion Correction” below)

The census’s “delegated assertion” recognition, `\b(assert|check)[A-Za-z_]*\s*\(`, **could not match
helpers with digits in their names**: `[A-Za-z_]*` does not include digits, so calls such as
`assert_slot_is_42(&interp)` were not recognized as assertions— and 2 of the 5 candidates in this
batch were consequently misbucketed as “no assertions in the body.”

After reviewing all 15 repository-wide “neither gate passes” entries item by item (allowing digits
in helper names and additionally recognizing the `match` + `panic!` form), **4 did in fact contain
assertions**—3 used the numeric helper above (`assert_slot_is_42`), and 1 was `test_weak_from_arc`
in `src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule
6.1 requires `assert!(matches!(...))`, but it does assert). The other 11 truly contained no
assertions.

**Directly removing these 4 based on the machine’s “neither gate passes” result would have wrongly
deleted tests that carry criteria**—this batch’s item-by-item human review prevented that; the
generator-side fix is described in “Generator Criterion Correction (0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots were fixed: digits in helper names, and the `match` + `panic!` form.**
This round **deletes no tests** (the repository-wide `#[test]` count remains 3411); it changes only
how the generator recognizes whether a body contains assertions.

### What Was Fixed

1. **Digits are now allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing such calls were
   treated as having “no assertions in the body.”
2. **Added the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the assertion is in an
   arm, and no `assert!` macro is visible lexically (test-specification rule 6.1 requires
   `assert!(matches!(...))`, but existing code contains this form). Detection method: scan the
   expression after the `match` keyword up to the first `{`, determine the block boundary by
   **balancing braces**, and only count it if `panic!(` appears within the block. **The decision is
   made on masked lines**— braces in strings and comments do not participate in balancing, and
   `panic!` in a comment is not treated as an assertion.

### Bucket Differences: 3 Tests (Gate and Disposition Changed)

| Test Name                                           | File                                           | Old Bucket                                 | New Bucket                          | Basis                                                                                                                                                                                                                                                              |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------------ | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither gate passes / Retirement candidate | Pending judgment / Pending judgment | The body contains `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither gate passes / Retirement candidate | Pending judgment / Pending judgment | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit and the old pattern did not match it; it is now recognized as an assertion                                                                                             |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither gate passes / Retirement candidate | Pending judgment / Pending judgment | Same as above (the serialization-roundtrip test)                                                                                                                                                                                                                   |

Another **77 tests changed only the “assertion summary” column** (the gate and disposition did not
change): most summaries gained `match_panicxN` (the `match` + `panic!` form), and 4 gained
`helperxN` (`src/driver/tests/manifest_error.rs`: 3; `src/package/tests/cache.rs`: 1—helper names
containing digits).

**The relaxation changes only the “neither gate passes” bucket**: that bucket is the machine’s
**confident verdict** (no assertions whatsoever in the body), and its criterion is a hard
requirement; “retirement candidate” is only a pool awaiting manual review, and the size of that pool
is not a verdict. Therefore, all other buckets, including the weak-assertion pool, follow the
original criteria. Result: gate—neither gate passes 14 → 11 · pending judgment 923 → 926;
disposition—retirement candidate 103 → 100 · pending judgment 411 → 414.

### Existing 11 “Neither Gate Passes” Tests: Item-by-Step Review Conclusions

After reading the original source of all 11 repository-wide “neither gate passes” tests, **all 11
truly contain no assertions** (including no `assert*` / `check*` helpers, no `match` + `panic!`
form, and no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                     | Review Conclusion                                       |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | Five `.expect(...)` calls, only guaranteeing that it does not crash                             | Truly no assertions (only proof that it does not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                  | Truly no assertions                                     |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them              | Truly no assertions                                     |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards it                                                     | Truly no assertions                                     |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Truly no assertions                                     |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                              | Truly no assertions                                     |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                        | Truly no assertions (only proof that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Truly no assertions (only proof that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Truly no assertions (only proof that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                            | Truly no assertions                                     |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Truly no assertions                                     |

Five of these (`test_normalizer_context` and the 4 in reducer.rs) were kept because the 0.6-11a
decision table **could not identify replacement coverage**; this review reaches the same conclusion.

### Reverse Spot Check: Did the Relaxation Go Too Far?

Five tests were read in their original source from the 88 newly recognized assertions using a fixed
seed (20261107), to confirm that they really assert:

| Spot-Checked Test                                           | File                                                 | Assertions in the Original Source                                                                                                |
| ----------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matched arm of a `match`; otherwise `panic!("Expected AssocType")`                                 |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matched arm; otherwise `panic!("expected InvalidPackage, got …")`                  |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | `assert_eq!` in the matched arm compares the constraint type; otherwise `panic!("不是 Dynamic 变体…")`                           |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matched arm; otherwise `panic!("Expected TokensDelta variant")`                        |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matched arm verify the error code and message; otherwise `panic!("不是 Disproved…")` |

All 5 assert concrete values or concrete variants (replacing the implementation with another correct
path would turn them red)—**the relaxation did not go too far**. The negative controls were also
added to `--selftest`: no `panic!` in a `match`, and `panic!` appearing only in a comment; neither
may be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

When the generator is rolled back to the old implementation (`HELPER_RE` without digits and with
`match` + `panic!` recognition disabled), all the newly added `--selftest` cases **turn red**, and
the exit code is 1; the new implementation passes all of them.

### Rollback

The generator and census table are in the same commit, so `git revert <sha>` restores the entire
state.

## Fourth Retirement Batch (Executed on 2026-11-07)

**Fourth batch: of the 10 “retirement candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend’s test directories), 1 was retired and 9 were
kept** (a single directory cluster; all three files were read before making any decision: 4 in
`tests/bytecode_load.rs` · 2 in `tests/weak.rs` · 4 in `executor/tests/debug.rs`). Criterion = the
**three gates** in Part Seven of coding-rules; keep criterion = “a unit-level observation point can
only be replaced by another unit test; corpus/integration tests belong to the process-boundary
layer”; **a deletion must identify a specific replacement coverage test name; if it cannot be
identified or understood, keep the test**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, one step, then
  `capture_stack`), and its assertions are **strictly stronger**: it asserts that the stack is
  nonempty and that `stack[0].function_name == "main"` (pinning down the top-of-stack position),
  whereas the listed test asserts only `stack.iter().any(|f| f.function_name == "main")` (any
  position is sufficient) —the former being true implies the latter ⇒ zero incremental criterion
  value.
- **9 kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each in its original
  source, each is the **sole unit observation point** for some property (the separate load
  magic-number/version branches, the YXBC routing layer, the two sides of Weak’s Some/None, stepping
  with no frames, the stack pop in step_out, and an empty stack after running to completion). No
  replacement unit test could be identified ⇒ keep under the hard requirement. See the commit
  message for the item-by-item basis (the decision table is not included in this page).
- **Numbers**: repository-wide `#[test]` 3409 → 3408; 1 deletion = `-#[test]` 1 line and `+#[test]`
  0 line as counted by `git diff`; the dominant test `test_capture_stack_during_execution` passes on
  its own. Census: retirement candidate 99 → 98 · pending judgment 925 → 924 · clause 1713 ·
  incident history 313 · proof 447 · neither gate passes 11 · pending verification 2121 · keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Fifth Retirement Batch (Executed on 2026-11-07)

**Fifth batch: 1353 unit tests were retired in bulk under the mechanical rule “for the same symbol
and result category, keep only the strongest test”** (only unit tests under `src/**/tests/**` were
changed; not a single test on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (mechanically applied, without case-by-case argument)**:
  - Symbol under test = a repository-internal symbol called by the test body (including one level of
    propagation through a helper in the same file); when multiple symbols are reached, classify by
    the **primary symbol** (the first repository-internal call). Symbols are qualified by
    repository-internal path and are not merged across modules by bare name.
  - Result category = a concrete value / a specific error code (E code) / a specific variant match /
    only ok / only err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; within a group of equal strength, keep the
    first occurrence in the file.
  - If a test asserts multiple categories simultaneously (making the primary category
    indeterminate), **keep it** rather than guessing.
- **Three exceptions (not a single test removed from them)**: ① the name or comment explicitly
  mentions `issue #` / regression / defect / incident (incident-history gate); ② a symbol has only
  one test (removing it would leave the symbol without a unit observation point); ③
  `#[should_panic]` is the symbol’s only should_panic case. Incident-history-gate tests 313 and
  “neither gate passes” tests 11 had **zero retirements**.
- **Numbers**: repository-wide `#[test]` 3408 → 2055; 1353 deletions = `-#[test]` 1353 lines and
  `+#[test]` 0 line as counted by `git diff`. Census by gate (current values after the repeal of the
  0.6-19 sampling audit): clause 808 · incident history 313 · proof 424 · pending judgment 499 ·
  neither gate passes 11. Census by disposition (current values): keep 737 · pending verification
  1054 · pending judgment 203 · retirement candidate 61. This batch’s retirement count by gate was:
  clause 902 · pending judgment 428 · proof 23 (0.6-19 reclassified 9 entries previously “confirmed
  by the audit as genuinely supporting” as pending verification, so the current values by gate and
  disposition differ from those at the time the fifth batch was closed).
- **Cleanup after deletion**: helper functions used only by deleted tests, imports that became
  unused, and struct fields that became obsolete were removed as well
  (`cargo clippy --all --all-features --all-targets -- -D warnings` reported 0 warnings).
- **Two out-of-bounds deletions that must be recorded**: 29 tests originally classified as “keep”
  were nevertheless deleted by this batch’s rules—23 current-state anchors (names containing
  unchanged/anchor/guard/preserve/baseline, many of which were false positives caused by **domain
  terms** such as “guard” or “preserves,” for example `test_while_guard_scope_balanced`) and 6
  clause citations confirmed by the audit as genuinely supporting. This batch followed the closed
  list of the card’s three exceptions and did not additionally protect these two categories; to
  restore them, backfill them according to the keep/delete table on this page.
- **Failure and repeal of the sampling audit**: the 40-case sample was drawn from the test pool
  using a fixed seed. After the fifth batch retired tests, 38 entries in the original sample no
  longer existed, so the audit’s `--check` always reported “38 missing, 38 extra.” This sampling
  audit was repealed and deleted in 0.6-19 (the script, data file, and generator dependencies were
  all removed); see “Sampling Audit Repeal” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls it back as a whole.

## Sixth Retirement Batch (Executed on 2026-11-26)

**Sixth batch: the mechanism rollback from the 4.9.9 narrowing round retired 28 tests deleted along
with their mechanism, while 4 prohibition criteria had never been entered into the inventory.** This
batch does not use the three gates to decide whether tests “should be kept.” Instead, the mechanisms
under test were **rolled back wholesale**: the criteria written during the first six rounds of 4.9.9
for downstream ambiguity mechanisms (candidate-provider priority / candidate-signature comparison /
owner-compatibility inference / IR-side named-argument fallback / the E5006 import-name
resolution-point prohibition) were retired along with the mechanisms themselves—the mechanisms no
longer exist, so the tests have no subject under test. Such deletions are registered by batch under
the “batch” provision of coding-rules 7.3 and do not enter the retirement candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 (28 deleted: 27 criteria for
    downstream mechanisms from the first six rounds
    - 1 criterion for alias-import named arguments). The 2 retained tests pin down the minimal fix
      for bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: all 4 E5006 prohibition criteria were retired with
    the file—the file existed only in an intermediate 4.9.9 commit, and neither the parent commit
    nor that version’s committed inventory included it; after the prohibition was withdrawn, there
    was no subject under test, so this batch does not decrement any inventory rows.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the candidate-injection surface for alias imports was rolled back, so the fixture’s
    behavior no longer holds); the same-named corpus differential baseline entry was deleted in the
    same batch. Baseline accounting is recorded separately: **relative to direct parent commit
    `ac810e97`, 405 → 404 lines** (the intermediate state had included that fixture); **relative to
    the pre-4.9.9 baseline `08ccacc5`, 403 → 404 lines** (+1 basic fixture
    `named-args-cross-unit-method`; the retired `alias-overload` never entered the final history).
- **Which gate applies**: the 2 retained tests satisfy the **incident-history gate** (they pin down
  a real defect registered in 4.9.9: cross-unit named arguments could not obtain declaration-time
  formal-parameter names, leaving `E1014 available` empty). Of the 28 retired tests, most criteria
  newly added during the first six rounds originally hit the incident-history gate, but the reason
  for deletion in this batch is **not** the three gates (it is not a judgment that they were
  worthless); the mechanism under test was rolled back.
- **Replacement coverage**: cross-unit named arguments for bare-name imports = the 2 retained
  driver-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity surface
  no longer has rejection logic (the requirement was rolled back), so there is no replacement
  criterion; 4.9.8’s dispatch-attribution criteria (10 driver tests + 5 integration tests) are
  retained unchanged.
- **Restoration**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the empty-name-table guard for
  `method_param_info` is retained; this form is unreachable in the public syntax and is a defensive
  guard, so the unit test exists solely to prevent accidental removal).
- **Numbers** (after this batch’s regeneration): repository-wide `#[test]` 2680 across 334 files;
  the previous committed inventory had 2688 entries → the current tree has 2680 (−8); there is 1
  test in the current tree but not in the old table, 9 tests in the old table but not in the current
  tree, and 0 same-key rows whose contents changed.
- **Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit), so `git revert <sha>` rolls it back as a whole.

## Sampling Audit Repeal (0.6-19)

**Criterion**: the sampling audit (0.6-10) measured the “true support rate of clause citations,” but
after 0.6-18 the retention rule no longer examines citations (it changed to “for the same symbol and
result category, keep only the strongest test,” reducing the test pool from 3408 to 2055). Of the
original 40 samples, 38 no longer existed, so the audit’s `--check` always reported “38 missing, 38
extra”—keeping it would only produce continuous alerts. It was therefore **repealed and deleted**.

**What was deleted**: the sampling-audit script under `scripts/dev/` (the one that reproduced the
40-case sample using a fixed seed) and its JSON data file (the 40 item-by-item decisions and their
bases).

**Generator dependency removed in sync**: the generator previously read that JSON and classified
“confirmed by the audit as genuinely supporting” as “clause gate + keep”; that branch has been
deleted—**clause citations no longer have an “audit-confirmed” tier** and always fall under “clause
/ pending verification.”

**Distribution changes** (all 2055 repository-wide `#[test]` tests **remain unchanged**, and the TSV
was regenerated): keep 746 → 737 (−9: the 9 entries previously confirmed by the audit as genuinely
supporting returned to pending verification); pending verification 1045 → 1054; by gate: clause 811
→ 808, pending judgment 496 → 499 (the others are unchanged: incident history 313 · proof 424 ·
neither gate passes 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit), so `git revert <sha>` rolls it back as a whole.

## Rerunning and Self-Checking

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # Before CI/commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical classification only; the boundaries are stated honestly:

- **Delegated assertions are recognized by name only** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as “no assertions”—the first batch’s 26 tests (21 retired / 5 kept) and the second
  batch’s 18 tests (11 retired / 7 kept) have all had their function bodies manually reviewed.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator extracts a function body, it
  balances braces; braces in strings, character literals, and comments skew the balance and
  therefore truncate the function body—`test_prefix_info_lbrace` (whose body contains
  `tokenize("{")`) was once truncated to the end of the file, causing the “assertion summary” to
  include the remainder of the file and the test to be bucketed as “pending judgment” rather than
  “retirement candidate.” It now masks non-code regions (strings / character literals / line
  comments / block comments / raw strings) before balancing the braces; see “Generator Correction”
  below.
- **Delegated-assertion recognition (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not include digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions—tests containing such calls were
  misclassified as having “no assertions in the body.” At the time, 4 of the 15 repository-wide
  “neither gate passes” tests actually contained assertions (3 numeric helpers + 1 `match` +
  `panic!` form). The name portion has now been relaxed to `[A-Za-z0-9_]*`, and recognition of the
  `match` + `panic!` form has been added; see “Generator Criterion Correction” below.
- **The proof gate is not judged**: every test for which “neither of the first two gates passes” is
  classified only as “pending judgment” or “retirement candidate”; the generator never declares that
  the test “should be deleted.”
- **No cross-file deduplication**: completely duplicate tests are not identified in this table
  (0.6-2 measured them at only 1%, so they do not constitute a retirement surface).
- **Clause citations are always pending verification**: a verifiable section number in the file
  header only means “the test maps to one clause that can be identified by name”; it does not mean
  the citation supports the assertions. The machine does not judge whether it does, so a clause hit
  still stops at “pending verification,” and a human must verify it before the test can be kept.
