---
title: 'Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide the retirement candidate list and the per-batch retirement records'
---

# Test Three-Gate Census (Prerequisite for the 0.6 Major Cleanup)

The prerequisite material for WBS 0.6 “Major Cleanup”: mechanically classify every `#[test]` in the
repository according to the **three gates (clause / incident history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a rerunnable item-by-item decision
table. A stratified sampling audit (40 items) of the clause gate once measured the proportion of
tests that “cite a clause but cannot support their assertions,” which is why the criteria were
**tightened**; that audit was retired and deleted in 0.6-19. **This page and the generator itself
only read tests and do not modify them.** Test retirement is performed in batches (one commit per
batch, with the whole batch revertible via `git revert`), and execution results are recorded in the
per-batch retirement sections below and in `09a`.

Generated artifacts:

| Artifact                            | Purpose                                                                                              |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Item-by-item decision table (2055 rows plus the header), in the same directory as this page           |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, with `--check` and `--selftest`)                                               |

## Definitions

### Scan scope

All `#[test]` functions in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Surface                                                                  | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root package lib (`src/`, including each module’s `tests/` subdirectory) | 1581                 |
| `tests/integration`                                                      | 335                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 75                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **2055** (285 files) |

### Machine criteria for the three gates

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                           | Meaning of a hit                                           |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | A **verifiable item** appears in the first 25 lines of the file header: section-number form `§<number>` (including `RFC-<number> §<number>` and `SPEC §<number>`); for files under `src/**/tests/`, the parent `mod.rs` is inspected as well                                                                                                                                                                                                                | The test can be mapped to an identifiable clause           |
| Incident-history gate             | The test body, function doc comment, or **file header** (`//! Regression context: issue #226`, for example) identifies a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                          | The test pins an incident that actually occurred           |
| Proof gate                        | **The machine cannot determine this** (“Would it turn red if replaced with another correct implementation?” requires human judgment). Therefore, anything that fails the first two gates is classified as “pending judgment,” never automatically as “neither passes.” The sole exception is a test with **no assertions at all in its body** (including no delegation to `assert*`/`check*` helpers, no `match` + `panic!` form, and no `#[should_panic]`) | Only “pending judgment” or “no assertions” can be assigned |

Rationale for tightening: The 0.6-9 clause gate only required the file header to **contain
`§`/`RFC`/`SPEC`**, so 2403 tests were classified as “keep.” The 40-item stratified sample in 0.6-10
found that **half of the citations could not support the assertions** (50% mis-citation rate, 95%
interval 35%–65%)—that measurement is the **direct reason for giving up “keep just because a clause
is cited.”** After tightening, only section-number forms are accepted, and **having a citation still
does not mean that the citation supports the assertion**—a clause citation can only reach “pending
verification.” (That sampling audit was retired and deleted in 0.6-19: as tests were retired the
audit pool shrank until 38 of its 40 samples no longer existed and `--check` permanently reported
“missing 38, extra 38.” The conclusion is kept; the audit artifacts are no longer maintained.)

Rationale for the file-header radius: This repository conventionally writes “Regression context:
issue #N” in the file header and does not repeat it in individual test comments. Looking only at
each test’s own comments would miss the history of an entire file (measured: 7 tests in the test’s
own radius versus 291 in the file-header radius).

### Categories never judged for retirement

The following tests are **always “keep”** and never enter the retirement-candidate pool:

- The `tests/yaoxiang/**` corpus and `tests/project-corpus/**` project shapes (behavioral criterion
  surfaces);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and across process boundaries);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition semantics

| Disposition          | Meaning                                                                                                                                                                                                           |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Only two cases: the incident-history gate is hit; the proof gate applies (spec-shaped directory / current-state-anchor name)                                                                                         |
| Pending verification | A clause is cited (with a section number, or only an entire RFC without a section number), but the citation has not been confirmed by a human—a human must verify it before it can be kept |
| Retirement candidate | Both machine-adjudicable gates fail and the assertion surface contains only weak assertions (or none)—a candidate is not a verdict; the proof gate still requires human review                                    |
| Pending judgment     | Both machine-adjudicable gates fail but strong assertions are present, so the machine assigns no conclusion                                                                                                       |

## Distribution (after tightening)

| Gate                                                                              | Count    |
| --------------------------------------------------------------------------------- | -------- |
| Clause (with a verifiable item)                                                   | 808      |
| Incident history                                                                  | 313      |
| Proof (spec-shaped criterion surface / current-state anchor)                      | 424      |
| Pending judgment (both machine-adjudicable gates fail; proof awaits human review) | 499      |
| Neither passes (no assertions in the body)                                        | 11       |
| **Total**                                                                         | **2055** |

| Disposition          | Count    | Compared with 0.6-9                                                                                                                                                                                      |
| -------------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | **737**  | 2897 (−2160)                                                                                                                                                                                             |
| Pending verification | 1054     | 0 (new tier)                                                                                                                                                                                             |
| Pending judgment     | 203      | 412 (−1 after the second retirement batch, +3 from the 0.6-15 criterion correction, −211 from the fifth batch)                                                                                            |
| Retirement candidate | 61       | 136 (−21 from the first batch, −11 from the second, +1 from the generator correction, −2 from the third, −3 from the 0.6-15 criterion correction, −1 from the redundancy scan, −1 from the fourth batch, −37 from the fifth batch) |
| **Total**            | **2055** | —                                                                                                                                                                                                        |

Two changes to the criteria need to be explained:

1. **Structure of “keep”**: 737 = 424 proof-gate tests + 313 incident-history-gate tests. The 2121
   tests in the original 2897 that were “kept based on a single citation” are now pending
   verification (after the 0.6-19 retirement of the sampling audit, the 9 entries that had been kept
   because the audit confirmed genuine support also fall back to pending verification—clause
   citations no longer have an “audit-confirmed” tier).
2. **Incident-history gate 47 → 313**: This does not represent newly discovered incident histories;
   it is a **relocation of labels**. In 0.6-9, a test that hit both the clause and incident-history
   gates was recorded as a clause test. After tightening, a clause hit no longer automatically means
   “keep,” so the incident-history gate must catch these tests, and its count includes them. The
   total number kept is therefore unaffected (those tests were also kept in 0.6-9).

File-level scope: Of 285 test files, **136** have a **verifiable section number** in the file header
(the old 0.6-9 criterion yielded 221).

## Reconciliation with the 0.6-2 Survey

The 0.6-2 survey figures form the first comparison baseline. The reasons for each difference are
listed below:

| Item                            | 0.6-2 Survey                  | This Census                                                                                                 | Reason for the Difference                                                                                                                                                                                                                                                   |
| ------------------------------- | ----------------------------- | ----------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                        | Two tests were added after 0.6-2 (the check-criterion regression tests for E3005 and E3022), then 21 no-assertion tests were deleted according to the first batch, 11 weak-assertion tests according to the second batch, 2 tests according to the third batch, 1 according to the fourth batch, and 1353 according to the fifth batch; net −388 |
| Number of test files            | 296                           | 285                                                                                                         | The fifth batch retired 11 whole files                                                                                                                                                                                                                                                                      |
| Clause gate                     | 227 files (77%)               | 136 files after tightening; 808 by test                                                                    | The criterion was tightened from “the word appears” to “there is a section number `§<number>`,” so the file count was nearly halved; the fifth batch retired 902 clause tests, and the 0.6-19 retirement of the sampling audit moved another 9 entries from “keep” to pending verification                                                                          |
| Incident-history signal         | 442 tests / 148 files         | 313 tests in the gate; three radii: test itself 7 / file header 291 (38 files) / whole file 1218 (79 files) | The 148 files in 0.6-2 were broader than the 79 files in this census’s whole-file radius and used a broader vocabulary and scope; this census strictly uses the four defined signals.                                                                                       |
| Characterization strong signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                | Characterization is not one of the three gates; mixing it into the retirement conclusion would exceed the census’s authority                                                                                                                                                |
| Weak assertions                 | 473 (14%)                     | See the TSV “Assertion Summary” column                                                                      | The counting units differ: 0.6-2 counted the pattern “asserts only is_ok/is_some,” whereas this census uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                                       |

## Conclusion Summary

1. **The clause gate was previously significantly overstated**: under the old criterion, 2403 tests
   were classified as “clause/keep,” while the sample measured a 50% mis-citation rate (95% interval
   35%–65%). “Having a citation” and “having a citation that supports the assertion” are different
   things; the old criterion treated the former as the latter. (That sampling audit was retired and
   deleted in 0.6-19; the conclusion is kept—see “Sampling-Audit Retirement” below.)
2. **Approximately 1202 tests genuinely have clause support after tightening** (846–1557); the rest
   must either await a backfilled clause or be judged through the incident-history/proof gates.
3. **The retirement surface is unrelated to clause mis-citations**: the 136 candidates were
   unrelated to clauses (no assertions / weak assertions). The first batch retired 21 tests, the
   second retired 11, the generator correction added 1, the third batch removed 2, the 0.6-15
   criterion correction removed 3, the redundancy scan removed 1, the fourth batch removed 1, and the
   fifth batch removed 1353 (by the mechanical rule “keep only the strongest test per symbol and per
   result category”), reducing the candidates to 61. The fifth batch’s retirement volume by gate:
   clause 902 · pending judgment 428 · proof 23 · incident history 0 · neither passes 0—**the
   incident-history gate and the zero-assertion line lost nothing**.
4. **The incident-history gate is the largest legitimate source of “keep”** (313 tests): it
   demonstrates that “pinning a real incident” is more reliable than “copying an RFC sentence.”

## First Retirement Batch (Executed 2026-11-06)

**First batch: of the 26 “no assertion” tests in `src/frontend/core/types/eval/tests/`, 21 were
retired and 5 were kept** (a single directory cluster; criterion = **all three gates fail**
according to Part Seven of coding-rules: no clause-gate hit, no incident-history-gate hit, and no
assertions anywhere in the body). The item-by-item decisions and “same-cell coverage” are provided
by the 0.6-11a decision table (21 deleted / 5 kept); this page does not repeat the rationales and
records only the execution results.

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
  commit), so `git revert <sha>` rolls back the whole batch.

## Second Retirement Batch (Executed 2026-11-06)

**Second batch: of the 18 “weak assertion” tests in `src/frontend/core/parser/pratt/tests/nud.rs`,
11 were retired and 7 were kept** (a single directory cluster = the pratt prefix-parsing module;
criterion = **all three gates fail** according to Part Seven of coding-rules: no clause-gate hit, no
incident-history-gate hit, and the body contains only weak assertions such as `is_some()` /
`is_none()`—neither pinning binding strength nor the parse result).

The difference between this batch and the first needs to be made explicit: these tests **do contain
assertions** and are not empty tests. The reason for retirement is “all three gates fail + zero
criterion increment,” not “no assertions.” Every test selected for deletion identifies a
**unit-level replacement-coverage test in the same module**.

- **11 tests retired**: `test_prefix_info_int_literal`, `test_prefix_info_float_literal`,
  `test_prefix_info_string_literal`, `test_prefix_info_bool_literal`, `test_prefix_info_lparen`,
  `test_prefix_info_lbracket`, `test_prefix_info_if`, `test_prefix_info_while`,
  `test_prefix_info_for`, `test_prefix_info_ref`, `test_prefix_info_return`. Every replacement
  follows the **same** `parse_expression → parse_prefix → prefix_info` path and has a stronger
  assertion (pinning the AST shape rather than merely checking `is_some`): `test_parse_int_literal`
  / `test_parse_float_literal` / `test_parse_string_literal` / `test_parse_bool_true` /
  `test_parse_if_expr` / `test_parse_while_expr` / `test_parse_for_expr` / `test_parse_return_expr`
  in the same file, plus `src/frontend/core/parser/tests/expressions.rs`’s `test_grouped_expr` /
  `test_tuple` / `test_list_literal` / `test_list_empty` / `test_ref_expr`.
- **7 tests kept**: `test_prefix_info_match`, `test_prefix_info_unsafe`, `test_prefix_info_spawn`,
  `test_prefix_info_at`, `test_prefix_info_break_removed`, `test_prefix_info_continue_removed`,
  `test_prefix_info_eof`—see the item-by-item rationales in the “VI. Second Test Retirement Batch”
  decision table in `09a-wbs-history.md`. There are two reasons for keeping them: **the proof gate
  applies** (the first three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are the
  sole unit-level observation points for their corresponding facts, and the repository has no second
  unit test that observes them), or **the incident-history gate applies** (`break_removed` /
  `continue_removed` explicitly name #314).
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls back the whole batch.

## Generator Correction (0.6-13)

**`body_span` truncates function bodies at braces inside literals — fixed.**

The old implementation located a function body by balancing braces without distinguishing real code
from braces in string / character literals or comments. Forms such as `tokenize("{")` are common in
Rust source, so the balance was thrown off and the function body was truncated: the body of
`test_prefix_info_lbrace` was once truncated at the end of the file, causing the “assertion summary”
to capture the remainder of the file (`matchesx11 assertx24 …`). It was consequently binned as
“pending judgment” instead of “retirement candidate”—**the wrong bin**.

Fix: add `mask_noncode` to replace the contents of string literals / character literals / line
comments / block comments (which may be nested) / raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (preserving newlines exactly, so line numbers and line counts do not change),
and only then balance the braces. **The mask is used only to locate function-body boundaries**;
assertion counting and the incident-history gate still read the original text, and the criteria
themselves have not been loosened. `--selftest` now includes four cases (paired / unpaired braces in
ordinary strings, braces in raw strings and block comments, and braces in character literals), each
followed by a sentinel test—the old implementation found the wrong boundary in all four cases, while
the new implementation finds all four correctly.

### Tests affected by the same bug: 11

Counting method: compare the **function-body boundaries before and after the fix** test by test,
file by file (old implementation: balance braces directly; new implementation: balance after
masking). Tests with different boundaries are affected. Of 3413 tests in the repository, the old
implementation found the wrong boundary for **11**:

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

### Bin differences before and after the fix: 2

For the other nine, **eight only changed the “assertion summary” column** (their bin was unchanged),
and **one did not change even the summary** (`test_fstring_close_brace_escape_preserved_raw`: the
old implementation omitted the closing `}` line, which contained no assertion terms). The eight
summary changes are listed individually in the corresponding section of `09a-wbs-history.md`.

| Test Name                   | File                                          | Old Bin                             | New Bin                                     | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --------------------------- | --------------------------------------------- | ----------------------------------- | ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending judgment / Pending judgment | Pending judgment / **Retirement candidate** | The real function body contains only one weak assertion, `assert!(state.prefix_info().is_some())`; the file header and function body contain neither a § section number nor issue/regression/defect/incident terms ⇒ both the clause gate and incident-history gate fail, and the body contains only a weak assertion such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires human review)                                         |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | Incident history / Keep             | Clause / **Pending verification**           | The old implementation truncated the body at the end of the file and swallowed text containing “regression / defect / incident,” falsely triggering the incident-history gate; the real function body contains no such terms ⇒ the incident-history gate no longer applies; the file header contains verifiable items (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate is hit, but the citation was not confirmed by a human ⇒ pending verification |

The distribution consequently changes: clause 1713 → 1714 · incident history 314 → 313 · keep 776 →
775 · pending verification 2121 → 2122 · pending judgment 412 → 411 · retirement candidate 104
→ 105. **The repository-wide `#[test]` count is unchanged (3413)**—this round changed only the
implementation of the criteria, not any test.

## Third Retirement Batch (Executed 2026-11-07)

**Third batch: of the 5 retirement candidates in `src/middle/passes/codegen/tests/`, 2 were retired
and 3 were kept.** (a single directory cluster = the `middle::passes::codegen` module under test;
criterion = **all three gates fail** according to Part Seven of coding-rules.)

This batch continues the **retention criterion** established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus / integration tests belong
to the process-boundary layer and cannot replace a unit-level observation point.

- **2 tests retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—its function body is **word-for-word
    identical** to `test_global_slot_store_then_load_roundtrip` in the same file (apart from the
    function name): both are `run_fixture()` + `assert_slot_is_42(&interp)`. Its doc comment claims
    to verify that “the `execute_module` entry path returns non-Void,” but the body never observes
    the return value of `execute_module` (`run_fixture` discards it via `.expect(...)`)— the claimed
    intent is not implemented, and the function body is a pure duplicate ⇒ zero criterion increment.
    Replacement coverage = `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file strictly covers it:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` has the same shape as the inline `Symbol` in this
    test). “A name that was never inserted cannot be found” and “a name cannot be found after it is
    popped” follow the same path in `SymbolScopeManager::lookup` (walk each `scope.get(name)` layer,
    returning `None` when not found) ⇒ this test adds zero coverage. Replacement coverage =
    `test_scope_nesting`.
- **3 tests kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that does
    “not use serialization and directly executes” (the serialization variant is a different path),
    and `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization round trip (`write_to` → `read_from` → execute).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit-level observation point for the
    `id > 255` rejection branch of `OperandResolver::to_reg` (the bytecode layer has independent
    overflow checks and does not cover this function; no second test in the repository makes
    `to_reg` go out of range). `is_err()` is a discriminating assertion (removing the guard would
    turn the test red), not merely “proof that it did not panic.”
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls back the whole batch.

### One census blind spot found in this batch (**fixed in 0.6-15**, see “Generator Criterion Correction” below)

The census’s “delegated assertion” recognition pattern `\b(assert|check)[A-Za-z_]*\s*\(` **does not
match helpers whose names contain digits**: `[A-Za-z_]*` does not include digits, so a call such as
`assert_slot_is_42(&interp)` **is not recognized as an assertion**— 2 of the 5 candidates in this
batch were consequently mis-binned as “no assertions anywhere in the body.”

After reviewing all 15 tests classified as “neither passes” item by item (broadening helper names to
include digits and additionally recognizing the `match` + `panic!` form), **4 tests do in fact
contain assertions**—3 use the numeric helpers above (`assert_slot_is_42`), and 1 is
`test_weak_from_arc` in `src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form;
test-specification rule 6.1 requires `assert!(matches!(...))`, but the test does in fact assert).
The remaining 11 tests genuinely contain no assertions.

**Acting directly on the machine’s “neither passes” classification would have deleted these 4 tests
even though they contain criteria.** The item-by-item human review in this batch prevented that; the
generator-side fix is described in “Generator Criterion Correction (0.6-15)” below.

## Generator Criterion Correction (0.6-15)

**Two criterion blind spots have been fixed: helper names containing digits, and the `match` +
`panic!` form.** This round **does not delete any test** (the repository-wide `#[test]` count
remains 3411); it only changes how the generator recognizes “whether the body contains assertions.”

### What was fixed

1. **The name portion of delegated-assertion recognition now permits digits**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern did not
   match helpers such as `assert_slot_is_42(&interp)`, so tests containing such calls were treated
   as having “no assertions anywhere in the body.”
2. **Added the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the assertion is written
   in one of the arms, so no `assert!` macro is visible lexically (test-specification rule 6.1
   requires `assert!(matches!(...))`, but existing code contains this form). Detection method: the
   `match` keyword and the first `{` (while scanning the expression) open a block, whose boundary is
   determined by **brace balancing**; the form counts only if `panic!(` appears inside it.
   **Detection operates on masked lines**— braces in strings and comments do not participate in
   balancing, and `panic!` in a comment is not treated as an assertion.

### Bin differences: 3 tests (both gate and disposition changed)

| Test Name                                           | File                                           | Old Bin                               | New Bin                             | Rationale                                                                                                                                                                                                                                                    |
| --------------------------------------------------- | ---------------------------------------------- | ------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Neither passes / Retirement candidate | Pending judgment / Pending judgment | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, and the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending judgment / Pending judgment | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains a digit, which the old pattern did not match; it is now recognized as an assertion                                                                                       |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Neither passes / Retirement candidate | Pending judgment / Pending judgment | Same as above (the serialization-round-trip test)                                                                                                                                                                                                            |

A further **77 tests changed only the “assertion summary” column** (the gate and disposition were
unchanged): most gained `match_panicxN` in the summary (the body contains the `match` + `panic!`
form), and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs` has 3, and
`src/package/tests/cache.rs` has 1—helper names containing digits).

**The relaxation changes only the “neither passes” tier**: that tier is the machine’s **confident
verdict** (no assertions anywhere in the body), and its criterion is a hard requirement; “retirement
candidate” is only a pool awaiting human review, so the size of the pool is not a verdict. All other
bins, including the weak-assertion pool, continue to use the previous criteria. Result: gate—neither
passes 14 → 11 · pending judgment 923 → 926; disposition—retirement candidate 103 → 100 · pending
judgment 411 → 414.

### Item-by-item review of the 11 remaining “neither passes” tests

After reading the original source of all 11 tests classified as “neither passes,” **all 11 genuinely
contain no assertions** (including no `assert*` / `check*` helpers, no `match` + `panic!` form, and
no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                     | Review Conclusion                                           |
| ----------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | Five `.expect(...)` calls that merely guarantee it does not crash                               | Genuinely no assertions (proves only that it did not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains a `context()` handle and discards it                                                    | Genuinely no assertions                                     |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer`s, obtains their contexts, and discards them                       | Genuinely no assertions                                     |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                             | Genuinely no assertions                                     |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                       | Genuinely no assertions                                     |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                              | Genuinely no assertions                                     |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` with invalid content and discards the result                      | Genuinely no assertions (proves only that it did not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                                | Genuinely no assertions (proves only that it did not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                          | Genuinely no assertions (proves only that it did not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                            | Genuinely no assertions                                     |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs a `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertions                                     |

Five of these (`test_normalizer_context` and the four in reducer.rs) were kept because the 0.6-11a
decision table **could not identify replacement coverage**; this review reaches the same conclusion.

### Reverse spot-check: Did the relaxation go too far?

Five tests were selected with a fixed seed (20261107) from the 88 items “newly recognized as
assertions,” and their source was read to confirm that they really assert:

| Spot-checked Item                                           | File                                                 | Assertion in the Source                                                                                                                              |
| ----------------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | The matching arm contains two `assert_eq!`; the non-matching arm uses `panic!("Expected AssocType")`                                                 |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | The matching arm contains `assert!(msg.contains("src/evil.yx"))`; the non-matching arm uses `panic!("expected InvalidPackage, got …")`               |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | The matching arm uses `assert_eq!` to compare constraint types; the non-matching arm uses `panic!("不是 Dynamic 变体…")`                             |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | The matching arm contains `assert!(!delta.edits.is_empty())`; the non-matching arm uses `panic!("Expected TokensDelta variant")`                     |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | The matching arm contains three `assert_eq!` / `assert!` checks of the error code and message; the non-matching arm uses `panic!("不是 Disproved…")` |

All five assert a concrete value or variant (replacing the implementation with another correct path
would turn them red)—**the relaxation did not go too far**. The reverse controls were also added to
`--selftest`: a `match` with no `panic!`, and a `panic!` that appears only in a comment; **neither
may** be recognized as an assertion.

### The criteria have teeth (developmental proof)

Revert the generator to the old implementation (`HELPER_RE` without digits + `match` + `panic!`
recognition disabled), and the newly added `--selftest` cases all turn **red**, with exit code 1;
the new implementation passes them all.

### Rollback

The generator and census table are in the same commit, so `git revert <sha>` restores the entire
state.

## Fourth Retirement Batch (Executed 2026-11-07)

**Fourth batch: of the 10 “retirement candidates” in the directory cluster
`src/backends/interpreter/` (the interpreter backend’s test directories), 1 was retired and 9 were
kept** (a single directory cluster, with all three files read before making any decisions: 4 in
`tests/bytecode_load.rs` · 2 in `tests/weak.rs` · 4 in `executor/tests/debug.rs`). Criterion = the
**three gates** according to Part Seven of coding-rules; retention criterion = “a unit-level
observation point can only be replaced by another unit test; corpus/integration tests belong to the
process-boundary layer”; **deletion requires identifying a specific replacement-coverage test; if no
replacement can be identified or the test cannot be understood, retain it**.

- **1 test retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, followed by one step
  and retrieval of `capture_stack`), and its assertion is **strictly stronger**: it asserts that the
  stack is nonempty and that `stack[0].function_name == "main"` (pinning the top-of-stack position),
  whereas the listed test only asserts `stack.iter().any(|f| f.function_name == "main")` (any
  position is sufficient) —the former being true implies the latter ⇒ zero criterion increment.
- **9 tests kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each test, each is the
  **sole unit-level observation point** for a property (the load magic-number/version branches are
  distinct, the YXBC routing layer, the Some/None sides of Weak, stepping with no frame, step_out
  popping the stack, and running to completion with an empty stack), and no replacement unit test
  can be identified ⇒ retain under the hard requirement. See Section Eleven of
  `docs/src/dev/architecture/09a-wbs-history.md` for the item-by-item rationales (the decision table
  is kept in the transcript rather than on this page).
- **Numbers**: Repository-wide `#[test]` 3409 → 3408; one deletion = `git diff` counts one
  `-#[test]` line and zero `+#[test]` lines; the dominating test
  `test_capture_stack_during_execution` passes when run alone. Census: retirement candidate 99 → 98
  · pending judgment 925 → 924 · clause 1713 · incident history 313 · proof 447 · neither passes 11
  · pending verification 2121 · keep 775.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls back the whole batch.

## Fifth Retirement Batch (Executed 2026-11-07)

**Fifth batch: 1353 unit tests retired in bulk by the mechanical rule “for one symbol and one result
category, keep only the strongest assertion”** (only unit tests under `src/**/tests/**` were
touched; the E2E surfaces `tests/yaoxiang/**`, `tests/integration/**`,
`tests/formatter_integration.rs`, `tests/yx_runner*.rs` and `tests/project-corpus/**` were left
untouched).

- **Rule (mechanical, no per-item argumentation)**: for one symbol under test and one assertion result
  category, keep only the test with the strongest assertion and delete the rest.
  - The symbol under test = the repository-internal symbol called in the test body (one level through
    helpers in the same file); a test that reaches several symbols is classified by its **main
    symbol** (the first repository-internal call). Symbols are qualified by their repository-internal
    path; bare names are never merged across modules.
  - Result category = a concrete value / a specific error code (E code) / a specific variant match /
    only-ok / only-err / only-should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or `match` + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; within one group of equal strength the
    first occurrence in the file is kept.
  - A test that asserts several categories at once (no determinable main category) is **kept**, not
    guessed.
- **Three exceptions (none of them lost)**: ① tests whose name or comment names an `issue #` /
  regression / defect / incident (the incident-history gate); ② a symbol that has only one test (the
  observation point would disappear with it); ③ `#[should_panic]` that is the symbol’s only
  should_panic case. The 313 incident-history tests and the 11 “neither passes” tests lost **0**
  entries.
- **Numbers**: Repository-wide `#[test]` 3408 → 2055; 1353 deletions = `git diff` counts 1353
  `-#[test]` lines and 0 `+#[test]` lines. Census by gate (current values, after the 0.6-19
  retirement of the sampling audit): clause 808 · incident history 313 · proof 424 · pending judgment
  499 · neither passes 11. Census by disposition (current values): keep 737 · pending verification
  1054 · pending judgment 203 · retirement candidate 61. The batch’s own retirement volume by gate:
  clause 902 · pending judgment 428 · proof 23 (0.6-19 reclassified the 9 “the audit confirms genuine
  support” entries as pending verification, which is why the current values differ from the ones at
  the close of the fifth batch).
- **Cleanup with the deletions**: helpers used only by deleted tests, the imports that became
  unused, and the struct fields that became dead were removed as well
  (`cargo clippy --all --all-features --all-targets -- -D warnings` reports 0 warnings).
- **Two kinds of over-deletion that must be recorded**: 29 entries that the census had classified as
  “keep” were still deleted by this batch’s rule—23 current-state anchors (names containing
  unchanged/anchor/guard/preserve/baseline, mostly **domain words** that matched by accident, such
  as `test_while_guard_scope_balanced`) and 6 entries whose clause support the audit had confirmed.
  The batch executed the closed list of three exceptions from the card and did not protect these two
  kinds additionally; restoring them only takes a small follow-up commit.
- **Rollback**: The entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit), so `git revert <sha>` rolls back the whole batch.

## Sampling-Audit Retirement (0.6-19)

**Criterion**: The sampling audit (0.6-10) measured the “real support rate of clause citations,”
but after 0.6-18 the retention rule no longer looks at citations at all (it became “for one symbol
and one result category, keep only the strongest,” and the pool shrank from 3408 to 2055). 38 of the
original 40 samples no longer exist, so the audit’s `--check` permanently reported “missing 38, extra
38”—keeping it would only produce a permanent alarm. Hence it was **retired and deleted**.

**What was deleted**: the sampling-audit script (the one that reproduces the 40-item sample with a
fixed seed) and its JSON data file (the 40 item-by-item decisions and rationales), both under
`scripts/dev/`.

**The generator was de-coupled at the same time**: it used to read that JSON and classify “the audit
confirms genuine support” as “clause gate + keep.” That branch is gone—**clause citations no longer
have an “audit-confirmed” tier** and always land in “clause / pending verification.”

**Distribution change** (the repository-wide `#[test]` count of 2055 is **unchanged**, and the TSV
has been regenerated): keep 746 → 737 (−9: the 9 entries the audit had confirmed fall back to
pending verification); pending verification 1045 → 1054; by gate: clause 811 → 808, pending judgment
496 → 499 (the others are unchanged: incident history 313 · proof 424 · neither passes 11).

**Rollback**: The entire batch is one commit (this page, `test-gate-census.tsv` and the generator are
in the same commit), so `git revert <sha>` rolls back the whole batch.

## Rerun and Self-Check

```bash
python scripts/dev/test-gate-census.py            # Recalculate and overwrite the TSV (two runs are byte-for-byte identical)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: check whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical determination only, and states its boundaries honestly:

- **Delegated assertions are identified by name only** (`assert*`/`check*` helper functions). If a
  test’s assertion is hidden in a helper whose name does not have one of these prefixes, it is
  misclassified as “no assertions”—the first batch of 26 (21 retired / 5 kept) and the second batch
  of 18 (11 retired / 7 kept) were all manually reviewed function by function.
- **Function-body boundaries (fixed in 0.6-13)**: The generator balances braces when locating a
  function body, but braces in string / character literals and comments skew the balance and
  truncate the body—`test_prefix_info_lbrace` (which contains `tokenize("{")`) was once truncated at
  the end of the file, causing the “assertion summary” to capture the rest of the file and the test
  to be binned as “pending judgment” instead of “retirement candidate.” The generator now masks
  non-code regions (strings / character literals / line comments / block comments / raw strings)
  before balancing the braces; see “Generator Correction” below.
- **Delegated-assertion recognition (fixed in 0.6-15)**: The old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` does not include digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions, and tests containing such calls
  were misclassified as having “no assertions anywhere in the body.” At the time, 4 of the
  repository’s 15 “neither passes” tests actually contained assertions (3 numeric helpers + 1
  `match` + `panic!` form). The name portion is now `[A-Za-z0-9_]*`, and the `match` + `panic!` form
  is recognized; see “Generator Criterion Correction” below.
- **The proof gate is not adjudicated**: Any test that fails both machine-adjudicable gates is
  classified only as “pending judgment” or “retirement candidate”; the generator never announces
  that it “should be deleted.”
- **No cross-file deduplication**: Exactly duplicated tests are not identified in this table (0.6-2
  measured only 1%, which does not constitute a retirement surface).
- **Clause citations are always pending verification**: a verifiable section number in the file
  header only means the test can be mapped to an identifiable clause; it does not mean that the
  clause supports the assertion. The machine does not adjudicate that, so a clause hit reaches only
  “pending verification” and a human must verify it before it can be kept.
