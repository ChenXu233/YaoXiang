---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Great Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement-candidate list and retirement records for each batch'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Great Cleanup)

Preparatory material for WBS 0.6 “Great Cleanup”: mechanically classify every `#[test]` in the
repository according to the **Three Gates (Clause / Incident History / Proof)** in
[Part Seven of coding-rules](./coding-rules.md), producing an item-by-item decision table that can
be rerun; a sample audit of the Clause Gate (40 entries) once measured the proportion that “cited a
clause but could not support their assertions” and consequently **tightened the criteria**; that
audit was retired and deleted in 0.6-19. **This page and the generator itself only read tests and do
not modify them**; test retirement is executed in batches (one commit per batch, revertible as a
whole with `git revert`), and the execution results are recorded in the retirement subsections for
each batch below and in the commit messages.

Generated artifacts:

| Artifact                            | Purpose                                                                              |
| ----------------------------------- | ------------------------------------------------------------------------------------ |
| `docs/src/dev/test-gate-census.tsv` | Item-by-item decision table (2701 rows + header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (can be rerun, supports `--check`, supports `--selftest`)                  |

## Criteria

### Scan Surface

Every `#[test]` in all `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module's `tests/` subdirectory) | 2097                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2701** (337 files) |

### Machine Criteria for the Three Gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Meaning of a hit                                                |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Clause Gate (tightened in 0.6-10) | Within the first 25 lines of the file header, a **verifiable clause entry** appears: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); files under `src/**/tests/` are examined together with their parent `mod.rs`                                                                                                                                                                                                               | The test can be matched to a specific, nameable clause          |
| History Gate                      | The test function body, function doc comment, or **file header** (`//! Regression context: issue #226` and similar) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                      | The test pins down an incident that has actually occurred       |
| Proof Gate                        | **Cannot be determined mechanically** (“If replaced with another correct implementation, would it turn red?” requires human judgment). Therefore, anything that fails the first two gates is always classified as “pending determination,” never automatically as “fails all gates”; the sole exception is when the body contains **not a single assertion** (including no delegation to `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only “pending determination” or “no assertions” can be assigned |

Rationale for tightening: the Clause Gate in 0.6-9 only required the file header to **contain
`§`/`RFC`/`SPEC`**, so 2403 entries were classified as “Keep.” The 40-entry stratified sample
measured in 0.6-10 showed that **half of the citations did not support their assertions**
(mis-citation rate 50%, 95% interval 35%–65%)— this measurement was the **direct reason for
abandoning “keep based on a citation alone.”** After tightening, only section-number forms are
accepted, and **having a citation still does not mean that the citation supports the assertion**—all
clause citations are classified only as “Pending verification.” (The sample audit was retired and
deleted in 0.6-19: as the audit pool shrank along with test retirement, 38 of the original 40
samples no longer existed, and `--check` consistently reported “38 missing, 38 extra”; the
conclusion is retained, but the audit artifacts are no longer maintained.)

Rationale for the file-header radius: the repository convention is to write “Regression context:
issue #N” in the file header and not repeat it in the comments for individual tests; looking only at
each test's own comments would miss the entire file's incident history (measured: 7 entries at the
test's own radius, 291 at the file-header radius).

### Categories That Are Never Retired

The following tests are **always “Keep”** and do not enter the retirement-candidate pool:

- Corpus `tests/yaoxiang/**` and project-shape `tests/project-corpus/**` (behavior-criterion
  surfaces);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and crossing the process boundary);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Meaning of Each Disposition

| Disposition           | Meaning                                                                                                                                                                                               |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                  | Only two cases are assigned: the History Gate is hit, or the Proof Gate applies (spec-shaped directory / current-state anchor name)                                                                   |
| Pending verification  | A clause is cited (with a section number, or by citing an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a human before it can be kept |
| Retirement candidate  | Both gates fail and the assertion surface contains only weak assertions (or none)—a candidate is not a verdict; the Proof Gate still requires manual review                                           |
| Pending determination | Both gates fail, but strong assertions are present; the machine reaches no conclusion                                                                                                                 |

## Distribution (Regenerated 2026-11-27; 4.9.9 Narrowing Round, Batch 8.7, 4.9.10-L0, init Test Rewrite, and D53 Batch)

| Gate                                                                    | Count    |
| ----------------------------------------------------------------------- | -------- |
| Clause (has a verifiable entry)                                         | 1037     |
| History                                                                 | 565      |
| Proof (spec-shaped criterion surface / current-state anchor)            | 571      |
| Pending determination (both gates fail; Proof Gate awaits human review) | 522      |
| Fails all gates (no assertions in the body)                             | 6        |
| **Total**                                                               | **2701** |

| Disposition           | Count    | Compared with 0.6-19 (2026-11-07) |
| --------------------- | -------- | --------------------------------- |
| Keep                  | **1136** | 737 (+399)                        |
| Pending verification  | 1284     | 1054 (+230)                       |
| Pending determination | 223      | 203 (+20)                         |
| Retirement candidate  | 58       | 61 (−3)                           |
| **Total**             | **2701** | 2055 (+646)                       |

Structure of Keep: 1136 = Proof Gate 571 (spec-shaped criterion surfaces 518 + current-state
anchors 53) + History Gate 565. File-level scope: among 337 test files, the **Clause Gate** covers
**125**; **147** can be detected as having a verifiable section number (`§` + digits) by scanning
the first 25 lines of the file header (including the parent `mod.rs` for `src/**/tests*/`).

Regeneration reconciliation: the previous checked-in table had 2690 entries, while the current tree
has 2701; 11 are in the current tree but not the old table (D53 batch additions: parser 7 / sema 3 /
formatter 1), 0 are in the old table but not the current tree, and 0 same-key rows had their
contents recomputed. Per-key records are in the TSV.

Latest full regeneration (after the test rewrite in `src/package/commands/tests/init.rs`): the row
count is unchanged (2690), and only two assertion summaries were recomputed (the `assert_eq` /
`unwrap` counts for `test_init_here_fails_when_project_already_exists` and
`test_init_here_preserves_preexisting_files`); the TSV is consistent with the current tree
(`--check` exits 0).

### Two Changes in Criteria at the 0.6-19 Tightening Point

1. **Structure of Keep**: 737 = Proof Gate 424 (spec-shaped criterion surfaces 390 + current-state
   anchors 34) + History Gate 313. Of the original 2897 entries, the 2121 that had been “kept based
   on a single citation” are now Pending verification (after the sample audit was retired in 0.6-19,
   the 9 entries that had been classified as “audit-confirmed as genuinely supported” were also
   moved to Pending verification—clause citations no longer have an “audit-confirmed” tier).
2. **History Gate 47 → 313**: these are not new incident histories, but **label reassignment**—in
   0.6-9, entries that hit both the Clause and History Gates were recorded under Clause; after
   tightening, Clause no longer automatically meant Keep, so these had to be caught by the History
   Gate, and the counts were folded into History. The total number kept is therefore unaffected
   (these tests were also kept in 0.6-9).

File-level scope at the time: among 285 test files, **136** had a verifiable section number in the
file header (the old 0.6-9 criterion found 221).

## Reconciliation Between the 0.6-19 Snapshot and the 0.6-2 Survey

The 0.6-2 survey figures are the first-version baseline for comparison. The reasons for each
difference are listed below:

| Item                           | 0.6-2 survey                  | Current                                                                                                            | Reason for the difference                                                                                                                                                                                                                                                                                   |
| ------------------------------ | ----------------------------- | ------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`      | 3443                          | 2055                                                                                                               | Two entries were added after 0.6-2 (check-criterion regression tests for E3005 and E3022), followed by deletion of 21 no-assertion tests under the first batch, 11 weak-assertion tests under the second batch, 2 under the third batch, 1 under the fourth batch, and 1353 under the fifth batch; net −388 |
| Number of test files           | 296                           | 285                                                                                                                | The fifth batch removed 11 entire files                                                                                                                                                                                                                                                                     |
| Clause Gate                    | 227 files (77%)               | 136 files after tightening; 808 entries                                                                            | The criterion was tightened from “the token appears” to “there is a section number `§<number>`,” nearly halving the file count; the fifth batch retired 902 entries, and after the sample audit was retired in 0.6-19, another 9 entries moved from “Keep” to Pending verification                          |
| History signal                 | 442 entries / 148 files       | 313 entries within the gate; three radii: test itself 7 / file header 291 (38 files) / entire file 1218 (79 files) | The 148 files in the 0.6-2 survey were broader than the current entire-file radius of 79 files, and its vocabulary/scope was larger; this round strictly follows the four signals specified on the task card                                                                                                |
| Characterization strong signal | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                       | Characterization is not one of the three gates; mixing it into retirement conclusions would exceed the mandate                                                                                                                                                                                              |
| Weak assertions                | 473 (14%)                     | See the TSV “Assertion Summary” column                                                                             | The counting units differ: 0.6-2 counted the pattern “asserting only is_ok/is_some”; this round uses lexical counts and distinguishes strong, weak, and delegated assertions                                                                                                                                |

## Summary of the 0.6-19 Conclusions (Historical)

1. **The Clause Gate had previously been significantly inflated**: under the old criterion, 2403
   entries were classified as “Clause/Keep.” The sample measured a 50% mis-citation rate (95%
   interval 35%–65%)—“citation” and “a citation that supports the assertion” are different things,
   and the old criterion treated the former as the latter. (The sample audit was retired and deleted
   in 0.6-19; its conclusion is retained. See “Retirement of the Sample Audit” below for the reason
   for deletion.)
2. **After tightening, approximately 1202 entries genuinely had clause support** (range 846–1557);
   the rest must either await supporting review or should originally have been adjudicated through
   the History/Proof gates.
3. **The retirement surface was unrelated to mis-cited clauses**: the 136 candidates were unrelated
   to clauses (no assertions/weak assertions); 21 tests were retired in the first batch and 11 in
   the second batch, the generator correction added 1, the third batch removed 2, the 0.6-15
   criterion correction removed 3, the redundancy scan removed 1, the fourth batch removed 1, and
   the fifth batch removed 1353 (bulk retirement under the mechanical rule “for the same symbol and
   result category, keep only the strongest entry”), reducing the candidates to 61. Fifth-batch
   retirement by gate: Clause 902 · Pending determination 428 · Proof 23 · History 0 · Fails all
   0—**not a single entry from the History Gate or zero-assertion line was removed**.
4. **The History Gate was the largest legitimate source of Keep** (313 entries): it demonstrates
   that “pinning down a real incident” is more reliable than “copying a sentence from an RFC.”

## First Retirement Batch (Executed 2026-11-06)

**First batch: of the 26 “no assertion” tests in `src/frontend/core/types/eval/tests/`, retire 21
and keep 5** (a single directory cluster; criterion = all **Three Gates** in Part Seven of
coding-rules fail: no Clause Gate, no History Gate, and no assertions in the body). The item-by-item
determinations and “same-slot coverage” are provided by the 0.6-11a decision table (delete 21 / keep
5); this page does not repeat the reasons and only records the execution results.

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
- **5 kept** (the decision table could not identify replacement coverage, so they were kept per the
  hard requirement): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Second Retirement Batch (Executed 2026-11-06)

**Second batch: of the 18 “weak assertion” tests in `src/frontend/core/parser/pratt/tests/nud.rs`,
retire 11 and keep 7** (a single directory cluster = the Pratt prefix-parsing module; criterion =
all **Three Gates** in Part Seven of coding-rules fail: no Clause Gate, no History Gate, and the
body contains only weak assertions such as `is_some()` / `is_none()`—neither pinning down binding
power nor the parsing result).

The difference between this batch and the first must be stated clearly: these tests **do contain
assertions** and are not empty tests. The reason for retirement is “all Three Gates fail + zero
criterion increment,” not “there are no assertions”—each test slated for deletion names a
**unit-level replacement coverage test in the same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. The replacement
  coverage for every entry follows the **same** parse_expression → parse_prefix → prefix_info path
  and has stronger assertions (pinning down the AST shape rather than merely checking `is_some`):
  `test_parse_int_literal` / `test_parse_float_literal` / `test_parse_string_literal` /
  `test_parse_bool_true` / `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` /
  `test_parse_return_expr` in the same file, and `test_grouped_expr` / `test_tuple` /
  `test_list_literal` / `test_list_empty` / `test_ref_expr` in
  `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the commit message (task 0.6, second test-retirement batch) for the
  item-by-item reasons. The reasons fall into two categories: **the Proof Gate holds** (the first
  three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are the sole unit-level
  traversal points for the corresponding facts; no second unit test in the repository observes
  them), and **the History Gate holds** (`break_removed` / `continue_removed` name #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Generator Correction (0.6-13)

**`body_span` truncated function bodies when braces appeared in literals — fixed.**

The old implementation took the function body by balancing braces without distinguishing real code
from braces in string/character literals or comments. Constructs such as `tokenize("{")` are common
in Rust source, so the balancing was thrown off and the function body was truncated: the body of
`test_prefix_info_lbrace` was once truncated to the end of the file, causing the “Assertion Summary”
to pick up the rest of the file (`matchesx11 assertx24 …`), and it was therefore binned as “Pending
determination” rather than “Retirement candidate”—**wrong binning**.

Fix: added `mask_noncode`, which first replaces the contents of string literals / character literals
/ line comments / block comments (which may be nested) / raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (newlines are preserved as-is, and line numbers and line counts remain
unchanged), then balances the braces. **The mask is used only to locate function-body boundaries**;
assertion statistics and the History Gate still read the original source, and the criteria
themselves were not relaxed. `--selftest` adds 4 cases (paired/unpaired braces in ordinary strings,
braces in raw strings and block comments, and braces in character literals), each followed by a
sentinel test—the old implementation got all four boundaries wrong, while the new implementation
gets all of them right.

### Tests Affected by the Same Bug: 11

Counting method: compare **the function-body boundaries before and after the fix** file by file and
test by test (the old implementation balanced literals, whereas the new implementation balances
after masking); entries with different boundaries are affected. Among all 3413 tests in the
repository, the old implementation got the boundaries wrong for **11**:

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

### Difference in Binning Before and After the Fix: 2

Of the other 9, **8 affect only the “Assertion Summary” column** (the bin is unchanged), and **1 did
not even change the summary** (`test_fstring_close_brace_escape_preserved_raw`: the old
implementation omitted the closing `}` line, and that line contains no assertion keyword). The 8
summary changes are listed item by item in the commit message.

| Test name                   | File                                          | Old bin                                       | New bin                                          | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| --------------------------- | --------------------------------------------- | --------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending determination / Pending determination | Pending determination / **Retirement candidate** | The actual function body contains only one weak assertion, `assert!(state.prefix_info().is_some())`; neither the file header nor the function body has a § section number or issue/regression/defect/incident ⇒ both the Clause and History Gates fail, and the body contains only a weak assertion such as `is_some()` ⇒ Retirement candidate (a candidate is not a verdict; the Proof Gate still requires manual review)                                                               |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | History / Keep                                | Clause / **Pending verification**                | The old implementation truncated the function body to the end of the file, swallowing text containing “regression / defect / incident” and falsely classifying it under the History Gate; the actual function body contains no such wording ⇒ the History Gate no longer holds; the file header has verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the Clause Gate is hit but the citation has not been confirmed by the sample audit ⇒ Pending verification |

The distribution changes accordingly: Clause 1713 → 1714 · History 314 → 313 · Keep 776 → 775 ·
Pending verification 2121 → 2122 · Pending determination 412 → 411 · Retirement candidate 104 → 105.
**The repository-wide `#[test]` count is unchanged (3413)**—this round changes only the
implementation of the criteria, not any tests.

## Third Retirement Batch (Executed 2026-11-07)

**Third batch: make item-by-item determinations for the 5 retirement candidates in
`src/middle/passes/codegen/tests/`; retire 2 and keep 3.** (A single directory cluster = the tested
module `middle::passes::codegen`; criterion = all **Three Gates** in Part Seven of coding-rules
fail.)

This batch retains the **Keep criterion** established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus/integration tests belong to
the process-boundary layer and cannot replace a unit-level observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (only the function
    name differs after comments are removed): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that the `execute_module` entry
    path returns a non-Void value, but the body never observes the return value from
    `execute_module` (`run_fixture` discards it with `.expect(...)`)— the claimed intent was not
    implemented, and the body is a pure duplicate ⇒ zero criterion increment. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file covers it strictly:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` has the same shape as the inline `Symbol` in this
    test). “A name that was never inserted cannot be found” and “it cannot be found after being
    popped” use the same path in `SymbolScopeManager::lookup` (walk each layer through
    `scope.get(name)`, returning `None` when it is not found) ⇒ this test adds zero increment.
    Replacement coverage = `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that
    “executes directly without serialization” (the serialization variant is a different path), and
    `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization roundtrip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit-level observation point for the
    rejection branch where `id > 255` in `OperandResolver::to_reg` (the bytecode layer has
    independent overflow checks that do not cover this function, and no second case in the
    repository pushes `to_reg` out of range). `is_err()` is a discriminating assertion (removing the
    guard would make it turn red), not merely “proof that it did not panic.”
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

### One Census Blind Spot Found in This Batch (**Fixed in 0.6-15**, See “Generator Criterion Correction” Below)

The census's recognition of “delegated assertions,” `\b(assert|check)[A-Za-z_]*\s*\(`, **could not
match helper names containing digits**: `[A-Za-z_]*` does not include digits, so calls such as
`assert_slot_is_42(&interp)` were not recognized as assertions— and 2 of the 5 candidates in this
batch were consequently mis-binned as “no assertions in the body.”

After reviewing all 15 repository-wide “Fails all gates” entries one by one (after broadening helper
names to include digits and additionally recognizing the `match` + `panic!` form): **4 actually
contain assertions**—3 use the numeric helpers above, and 1 is `test_weak_from_arc` in
`src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule 6.1
requires `assert!(matches!(...))`, but it does in fact assert). The remaining 11 genuinely contain
no assertions.

**Directly removing these 4 under the machine's “Fails all gates” classification would incorrectly
delete tests with criteria**—the item-by-item human review in this batch prevented that; the
generator-side fix is described under “Generator Criterion Correction (0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots were fixed: helper names containing digits, and the `match` + `panic!`
form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411); it only
changes how the generator recognizes whether a body contains assertions.

### What Was Fixed

1. **Digits were allowed in the name portion of delegated assertions**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing such calls were
   treated as “having no assertions in the body.”
2. **Added recognition of the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the
   assertion is written in one of the arms, so lexically there is no `assert!` macro
   (test-specification rule 6.1 requires `assert!(matches!(...))`, but existing code contains this
   form). Recognition method: scan the expression after the `match` keyword up to the first `{`,
   determine the block boundary using **brace balancing**, and count it only if `panic!(` appears
   inside the block. **The determination is performed on masked lines**— braces in strings and
   comments do not participate in balancing, and `panic!` in a comment is not treated as an
   assertion.

### Difference in Binning: 3

| Test name                                           | File                                           | Old bin                                | New bin                                       | Basis                                                                                                                                                                                                                                                             |
| --------------------------------------------------- | ---------------------------------------------- | -------------------------------------- | --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Fails all gates / Retirement candidate | Pending determination / Pending determination | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not adjudicate its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Fails all gates / Retirement candidate | Pending determination / Pending determination | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit and the old pattern could not match it; it is now recognized as an assertion                                                                                          |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Fails all gates / Retirement candidate | Pending determination / Pending determination | The same (the serialization-roundtrip entry)                                                                                                                                                                                                                      |

Another **77 entries changed only the “Assertion Summary” column** (the gate and disposition are
unchanged): most summaries gained `match_panicxN` (the body contains the `match` + `panic!` form),
and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3 entries,
`src/package/tests/cache.rs`: 1 entry—helper names containing digits).

**The relaxation changes only the “Fails all gates” tier**: this tier is the machine's **confident
verdict** (not a single assertion in the body), and its criterion is a hard requirement; “Retirement
candidate” is only a pool awaiting manual review, and the size of the pool is not a verdict. The
weak-assertion pool and all other bins therefore continue under the original criteria. The result:
gate Fails all 14 → 11 · Pending determination 923 → 926; disposition Retirement candidate 103 → 100
· Pending determination 411 → 414.

### Item-by-item Review of the Existing 11 “Fails All Gates” Entries

After reading the original source of all 11 repository-wide “Fails all gates” entries one by one,
**all 11 genuinely contain no assertions** (including no `assert*` / `check*` helpers, no `match` +
`panic!` form, and no `#[should_panic]`):

| Test name                                       | File                                             | What the body actually does                                                                   | Review conclusion                                          |
| ----------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | Five `.expect(...)` calls, guaranteeing only that it does not crash                           | Genuinely no assertions (proof only that it did not panic) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                | Genuinely no assertions                                    |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them            | Genuinely no assertions                                    |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                           | Genuinely no assertions                                    |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                     | Genuinely no assertions                                    |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                            | Genuinely no assertions                                    |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                      | Genuinely no assertions (proof only that it did not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                              | Genuinely no assertions (proof only that it did not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                        | Genuinely no assertions (proof only that it did not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                          | Genuinely no assertions                                    |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertions                                    |

Of these, 5 (`test_normalizer_context` and the 4 entries in reducer.rs) were kept by the 0.6-11a
decision table because **no replacement coverage could be identified**; the conclusions of the
current review agree.

### Reverse Sample: Did the Relaxation Go Too Far?

Five entries were sampled with a fixed seed (20261107) from the 88 newly recognized assertions, and
their original source was read to confirm that they really assert:

| Sampled entry                                               | File                                                 | Assertion in the original source                                                                                                                |
| ----------------------------------------------------------- | ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching `match` arm; `panic!("Expected AssocType")` if it does not match                                         |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm; `panic!("expected InvalidPackage, got …")` if it does not match                     |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | An `assert_eq!` comparing the constraint type in the matching arm; `panic!("不是 Dynamic 变体…")` if it does not match                          |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm; `panic!("Expected TokensDelta variant")` if it does not match                           |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm verifying the error code and message; `panic!("不是 Disproved…")` if it does not match |

All 5 assert a concrete value or variant (replacing the implementation with another correct path
would make them turn red)—**the relaxation did not go too far**. Reverse controls were also added to
`--selftest`: a `match` without `panic!`, and a `panic!` that appears only in a comment; neither may
be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

When the generator is rolled back to the old implementation (`HELPER_RE` without digits + disabling
`match` + `panic!` recognition), all newly added `--selftest` cases **fail**, with exit code 1; the
new implementation passes all of them.

### Rollback

The generator and census table are in the same commit; `git revert <sha>` restores the whole thing.

## Fourth Retirement Batch (Executed 2026-11-07)

**Fourth batch: of the 10 “Retirement candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend's test directories), retire 1 and keep 9** (a
single directory cluster, with all three files read in one pass: `tests/bytecode_load.rs`: 4 entries
· `tests/weak.rs`: 2 entries · `executor/tests/debug.rs`: 4 entries). Criterion = the **Three
Gates** in Part Seven of coding-rules; Keep criterion = “a unit-level observation point can only be
replaced by another unit test; corpus/integration tests belong to the process-boundary layer”;
**deletion requires naming a concrete replacement coverage test; if one cannot be identified or
understood, the entry is kept**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, one step, and then
  obtain `capture_stack`), and its assertions are **strictly stronger**: it asserts that the stack
  is nonempty and that `stack[0].function_name == "main"` (pinning down the top-of-stack position),
  whereas the listed entry only asserts `stack.iter().any(|f| f.function_name == "main")` (any
  position is sufficient) —the truth of the former necessarily implies the truth of the latter ⇒
  zero criterion increment.
- **9 kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading their original source
  one by one, each is the **sole unit-level observation point** for some property (the
  magic-number/version branches for loading are different, as are the YXBC routing layer, the two
  Weak Some/None sides, stepping with no frame, step_out popping the stack, and running to
  completion with an empty stack). No unit test that could replace them can be identified ⇒ keep per
  the hard requirement. The item-by-item evidence is in the commit message (the decision table is
  not included on this page).
- **Figures**: repository-wide `#[test]` 3409 → 3408; the one deletion means `git diff` counts
  `-#[test]`: 1 line and `+#[test]`: 0 lines; the dominant test
  `test_capture_stack_during_execution` passes on its own. Census: Retirement candidate 99 → 98 ·
  Pending determination 925 → 924 · Clause 1713 · History 313 · Proof 447 · Fails all 11 · Pending
  verification 2121 · Keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Fifth Retirement Batch (Executed 2026-11-07)

**Fifth batch: mechanically retire 1353 unit tests under the rule “for the same symbol and result
category, keep only the strongest entry”** (only unit tests under `src/**/tests/**` were changed;
not a single entry on the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or `tests/project-corpus/**` was touched).

- **Rule (mechanically executed, without item-by-item argumentation)**: for the same symbol under
  test and the same assertion result category, keep only the entry with the strongest assertion and
  delete the rest.
  - Symbol under test = a repository symbol called in the test body (including one level of
    propagation through a helper in the same file); when multiple symbols are touched, classify by
    the **primary symbol** (the first repository call). Symbols are qualified by their path within
    the repository and are not merged across modules by bare name.
  - Result category = concrete value / a specific error code (E code) / a specific variant match /
    ok only / err only / should_panic only.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when entries in the same group have equal
    strength, keep the one that appears first in the file.
  - If a test asserts multiple categories simultaneously (making the primary category
    indeterminate), **keep it** and do not guess.
- **Three exceptions (not one was removed)**: ① the name or comment names `issue #` / regression /
  defect / incident (History Gate); ② a symbol has only one test (deleting it would leave the symbol
  without a unit-level observation point); ③ `#[should_panic]` is the symbol's only should_panic
  case. The History Gate's 313 entries and the 11 “Fails all gates” entries had **0 retirements**.
- **Figures**: repository-wide `#[test]` 3408 → 2055; 1353 deletions mean `git diff` counts
  `-#[test]`: 1353 lines and `+#[test]`: 0 lines. Census (by gate, current values after the 0.6-19
  retirement of the sample audit): Clause 808 · History 313 · Proof 424 · Pending determination 499
  · Fails all 11. Census (by disposition, current values): Keep 737 · Pending verification 1054 ·
  Pending determination 203 · Retirement candidate 61. The fifth batch's retirement count by gate
  was: Clause 902 · Pending determination 428 · Proof 23 (0.6-19 reclassified 9 “audit-confirmed as
  genuinely supported” entries as Pending verification, so the current values by gate/disposition
  differ from those when the fifth batch was closed).
- **Cleanup alongside deletions**: helper functions used exclusively by deleted tests, imports that
  became invalid, and struct fields that became invalid were removed as well
  (`cargo clippy --all --all-features --all-targets -- -D warnings` reported 0 warnings).
- **Two over-deletions that must be recorded**: the census originally classified 29 entries as
  “Keep,” but this batch's rule still deleted them—23 current-state anchors (names containing
  unchanged/anchor/guard/preserve/baseline, many of which were false-positive matches on domain
  terms such as “guard” and “preserves,” such as `test_while_guard_scope_balanced`) and 6 entries
  with audit-confirmed clauses that genuinely supported their assertions. This batch followed the
  closed list of three exceptions on the task card and did not additionally protect these two
  categories; to restore them, fill them back in using the Keep/Delete comparison table on this
  page.
- **Invalidation and retirement of the sample audit**: the 40-entry sample was drawn from the test
  pool with a fixed seed, and after the fifth batch's retirement, 38 of the original samples no
  longer existed. The audit's `--check` therefore consistently reported “38 missing, 38 extra.” The
  sample audit was retired and deleted in 0.6-19 (the script, data files, and the generator's
  dependency on it were removed as well); see “Retirement of the Sample Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the whole batch.

## Sixth Retirement Batch (Executed 2026-11-26)

**Sixth batch: mechanism rollback from the 4.9.9 narrowing round—retire 28 tests removed along with
their mechanism; a further 4 prohibition criteria were never checked in.** This batch does not use
the Three Gates to decide whether something should be kept; instead, the mechanism under test was
**rolled back wholesale**: the criteria written during the preceding six rounds for the downstream
ambiguity mechanism (candidate-provider priority / candidate-signature comparison / owner
compatibility inference / IR-side named-argument fallback / import-name resolution-point E5006
prohibition) were retired along with the mechanism—the mechanism no longer exists, so the tests have
no subject under test. Such deletions are registered as a “batch” under coding-rules 7.3 and do not
enter the retirement-candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 entries (28 deleted: 27
    downstream-mechanism criteria from the preceding six rounds
    - 1 alias-import named-argument criterion). The 2 retained entries pin down the minimal fix for
      bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: the entire file containing 4 E5006 prohibition
    criteria was retired—the file existed only in an intermediate 4.9.9 commit, and neither the
    parent commit nor that version's checked-in table included it; after the prohibition was
    withdrawn there was no subject under test, so this batch does not decrement the checked-in
    table.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the candidate-injection surface for alias imports was rolled back, so the fixture's
    behavior no longer holds); the same-named corpus differential-baseline entry was deleted in the
    same batch. The baselines are stated separately: **relative to the direct parent commit
    `ac810e97`, 405 → 404 lines** (the intermediate state contained the fixture); **relative to the
    pre-4.9.9 baseline `08ccacc5`, 403 → 404 lines** (+1 base fixture
    `named-args-cross-unit-method`; the retired `alias-overload` never entered the final history).
- **Which gate applies**: the 2 retained entries = **History Gate** (they pin down a real defect
  registered in 4.9.9: a cross-unit named argument could not obtain the declared parameter name,
  leaving `E1014 available` empty); most of the 28 retired entries had previously been classified
  under the History Gate as downstream-mechanism criteria, but the reason for their deletion **is
  not** the Three Gates (it is not a decision that they were worthless); the mechanism was rolled
  back.
- **Replacement coverage**: cross-unit named arguments for bare-name imports = the 2 retained
  harness-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity surface
  no longer has rejection logic (the requirement was rolled back), so there is no replacement
  criterion; the 4.9.8 dispatch-attribution criteria (10 harness + 5 integration entries) are
  retained unchanged.
- **Restoration**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the empty-name-table guard for
  `method_param_info` is retained; this form is unreachable under the public syntax and is a
  defensive guard whose unit test merely prevents accidental deletion).
- **Figures** (after regeneration for this batch): repository-wide `#[test]`: 2680 entries / 334
  files; the previous checked-in table had 2688 entries → the current tree has 2680 entries (−8); 1
  is in the current tree but not the old table, 9 are in the old table but not the current tree, and
  0 same-key rows had their contents recomputed.
- **Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit); `git revert <sha>` rolls back the whole batch.

## Retirement of the Sample Audit (0.6-19)

**Criterion**: the sample audit (0.6-10) measured the “real support rate of clause citations,” but
the retention rule after 0.6-18 no longer examines citations (it changed to “for the same symbol and
result category, keep only the strongest entry,” reducing the test pool from 3408 to 2055). Of the
original 40 samples, 38 no longer existed, so the audit's `--check` consistently reported “38
missing, 38 extra”—keeping it would only produce continuous alarms. It was therefore **retired and
deleted**.

**What was deleted**: the sample-audit script under `scripts/dev/` (the one that reproduced the
40-entry sample with a fixed seed) and its JSON data file (the item-by-item determinations and
evidence for the 40 entries).

**The generator's dependency was removed at the same time**: the generator previously read that JSON
and classified “audit-confirmed as genuinely supported” as “Clause Gate + Keep”; that branch has
been removed—**clause citations no longer have an “audit-confirmed” tier** and are always classified
as “Clause / Pending verification.”

**Distribution change** (all 2055 repository-wide `#[test]` entries are **untouched**, and the TSV
has been regenerated): Keep 746 → 737 (−9: the 9 entries previously confirmed by the audit as
genuinely supported moved back to Pending verification); Pending verification 1045 → 1054; by gate:
Clause 811 → 808, Pending determination 496 → 499 (the others are unchanged: History 313 · Proof 424
· Fails all 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` rolls back the whole batch.

## Rerunning and Self-Checks

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (both runs produce byte-identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical determination only; its boundaries are stated truthfully:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test's assertion is hidden in a helper whose name lacks these two prefixes, it will be
  misclassified as “no assertions”—the first batch's 26 entries (21 retired / 5 kept) and the second
  batch's 18 entries (11 retired / 7 kept) have all had their function bodies manually reviewed.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator takes a function body, it
  balances braces; braces in string/character literals and comments throw off the balancing and
  truncate the body—`test_prefix_info_lbrace` (which contains `tokenize("{")`) was once truncated to
  the end of the file, causing the “Assertion Summary” to pick up the rest of the file and bin it as
  “Pending determination” rather than “Retirement candidate.” It now masks non-code regions (strings
  / character literals / line comments / block comments / raw strings) before balancing; see
  “Generator Correction” below.
- **Recognition of “delegated assertions” (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` did not include digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions—tests containing such calls would
  be misclassified as “having no assertions in the body.” At the time, 4 of the repository's 15
  “Fails all gates” entries actually contained assertions (3 numeric helpers + 1 `match` + `panic!`
  form). The name portion is now broadened to `[A-Za-z0-9_]*`, and recognition of the `match` +
  `panic!` form has been added; see “Generator Criterion Correction” below.
- **The Proof Gate is not adjudicated**: any test that “fails both gates” is classified only as
  “Pending determination” or “Retirement candidate”; the generator never declares that it “should be
  deleted.”
- **No cross-file deduplication**: exactly duplicated tests are not identified in this table (0.6-2
  measured only 1%, which does not constitute a retirement surface).
- **All clause citations require verification**: a verifiable section number in the file header only
  means that the test “can be matched to a specific, nameable clause”; it does not mean that the
  citation supports the assertion. The machine does not adjudicate whether the citation is
  sufficient, so even a Clause Gate hit is classified only as “Pending verification” and must be
  verified by a human before it can be kept.
