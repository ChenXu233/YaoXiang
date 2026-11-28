---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Grand Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement-candidate list plus retirement records for each batch'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Grand Cleanup)

Preparatory material for WBS 0.6, the “Grand Cleanup”: mechanically classify every `#[test]` in the
repository according to the **Three Gates (Clause / Incident History / Proof)** in
[Part Seven of coding-rules](./coding-rules.md), producing a rerunnable per-test determination
table. A 40-entry sample audit of the clause gate measured the proportion of tests that “cited a
clause but whose assertions could not support it” and was used to **tighten the criteria**; that
audit was deprecated and deleted in 0.6-19. **This page and the generator itself are read-only and
do not modify tests**; test retirement is carried out in batches (one commit per batch, fully
revertible with `git revert`), and the results are recorded in the batch retirement subsections
below and in the commit messages.

Generated artifacts:

| Artifact                            | Purpose                                                                                |
| ----------------------------------- | -------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Per-test determination table (2,732 rows + header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                            |

## Criteria

### Scan Surface

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root-package lib (`src/`, including each module’s `tests/` subdirectory) | 2128                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2732** (343 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine Criterion                                                                                                                                                                                                                                                                                                                                                                                                                                             | Meaning of a Hit                                                |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | The first 25 lines of the file contain a **verifiable entry**: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); for files under `src/**/tests/`, inspect the parent `mod.rs` along with the file                                                                                                                                                                                                                    | The test maps to an identifiable clause                         |
| Incident-history gate             | The test body, function doc comment, or **file header** (such as `//! 回归背景：issue #226`) names an actual failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                             | The test pins an incident that has actually occurred            |
| Proof gate                        | **The machine cannot decide this** (“If you replace it with another correct implementation, will it turn red?” requires human judgment). Therefore, anything that passes neither of the first two gates is classified as “pending determination,” not automatically as “neither passes”; the sole exception is a body with **no assertions at all** (including no delegated `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only “pending determination” or “no assertions” can be assigned |

Rationale for tightening: The 0.6-9 clause gate only required the words `§`/`RFC`/`SPEC` to appear
in the file header, so 2,403 entries were classified as “keep.” The 40-entry stratified sample
measured in 0.6-10 found that **half of the citations could not support the assertions**
(mis-citation rate: 50%, 95% interval: 35%–65%)—this measurement was the **direct reason for
abandoning “keep based on a single citation.”** After tightening, only section-number forms are
recognized, and **having a citation still does not mean the citation supports the
assertions**—clause citations always reach only “pending review.” (The sample audit was deprecated
and deleted in 0.6-19: as the test pool shrank through retirement, 38 of the original samples no
longer existed, and `--check` consistently reported “missing 38 entries, extra 38 entries.” The
conclusion is retained, but the audit artifact is no longer maintained.)

Rationale for the file-header radius: This repository conventionally places text such as
“回归背景：issue #N” in the file header rather than repeating it in each test’s comment. Looking
only at the test’s own comment would miss the entire file’s incident history (measured: 7 entries
within the test’s own radius, 291 entries within the file-header radius).

### Categories That Are Never Retired

The following tests are **always classified as “keep”** and never enter the retirement-candidate
pool:

- Corpus `tests/yaoxiang/**` and project-form `tests/project-corpus/**` (behavior-criteria
  surfaces);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and traversing process boundaries);
- Tests whose names contain `unchanged` / `anchor` / `guard` / `preserv` / `baseline`, which serve
  as status anchors.

### Disposition Semantics

| Disposition           | Meaning                                                                                                                                                                                              |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                  | Only two grounds: an incident-history gate hit, or the proof gate (spec-shaped directories / status-anchor names)                                                                                    |
| Pending review        | Cites a clause (with a section number, or citing only an entire RFC without a section number), but the citation has not been manually confirmed—it must be reviewed by a human before it can be kept |
| Retirement candidate  | Passes neither gate and has only weak assertions (or no assertions) in its assertion surface—a candidate is not a verdict, and the proof gate still requires human review                            |
| Pending determination | Passes neither gate but has strong assertions; the machine reaches no conclusion                                                                                                                     |

## Distribution (regenerated 2026-11-28; 4.9.9 narrowing round, 8.7 batch, 4.9.10-L0, init-test rewrite, D53 batch, L3-1b S1, 7e batch, #593 criteria, and L3-1 identity switch)

| Gate                                                                     | Count    |
| ------------------------------------------------------------------------ | -------- |
| Clause (has a verifiable entry)                                          | 1045     |
| Incident history                                                         | 568      |
| Proof (spec-shaped criteria surface / status anchor)                     | 574      |
| Pending determination (passes neither gate; proof awaits human judgment) | 539      |
| Neither passes (no assertions in the body)                               | 6        |
| **Total**                                                                | **2732** |

| Disposition           | Count    | Compared with 0.6-19 (2026-11-07) |
| --------------------- | -------- | --------------------------------- |
| Keep                  | **1142** | 737 (+405)                        |
| Pending review        | 1306     | 1054 (+252)                       |
| Pending determination | 225      | 203 (+22)                         |
| Retirement candidate  | 59       | 61 (−2)                           |
| **Total**             | **2732** | 2055 (+677)                       |

Composition of “keep”: 1142 = proof gate 574 (spec-shaped criteria surface 518 + status
anchors 56) + incident-history gate 568. File-level metric: Among 345 test files, the **clause
gate** covers **128**.

Regeneration reconciliation: This regeneration matches the checked-in table key for key (0 added, 0
removed, 0 content recomputations), with 2,732 rows. Per-key records are in the TSV.

The latest full regeneration (after the L3-1 identity switch and lower finalization) produced 2,732
rows and matches the current tree key for key (`--check` exits 0).

### Two Criteria Changes at the 0.6-19 Tightening Point

1. **Composition of “keep”**: 737 = proof gate 424 (spec-shaped criteria surface 390 + status
   anchors 34) + incident-history gate 313. Of the original 2,897 entries, the 2,121 that had been
   kept “on the strength of a single citation” now fall into pending review (after the sample audit
   was deprecated in 0.6-19, the 9 previously classified as “audit-confirmed genuine support” also
   fell into pending review—clause citations no longer have an “audit-confirmed” tier).
2. **Incident-history gate 47 → 313**: These are not newly added incident histories, but **label
   reassignment**—in 0.6-9, a simultaneous clause and incident-history hit was recorded as a clause.
   After tightening, a clause no longer automatically meant “keep,” so these entries had to be
   caught by the incident-history gate, and their count was folded into that gate. The total number
   kept was therefore unaffected (this batch was also kept in 0.6-9).

File-level metric at the time: Among 285 test files, **136** had a verifiable section number in the
file header (the old 0.6-9 criterion identified 221).

## Reconciliation Between the 0.6-19 Point and the 0.6-2 Survey

The 0.6-2 survey figures form the first comparison baseline. The reasons for each difference are
listed individually:

| Item                            | 0.6-2 Survey                  | This Run                                                                                                      | Reason for the Difference                                                                                                                                                                                                                                                                                                         |
| ------------------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                          | 2 entries were added after 0.6-2 (check-criterion regression tests for E3005 and E3022); 21 no-assertion tests were removed according to the first batch, 11 weak-assertion tests according to the second batch, 2 according to the third batch, 1 according to the fourth batch, and 1353 according to the fifth batch; net −388 |
| Number of test files            | 296                           | 285                                                                                                           | 11 entire files retired in the fifth batch                                                                                                                                                                                                                                                                                        |
| Clause gate                     | 227 files (77%)               | 136 files after tightening; 808 entries                                                                       | The criterion was tightened from “the word appears” to “a section number `§<number>` is present,” nearly halving the file count; the fifth batch retired 902 entries, and after the 0.6-19 sample-audit deprecation another 9 moved from “keep” to pending review                                                                 |
| Incident-history signals        | 442 entries / 148 files       | 313 entries in the gate; three radii: test’s own 7 / file header 291 (38 files) / entire file 1218 (79 files) | The 148 files in the 0.6-2 survey were broader even than the 79 files in this run’s entire-file radius, and its vocabulary/scope was larger; this run strictly uses the four card-face signals                                                                                                                                    |
| Characterization strong signals | 1176 (34%) / strong 631 (18%) | Not tested                                                                                                    | Characterization is not one of the three gates, and mixing it into the retirement conclusion would exceed the gate’s authority                                                                                                                                                                                                    |
| Weak assertions                 | 473 (14%)                     | See the “assertion summary” column in the TSV                                                                 | The counting units differ: 0.6-2 counted patterns that asserted only `is_ok`/`is_some`, whereas this run uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                                                                                           |

## 0.6-19 Conclusion Summary (Historical)

1. **The clause gate had previously been significantly inflated**: Under the old criterion, 2,403
   entries were classified as “clause/keep.” The sample measurement found a 50% mis-citation rate
   (95% interval: 35%–65%)—“citation” and “citation supporting the assertions” are different things,
   and the old criterion treated the former as the latter. (The sample audit was deprecated and
   deleted in 0.6-19; its conclusion is retained. See “Sample Audit Deprecation” below for the
   reason for deletion.)
2. **After tightening, approximately 1,202 entries genuinely had clause support** (range: 846–1557);
   the rest either await review to supply the missing evidence or should be judged through the
   incident-history/proof gates.
3. **The retirement surface is unrelated to clause mis-citations**: The 136 candidates were
   unrelated to clauses (no assertions/weak assertions). The first batch retired 21 entries, the
   second retired 11, the generator correction added 1, the third batch removed 2, the 0.6-15
   criterion correction removed 3, the redundancy scan removed 1, the fourth batch removed 1, and
   the fifth batch removed 1353 (using the mechanical rule “for the same symbol and result category,
   keep only the strongest entry”), reducing the candidate count to 61. The fifth batch’s retirement
   count by gate was: clause 902 · pending determination 428 · proof 23 · incident history 0 ·
   neither passes 0—**not a single incident-history gate or zero-assertion entry was removed**.
4. **The incident-history gate is the largest legitimate source of “keep”** (313 entries): it
   demonstrates that “pinning an actual incident” is more reliable than “copying a line from an
   RFC.”

## First Retirement Batch (Executed, 2026-11-06)

**First batch: retire 21 and keep 5 among the 26 “no-assertion” tests in
`src/frontend/core/types/eval/tests/`** (a single directory cluster; criterion = all **Three Gates
fail** under Part Seven of coding-rules: no clause gate, no incident-history gate, and no assertions
in the body). The per-test determinations and “same-slot coverage” are given by the 0.6-11a
determination table (delete 21 / keep 5); this page does not repeat the reasons and only records the
execution results.

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
- **5 kept** (the determination table could not identify replacement coverage, so they were kept
  under the hard requirement): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), and `git revert <sha>` rolls it back in full.

## Second Retirement Batch (Executed, 2026-11-06)

**Second batch: retire 11 and keep 7 among the 18 “weak-assertion” tests in
`src/frontend/core/parser/pratt/tests/nud.rs`** (a single directory cluster = the Pratt
prefix-parsing module; criterion = all **Three Gates fail** under Part Seven of coding-rules: no
clause gate, no incident-history gate, and the body contains only weak assertions such as
`is_some()` / `is_none()`—it does not lock down binding power or the parse result).

The difference between this batch and the first must be stated clearly: these tests **contain
assertions** and are not empty tests. The reason for retirement is “all Three Gates fail + zero
criterion increment,” not “no assertions”—every test selected for deletion identifies a **unit-level
replacement coverage test name in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Every replacement
  follows the **same** `parse_expression → parse_prefix → prefix_info` path and makes stronger
  assertions (locking down the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, as well as `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—the per-test reasons are in the commit message (task 0.6, second
  test-retirement batch). The reasons fall into two categories: **the proof gate holds** (the first
  three, along with `test_prefix_info_at` / `test_prefix_info_eof`, are the corresponding facts’
  only walkthrough points **at the unit level**, and no second unit test in the repository observes
  them), and **the incident-history gate holds** (`break_removed` / `continue_removed` name #314).
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), and `git revert <sha>` rolls it back in full.

## Generator Correction (0.6-13)

**`body_span` encounters braces inside literals and truncates the function body — fixed.**

The old implementation obtained the function body by balancing braces without distinguishing real
code from string/char literals or braces in comments. Forms such as `tokenize("{")` are common in
Rust source, so the balance was thrown off and the function body was truncated: the body of
`test_prefix_info_lbrace` was once truncated to the end of the file, causing the “assertion summary”
to pick up the remainder of the file (`matchesx11 assertx24 …`) and be classified as “pending
determination” rather than “retirement candidate”—**a misclassification**.

Fix: Added `mask_noncode`, which first replaces the contents of string literals / char literals /
line comments / block comments (nestable) / raw strings (`r"…"` / `r#"…"#` / `br"…"` / `cr#"…"#`)
with spaces (newlines are preserved as-is, and line numbers and line counts do not change), then
balances the braces. **The mask is used only to find function-body boundaries**; assertion counting
and the incident-history gate still read the original source, and the criterion itself was not
relaxed. `--selftest` gained 4 cases (paired/unpaired braces in ordinary strings, braces in raw
strings and block comments, and braces in char literals), each followed by a sentinel test—the old
implementation obtained the wrong boundary in all 4 cases, while the new implementation obtains all
of them correctly.

### Tests Affected by the Same Bug: 11 Entries

Counting method: Compare the **function-body boundaries before and after the fix** test by test and
file by file (the old implementation balanced braces literally, while the new implementation
balances them after masking); any entry whose boundaries differ is affected. Of the 3,413 tests in
the repository, the old implementation obtained the wrong boundary for **11**:

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

### Classification Differences Before and After the Fix: 2 Entries

For the other 9 entries, **8 affected only the “assertion summary” column** (classification did not
change), while **1 did not change even the summary**
(`test_fstring_close_brace_escape_preserved_raw`: the old implementation omitted the closing `}`
line, which contains no assertion keyword). The 8 changed summaries are listed individually in the
commit message.

| Test Name                   | File                                          | Old Classification                            | New Classification                               | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| --------------------------- | --------------------------------------------- | --------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending determination / Pending determination | Pending determination / **Retirement candidate** | The real function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; the file header and body contain no § section number or issue/regression/defect/incident wording ⇒ both the clause gate and incident-history gate fail, and the body contains only a weak assertion such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires human review)                                                 |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident history / Keep                       | Clause / **Pending review**                      | The old implementation truncated the body to the end of the file, swallowed text containing “regression / defect / incident,” and falsely classified it under the incident-history gate; the real function body contains no such wording ⇒ the incident-history gate no longer holds; the file header has verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate is hit but has not been confirmed by the sample audit ⇒ pending review |

The distribution changed accordingly: clause 1713 → 1714 · incident history 314 → 313 · keep 776 →
775 · pending review 2121 → 2122 · pending determination 412 → 411 · retirement candidate 104 → 105.
**The repository-wide `#[test]` count did not change (3413)**—this round changed only the
implementation of the criteria, not any tests.

## Third Retirement Batch (Executed, 2026-11-07)

**Third batch: adjudicate the 5 retirement candidates in `src/middle/passes/codegen/tests/`,
retiring 2 and keeping 3.** (a single directory cluster = the `middle::passes::codegen` module under
test; criterion = the **Three Gates fail** under Part Seven of coding-rules.)

This batch follows the **keep criterion** established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus/integration tests belong to
the process-boundary layer and cannot replace a unit observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is **verbatim
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (after comments are
    removed, only the function name differs): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that “the `execute_module` entry
    path returns non-Void,” but the body never observes `execute_module`’s return value
    (`run_fixture` discards it via `.expect(...)`)— the claimed intent is not implemented, and the
    body is a pure duplicate ⇒ zero criterion increment. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file strictly covers it:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the inserted `Symbol` shape (`symbol("a", Int(64), Local(0), false, 0)`
    has the same shape as the inline `Symbol` in this test). “A name that was never inserted cannot
    be found” and “it cannot be found after being popped” follow the same path in
    `SymbolScopeManager::lookup` (iterating through `scope.get(name)` and returning `None` when not
    found) ⇒ this test adds zero increment. Replacement coverage = `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip`—the only observation point for the path that “does
    not go through serialization to execute directly” (the serialization variant is a different
    path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the only observation point for the
    serialization roundtrip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **only** unit observation point for
    `OperandResolver::to_reg`’s `id > 255` rejection branch (the bytecode layer has its own
    independent overflow check and does not cover this function; no second test in the repository
    drives `to_reg` out of range). `is_err()` is a discriminating assertion (removing the guard
    would turn it red), not merely proof that it did not panic.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), and `git revert <sha>` rolls it back in full.

### Census Blind Spot Found in This Batch (**Fixed in 0.6-15**, see “Generator Criterion Correction” below)

The census’s recognition of “delegated assertions,” `\b(assert|check)[A-Za-z_]*\s*\(`, **could not
match helper names containing digits**: `[A-Za-z_]*` does not include digits, so a call such as
`assert_slot_is_42(&interp)` **was not recognized as an assertion**— 2 of the 5 candidates in this
batch were consequently misclassified as “no assertions in the body.”

After reviewing all 15 repository-wide “neither passes” entries (broadening helper names to include
digits and additionally recognizing the `match` + `panic!` form), **4 actually had assertions**—3
were the numeric helpers above, and 1 was `test_weak_from_arc` in
`src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule 6.1
requires `assert!(matches!(...))`, but it does in fact assert). The remaining 11 genuinely had no
assertions.

**If these 4 had been removed directly under the machine’s “neither passes” classification, tests
with criteria would have been wrongly deleted**—they were saved by the per-test human review in this
batch; the corresponding generator-side fix is described under “Generator Criterion Correction
(0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots were fixed: helper names containing digits, and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411); it
changes only how the generator recognizes whether a body contains assertions.

### What Was Fixed

1. **Digits were allowed in delegated-assertion helper names**: `\b(assert|check)[A-Za-z_]*\s*\(` →
   `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could not match helper names such as
   `assert_slot_is_42(&interp)`, so a test containing such a call was treated as “the body contains
   no assertions.”
2. **Added the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the assertion is written
   in one arm, so the `assert!` macro is not visible lexically (test-specification rule 6.1 requires
   `assert!(matches!(...))`, but existing code contains this form). Recognition method: scan the
   expression after the `match` keyword up to the first `{`, then determine the opened block’s
   boundary through **brace balancing**; the form counts only if `panic!(` appears inside the block.
   **The determination is made on masked lines**— braces in strings and comments do not participate
   in balancing, and `panic!` in a comment is not treated as an assertion.

### Classification Differences: 3 Entries (Gate and Disposition Changed)

| Test Name                                           | File                                           | Old Classification                    | New Classification                            | Basis                                                                                                                                                                                                                                                        |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither passes / Retirement candidate | Pending determination / Pending determination | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending determination / Pending determination | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern could not match; it is now recognized as an assertion                                                                                     |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending determination / Pending determination | Same as above (the serialization-roundtrip entry)                                                                                                                                                                                                            |

A further **77 entries changed only the “assertion summary” column** (their gate and disposition did
not change): most gained `match_panicxN` in the summary (the body contains a `match` + `panic!`
form), and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3;
`src/package/tests/cache.rs`: 1—helper names containing digits).

**The relaxation changes only the “neither passes” tier**: That tier is the machine’s **confident
verdict** (not a single assertion in the body), and its criterion is a hard requirement; “retirement
candidate” is only a pool awaiting human review, so the size of that pool is not a verdict. The
weak-assertion pool and all other classifications continue to use the original criterion. Result:
gates—neither passes 14 → 11 · pending determination 923 → 926; dispositions—retirement candidate
103 → 100 · pending determination 411 → 414.

### Existing 11 “Neither Passes” Entries: Individual Review Conclusions

After reading all 11 repository-wide “neither passes” entries in the original source, **all 11
genuinely contain no assertions** (including no `assert*` / `check*` helpers, no `match` + `panic!`
form, and no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                     | Review Conclusion                                      |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | Uses `.expect(...)` in 5 places, only guaranteeing that it does not crash                       | Genuinely no assertions (only proves it did not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                  | Genuinely no assertions                                |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer`s, obtains their contexts, and discards them                       | Genuinely no assertions                                |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Genuinely no assertions                                |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Genuinely no assertions                                |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                              | Genuinely no assertions                                |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                        | Genuinely no assertions (only proves it did not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Genuinely no assertions (only proves it did not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Genuinely no assertions (only proves it did not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                            | Genuinely no assertions                                |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertions                                |

Of these, 5 (`test_normalizer_context` and the 4 entries in reducer.rs) were kept because the
0.6-11a determination table **could not identify replacement coverage**; this review reaches the
same conclusion.

### Reverse Spot Check: Did the Relaxation Go Too Far?

Five entries were selected with a fixed seed (20261107) from the 88 entries “newly recognized as
assertions,” and their original source was read to confirm that they really assert:

| Spot-Checked Entry                                          | File                                                 | Assertion in the Original Source                                                                                                       |
| ----------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching arm, with `panic!("Expected AssocType")` otherwise                                              |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm, with `panic!("expected InvalidPackage, got …")` otherwise                  |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | `assert_eq!` in the matching arm compares the constraint type, with `panic!("不是 Dynamic 变体…")` otherwise                           |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm, with `panic!("Expected TokensDelta variant")` otherwise                        |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm verify the error code and message, with `panic!("不是 Disproved…")` otherwise |

All 5 assert a concrete value or concrete variant (replacing the implementation with another correct
path would make them fail)—**the relaxation did not go too far**. Reverse controls were also added
to `--selftest`: a `match` without `panic!`, and a `panic!` appearing only in a comment; neither may
be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

When the generator was reverted to the old implementation (`HELPER_RE` without digits + `match` +
`panic!` recognition disabled), all the newly added `--selftest` cases **failed**, with exit code 1;
the new implementation passes all of them.

### Rollback

The generator and census table are in the same commit, and `git revert <sha>` restores everything in
full.

## Fourth Retirement Batch (Executed, 2026-11-07)

**Fourth batch: retire 1 and keep 9 among the 10 “retirement candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend’s test directory)** (a single directory
cluster; all three files were read before making any determination: `tests/bytecode_load.rs`: 4 ·
`tests/weak.rs`: 2 · `executor/tests/debug.rs`: 4). Criterion = the **Three Gates** under Part Seven
of coding-rules; keep criterion = “a unit-level observation point can only be replaced by another
unit test; corpus/integration tests belong to the process-boundary layer”; **deletion requires
identifying a specific replacement coverage test name; if it cannot be identified or understood, the
test is kept**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, one additional step,
  then obtain `capture_stack`), and its assertions are **strictly stronger**: it asserts that the
  stack is nonempty and that `stack[0].function_name == "main"` (pinning down the top-of-stack
  position), while the listed test asserts only `stack.iter().any(|f| f.function_name == "main")`
  (any position is sufficient) —the former being true implies the latter ⇒ zero criterion increment.
- **9 kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each one, they are
  individually the **only unit observation point** for some property (the load magic-number/version
  branches are distinct, the YXBC routing layer, both the Some/None sides of Weak, stepping with no
  frames, step_out popping the stack, and running to completion with an empty stack). No unit test
  capable of replacing them could be identified ⇒ kept under the hard requirement. The per-test
  rationale is in the commit message (the determination table is not included in this page).
- **Numbers**: Repository-wide `#[test]`: 3409 → 3408; 1 deleted = `git diff` counts `-#[test]` on 1
  line and `+#[test]` on 0 lines; the dominating test `test_capture_stack_during_execution` passes
  when run on its own. Census: retirement candidate 99 → 98 · pending determination 925 → 924 ·
  clause 1713 · incident history 313 · proof 447 · neither passes 11 · pending review 2121 ·
  keep 775.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), and `git revert <sha>` rolls it back in full.

## Fifth Retirement Batch (Executed, 2026-11-07)

**Fifth batch: mechanically retire 1,353 unit tests using the rule “for the same symbol and result
category, keep only the strongest entry”** (only unit tests under `src/**/tests/**` were changed;
not a single entry on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (executed mechanically, without per-test argument)**:
  - Tested symbol = a repository symbol invoked in the test body (including one level of propagation
    through helper functions in the same file); when multiple symbols are reached, classify by the
    **primary symbol** (the first repository invocation). Symbols are qualified by repository path
    and are not merged across modules by bare name.
  - Result category = a concrete value / a particular error code (E code) / a particular variant
    match / only ok / only err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when entries are equally strong, keep the
    first one in the file.
  - If a test asserts multiple categories (making the primary category indeterminable), **keep it**
    rather than guessing.
- **Three exceptions (not one entry was removed under them)**: ① the name or comment names `issue #`
  / regression / defect / incident (incident-history gate); ② a symbol has only one test (removing
  it would leave the symbol without a unit observation point); ③ `#[should_panic]` and the symbol’s
  only should_panic case. **0 entries from the 313-entry incident-history gate and the 11-entry
  “neither passes” tier were retired.**
- **Numbers**: Repository-wide `#[test]`: 3408 → 2055; 1,353 deleted = `git diff` counts `-#[test]`
  on 1353 lines and `+#[test]` on 0 lines. Census (by gate, current values after the 0.6-19
  sample-audit deprecation): clause 808 · incident history 313 · proof 424 · pending determination
  499 · neither passes 11. Census (by disposition, current values): keep 737 · pending review 1054 ·
  pending determination 203 · retirement candidate 61. This batch’s retirement count by gate was:
  clause 902 · pending determination 428 · proof 23 (because 0.6-19 reclassified 9 entries
  “confirmed by the audit as genuinely supporting their clauses” as pending review, the current
  gate/disposition values differ from those at the fifth batch’s original close).
- **Cleanup alongside deletion**: Helper functions used exclusively by deleted tests, imports that
  became invalid, and struct fields were removed as well
  (`cargo clippy --all --all-features --all-targets -- -D warnings` produced 0 warnings).
- **Two out-of-bounds deletions that must be recorded**: There were 29 entries originally classified
  as “keep” that this batch’s rules nevertheless deleted—23 status anchors (names containing
  unchanged/anchor/guard/preserve/baseline, many of which were **domain-term** false positives such
  as `test_while_guard_scope_balanced`) and 6 entries whose clause support had been confirmed by the
  audit. This batch followed the closed list of three exceptions on the card face and did not
  additionally protect these two categories; to restore them, simply fill them back in using the
  keep/delete comparison table on this page.
- **Failure and deprecation of the sample audit**: The 40-entry sample was drawn from the test pool
  with a fixed seed. After the fifth batch’s retirement, 38 entries from the original sample no
  longer existed, so the audit’s `--check` consistently reported “missing 38 entries, extra 38
  entries.” The sample audit was deprecated and deleted in 0.6-19 (the script, data file, and the
  generator’s dependency on it were all removed); see “Sample Audit Deprecation” below.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), and `git revert <sha>` rolls it back in full.

## Sixth Retirement Batch (Executed, 2026-11-26)

**Sixth batch: the mechanism rollback from the 4.9.9 narrowing round—retire 28 tests deleted along
with their mechanism, while 4 prohibition criteria had never entered the repository.** This batch
does not use the Three Gates to decide “whether a test should remain.” Instead, the mechanism under
test was **rolled back in full**: the criteria written for the first six rounds of the downstream
ambiguity mechanism (candidate-provider priority / candidate-signature comparison /
owner-compatibility inference / IR-side named-argument fallback / E5006 import-name-resolution
prohibition) were retired along with the mechanism. Without the mechanism, the tests have no subject
under test. These deletions are registered as a batch under coding-rules 7.3 and do not enter the
retirement-candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 (28 deleted: 27 downstream-mechanism
    criteria from the first six rounds
    - 1 alias-import named-argument criterion). The 2 retained entries pin down the minimal fix for
      bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: all 4 E5006 prohibition criteria were retired with
    the file—this file existed only in an intermediate 4.9-9 commit and appeared in neither the
    parent commit nor that version’s inventory table. After the prohibition was withdrawn, there was
    no subject under test, so this batch does not reduce the inventory table.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the candidate-injection surface for alias imports was rolled back, so the fixture’s
    behavior no longer holds); the corpus differential-baseline entry with the same name was deleted
    in the same batch. Baseline accounting is stated separately: **relative to direct parent commit
    `ac810e97`: 405 → 404 lines** (the intermediate state had contained the fixture); **relative to
    the pre-4.9.9 baseline `08ccacc5`: 403 → 404 lines** (+1 base fixture
    `named-args-cross-unit-method`; the retired `alias-overload` never entered the final history).
- **Which gate**: The 2 retained entries = **incident-history gate** (they pin down an actual defect
  registered in 4.9.9: cross-unit named arguments could not obtain declaration-time parameter names,
  leaving `E1014 available` empty). Of the 28 retired entries, most of the downstream-mechanism
  criteria originally hit the incident-history gate, but the reason for this batch’s deletion is
  **not** the Three Gates (it is not a judgment that they lacked value); the mechanism was rolled
  back.
- **Replacement coverage**: Cross-unit named arguments with bare-name imports = the 2 retained
  driver-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity surface
  no longer has rejection logic (the requirement was rolled back), so there is no replacement
  criterion; the 4.9.8 dispatch-attribution criteria remain unchanged (10 driver + 5 integration).
- **Restoration**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the `method_param_info`
  empty-name-table guard is retained; this form is unreachable in the public syntax and is a
  defensive guard whose unit test prevents accidental deletion).
- **Numbers** (after this batch’s regeneration): Repository-wide `#[test]`: 2680 entries / 334
  files; previous inventory table: 2688 entries → current tree: 2680 entries (−8); 1 entry exists in
  the current tree but not the old table, 9 entries exist in the old table but not the current tree,
  and 0 same-key rows required content recomputation.
- **Rollback**: The entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit), and `git revert <sha>` rolls it back in full.

## Sample Audit Deprecation (0.6-19)

**Criterion**: The sample audit (0.6-10) measured the “genuine support rate” of clause citations,
but the keep rule after 0.6-18 no longer looks at citations (it changed to “for the same symbol and
result category, keep only the strongest entry,” and the test pool shrank from 3408 to 2055). Of the
original 40 samples, 38 no longer existed, so the audit’s `--check` consistently reported “missing
38 entries, extra 38 entries”—keeping it would only produce a continuous stream of alarms.
Therefore, it was **deprecated and deleted**.

**What was deleted**: The sample-audit script under `scripts/dev/` (the one that reproduced the 40
samples with a fixed seed) and its JSON data file (the per-test determinations and rationale for all
40 entries).

**The generator’s dependency was removed at the same time**: The generator previously read that JSON
and classified “audit-confirmed genuine support” as “clause gate + keep”; that branch has been
deleted—**clause citations no longer have an “audit-confirmed” tier** and always fall into “clause /
pending review.”

**Distribution changes** (all 2055 repository-wide `#[test]` entries are unchanged, and the TSV was
regenerated): keep 746 → 737 (−9: the 9 entries previously confirmed by the audit as genuinely
supporting their clauses fell back into pending review); pending review 1045 → 1054; by gate: clause
811 → 808, pending determination 496 → 499 (all others unchanged: incident history 313 · proof 424 ·
neither passes 11).

**Rollback**: The entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit), and `git revert <sha>` rolls it back in full.

## Rerunning and Self-Checks

```bash
python scripts/dev/test-gate-census.py            # Recalculate and overwrite the TSV (two runs produce byte-identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-checks (does not scan the repository)
```

The generator performs lexical determination only, and its boundaries are stated explicitly:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test hides its assertion in a helper whose name lacks these two prefixes, it will be misclassified
  as “no assertions”—the first batch’s 26 entries (retire 21 / keep 5) and the second batch’s 18
  entries (retire 11 / keep 7) have all had their function bodies manually reviewed.
- **Function-body boundaries (fixed in 0.6-13)**: When the generator obtains a function body, it
  balances braces. Braces in string/char literals and comments throw off the balance and truncate
  the body—`test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was once truncated to the
  end of the file, causing the “assertion summary” to pick up the remainder of the file and be
  classified as “pending determination” rather than “retirement candidate.” The generator now first
  masks non-code regions (strings / char literals / line comments / block comments / raw strings)
  and then balances the braces; see “Generator Correction” below.
- **Delegated-assertion recognition (fixed in 0.6-15)**: The old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` has no digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions—any test containing such a call
  would be misclassified as “no assertions in the body.” At the time, 4 of the repository-wide 15
  “neither passes” entries actually had assertions (3 numeric helpers + 1 `match` + `panic!` form).
  The name portion has now been broadened to `[A-Za-z0-9_]*`, and the `match` + `panic!` form is
  also recognized; see “Generator Criterion Correction” below.
- **The proof gate is not decided automatically**: Any test that “passes neither gate” falls only
  into “pending determination” or “retirement candidate”; the generator never declares that it
  “should be deleted.”
- **No cross-file deduplication**: Exactly duplicated tests are not identified in this table (0.6-2
  measured them at only 1%, which does not constitute a retirement surface).
- **Clause citations always require review**: A verifiable section number in the file header only
  means that the test “maps to an identifiable clause”; it does not mean the citation supports the
  assertions. The machine does not judge whether it does, so even a clause-gate hit reaches only
  “pending review” and must be reviewed by a human before it can be kept.
