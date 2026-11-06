---
title: 'Three-Gate Test Census (Prerequisite for the 0.6 Great Cleanup)'
description:
  'Mechanically classify every test in the repository according to the Three Gates in Part Seven of
  coding-rules, and provide an initial list of retirement candidates'
---

# Three-Gate Test Census (Prerequisite for the 0.6 Great Cleanup)

The first step of WBS 0.6 “Great Cleanup” is to mechanically classify every `#[test]` in the
repository according to the **Three Gates (Clause / Incident History / Proof)** in
[Part Seven of coding-rules](./coding-rules.md), and produce a rerunnable, per-test decision table
along with an initial list of retirement candidates. **This page and the generator are read-only
with respect to tests**—at this stage, they do not delete or modify any test.

Generated artifacts:

| Artifact                            | Purpose                                                                          |
| ----------------------------------- | -------------------------------------------------------------------------------- |
| `docs/src/dev/test-gate-census.tsv` | Per-test decision table (3445 rows + header), in the same directory as this page |
| `scripts/dev/test-gate-census.py`   | Generator (rerunnable; supports `--check` and `--selftest`)                      |

## Criteria

### Scan Scope

All `#[test]`s in `.rs` files under `src/`, `tests/`, `benches/`, and `tools/`:

| Area                                                                     | Count                |
| ------------------------------------------------------------------------ | -------------------- |
| Root-package lib (`src/`, including each module's `tests/` subdirectory) | 2971                 |
| `tests/integration`                                                      | 335                  |
| `tests/formatter_integration.rs`                                         | 52                   |
| `tests/yx_runner.rs` / `tests/yx_multifile_runner.rs`                    | 2 / 1                |
| `tools/`                                                                 | 75                   |
| `benches/`                                                               | 9                    |
| **Total**                                                                | **3445** (296 files) |

### Machine Criteria for the Three Gates

| Gate                  | Machine Criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Meaning of a Hit                                         |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| Clause Gate           | Within the first 25 lines of the file header, a match for `§\s*\d` / `RFC\s*\d{3}` / `SPEC\s*[A-Z0-9]` / `language-spec` / `语言规范` (“language specification”); for files under `src/**/tests/`, the parent `mod.rs` is searched as well                                                                                                                                                                                                                                                          | The test maps to a clause                                |
| Incident-History Gate | The test function body, function doc comment, or **file header** (such as `//! 回归背景：issue #226`) names a real failure: `issue #N` / 回归 (regression) / 缺陷 (defect) / 事故 (incident)                                                                                                                                                                                                                                                                                                        | The test pins down an incident that has already occurred |
| Proof Gate            | **Cannot be determined mechanically** (“If it were replaced with a correct alternative implementation, would it fail?” requires human judgment). Therefore, any test that passes neither of the first two gates is classified as “Pending,” rather than automatically as “Neither.” The sole exception is a test with **not a single assertion in its body** (including no delegated `assert*`/`check*` helper and no `#[should_panic]`)—this is not a weak criterion; there is no assertion at all | Only “Pending” or “No Assertions” can be assigned        |

The rationale for the file-header radius: this repository conventionally puts “regression
background: issue #N” in the file header and does not repeat it in comments for each test; looking
only at each test’s own comments misses the incident history of the entire file (measured results: 7
hits at the test’s own radius, 291 at the file-header radius).

### Categories Never Marked for Retirement

The following tests must always be **“Kept”** and must not enter the retirement candidates:

- Corpus `tests/yaoxiang/**` and project-shape `tests/project-corpus/**` (behavior-criterion
  surfaces);
- `tests/integration/**`, `tests/formatter_integration.rs`, and `tests/yx_runner*.rs` (spec-shaped
  and traversing the process boundary);
- Current-state anchors whose test names contain `unchanged` / `anchor` / `guard` / `preserv` /
  `baseline`.

### Disposition Semantics

A `Retirement Candidate` is **a candidate, not a verdict**: it means “neither the Clause Gate nor
the Incident-History Gate passes, and the assertion surface contains only weak assertions (or
none),” while the Proof Gate still requires manual review. This card only requires a list; it does
not perform deletions.

## Distribution

| Gate                                                                        | Count    |
| --------------------------------------------------------------------------- | -------- |
| Clause                                                                      | 2403     |
| Incident History                                                            | 47       |
| Proof (spec-shaped criterion surface / current-state anchors)               | 447      |
| Pending (neither of the first two gates passes; Proof awaits manual review) | 512      |
| Neither (no assertions anywhere in the body)                                | 36       |
| **Total**                                                                   | **3445** |

| Disposition          | Count    |
| -------------------- | -------- |
| Keep                 | 2897     |
| Pending              | 412      |
| Retirement Candidate | 136      |
| **Total**            | **3445** |

File-level criterion: of the 296 test files, **221 (74.7%)** have a SPEC/RFC/§ reference in their
file header.

Retirement candidates clustered by directory (top six):

| Directory                              | Candidate Count |
| -------------------------------------- | --------------- |
| `src/frontend/core/types/eval/tests`   | 26              |
| `src/lsp/handlers/tests`               | 19              |
| `src/frontend/core/parser/pratt/tests` | 18              |
| `src/lsp/tests`                        | 14              |
| `src/package/vendor/tests`             | 8               |
| `src/backends/interpreter/tests`       | 7               |

## Reconciliation with the 0.6-2 Survey

The 0.6-2 survey figures are the comparison baseline for this page. Each difference is explained
below rather than dismissed with a single “different criteria” statement:

| Item                            | 0.6-2 Survey                  | This Census                                                                                               | Reason for the Difference                                                                                                                                                                                                                                                                                                                                              |
| ------------------------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository-wide `#[test]`s      | 3443                          | 3445                                                                                                      | 2 tests were added after 0.6-2 (check-criterion regression tests for E3005 and E3022); root-package lib 2969 → 2971                                                                                                                                                                                                                                                    |
| Number of test files            | 296                           | 296                                                                                                       | Unchanged                                                                                                                                                                                                                                                                                                                                                              |
| Clause Gate (file-based)        | 227 (77%)                     | 221 (74.7%)                                                                                               | The criterion windows differ: this census uses “the first 25 lines + the parent `mod.rs` for `src/**/tests/`”; 0.6-2 used only the file header. The difference is 6 files                                                                                                                                                                                              |
| Incident-history signals        | 442 tests / 148 files         | Three radii: test itself, 7; file header, 291 / 38 files; whole file, 1218 / 79 files                     | The “148 files” from 0.6-2 is broader than this census’s 79 files even at the whole-file radius, indicating a wider term list or scope (this census strictly uses the four signals defined on this card: `issue #N`, 回归 (regression), 缺陷 (defect), and 事故 (incident)). This census reports the three radii accurately and does not collapse them into one number |
| Strong characterization signals | 1176 (34%) / strong 631 (18%) | Not measured                                                                                              | Characterization is **not** one of the Three Gates: it measures “how snapshot-like a test is.” This page only determines “whether it can be retired.” Using characterization would require a separate criterion, which must not be mixed into the retirement conclusion                                                                                                |
| Weak assertions                 | 473 (14%)                     | See the TSV `断言摘要` column (`assert_eq×N / is_ok×N / 无断言` per test; `无断言` means “no assertions”) | The counting units differ: 0.6-2 counted the pattern that asserts only `is_ok`/`is_some`; this census uses lexical counts and distinguishes strong assertions / weak assertions / delegated assertions. It is also **part of the disposition criterion**, rather than a table of its own                                                                               |

## Conclusion Summary

1. **The Clause Gate is the primary gate**: 2403 tests (69.8%) have a SPEC/RFC/§ reference in their
   file header, showing that “a test maps to a clause” is the norm in this codebase, not an
   exception.
2. **The retirement surface is small**: 136 candidates (3.9%). 0.6-2 showed that “exactly duplicated
   tests” account for only 1%, so the retirement benefit lies not in “deleting duplicates,” but in
   “deleting smoke tests that assert nothing + tests with weak assertions.”
3. **Tests that truly contain no assertions are few in number but conspicuous**: 36 tests have not a
   single assertion in their bodies, including delegated assertion helpers. They are smoke tests
   that merely “run once without panicking”—the initial list starts here.
4. **Pending is the largest unresolved area**: 512 tests pass neither of the first two gates but
   contain strong assertions, so the Proof Gate requires human judgment. This page draws no
   conclusion about them.

## Initial Retirement List (List Only; Do Not Delete)

**Batch: the 26 “no assertion” tests in `src/frontend/core/types/eval/tests/`** (a single directory
cluster, with the same rationale across the family: no assertions anywhere in the body). Selection
rationale: this is the repository’s only directory-wide cluster of assertion-free tests, so the
deletion case is strongest—when a test asserts nothing, deleting it loses no criterion. Another 327
tests in the same directory (source lines containing assertions: `conditional.rs` 127,
`const_eval.rs` 158, `operations.rs` 72, `normalizer.rs` 60, `reducer.rs` 60) cover the real
behavior of the same APIs.

| #   | Test Name                                       | File          | What It Asserts                                                                                    | Why Deleting It Loses No Criterion                                                                                                             |
| --- | ----------------------------------------------- | ------------- | -------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `test_normalizer_with_config`                   | normalizer.rs | No assertions: constructs `TypeNormalizer`, then discards the result of `normalize(Int(32))`       | There are no assertions to lose; `test_normalizer_normalize_primitives` in the same file pins down the normalization result with `assert_eq!`  |
| 2   | `test_normalizer_evaluator`                     | normalizer.rs | No assertions: only obtains the `evaluator()` handle and does not call it                          | There are no assertions to lose; handle access is incidentally covered by the reduction case with assertions in the same file                  |
| 3   | `test_normalizer_context`                       | normalizer.rs | No assertions: only obtains the `context()` handle                                                 | Same as above                                                                                                                                  |
| 4   | `test_type_computer_new_and_with_config`        | reducer.rs    | No assertions: constructs with `new()` / `with_config()` and discards the result                   | There are no assertions to lose; construction is a required step in the remaining computation cases, which contain assertions                  |
| 5   | `test_type_computer_register_alias`             | reducer.rs    | No assertions: registers an alias, then discards the result of `compute()`                         | There are no assertions to lose; `test_reducer_delta_reduces_alias` pins down the alias-reduction result with `assert_eq!`                     |
| 6   | `test_type_computer_compute_primitive`          | reducer.rs    | No assertions: discards `compute(Int(32))`                                                         | There are no assertions to lose; primitive-type computation is covered by `test_reducer_stuck_on_primitives` and other assertion-bearing cases |
| 7   | `test_type_computer_compute_type_ref`           | reducer.rs    | No assertions: registers an alias, then discards `compute(TypeRef)`                                | Same as #5                                                                                                                                     |
| 8   | `test_type_computer_set_context`                | reducer.rs    | No assertions: only obtains `context()` (the comment says “just verify can be created”)            | There are no assertions to lose                                                                                                                |
| 9   | `test_type_computer_compute_type_var`           | reducer.rs    | No assertions: discards `compute(TypeVar)`                                                         | There are no assertions to lose; type-variable reduction is covered by `test_reducer_reduce_type_var`                                          |
| 10  | `test_type_computer_compute_list`               | reducer.rs    | No assertions: discards `compute(List<Int>)`                                                       | There are no assertions to lose; list-type reduction is covered by `test_reducer_reduce_list_type`                                             |
| 11  | `test_type_computer_compute_tuple`              | reducer.rs    | No assertions: discards `compute((Int, String))`                                                   | There are no assertions to lose; tuple-type reduction is covered by `test_reducer_reduce_tuple_type`                                           |
| 12  | `test_type_computer_compute_fn`                 | reducer.rs    | No assertions: discards `compute(Fn)`                                                              | There are no assertions to lose; function-type reduction is covered by `test_reducer_reduce_function_type`                                     |
| 13  | `test_reducer_reduce_with_limit`                | reducer.rs    | No assertions: discards the result of `reduce_with_limit()`                                        | There are no assertions to lose; step-limit semantics are pinned down by `test_reducer_step_count` (`assert_eq!`)                              |
| 14  | `test_reducer_reduce_type_ref_with_alias_chain` | reducer.rs    | No assertions: discards the result of the chained alias reduction A→B→Int                          | There are no assertions to lose; single-hop alias reduction is pinned down by `test_reducer_delta_reduces_alias`                               |
| 15  | `test_reducer_reduce_type_ref_self_referential` | reducer.rs    | No assertions: discards the result of the self-referential alias reduction                         | There are no assertions to lose; the `assert_eq!` in `test_reducer_unknown_alias_stays_stuck` covers a self-reference remaining stuck          |
| 16  | `test_unifier_different_types`                  | reducer.rs    | No assertions: after `unify(Int, String)`, all three branches of the `match` are empty             | There are no assertions to lose; differing types not unifying is covered by `test_unifier_unify_different_concrete`                            |
| 17  | `test_unifier_substitution_after_unify`         | reducer.rs    | No assertions: after unification, retrieves and discards the substitution table                    | There are no assertions to lose; the substitution table is covered by `test_unifier_same_types` and other assertion-bearing cases              |
| 18  | `test_unifier_unify_option_same`                | reducer.rs    | No assertions: discards `unify(Option<Int>, Option<Int>)` (the comment says “may fail or succeed”) | There are no assertions to lose; this shape has no defined outcome in the first place, so deleting it loses no criterion                       |
| 19  | `test_unifier_unify_result_same`                | reducer.rs    | Same as above (Result)                                                                             | Same as above                                                                                                                                  |
| 20  | `test_unifier_unify_dict_same`                  | reducer.rs    | Same as above (Dict)                                                                               | Same as above                                                                                                                                  |
| 21  | `test_unifier_unify_set_same`                   | reducer.rs    | No assertions: discards `unify(Set<Bool>, Set<Bool>)`                                              | There are no assertions to lose; container-type unification is covered by `test_unifier_unify_composite_types`                                 |
| 22  | `test_unifier_unify_range_same`                 | reducer.rs    | No assertions (Range)                                                                              | Same as above                                                                                                                                  |
| 23  | `test_unifier_unify_arc_same`                   | reducer.rs    | No assertions (Arc)                                                                                | Same as above                                                                                                                                  |
| 24  | `test_unifier_unify_weak_same`                  | reducer.rs    | No assertions (Weak)                                                                               | Same as above                                                                                                                                  |
| 25  | `test_unifier_unify_enum_same`                  | reducer.rs    | No assertions (Enum)                                                                               | Same as above                                                                                                                                  |
| 26  | `test_unifier_unify_struct_same`                | reducer.rs    | No assertions (Struct)                                                                             | Same as above                                                                                                                                  |

**Preconditions for deletion (for the execution batch’s card; not handled here)**: ①
Assertion-bearing cases in the same directory still cover the same API (otherwise, first add
assertions to those cases); ② Verify the deletion in the same batch as `cargo test --lib` /
`run fast`; ③ Update this page and `test-gate-census.tsv` in the same commit (`--check` will fail);
④ Register it according to [Part Seven of coding-rules](./coding-rules.md).

## Rerun and Self-Test

```bash
python scripts/dev/test-gate-census.py            # Recalculate and overwrite the TSV (two runs produce byte-for-byte identical output)
python scripts/dev/test-gate-census.py --check    # CI/pre-commit: check whether the TSV matches the current tree
python scripts/dev/test-gate-census.py --selftest # Minimal positive/negative self-test (does not scan the repository)
python scripts/dev/test-gate-census.py --stdout   # Output only the TSV
```

The generator performs only lexical determination; its boundaries are stated as follows:

- **Delegated assertions are recognized by name only** (`assert*`/`check*` helper functions). If a
  test’s assertions are hidden in a helper whose name does not begin with either prefix, it will be
  misclassified as “No Assertions”—the function bodies of the first 26 tests were manually reviewed
  one by one, and the reasons on this page are based on that manual review.
- **The Proof Gate is not decided**: any test that passes neither of the first two gates is
  classified only as “Pending” or “Retirement Candidate”; the generator does not declare “delete
  this.”
- **No cross-file deduplication**: exactly duplicated tests are not identified in this table (0.6-2
  measured them at only 1%, so they do not constitute the retirement surface).
