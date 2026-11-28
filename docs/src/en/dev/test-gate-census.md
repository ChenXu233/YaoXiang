---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Great Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement-candidate list plus per-batch retirement records'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Great Cleanup)

Preparatory material for WBS 0.6 “Great Cleanup”: mechanically classify every `#[test]` in the
repository according to the **three gates (clause / history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a rerunnable per-test decision table. A
40-entry sample audit of the clause gate once measured the proportion of tests that “cited a clause
but whose assertions the clause did not support” and consequently **tightened the criteria**. That
audit was abolished and deleted at 0.6-19. **This page and the generator itself only read tests and
do not modify them**; test retirement is performed in batches (one commit per batch, fully
revertible with `git revert`), and the execution results are recorded in the per-batch retirement
subsections below and in commit messages.

Generated artifacts:

| Artifact                            | Purpose                                                                           |
| ----------------------------------- | --------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Per-test decision table (2,728 rows + header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                       |

## Criteria

### Scan surface

All `#[test]`s in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module's `tests/` subdirectory) | 2124                 |
| `tests/integration`                                                      | 463                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 77                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2728** (342 files) |

### Machine criteria for the three gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Meaning when matched                                     |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| Clause gate (tightened at 0.6-10) | A **verifiable reference** appears within the first 25 lines of the file header: section-number form `§<数字>` (including `RFC-<数字> §<数字>` and `SPEC §<数字>`); files under `src/**/tests/` are inspected together with their parent `mod.rs`                                                                                                                                                                                                                          | The test can be matched to an identifiable clause        |
| History gate                      | The test body, function doc comment, or **file header** (`//! 回归背景：issue #226` and the like) names an actual failure: `issue #N` or one of the Chinese markers 回归 / 缺陷 / 事故                                                                                                                                                                                                                                                                                     | The test pins an incident that has actually occurred     |
| Proof gate                        | **Cannot be determined mechanically** (“If it were replaced with another correct implementation, would it turn red?” requires human judgment). Therefore, anything that passes neither of the first two gates is classified as “pending adjudication,” never automatically as “passes neither”; the sole exception is a body with **not a single assertion** (including no delegation to `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Can only yield “pending adjudication” or “no assertions” |

Rationale for tightening: the 0.6-9 clause gate merely required the file header to contain
`§`/`RFC`/`SPEC`, so 2403 entries were classified as “keep.” The 40-entry stratified sample at
0.6-10 empirically found that **half of the citations could not support the assertions**
(mis-citation rate 50%, 95% interval 35%–65%)—this measurement was the **direct reason for
abandoning “keep based on a single citation.”** After tightening, only section-number forms count,
and **having a citation still does not mean that the citation supports the assertion**—all clause
citations stop at “pending verification.” (The sample audit was abolished and deleted at 0.6-19:
after the audit pool shrank as tests were retired, 38 of the original samples no longer existed, and
`--check` consistently reported “38 missing, 38 extra”; the conclusion is retained, but the audit
artifacts are no longer maintained.)

Rationale for the file-header radius: this repository conventionally places “回归背景：issue #N” in
the file header rather than repeating it in each test comment. Looking only at each test's own
comments would miss the history for the entire file (measured: 7 entries under the test-local
radius, 291 under the file-header radius).

### Categories that are never classified for retirement

The following tests are **always “kept”** and never enter the retirement-candidate pool:

- The `tests/yaoxiang/**` corpus and the project-shape `tests/project-corpus/**` corpus (behavioral
  criterion surface);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and crossing a process boundary);
- Status-quo anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition semantics

| Disposition          | Meaning                                                                                                                                                                                       |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Assigned in only two ways: a history-gate match, or the proof gate (spec-shaped criterion surface / status-quo anchor name)                                                                   |
| Pending verification | Cites a clause (with a section number, or only an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a human before it can be kept |
| Retirement candidate | Passes neither gate and the assertion surface contains only weak assertions (or none)—a candidate is not a verdict; the proof gate still requires manual review                               |
| Pending adjudication | Passes neither gate but has strong assertions; the machine gives no conclusion                                                                                                                |

## Distribution (regenerated 2026-11-28; 4.9.9 narrowing round, batch 8.7, 4.9.10-L0, init-test rewrite, D53 batch, L3-1b S1, batch 7e, and the #593 criterion)

| Gate                                                                      | Count    |
| ------------------------------------------------------------------------- | -------- |
| Clause (has a verifiable reference)                                       | 1037     |
| History                                                                   | 568      |
| Proof (spec-shaped criterion surface / status-quo anchors)                | 574      |
| Pending adjudication (passes both first gates; proof awaits human review) | 543      |
| Passes neither (no assertions at all)                                     | 6        |
| **Total**                                                                 | **2728** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1142** | 737 (+405)                        |
| Pending verification | 1297     | 1054 (+241)                       |
| Pending adjudication | 230      | 203 (+27)                         |
| Retirement candidate | 59       | 61 (−2)                           |
| **Total**            | **2728** | 2055 (+651)                       |

Keep breakdown: 1142 = proof gate 574 (spec-shaped criterion surface 518 + status-quo anchors 56) +
history gate 568. File-level scope: among 342 test files, **the clause gate covers 125 files**;
scanning the first 25 lines of file headers (`src/**/tests*/` together with the parent `mod.rs`)
detects a verifiable section number (`§` + digits) in **147 files**.

Reconciliation: the previously committed table had 2727 entries, while the current tree has 2728; 1
entry is present in the current tree but absent from the old table (the #593 call-site result-type
criterion), 0 entries are present in the old table but absent from the current tree, and 0 rows with
the same key were recalculated. Per-key records are in the TSV.

Latest full regeneration (after batch 7e and admission of the #593 criterion): 2728 rows, matching
the current tree key by key (`--check` exits 0).

### Two changes in criteria at the 0.6-19 point

1. **Keep breakdown**: 737 = proof gate 424 (spec-shaped criterion surface 390 + status-quo
   anchors 34) + history gate 313. Of the original 2897 entries, the 2121 that had been “kept based
   on a single citation” are now pending verification (after the 0.6-19 sample audit was abolished,
   the 9 entries previously classified as “audit-confirmed genuine support” also moved to pending
   verification—clause citations no longer have an “audit-confirmed” tier).
2. **History gate 47 → 313**: these are not newly discovered histories but **relabelled entries**.
   In 0.6-9, entries that matched both the clause and history gates were recorded under the clause
   gate. After tightening, a clause match no longer automatically means “keep,” so these entries had
   to be caught by the history gate and their counts folded into it. The total number kept is
   therefore unaffected (this batch of tests was also kept under 0.6-9).

File-level scope at the time: among 285 test files, **136** had a verifiable section number in the
file header (the old 0.6-9 criterion yielded 221).

## Reconciliation between the 0.6-19 point and the 0.6-2 survey

The 0.6-2 survey figures are the baseline for the first comparison. The reasons for each difference
are listed below:

| Item                            | 0.6-2 survey                  | This census                                                                                                      | Reason for the difference                                                                                                                                                                                                                                                                         |
| ------------------------------- | ----------------------------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                             | 2 were added after 0.6-2 (E3005 and E3022 check-criterion regression tests), then 21 no-assertion tests were removed under the first-batch list, 11 weak-assertion tests under the second-batch list, 2 under the third batch, 1 under the fourth batch, and 1353 under the fifth batch; net −388 |
| Number of test files            | 296                           | 285                                                                                                              | The fifth batch removed 11 entire files                                                                                                                                                                                                                                                           |
| Clause gate                     | 227 files (77%)               | 136 files after tightening; 808 entries                                                                          | The criterion was tightened from “the token appears” to “there is a section number `§<数字>`,” nearly halving the file count; the fifth batch retired 902 entries, and after the 0.6-19 sample audit was abolished, another 9 entries moved from “keep” to pending verification                   |
| History signals                 | 442 entries / 148 files       | 313 entries within the gate; three radii: test-local 7 / file header 291 (38 files) / whole file 1218 (79 files) | The 148 files in the 0.6-2 survey are broader even than this census's whole-file radius of 79 files because its vocabulary/scope was larger; this census strictly follows the four specified rubric signals                                                                                       |
| Characterization strong signals | 1176 (34%) / strong 631 (18%) | Not tested                                                                                                       | Characterization is not one of the three gates; mixing it into retirement conclusions would exceed the gate's authority                                                                                                                                                                           |
| Weak assertions                 | 473 (14%)                     | See the TSV “assertion summary” column                                                                           | The counting units differ: 0.6-2 counted the pattern “asserts only is_ok/is_some,” whereas this census uses lexical counts and distinguishes strong, weak, and delegated assertions                                                                                                               |

## 0.6-19 conclusion summary (historical)

1. **The clause gate was previously significantly overstated**: under the old criterion, 2403
   entries were classified as “clause/keep.” The sample empirically measured a 50% mis-citation rate
   (95% interval 35%–65%)—“citation” and “a citation that supports the assertion” are different
   things, and the old criterion treated the former as the latter. (The sample audit was abolished
   and deleted at 0.6-19; the conclusion is retained. See “Abolition of the Sample Audit” below for
   the reason.)
2. **After tightening, approximately 1202 entries had genuine clause support** (range 846–1557); the
   remainder must either obtain pending verification or be adjudicated under the history/proof
   gates.
3. **The retirement surface is unrelated to clause mis-citation**: 136 candidates have no clause (no
   assertions/weak assertions); 21 entries were retired in the first batch and 11 in the second
   batch, the generator correction added 1, the third batch reduced the count by 2, the 0.6-15
   criterion correction reduced it by 3, the redundancy scan reduced it by 1, the fourth batch
   reduced it by 1, and the fifth batch retired 1353 entries under the mechanical rule “for the same
   symbol and result category, keep only the strongest one,” reducing the candidates to 61. The
   fifth batch's retirement volume by gate was: clause 902 · pending adjudication 428 · proof 23 ·
   history 0 · passes neither 0—**not a single history-gate entry or zero-assertion entry was
   retired**.
4. **The history gate is the largest legitimate source of keeps** (313 entries): it demonstrates
   that “pinning an actual incident” is more reliable than “copying a sentence from an RFC.”

## First retirement batch (executed 2026-11-06)

**First batch: of the 26 “no assertions” tests under `src/frontend/core/types/eval/tests/`, 21 were
retired and 5 kept** (single directory cluster; criterion = **all three gates in Part Seven of
coding-rules fail**: no clause gate, no history gate, and not a single assertion in the body). The
entry-by-entry decisions and “same-cell coverage” are provided by the 0.6-11a decision table (21
deleted / 5 kept). This page does not repeat the rationale; it records only the execution result.

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
- **5 kept** (the decision table could not identify replacement coverage, so the hard requirement is
  to keep them): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls the entire batch back.

## Second retirement batch (executed 2026-11-06)

**Second batch: of the 18 “weak assertions” tests in `src/frontend/core/parser/pratt/tests/nud.rs`,
11 were retired and 7 kept** (single directory cluster = the Pratt prefix-parsing module; criterion
= **all three gates in Part Seven of coding-rules fail**, and the body contains only weak assertions
such as `is_some()` / `is_none()`—neither pinning binding power nor the parse result).

The difference from the first batch must be made explicit: these tests **do contain assertions** and
are not empty tests. The reason for retirement is “all three gates fail + zero criterion increment,”
not “no assertions”—every deleted entry identifies a **unit-level replacement coverage test in the
same module**.

- **11 retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Every replacement
  follows the **same** `parse_expression → parse_prefix → prefix_info` path and makes a stronger
  assertion (pinning the AST shape rather than merely checking `is_some`): `test_parse_int_literal`
  / `test_parse_float_literal` / `test_parse_string_literal` / `test_parse_bool_true` /
  `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` / `test_parse_return_expr`
  in the same file, plus `test_grouped_expr` / `test_tuple` / `test_list_literal` /
  `test_list_empty` / `test_ref_expr` in `src/frontend/core/parser/tests/expressions.rs`.
- **7 kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the commit message for the entry-by-entry rationale (task 0.6, second
  test-retirement batch). There are two categories of reason for keeping them: **the proof gate
  holds** (the first three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are the
  sole unit-level walkthrough points for their corresponding facts, and the repository has no second
  unit test that observes them), and **the history gate holds** (`break_removed` /
  `continue_removed` name #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls the entire batch back.

## Generator correction (0.6-13)

**`body_span` truncates function bodies when it encounters braces inside literals—fixed.**

The old implementation extracted function bodies by balancing braces, without distinguishing real
code from braces inside string/character literals or comments. Forms such as `tokenize("{")` are
common in Rust source, so the balance was thrown off and the function body was truncated: the body
of `test_prefix_info_lbrace` was once truncated to the end of the file, and the “assertion summary”
picked up the remainder of the file (`matchesx11 assertx24 …`), causing it to be placed in “pending
adjudication” rather than “retirement candidate”—**the wrong bucket**.

Fix: add `mask_noncode` to replace the contents of string literals, character literals, line
comments, block comments (including nested ones), and raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (newlines are preserved exactly, so line numbers and line counts do not
change), then balance the braces. **The mask is used only to locate function-body boundaries**;
assertion statistics and the history gate still read the original source, and the criteria
themselves have not been relaxed. `--selftest` gains 4 cases (paired/unpaired braces in ordinary
strings, braces in raw strings and block comments, and braces in character literals), each followed
by a sentinel test—the old implementation got all 4 boundaries wrong, while the new implementation
gets all of them right.

### Tests affected by the same bug: 11 entries

Counting method: compare the **function-body boundaries before and after the fix** file by file and
test by test (the old implementation balanced literal braces, whereas the new implementation
balances after masking); every difference is affected. Among all 3413 tests in the repository, the
old implementation got **11** boundaries wrong:

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

### Bucket differences before and after the fix: 2 entries

Of the other 9 entries, **8 affect only the “assertion summary” column** (the bucket is unchanged),
and **1 leaves even the summary unchanged** (`test_fstring_close_brace_escape_preserved_raw`: the
old implementation omitted the closing `}` line, which contains no assertion keyword). The 8 summary
changes are listed entry by entry in the commit message.

| Test name                   | File                                          | Old bucket                                  | New bucket                                      | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| --------------------------- | --------------------------------------------- | ------------------------------------------- | ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending adjudication / Pending adjudication | Pending adjudication / **Retirement candidate** | The real function body contains only one weak assertion, `assert!(state.prefix_info().is_some())`; neither the file header nor the function body has a § section number or an issue/regression/defect/incident marker ⇒ neither the clause gate nor the history gate passes, and the body contains only a weak assertion such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires manual review)                   |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | History / Keep                              | Clause / **Pending verification**               | The old implementation truncated the function body to the end of the file, swallowed text containing the Chinese markers 「回归 / 缺陷 / 事故」, and falsely matched the history gate; the real function body contains no such text ⇒ the history gate no longer holds; the file header has verifiable references (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate matches, but the sample audit did not confirm it ⇒ pending verification |

The distribution changes accordingly: clause 1713 → 1714 · history 314 → 313 · keep 776 → 775 ·
pending verification 2121 → 2122 · pending adjudication 412 → 411 · retirement candidate 104 → 105.
**The repository-wide `#[test]` count is unchanged (3413)**—this round changed only the
implementation of the criteria, not any test.

## Third retirement batch (executed 2026-11-07)

**Third batch: the 5 retirement candidates under `src/middle/passes/codegen/tests/` were adjudicated
entry by entry; 2 were retired and 3 kept.** (Single directory cluster = the
`middle::passes::codegen` module under test; criterion = **all three gates in Part Seven of
coding-rules fail**.)

This batch continues the **keep criterion** established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus/integration tests operate at
the process-boundary layer and cannot replace a unit observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (after removing
    comments, only the function name differs): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its doc comment claims to verify that “the `execute_module` entry
    path returns non-Void,” but the body never observes `execute_module`'s return value
    (`run_fixture` discards it with `.expect(...)`)— the claimed intent was not implemented, and the
    function body is a pure duplicate ⇒ zero criterion increment. Replacement coverage =
    `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file covers it strictly:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` has the same shape as the inline `Symbol` in this
    entry). “Looking up a name that was never inserted” and “looking it up after it was popped”
    follow the same path in `SymbolScopeManager::lookup` (walking each `scope.get(name)` layer and
    returning `None` when not found) ⇒ this entry has zero increment. Replacement coverage =
    `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that does
    “not go through serialization and execute directly” (the serialized variant is a different
    path), and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit observation point for the rejection
    branch of `OperandResolver::to_reg` when `id > 255` (the bytecode layer has its own independent
    overflow checks and does not cover this function; no second test in the repository drives
    `to_reg` out of bounds). `is_err()` is a discriminating assertion (removing the guard would turn
    the test red), not merely a check that it did not panic.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls the entire batch back.

### A census blind spot found in this batch (**fixed at 0.6-15**; see “Generator Criterion Corrections” below)

The census's “delegated assertion” recognition, `\b(assert|check)[A-Za-z_]*\s*\(`, **cannot match
helper names containing digits**: `[A-Za-z_]*` does not include digits, so calls such as
`assert_slot_is_42(&interp)` are not recognized as assertions— 2 of the 5 candidates in this batch
were consequently misbucketed as “no assertions in the body.”

After reviewing all 15 “passes neither” entries individually (expanding helper names to include
digits and additionally recognizing the `match` + `panic!` form), **4 did in fact contain
assertions**—3 used the numeric helpers above, and 1 was `test_weak_from_arc` in
`src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule 6.1
requires `assert!(matches!(...))`, but it does assert). The remaining 11 genuinely contain no
assertions.

**If these 4 had been removed directly based on the machine's “passes neither” result, tests with
criteria would have been mistakenly deleted**—manual adjudication in this batch prevented that; the
generator-side fix is described in “Generator Criterion Corrections (0.6-15)” below.

## Generator criterion corrections (0.6-15)

**Two criterion blind spots have been fixed: helper names containing digits and the `match` +
`panic!` form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411);
it changes only how the generator determines whether a body contains assertions.

### What was fixed

1. **The name portion of delegated assertions now permits digits**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing such calls were
   treated as “not a single assertion in the body.”
2. **Added the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the assertion is written
   in an arm, so `assert!` is not visible lexically (test-specification rule 6.1 requires
   `assert!(matches!(...))`, but legacy code contains this form). Identification method: scan the
   expression from the `match` keyword to the first `{`, determine the block boundary by **brace
   balancing**, and count it only if `panic!(` occurs inside the block. **The decision is made on
   masked lines**— braces in strings and comments do not participate in balancing, and `panic!` in a
   comment is not treated as an assertion.

### Bucket differences: 3 entries (the gate and disposition changed)

| Test name                                           | File                                           | Old bucket                            | New bucket                                  | Basis                                                                                                                                                                                                                                                           |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------- | ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Passes neither / Retirement candidate | Pending adjudication / Pending adjudication | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not judge its discriminating power |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Passes neither / Retirement candidate | Pending adjudication / Pending adjudication | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit and the old pattern could not match it; it is now recognized as an assertion                                                                                        |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Passes neither / Retirement candidate | Pending adjudication / Pending adjudication | Same as above (the serialization-round-trip entry)                                                                                                                                                                                                              |

Another **77 entries changed only the “assertion summary” column** (the gate and disposition are
unchanged): most summaries gained `match_panicxN` (the body contains the `match` + `panic!` form),
while 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3; `src/package/tests/cache.rs`:
1—helper names containing digits).

**The relaxation changes only the “passes neither” tier**: this tier is the machine's **confident
verdict** (not a single assertion in the body), and its criterion is a hard requirement; “retirement
candidate” is only a pool awaiting manual review, and its size is not a verdict. All other buckets,
including the weak-assertion pool, therefore retain the original criteria. The result is: gate
passes neither 14 → 11 · pending adjudication 923 → 926; disposition retirement candidate 103 → 100
· pending adjudication 411 → 414.

### Existing “passes neither” entries: 11, with individual review conclusions

After reading the source of all 11 repository-wide “passes neither” entries individually, **all 11
genuinely contain not a single assertion** (including no `assert*` / `check*` helper, no `match` +
`panic!` form, and no `#[should_panic]`):

| Test name                                       | File                                             | What the body actually does                                                                   | Review conclusion                                          |
| ----------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 uses of `.expect(...)`, only guaranteeing no crash                                          | Genuinely no assertion (only proves that it did not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                | Genuinely no assertion                                     |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` values, obtains their contexts, and discards them               | Genuinely no assertion                                     |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                           | Genuinely no assertion                                     |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                     | Genuinely no assertion                                     |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                            | Genuinely no assertion                                     |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` on invalid content and discards the result                      | Genuinely no assertion (only proves that it did not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                              | Genuinely no assertion (only proves that it did not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                        | Genuinely no assertion (only proves that it did not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                          | Genuinely no assertion                                     |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertion                                     |

Five of these (`test_normalizer_context` and the 4 entries in reducer.rs) were **kept by the 0.6-11a
decision table because it could not identify replacement coverage**; this review reaches the same
conclusion.

### Reverse spot check: did the relaxation go too far?

Five entries were read from the 88 entries newly recognized as assertions, using a fixed seed
(20261107), to confirm that they really assert:

| Spot-check entry                                            | File                                                 | Assertion in the source                                                                                                                             |
| ----------------------------------------------------------- | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching arm; `panic!("Expected AssocType")` if unmatched                                                             |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | The matching arm has `assert!(msg.contains("src/evil.yx"))`; the unmatched arm has `panic!("expected InvalidPackage, got …")`                       |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | The matching arm uses `assert_eq!` to compare constraint types; the unmatched arm has `panic!("不是 Dynamic 变体…")`                                |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | The matching arm has `assert!(!delta.edits.is_empty())`; the unmatched arm has `panic!("Expected TokensDelta variant")`                             |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | The matching arm has three `assert_eq!` / `assert!` calls that verify the error code and message; the unmatched arm has `panic!("不是 Disproved…")` |

All 5 assert a concrete value or concrete variant (replacing the implementation with another correct
path would turn them red)—**the relaxation did not go too far**. The reverse controls were also
added to `--selftest`: a `match` without `panic!`, and `panic!` appearing only in a comment; neither
may be recognized as an assertion.

### The criteria have teeth (developmental proof)

Reverting the generator to the old implementation (`HELPER_RE` without digits + `match` + `panic!`
recognition disabled) causes all newly added `--selftest` cases to **fail**, with exit code 1; the
new implementation passes them all.

### Rollback

The generator and census table are in the same commit; `git revert <sha>` restores the entire state.

## Fourth retirement batch (executed 2026-11-07)

**Fourth batch: of the 10 “retirement candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend's test directory), 1 was retired and 9 kept**
(single directory cluster; the three files were read together before adjudication:
`tests/bytecode_load.rs` 4 · `tests/weak.rs` 2 · `executor/tests/debug.rs` 4). Criterion = the
**three gates** in Part Seven of coding-rules; keep criterion = “a unit-level observation point can
only be replaced by another unit test; corpus/integration operate at the process-boundary layer”;
**deletion must identify a specific replacement coverage test name; if none can be identified or the
test cannot be understood, classify it as keep**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping` plus one step,
  followed by `capture_stack`), and its assertion is **strictly stronger**: it asserts that the
  stack is non-empty and that `stack[0].function_name == "main"` (pinning the top-of-stack
  position), while the listed test only asserts `stack.iter().any(|f| f.function_name == "main")`
  (any position is sufficient) —whenever the stronger assertion is true, the weaker one must also be
  true ⇒ zero criterion increment.
- **9 kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each entry
  individually, each is the **sole unit observation point** for some property (the distinct
  magic-number/version branches of load, the YXBC routing layer, both Weak Some/None sides, stepping
  with no frames, step_out popping the stack, and running to completion with an empty stack), and no
  replacement unit test could be identified ⇒ the hard requirement is to keep them. See the commit
  message for the entry-by-entry basis (the decision table is not included in this page).
- **Numbers**: repository-wide `#[test]` 3409 → 3408; 1 deleted = `git diff` counts `-#[test]` on 1
  line and `+#[test]` on 0 lines; the dominant test `test_capture_stack_during_execution` passes in
  isolation. Census: retirement candidate 99 → 98 · pending adjudication 925 → 924 · clause 1713 ·
  history 313 · proof 447 · passes neither 11 · pending verification 2121 · keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls the entire batch back.

## Fifth retirement batch (executed 2026-11-07)

**Fifth batch: 1353 unit tests were retired in bulk under the mechanical rule “for the same symbol
and result category, keep only the strongest one”** (only unit tests under `src/**/tests/**` were
changed; not a single entry was touched on the E2E surfaces `tests/yaoxiang/**`,
`tests/integration/**`, `tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or
`tests/project-corpus/**`).

- **Rule (executed mechanically, without entry-by-entry argument)**:
  - Tested symbol = a repository-internal symbol called by the test body (including one level of
    propagation through a helper in the same file); when multiple symbols are touched, classify by
    the **primary symbol** (the first repository-internal call). Symbols are qualified by repository
    path and are not merged across modules by bare name.
  - Result category = a concrete value / a specific error code (E code) / a specific variant match /
    only ok / only err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; within a group of equal strength, keep the
    first one to appear in the file.
  - If a test asserts multiple categories at once (making the primary category indeterminate),
    **keep it**; do not guess.
- **Three exceptions (not one was lost)**: ① the name or comment names `issue #`
  / 回归 / 缺陷 / 事故 (history gate); ② a symbol has only one test (deleting it would leave the
  symbol without a unit observation point); ③ `#[should_panic]` is the symbol's only should_panic
  use case. **Zero** of the 313 history-gate entries or 11 “passes neither” entries were retired.
- **Numbers**: repository-wide `#[test]` 3408 → 2055; 1353 deleted = `git diff` counts `-#[test]` on
  1353 lines and `+#[test]` on 0 lines. Census (by gate, current values after the 0.6-19 sample
  audit was abolished): clause 808 · history 313 · proof 424 · pending adjudication 499 · passes
  neither 11. Census (by disposition, current values): keep 737 · pending verification 1054 ·
  pending adjudication 203 · retirement candidate 61. This batch's retirement volume by gate was:
  clause 902 · pending adjudication 428 · proof 23 (0.6-19 changed 9 entries that had been
  “audit-confirmed genuine support” to pending verification, so the current values by
  gate/disposition differ from those at the fifth batch's closure).
- **Cleanup accompanying deletions**: helper functions used only by deleted tests, imports that
  became invalid, and struct fields that became unused were removed as well
  (`cargo clippy --all --all-features --all-targets -- -D warnings` reports 0 warnings).
- **Two out-of-bounds deletions that must be recorded**: the mechanical rule still deleted 29
  entries that the census had originally classified as “keep”—23 status-quo anchors (names
  containing unchanged/anchor/guard/preserve/baseline, with many false-positive domain-word matches
  such as “guard” and “preserves,” e.g. `test_while_guard_scope_balanced`) and 6 entries whose
  clauses had audit-confirmed genuine support. This batch followed the closed three-item exception
  list defined by the rubric and did not additionally protect these two categories; to restore them,
  fill them back using the keep/delete comparison table in this page.
- **Failure and abolition of the sample audit**: the 40-entry sample was drawn from the test pool
  with a fixed seed; after the fifth batch's retirement, 38 of the original samples no longer
  existed, so the audit's `--check` consistently reported “38 missing, 38 extra.” The sample audit
  was abolished and deleted at 0.6-19 (the script, data file, and the generator's dependency on it
  were all removed); see “Abolition of the Sample Audit” below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls the entire batch back.

## Sixth retirement batch (executed 2026-11-26)

**Sixth batch: mechanism rollback from the 4.9.9 narrowing round—28 tests deleted along with their
mechanisms were retired, and another 4 prohibition criteria had never been committed.** This batch
does not use the three gates to decide whether a test “should be kept.” Instead, the mechanism under
test was **rolled back wholesale**: the criteria written during the first six rounds of 4.9.9 for
downstream ambiguity mechanisms (candidate-provider priority / candidate-signature comparison /
owner-compatibility inference / IR-side named-argument fallback / import-name resolution-point E5006
prohibition) were retired along with the mechanisms. The mechanism no longer exists, so the tests
have no system under test. These deletions are recorded as a “batch” under coding-rules 7.3 and do
not enter the retirement-candidate pool.

- **What was deleted**:
  - `src/driver/tests/named_args_cross_unit_method.rs`: 30 → 2 (28 deleted: 27 downstream-mechanism
    criteria from the first six rounds and 1 alias-import named-argument criterion). The retained 2
    pin the minimal fix for bare-name imports.
  - `src/driver/tests/type_import_ambiguity.rs`: all 4 E5006 prohibition criteria were retired as an
    entire file—the file existed only in an intermediate 4.9.9 commit, and neither the parent commit
    nor that version's inventory table included it; after the prohibition was withdrawn, there was
    no system under test, so this batch does not decrement the inventory table.
  - The `tests/yaoxiang-multifile/named-args-cross-unit-method-alias-overload/` project fixture was
    retired (the candidate-injection surface for alias imports was rolled back, so the fixture's
    behavior no longer holds); the same-named corpus differential-baseline entry was deleted in the
    same batch. The baseline is stated separately: **relative to the direct parent commit
    `ac810e97`, 405 → 404 lines** (the intermediate state had included the fixture); **relative to
    the pre-4.9.9 baseline `08ccacc5`, 403 → 404 lines** (+1 for the base fixture
    `named-args-cross-unit-method`; the retired `alias-overload` never entered the final history).
- **Which gate applies**: the retained 2 = **history gate** (they pin a real defect registered in
  4.9.9: cross-unit named arguments could not obtain the declaration-phase parameter name, leaving
  `E1014 available` empty); most of the 28 retired entries had originally been history-gate entries,
  but the reason for deleting them **is not the three gates** (it is not a judgment that they lack
  value); rather, the mechanism under test was rolled back.
- **Replacement coverage**: cross-unit named arguments for bare-name imports = the retained 2
  driver-level criteria + the project fixture `named-args-cross-unit-method`; the ambiguity surface
  no longer has rejection logic (the requirement was rolled back), so there is no replacement
  criterion; 4.9.8's dispatch-attribution criteria (10 driver + 5 integration) remain unchanged.
- **Recovery**: `test_method_param_info_empty_name_table_returns_none` in
  `src/middle/lower/tests/query.rs` was restored in the same batch (the empty-name-table guard for
  `method_param_info` is retained; this form is unreachable in the public syntax and is a defensive
  guard, so the unit test exists only to prevent accidental deletion).
- **Numbers** (after this batch was regenerated): repository-wide `#[test]` 2680 entries / 334
  files; the previously committed table had 2688 entries → the current tree has 2680 entries (−8); 1
  entry is present in the current tree but absent from the old table, 9 entries are present in the
  old table but absent from the current tree, and 0 rows with the same key were recalculated.
- **Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the code are
  in the same commit); `git revert <sha>` rolls the entire batch back.

## Abolition of the sample audit (0.6-19)

**Criterion**: the sample audit (0.6-10) measured the “true support rate” of clause citations, but
after 0.6-18 the retention rule no longer examines citations (changed to “for the same symbol and
result category, keep only the strongest one,” reducing the test pool from 3408 to 2055). Of the
original 40 samples, 38 no longer exist, so the audit's `--check` consistently reports “38 missing,
38 extra”; retaining it would only cause continuous alarms. It was therefore **abolished and
deleted**.

**What was deleted**: the sample-audit script under `scripts/dev/` (the one that reproduced the
40-entry sample with a fixed seed) and its JSON data file (the 40 entry-by-entry decisions and
bases).

**The generator's dependency was removed at the same time**: the generator originally read that JSON
and classified “audit-confirmed genuine support” as “clause gate + keep”; that branch has been
removed—**clause citations no longer have an “audit-confirmed” tier** and uniformly fall under
“clause / pending verification.”

**Distribution changes** (not one of the repository-wide 2055 `#[test]` entries was changed; the TSV
was regenerated): keep 746 → 737 (−9: the 9 entries previously audit-confirmed as genuinely
supported moved back to pending verification); pending verification 1045 → 1054; by gate: clause 811
→ 808, pending adjudication 496 → 499 (the others are unchanged: history 313 · proof 424 · passes
neither 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` rolls the entire batch back.

## Rerun and self-check

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical classification only, and states its boundaries faithfully:

- **Delegated assertions are recognized only by name** (`assert*`/`check*` helper functions). If a
  test's assertion is hidden in a helper whose name lacks these two prefixes, it is misclassified as
  “no assertions”—the first batch of 26 (21 retired / 5 kept) and the second batch of 18 (11 retired
  / 7 kept) were each manually reviewed at the function-body level.
- **Function-body boundaries (fixed at 0.6-13)**: when the generator extracts a function body by
  balancing braces, braces inside string/character literals and comments throw the balance off,
  truncating the function body. The body of `test_prefix_info_lbrace` (which contains
  `tokenize("{")`) was once truncated to the end of the file, so the “assertion summary” picked up
  the remainder of the file and placed it in “pending adjudication” rather than “retirement
  candidate.” The generator now masks non-code regions (strings / character literals / line comments
  / block comments / raw strings) before balancing; see “Generator Correction” below.
- **Delegated-assertion recognition (fixed at 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` has a `[A-Za-z_]*` that does not include digits, so calls such
  as `assert_slot_is_42(&interp)` are not recognized as assertions, and tests containing such calls
  are misclassified as “not a single assertion in the body.” At the time, 4 of the repository-wide
  15 “passes neither” entries actually contained assertions (3 numeric helpers + 1 `match` +
  `panic!` form). The name portion has now been expanded to `[A-Za-z0-9_]*`, and the `match` +
  `panic!` form is now recognized; see “Generator Criterion Corrections” below.
- **The proof gate is not judged**: every test that passes both first gates falls under “pending
  adjudication” or “retirement candidate”; the generator never declares that it “should be deleted.”
- **No cross-file deduplication**: completely duplicate tests are not identified in this table
  (0.6-2 measured them at only 1%, so they do not constitute a retirement surface).
- **Clause citations are always pending verification**: a verifiable section number in the file
  header only means that the test “can be matched to an identifiable clause”; it does not mean that
  the citation supports the assertion. The machine does not judge whether it does, so even a
  clause-gate match stops at “pending verification”; a human must verify it before the test can be
  kept.
