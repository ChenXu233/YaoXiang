---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)'
description:
  'Mechanically classify every test in the repository according to the three gates in Part Seven of
  coding-rules, and provide a retirement-candidate list and retirement records for each batch'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Major Cleanup)

Pre-work for WBS 0.6 "Major Cleanup": mechanically classify every `#[test]` in the repository
according to the **three gates (clause / history / proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a rerunnable, row-by-row adjudication
table. The clause-gate sample audit (40 entries) measured the proportion of entries that "cited a
clause but could not support the assertions" and thereby **tightened the criteria**; that audit was
repealed and deleted in 0.6-19. **This page and the generator itself only read the tests and do not
modify them**; test retirements are performed in batches (one commit per batch, revertible as a
whole with `git revert`), and execution results are recorded in the per-batch retirement subsections
below and in commit messages.

Generated artifacts:

| Artifact                            | Purpose                                                                                       |
| ----------------------------------- | --------------------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Row-by-row adjudication table (2646 rows plus the header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable, supports `--check` and `--selftest`)                                   |

## Criteria

### Scan Surface

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

| Gate                              | Machine criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                | Meaning of a hit                                                       |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Clause gate (tightened in 0.6-10) | The first 25 lines of the file contain a **verifiable entry**: section-number form `§<数字>` (including `RFC-<数字> §<数字>` and `SPEC §<数字>`); files under `src/**/tests/` are considered together with their parent `mod.rs`                                                                                                                                                                                                                                 | The test maps to an identifiable clause                                |
| History gate                      | The test function body, function documentation comment, or **file header** (such as `//! 回归背景：issue #226`) names a real failure: `issue #N` / regression / defect / incident                                                                                                                                                                                                                                                                                | The test pins down an incident that actually occurred                  |
| Proof gate                        | **The machine cannot decide this** (whether a test would fail with a different but correct replacement implementation requires human judgment). Therefore, anything that misses both preceding gates is always classified as "pending adjudication", never automatically as "fails all gates"; the only exception is a body with **no assertions at all** (including no delegated `assert*/check*` helper, no `match` + `panic!` form, and no `#[should_panic]`) | It can only be classified as "pending adjudication" or "no assertions" |

Rationale for tightening: the 0.6-9 clause gate only required the file header to **contain the
strings `§`/`RFC`/`SPEC`**, so 2403 entries were classified as "keep". The 40-entry stratified
sample measured in 0.6-10 found that **half of the citations could not support the assertions**
(mis-citation rate: 50%; 95% interval: 35%–65%)—this measurement was the **direct reason for
abandoning the practice of keeping a test based on a single citation**. After tightening, only the
section-number form is recognized, and **having a citation still does not mean that the citation
supports the assertion**—clause citations always stop at "pending verification". (This sample audit
was repealed and deleted in 0.6-19: as the audit pool shrank with test retirements, 38 entries in
the original sample no longer existed, and `--check` always reported "missing 38, extra 38"; the
conclusion is retained, but the audit artifacts are no longer maintained.)

Rationale for the file-header radius: this repository conventionally places text such as "regression
background: issue #N" in the file header and does not repeat it in each test comment; inspecting
only the test's own comments would miss the history of the entire file (measured: 7 entries at the
test's own radius, 291 entries at the file-header radius).

### Categories That Are Never Retired

The following tests are **always classified as "keep"** and never enter the retirement-candidate
pool:

- Corpus tests under `tests/yaoxiang/**` and project-form corpora under `tests/project-corpus/**`
  (criterion surfaces);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and crossing a process boundary);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

| Disposition          | Meaning                                                                                                                                                                                        |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keep                 | Only two cases: the history gate is hit, or the proof gate applies (spec-shaped directories / current-state anchor names)                                                                      |
| Pending verification | A clause is cited (with a section number, or as an entire RFC without a section number), but the citation has not been manually confirmed—it must be verified by a human before it can be kept |
| Retirement candidate | Both gates are missed and the assertion surface contains only weak assertions (or no assertions)—a candidate is not a verdict; the proof gate still requires manual review                     |
| Pending adjudication | Both gates are missed, but strong assertions are present; the machine gives no verdict                                                                                                         |

## Distribution (Regenerated on 2026-11-24)

| Gate                                                                  | Count    |
| --------------------------------------------------------------------- | -------- |
| Clause (has a verifiable entry)                                       | 997      |
| History                                                               | 539      |
| Proof (spec-shaped criterion surface / current-state anchors)         | 568      |
| Pending adjudication (misses both gates; proof awaits human judgment) | 536      |
| Fails all gates (no assertions anywhere in the body)                  | 6        |
| **Total**                                                             | **2646** |

| Disposition          | Count    | Compared with 0.6-19 (2026-11-07) |
| -------------------- | -------- | --------------------------------- |
| Keep                 | **1107** | 737 (+370)                        |
| Pending verification | 1259     | 1054 (+205)                       |
| Pending adjudication | 222      | 203 (+19)                         |
| Retirement candidate | 58       | 61 (−3)                           |
| **Total**            | **2646** | 2055 (+591)                       |

Keep structure: 1107 = proof gate 568 (516 spec-shaped criterion-surface entries + 52 current-state
anchors) + history gate 539. File-level basis: among 328 test files, **160** have a verifiable
section number in the file header.

Regeneration reconciliation: the old table contains 2055 entries, while the current tree contains
2646; there are 639 keys present in the current tree but absent from the old table. Of these, 25
were added by task 8.8 in this round, 606 were existing entries that had not previously been
regenerated, and 8 were added by the `grammar_probe` in the 8.3 go/no-go probe. There are 48 keys
present in the old table but absent from the current tree. Per-key records for the 25 new tests, 606
existing entries, and 8 probes are all in the TSV; the per-entry destination of the 48 stale
keys—"deleted / renamed / moved"—is recorded in the task 8.3 commit message. Another 46 entries with
the same keys had their line contents recomputed against the current source; they are not counted as
added or deleted keys.

### Two Changes in Criteria at the 0.6-19 Cutoff

1. **Keep structure**: 737 = proof gate 424 (390 spec-shaped criterion-surface entries + 34
   current-state anchors) + history gate 313. Of the original 2897 entries, the 2121 that were kept
   "on the strength of a single citation" are now pending verification (after the sample audit was
   repealed in 0.6-19, the 9 entries previously classified as "audit confirmed genuine support" also
   moved to pending verification—clause citations no longer have an "audit confirmed" tier).
2. **History gate 47 → 313**: this is not newly added history, but **relocation of labels**—in
   0.6-9, entries that hit both the clause and history gates were recorded as clause hits. After
   tightening, a clause hit no longer automatically results in a keep, so these entries must be
   caught by the history gate; their counts are therefore folded into history. The total number kept
   is unaffected (this batch of tests was also kept in 0.6-9).

File-level basis at the time: among 285 test files, **136** had a verifiable section number in the
file header (the old 0.6-9 criterion produced 221).

## Reconciliation Between the 0.6-19 Cutoff and the 0.6-2 Survey

The 0.6-2 survey figures are the first comparison baseline. The reasons for each difference are
listed below:

| Item                            | 0.6-2 Survey                  | This Run                                                                                                           | Reason for the Difference                                                                                                                                                                                                                                                                           |
| ------------------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`       | 3443                          | 2055                                                                                                               | 2 entries were added after 0.6-2 (regression tests for the E3005 and E3022 check criteria), followed by the deletion of 21 assertion-free tests in the first batch, 11 weak-assertion tests in the second batch, 2 in the third batch, 1 in the fourth batch, and 1353 in the fifth batch; net −388 |
| Number of test files            | 296                           | 285                                                                                                                | 11 entire files were retired in the fifth batch                                                                                                                                                                                                                                                     |
| Clause gate                     | 227 files (77%)               | 136 files after tightening; 808 entries                                                                            | The criterion was tightened from "contains a keyword" to "has a section number `§<数字>`", nearly halving the file count; the fifth batch retired 902 entries, and another 9 moved from "keep" to pending verification after the sample audit was repealed in 0.6-19                                |
| History signals                 | 442 entries / 148 files       | 313 entries within the gate; three radii: test itself 7 / file header 291 (38 files) / entire file 1218 (79 files) | The 148 files in the 0.6-2 survey were broader than the current entire-file radius of 79 files and used a larger vocabulary/scope; this run strictly follows the four criteria-card signals                                                                                                         |
| Strong characterization signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                                       | Characterization is not one of the three gates; mixing it into the retirement conclusion would exceed the gate's authority                                                                                                                                                                          |
| Weak assertions                 | 473 (14%)                     | See the TSV "assertion summary" column                                                                             | The counting units differ: 0.6-2 counted patterns that only asserted `is_ok/is_some`; this run uses lexical counting and distinguishes strong, weak, and delegated assertions                                                                                                                       |

## 0.6-19 Conclusion Summary (Historical)

1. **The clause gate had previously been significantly overstated**: under the old criterion, 2403
   entries were classified as "clause/keep", while the measured mis-citation rate was 50% (95%
   interval: 35%–65%)—"citing a clause" and "having the citation support the assertion" are two
   different things, and the old criterion treated the former as the latter. (This sample audit was
   repealed and deleted in 0.6-19; the conclusion is retained. See "Sample Audit Repeal" below for
   the reason for deletion.)
2. **After tightening, approximately 1202 entries genuinely have clause support** (range: 846–1557);
   the rest either require pending verification and supporting evidence or should have been
   adjudicated through the history/proof gates.
3. **The retirement surface is unrelated to the clause mis-citation surface**: the 136 candidates
   have no clause support (no assertions/weak assertions); the first batch retired 21, the second
   batch retired 11, the generator correction added 1, the third batch removed 2, the 0.6-15
   criterion correction removed 3, the redundancy scan removed 1, the fourth batch removed 1, and
   the fifth batch removed 1353 (using the mechanical rule "for the same symbol and result category,
   keep only the strongest entry"), reducing the candidates to 61. The fifth-batch retirement
   distribution by gate was: clause 902 · pending adjudication 428 · proof 23 · history 0 · fails
   all gates 0—**not a single entry from the history gate or zero-assertion line was removed**.
4. **The history gate is the largest legitimate source of keeps** (313 entries): it demonstrates
   that "pinning down a real incident" is more reliable than "copying a sentence from an RFC".

## First Retirement Batch (Executed on 2026-11-06)

**First batch: of 26 assertion-free tests under `src/frontend/core/types/eval/tests/`, 21 were
retired and 5 were kept** (a single directory cluster; criterion = all **three gates fail**
according to Part Seven of coding-rules: no clause gate, no history gate, and no assertions anywhere
in the body). The per-entry adjudications and "same-cell coverage" are provided by the 0.6-11a
adjudication table (21 deleted / 5 kept). This page does not repeat the reasons; it records only the
execution result.

- **21 retired** (`normalizer.rs`: 2; `reducer.rs`: 19): `test_normalizer_with_config`,
  `test_normalizer_evaluator`, `test_type_computer_register_alias`,
  `test_type_computer_compute_primitive`, `test_type_computer_set_context`,
  `test_type_computer_compute_type_var`, `test_type_computer_compute_list`,
  `test_type_computer_compute_tuple`, `test_type_computer_compute_fn`,
  `test_reducer_reduce_with_limit`, `test_unifier_different_types`,
  `test_unifier_substitution_after_unify`, `test_unifier_unify_option_same`,
  `test_unifier_unify_result_same`, `test_unifier_unify_dict_same`, `test_unifier_unify_set_same`,
  `test_unifier_unify_range_same`, `test_unifier_unify_arc_same`, `test_unifier_unify_weak_same`,
  `test_unifier_unify_enum_same`, `test_unifier_unify_struct_same`.
- **5 kept** (the adjudication table could not identify replacement coverage, so they were kept as
  required): `test_normalizer_context`, `test_type_computer_new_and_with_config`,
  `test_type_computer_compute_type_ref`, `test_reducer_reduce_type_ref_with_alias_chain`,
  `test_reducer_reduce_type_ref_self_referential`.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the entire batch.

## Second Retirement Batch (Executed on 2026-11-06)

**Second batch: of 18 "weak assertion" tests in `src/frontend/core/parser/pratt/tests/nud.rs`, 11
were retired and 7 were kept** (a single directory cluster = the Pratt prefix-parsing module;
criterion = all **three gates fail** according to Part Seven of coding-rules: no clause gate, no
history gate, and the body contains only weak assertions such as `is_some()` / `is_none()`—it does
not pin down binding power or the parse result).

The difference from the first batch must be stated clearly: these tests **do contain assertions**;
they are not empty tests. The reason for retirement is "all three gates fail + zero criterion
increment", not "no assertions". Every entry selected for deletion identifies a **replacement
unit-test name in the same module**.

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
  `test_prefix_info_eof`—see the commit message for the reason for each (task 0.6, second
  test-retirement batch). The reasons fall into two categories: **the proof gate applies** (the
  first three, together with `test_prefix_info_at` / `test_prefix_info_eof`, are the sole unit-level
  observation points for the corresponding facts, and the repository has no second unit test that
  observes them), or **the history gate applies** (`break_removed` / `continue_removed` name #314).
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the entire batch.

## Generator Correction (0.6-13)

**`body_span` truncates function bodies when it encounters braces inside literals — fixed.**

The old implementation obtained the function body by balancing braces and did not distinguish real
code from braces in string/character literals or comments. Forms such as `tokenize("{")` are common
in Rust source, so the balance was skewed and the function body was truncated: the body of
`test_prefix_info_lbrace` was once truncated at the end of the file, causing the "assertion summary"
to consume the remainder of the file (`matchesx11 assertx24 …`) and the entry to be classified as
"pending adjudication" rather than "retirement candidate"—a **misclassification**.

Fix: add `mask_noncode`, first replace the contents of string literals / character literals / line
comments / block comments (which may be nested) / raw strings (`r"…"` / `r#"…"#` / `br"…"` /
`cr#"…"#`) with spaces (newlines are preserved, so line numbers and line counts remain unchanged),
then balance the braces. **The mask is used only to locate function-body boundaries**; assertion
statistics and the history gate still read the original text, and the criteria themselves have not
been relaxed. `--selftest` adds 4 cases (paired/unpaired braces in an ordinary string, braces in raw
strings and block comments, and braces in a character literal), each followed by a sentinel test—the
old implementation obtained the wrong boundary in all 4 cases, while the new implementation obtains
all of them correctly.

### Tests Affected by the Same Bug: 11

Counting method: compare the **function-body boundaries before and after the fix** file by file and
test by test (the old implementation balanced braces literally, whereas the new implementation
balances them after masking); an entry is affected if the boundaries differ. Of 3413 tests in the
entire repository, the old implementation obtained the wrong boundary for **11**:

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

For the other 9 entries, **8 affected only the "assertion summary" column** (the classification did
not change), while **1 did not even change the summary**
(`test_fstring_close_brace_escape_preserved_raw`: the old implementation omitted the closing `}`
line, which contains no assertion keywords). The 8 summary changes are recorded individually in the
commit message.

| Test Name                   | File                                          | Old Classification                          | New Classification                              | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| --------------------------- | --------------------------------------------- | ------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_prefix_info_lbrace`   | `src/frontend/core/parser/pratt/tests/nud.rs` | Pending adjudication / Pending adjudication | Pending adjudication / **Retirement candidate** | The real function body contains only the weak assertion `assert!(state.prefix_info().is_some())`; neither the file header nor the function body has a § section number, issue/regression/defect/incident, or similar reference ⇒ both the clause gate and the history gate fail; the body contains only weak assertions such as `is_some()` ⇒ retirement candidate (a candidate is not a verdict; the proof gate still requires manual review)          |
| `test_fstring_escape_brace` | `src/frontend/core/lexer/tests/literals.rs`   | History / Keep                              | Clause / **Pending verification**               | The old implementation truncated the body at the end of the file and swallowed text containing "regression / defect / incident", falsely triggering the history gate; the real function body contains no such wording ⇒ the history gate no longer applies; the file header has verifiable entries (`§1.6.1` / `§1.6.2` / `§1.6.3` / `§1.6.4` / RFC-012) ⇒ the clause gate is hit but has not been confirmed by the sample audit ⇒ pending verification |

The distribution changes accordingly: clause 1713 → 1714 · history 314 → 313 · keep 776 → 775 ·
pending verification 2121 → 2122 · pending adjudication 412 → 411 · retirement candidate 104 → 105.
**The repository-wide `#[test]` count remains unchanged (3413)**—this round changes only the
implementation of the criteria, not any tests.

## Third Retirement Batch (Executed on 2026-11-07)

**Third batch: 5 retirement candidates under `src/middle/passes/codegen/tests/` were adjudicated
individually; 2 were retired and 3 were kept.** (a single directory cluster = the
`middle::passes::codegen` module under test; criterion = all **three gates fail** according to Part
Seven of coding-rules.)

This batch retains the **keep criterion** established by the previous batch: a unit-level
observation point can only be replaced by another **unit test**; corpus/integration tests operate at
the process-boundary layer and cannot replace a unit-level observation point.

- **2 retired**:
  - `test_execute_module_with_global_slot` (`global_slot.rs`)—the function body is
    **character-for-character identical** to `test_global_slot_store_then_load_roundtrip` in the
    same file (apart from the function name and comments): both are `run_fixture()` +
    `assert_slot_is_42(&interp)`. Its documentation comment claims to verify that "the
    `execute_module` entry path returns non-Void", but the body never observes the return value of
    `execute_module` (`run_fixture` discards it with `.expect(...)`)— the claimed intent is not
    implemented, and the function body is a pure duplicate ⇒ zero criterion increment. Replacement
    coverage = `test_global_slot_store_then_load_roundtrip`.
  - `test_scope_manager_basic` (`flow.rs`)—`test_scope_nesting` in the same file covers it strictly:
    positive lookup (`lookup("a").is_some()`), negative lookup (`lookup("b").is_none()`), the
    `scope_level` count, and the shape of the inserted `Symbol`
    (`symbol("a", Int(64), Local(0), false, 0)` is the same shape as the inline `Symbol` in this
    entry). "A name that was never inserted cannot be found" and "it cannot be found after being
    popped" follow the same path in `SymbolScopeManager::lookup` (walk each `scope.get(name)` until
    there is no match, then return `None`) ⇒ this entry has zero increment. Replacement coverage =
    `test_scope_nesting`.
- **3 kept**:
  - `test_global_slot_store_then_load_roundtrip`—the sole observation point for the path that "does
    not execute directly through serialization" (the serialization variant is a different path), and
    `assert_slot_is_42` asserts a concrete value.
  - `test_global_slot_survives_serialization_roundtrip`—the sole observation point for the
    serialization round trip (`write_to` → `read_from` → execution).
  - `test_register_overflow` (`operand.rs`)—the **sole** unit-level observation point for the
    rejection branch `id > 255` in `OperandResolver::to_reg` (the bytecode layer has independent
    overflow checks and does not cover this function; no second case in the repository pushes
    `to_reg` out of range). `is_err()` is a discriminating assertion (removing the guard would make
    the test fail), not merely proof that nothing panicked.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the entire batch.

### Census Blind Spot Found in This Batch (**Fixed in 0.6-15**; see "Generator Criteria Correction" below)

The census's "delegated assertion" recognition pattern `\b(assert|check)[A-Za-z_]*\s*\(` **could not
match helper names containing digits**: `[A-Za-z_]*` does not include digits, so calls such as
`assert_slot_is_42(&interp)` were **not recognized as assertions**— and the 2 affected entries among
this batch's 5 candidates were therefore misclassified as "no assertions anywhere in the body".

After all 15 repository-wide "fails all gates" entries were reviewed individually (broadening helper
names to allow digits and additionally recognizing the `match` + `panic!` form), **4 actually
contained assertions**—3 used the numeric helpers above, and 1 was `test_weak_from_arc` in
`src/backends/interpreter/tests/weak.rs` (the `match` + `panic!` form; test-specification rule 6.1
requires `assert!(matches!(...))`, but it does in fact assert). The other 11 genuinely contained no
assertions.

**Taking these 4 entries directly from the machine's "fails all gates" classification would wrongly
delete tests that contain criteria**—individual human adjudication in this batch prevented that; the
generator-side fix is described in "Generator Criteria Correction (0.6-15)" below.

## Generator Criteria Correction (0.6-15)

**Two criteria blind spots have been fixed: numeric characters in helper names, and the `match` +
`panic!` form.** This round **deletes no tests** (the repository-wide `#[test]` count remains 3411);
it changes only how the generator recognizes whether a body contains assertions.

### What Was Fixed

1. **Numeric characters are now allowed in delegated-assertion names**:
   `\b(assert|check)[A-Za-z_]*\s*\(` → `\b(assert|check)[A-Za-z0-9_]*\s*\(`. The old pattern could
   not match helper names such as `assert_slot_is_42(&interp)`, so tests containing them were
   treated as "no assertions anywhere in the body".
2. **Added the `match` + `panic!` form**: `match <expr> { … panic!(…) … }`—the assertion is written
   in one of the match arms, and no `assert!` macro is visible lexically (test-specification rule
   6.1 requires `assert!(matches!(...))`, but existing code contains this form). Recognition method:
   scan the expression after the `match` keyword up to the first `{`, determine the block boundary
   by **balancing braces**, and require a `panic!(` within that block. **The decision is made on
   masked lines**— braces in strings and comments do not participate in balancing, and `panic!` in
   comments is not treated as an assertion.

### Classification Differences: 3 (Gate and Disposition Changed)

| Test Name                                           | File                                           | Old Classification                     | New Classification                          | Basis                                                                                                                                                                                                                                                          |
| --------------------------------------------------- | ---------------------------------------------- | -------------------------------------- | ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_weak_from_arc`                                | src/backends/interpreter/tests/weak.rs         | Fails all gates / Retirement candidate | Pending adjudication / Pending adjudication | The body is `match weak { RuntimeValue::Weak(_) => {} _ => panic!("Expected Weak variant") }`—it asserts the variant, but the form was not previously recognized; it is now recognized as an assertion, while the machine does not judge its coverage strength |
| `test_global_slot_store_then_load_roundtrip`        | src/middle/passes/codegen/tests/global_slot.rs | Fails all gates / Retirement candidate | Pending adjudication / Pending adjudication | The assertion is delegated to `assert_slot_is_42(&interp)`—the helper name contains digits, so the old pattern could not match it; it is now recognized as an assertion                                                                                        |
| `test_global_slot_survives_serialization_roundtrip` | src/middle/passes/codegen/tests/global_slot.rs | Fails all gates / Retirement candidate | Pending adjudication / Pending adjudication | Same as above (the serialization round-trip entry)                                                                                                                                                                                                             |

Another **77 entries changed only the "assertion summary" column** (the gate and disposition
remained unchanged): most summaries gained `match_panicxN` (the body contains the `match` + `panic!`
form), and 4 gained `helperxN` (`src/driver/tests/manifest_error.rs`: 3;
`src/package/tests/cache.rs`: 1—helper names containing digits).

**The relaxation changes only the "fails all gates" tier**: this tier is the machine's
**high-confidence verdict** (there are no assertions anywhere in the body), and its criterion is a
hard requirement; "retirement candidate" is only a pool awaiting manual review, so the size of the
pool is not a verdict. All other classifications, including the weak-assertion pool, continue to use
the original criteria. Result: gate fails all gates 14 → 11 · pending adjudication 923 → 926;
disposition retirement candidate 103 → 100 · pending adjudication 411 → 414.

### Existing "Fails All Gates" Entries: 11 — Individual Review Conclusions

After all 11 repository-wide "fails all gates" entries were read and reviewed individually, **all 11
genuinely contain no assertions** (including no `assert*` / `check*` helpers, no `match` + `panic!`
form, and no `#[should_panic]`):

| Test Name                                       | File                                             | What the Body Actually Does                                                                   | Review Conclusion                                            |
| ----------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `test_run_bytecode_file_roundtrip`              | src/backends/interpreter/tests/bytecode_load.rs  | 5 `.expect(...)` calls, guaranteeing only that it does not crash                              | Genuinely no assertions (proves only that it does not crash) |
| `test_normalizer_context`                       | src/frontend/core/types/eval/tests/normalizer.rs | Obtains the `context()` handle and discards it                                                | Genuinely no assertions                                      |
| `test_type_computer_new_and_with_config`        | src/frontend/core/types/eval/tests/reducer.rs    | Constructs two `TypeComputer` instances, obtains their contexts, and discards them            | Genuinely no assertions                                      |
| `test_type_computer_compute_type_ref`           | src/frontend/core/types/eval/tests/reducer.rs    | Computes `compute(TypeRef)` and discards the result                                           | Genuinely no assertions                                      |
| `test_reducer_reduce_type_ref_with_alias_chain` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a chained-alias `reduce` and discards the result                                     | Genuinely no assertions                                      |
| `test_reducer_reduce_type_ref_self_referential` | src/frontend/core/types/eval/tests/reducer.rs    | Performs a self-referential-alias `reduce` and discards the result                            | Genuinely no assertions                                      |
| `test_document_symbol_items_invalid_code`       | src/lsp/handlers/tests/completion.rs             | Calls `document_symbol_items` with invalid content and discards the result                    | Genuinely no assertions (proves only that it does not panic) |
| `test_handle_formatting`                        | src/lsp/handlers/tests/formatting.rs             | Calls `handle_formatting` and discards the edits                                              | Genuinely no assertions (proves only that it does not panic) |
| `test_handle_range_formatting`                  | src/lsp/handlers/tests/formatting.rs             | Calls `handle_range_formatting` and discards the edits                                        | Genuinely no assertions (proves only that it does not panic) |
| `test_cli_modules_available_on_native`          | src/tests/feature_gate.rs                        | Constructs lsp / repl and discards them (inside the wasm32 cfg gate)                          | Genuinely no assertions                                      |
| `test_z3_modules_available_on_native`           | src/tests/feature_gate.rs                        | Constructs `Z3Backend`; `let _: Z3_context = std::ptr::null_mut();` is only a type annotation | Genuinely no assertions                                      |

Five of these (`test_normalizer_context` and the four entries in reducer.rs) were classified as
"keep" in the 0.6-11a adjudication table because **replacement coverage could not be identified**;
this review reaches the same conclusion.

### Reverse Spot Check: Did the Relaxation Go Too Far?

Five entries were selected with a fixed seed (20261107) from the 88 entries newly recognized as
assertions, and their source was read to confirm that they really assert:

| Spot-Check Entry                                            | File                                                 | Assertion in the Source                                                                                                                      |
| ----------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_substitute_through_assoc_type`                        | src/frontend/core/types/tests/substitute.rs          | Two `assert_eq!` calls in the matching `match` arm; `panic!("Expected AssocType")` if it does not match                                      |
| `test_unlisted_file_rejected`                               | src/package/tests/yxpkg.rs                           | `assert!(msg.contains("src/evil.yx"))` in the matching arm; `panic!("expected InvalidPackage, got …")` if it does not match                  |
| `test_assignment_constraint_dynamic`                        | src/frontend/core/sema/inference/tests/assignment.rs | `assert_eq!` compares the constrained types in the matching arm; `panic!("不是 Dynamic 变体…")` if it does not match                         |
| `test_delta_add_token`                                      | src/lsp/handlers/tests/semantic_tokens.rs            | `assert!(!delta.edits.is_empty())` in the matching arm; `panic!("Expected TokensDelta variant")` if it does not match                        |
| `test_e2e_phase4_into_diagnostic_predicate_violation_e4018` | src/frontend/core/sema/tests/rfc027_phase4_e2e.rs    | Three `assert_eq!` / `assert!` calls in the matching arm verify the error code and message; `panic!("不是 Disproved…")` if it does not match |

All 5 assert concrete values or concrete variants (replacing the implementation with another correct
path would make them fail)—**the relaxation did not go too far**. The reverse controls were also
added to `--selftest`: a `match` without `panic!`, and `panic!` appearing only in a comment; neither
may be recognized as an assertion.

### The Criteria Have Teeth (Developmental Proof)

After rolling the generator back to the old implementation (`HELPER_RE` without digits + disabling
`match` + `panic!` recognition), all newly added `--selftest` cases **failed**, with exit code 1;
the new implementation passes all of them.

### Rollback

The generator and census table are in the same commit; `git revert <sha>` restores the entire state.

## Fourth Retirement Batch (Executed on 2026-11-07)

**Fourth batch: of 10 "retirement candidates" in the directory cluster `src/backends/interpreter/`
(the interpreter backend's test directory), 1 was retired and 9 were kept** (a single directory
cluster; all three files were read before adjudication: `tests/bytecode_load.rs`: 4 ·
`tests/weak.rs`: 2 · `executor/tests/debug.rs`: 4). Criterion = the **three gates** in Part Seven of
coding-rules; keep criterion = "a unit-level observation point can only be replaced by another unit
test; corpus/integration tests operate at the process-boundary layer"; **deletion requires
identifying a concrete replacement-coverage test name; if none can be identified or the test cannot
be understood, it is kept**.

- **1 retired**: `test_capture_stack_includes_current_frame_during_step`
  (`src/backends/interpreter/executor/tests/debug.rs`)—`test_capture_stack_during_execution` in the
  same file follows the same path (`make_module` + `load_module_for_stepping`, one step, then obtain
  `capture_stack`), and its assertions are **strictly stronger**: it asserts that the stack is
  nonempty and that `stack[0].function_name == "main"` (pinning the top of the stack), whereas the
  retired entry asserts only `stack.iter().any(|f| f.function_name == "main")` (any position is
  sufficient) —the former being true necessarily implies the latter ⇒ zero criterion increment.
- **9 kept**: `test_run_bytecode_file_roundtrip` · `test_run_bytecode_file_invalid_magic` ·
  `test_run_bytecode_file_version_mismatch` ·
  `test_run_file_with_yxbc_magic_uses_bytecode_load_path` · `test_weak_upgrade_some` ·
  `test_weak_upgrade_none` · `test_step_on_empty_stack_returns_ok` ·
  `test_step_out_returns_to_caller` · `test_run_to_completion`—after reading each source, they are
  individually the **sole unit-level observation point** for a property (the two load branches for
  magic number/version are distinct, the YXBC routing layer, the two Weak Some/None sides, stepping
  with no frames, `step_out` popping the stack, and running to completion with an empty stack); no
  replacement unit test could be identified ⇒ they are kept as required. See the commit message for
  the basis of each entry (the adjudication table is not included on this page).
- **Numbers**: repository-wide `#[test]` 3409 → 3408; 1 deletion = `git diff` counts `-#[test]` 1
  line and `+#[test]` 0 lines; the dominating test `test_capture_stack_during_execution` passes on
  its own. Census: retirement candidate 99 → 98 · pending adjudication 925 → 924 · clause 1713 ·
  history 313 · proof 447 · fails all gates 11 · pending verification 2121 · keep 775.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the entire batch.

## Fifth Retirement Batch (Executed on 2026-11-07)

**Fifth batch: 1353 unit tests were retired in bulk under the mechanical rule "for the same symbol
and result category, keep only the strongest entry"** (only unit tests under `src/**/tests/**` were
changed; not a single entry was changed on the E2E surfaces `tests/yaoxiang/**`,
`tests/integration/**`, `tests/formatter_integration.rs`, `tests/yx_runner*.rs`, or
`tests/project-corpus/**`).

- **Rule (mechanically executed, without arguing each entry individually)**: for the same symbol
  under test + the same assertion-result category, keep only the entry with the strongest assertion
  and delete the rest.
  - Symbol under test = a repository symbol invoked in the test body (including one level of
    propagation through a helper function in the same file); when multiple symbols are reached,
    classify by the **primary symbol** (the first repository invocation). Symbols are qualified by
    repository path and are not merged across modules by bare name.
  - Result category = a concrete value / a specific error code (E code) / a specific variant match /
    only ok / only err / only should_panic.
  - Strength = `assert_eq!`/`assert_ne!` > `assert!`/`matches!` (or match + `panic!`) >
    `is_ok`/`is_some`/`is_err` > `should_panic` > none; when strengths are equal, keep the first
    entry in the file.
  - When a test asserts multiple categories (making the primary category indeterminate), **keep
    it**; do not guess.
- **Three exceptions (not one entry was removed)**: ① the name or comment names `issue #` /
  regression / defect / incident (history gate); ② a symbol has only one test (deleting it would
  leave the symbol without a unit-level observation point); ③ `#[should_panic]` is the symbol's only
  should_panic case. **Zero** entries from the history gate's 313 and the "fails all gates" tier's
  11 were retired.
- **Numbers**: repository-wide `#[test]` 3408 → 2055; 1353 deletions = `git diff` counts `-#[test]`
  1353 lines and `+#[test]` 0 lines. Census by gate (current values after the 0.6-19 repeal of the
  sample audit): clause 808 · history 313 · proof 424 · pending adjudication 499 · fails all
  gates 11. Census by disposition (current values): keep 737 · pending verification 1054 · pending
  adjudication 203 · retirement candidate 61. The retirement distribution for this batch by gate is:
  clause 902 · pending adjudication 428 · proof 23 (0.6-19 reclassified 9 entries previously
  classified as "audit confirmed genuine support" as pending verification, so the current values by
  gate and disposition differ from those at the fifth-batch cutoff).
- **Cleanup accompanying the deletions**: helper functions used only by deleted tests, imports that
  became unused, and fields that became unused were removed
  (`cargo clippy --all --all-features --all-targets -- -D warnings` produced 0 warnings).
- **Two kinds of out-of-bounds deletion that must be recorded**: 29 entries originally classified as
  "keep" were still deleted by this batch's rule—23 current-state anchors (names containing
  unchanged/anchor/guard/preserve/baseline, many of which were false positives caused by **domain
  terms** such as "guard" and "preserves", for example `test_while_guard_scope_balanced`) and 6
  entries whose clause support had been confirmed by the audit. This batch followed the closed list
  of three exceptions on the criteria card and did not additionally protect these two categories; to
  restore them, backfill according to the keep/delete comparison table on this page.
- **Failure and repeal of the sample audit**: the 40-entry sample was drawn with a fixed seed from
  the test pool; after the fifth batch's retirements, 38 entries in the original sample no longer
  existed, so the audit's `--check` always reported "missing 38, extra 38". This sample audit was
  repealed and deleted in 0.6-19 (the script, data file, and the generator's dependency on it were
  all removed); see "Sample Audit Repeal" below.
- **Rollback**: the entire batch is one commit (this page and `test-gate-census.tsv` are in the same
  commit); `git revert <sha>` rolls back the entire batch.

## Sample Audit Repeal (0.6-19)

**Criterion**: the sample audit (0.6-10) measured the "true support rate" of clause citations, but
after 0.6-18 the retention rules no longer examine citations (they changed to "for the same symbol
and result category, keep only the strongest entry", reducing the test pool from 3408 to 2055). Of
the original 40 sample entries, 38 no longer existed, so the audit's `--check` always reported
"missing 38, extra 38"—keeping it would only produce continuous alarms. Therefore, it was **repealed
and deleted**.

**What was deleted**: the sample-audit script under `scripts/dev/` (the one that reproduced the
40-entry sample with a fixed seed) and its JSON data file (the 40 per-entry decisions and bases).

**Generator dependency removed in sync**: the generator previously read that JSON and classified
"audit confirmed genuine support" as "clause gate + keep"; that branch has been deleted—**clause
citations no longer have an "audit confirmed" tier** and always fall under "clause / pending
verification".

**Distribution changes** (the repository-wide `#[test]` count of 2055 was **not changed at all**,
and the TSV was regenerated): keep 746 → 737 (−9: the 9 entries previously confirmed by the audit as
genuinely supported fell back to pending verification); pending verification 1045 → 1054; by gate:
clause 811 → 808, pending adjudication 496 → 499 (the others remain unchanged: history 313 · proof
424 · fails all gates 11).

**Rollback**: the entire batch is one commit (this page, `test-gate-census.tsv`, and the generator
are in the same commit); `git revert <sha>` rolls back the entire batch.

## Rerun and Self-Check

```bash
python scripts/dev/test-gate-census.py            # Recompute and overwrite the TSV (the two outputs are byte-for-byte identical)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
```

The generator performs lexical adjudication only, and states its boundaries honestly:

- **Delegated assertions are recognized only by name** (`assert*/check*` helper functions). If a
  test's assertion is hidden in a helper whose name does not begin with these two prefixes, it is
  misclassified as "no assertions"—the 26 entries in the first batch (21 retired / 5 kept) and the
  18 entries in the second batch (11 retired / 7 kept) were all manually reviewed function by
  function.
- **Function-body boundaries (fixed in 0.6-13)**: when the generator obtains a function body, it
  balances braces; braces in string/character literals or comments can skew that balance and
  truncate the function body—`test_prefix_info_lbrace` (whose body contains `tokenize("{")`) was
  once truncated at the end of the file, causing the "assertion summary" to consume the remainder of
  the file and the entry to be classified as "pending adjudication" rather than "retirement
  candidate". The generator now masks non-code regions (strings / character literals / line comments
  / block comments / raw strings) before balancing; see the "Generator Correction" section below.
- **Delegated-assertion recognition (fixed in 0.6-15)**: the old pattern
  `\b(assert|check)[A-Za-z_]*\s*\(` does not allow digits in `[A-Za-z_]*`, so calls such as
  `assert_slot_is_42(&interp)` were not recognized as assertions, and tests containing such calls
  were misclassified as "no assertions anywhere in the body". At the time, 4 of the repository's 15
  "fails all gates" entries actually contained assertions (3 numeric helpers + 1 `match` + `panic!`
  form). The name portion has now been broadened to `[A-Za-z0-9_]*`, and recognition of the
  `match` + `panic!` form has been added; see the "Generator Criteria Correction" section below.
- **The proof gate is not adjudicated**: every test that misses both gates is classified only as
  "pending adjudication" or "retirement candidate"; the generator never declares "delete this".
- **No cross-file deduplication**: completely duplicate tests are not identified in this table
  (0.6-2 measured only 1%, which does not constitute a retirement surface).
- **Clause citations always remain pending verification**: a verifiable section number in the file
  header only means that the test "maps to an identifiable clause"; it does not mean that the
  citation supports the assertion. The machine does not judge whether it does, so a clause hit still
  stops at "pending verification" and must be verified by a human before it can be kept.
