---
title: 'RFC-036: std.test Testing Framework and yaoxiang test Command'
status: 'Accepted'
author: 'Chenxu'
created: '2026-07-26'
updated: '2026-09-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test Testing Framework and yaoxiang test Command

## Summary

Introduce the standard testing framework `std.test` module and the `yaoxiang test` CLI subcommand
for YaoXiang. Test files are ordinary `.yx` files, with the overall pass/fail determined by the
subprocess exit code; a file can contain multiple test functions internally—assertion failures are
expressed as `Err` values (value semantics), with the suite collecting per-test verdicts (§7). The
`std.test` module is implemented in pure YaoXiang and is the first dogfooding library.
`yaoxiang test` is a CLI tool, not a compiler feature—it does not involve any changes to the parser,
IR, bytecode, or executor.

## Motivation

### Why is a testing framework needed?

Current YaoXiang test coverage depends on Rust-side `#[test]` and `tests/` integration tests. This
means:

1. The standard library (std.math / std.list / std.dict / std.convert / std.io) unit tests cannot be
   written in YaoXiang
2. Unit test coverage of standard library modules is blocked because no test infrastructure is
   available
3. Regression tests for language features (such as the RFC-032 spawn semantics change) lack
   automated means

### Key Constraints

- **17-keyword iron rule**: do not introduce any new keywords or syntactic structures
- **Zero compiler changes**: do not touch the parser, IR, bytecode, or executor
- **Self-hosting priority**: the test library is written in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI layer:  yaoxiang test [--filter --fail-fast --json ...]  │
│              │                                               │
│  Discovery:   read yaoxiang.toml → [tool.test] patterns      │
│              default: tests/**/*.yx                           │
│              │                                               │
│  Execution:   for each file: yaoxiang run <file>              │
│              check exit code → serial execution              │
│              │                                               │
│  Reporting:   PASS/FAIL → summary                            │
│              supports --json / --verbose / --fail-fast       │
│                                                              │
│  Assertion:   std.test (pure YaoXiang, self-hosted)          │
│              low-level: std.assert.assert                    │
│              diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **The testing framework is not a compiler feature, but a CLI tool** — `yaoxiang run` can already
   "execute tests"; `yaoxiang test` merely helps you run all the files and shows you the report
2. **Zero compiler changes** — no `@test` annotation scanning, bytecode metadata sections, or
   executor special entry points
3. **Self-hosted** — the `std.test` module is implemented in pure YaoXiang, with underlying
   capabilities from `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files** — files run as subprocesses, with the exit code
   determining overall pass/fail
5. **Assertion failures are values, not process events** — test functions return `Result`, with
   assertion failures expressed as `Err`, and the suite collecting per-test verdicts one by one
   (§7); process-level aborts belong only to runtime guards and are not used for test assertions

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Only run tests whose filename contains <NAME>
  --fail-fast         Stop at the first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              Only list test files, do not run
  --no-progress       Suppress progress output (header and PASS lines); FAIL details and summary remain (CI scenario)
  --json              Output results in JSON format (for CI integration)
  --parallel          Execute in parallel (one worker per core; ORed with [tool.test].parallel)
```

#### Output Format

**Default output** (per-test verdicts come from the in-file suite collection, see §7):

```
Running 3 test files...

tests/math_test.yx ........................ PASS (0.002s)
tests/list_test.yx ........................ FAIL (0.003s)
  `-- [FAIL] push_grows_len: Expected 3, got 2
  `-- [ ok ] pop_returns_last
Results: 2 files passed, 1 file failed, 0 skipped (0.006s)
Categories: 2 behavior, 0 compile-error, 0 runtime-error
```

**JSON output** (`--json`):

```json
{
  "summary": {
    "total": 3,
    "passed": 2,
    "failed": 1,
    "skipped": 0,
    "by_kind": { "behavior": 3, "compile-error": 0, "runtime-error": 0, "invalid": 0 },
    "time_secs": 0.006
  },
  "files": [
    { "file": "tests/math_test.yx", "kind": "behavior", "passed": true, "time_secs": 0.002 },
    {
      "file": "tests/list_test.yx",
      "kind": "behavior",
      "passed": false,
      "time_secs": 0.003,
      "exit_code": 1,
      "stderr": "error [E1024]: one is not two",
      "tests": [
        { "name": "push_grows_len", "passed": false, "error": "Expected 3, got 2" },
        { "name": "pop_returns_last", "passed": true }
      ]
    }
  ]
}
```

- Failed files additionally carry `exit_code` and `stderr` (ANSI-stripped subprocess diagnostics,
  for CI forensics); when `--verbose` and `--json` are combined, all files carry `stdout` / `stderr`
- `--no-progress` only suppresses progress output (header and PASS lines)—FAIL details and summary
  are always output; failures cannot be silenced; `--list` outputs one test file path per line
  without execution
- Each file has a `kind` (behavior / compile-error / runtime-error / invalid, §8.2); the summary
  includes a `by_kind` execution count (fixed four keys, excluding skipped); the human summary
  includes a `Categories:` distribution line
- Under `--parallel`, human progress lines are streamed in **completion order** (whole blocks do not
  interleave), and JSON `files` is sorted by `file` path to ensure stable output (CI-diff friendly)
- The in-file per-test `tests` array comes from the §7 suite collection, taking effect with the
  value-typed model landing
- The official CI (`.github/workflows/ci.yml` test job) consumes this as follows: the cargo side
  runs `--test integration` (CLI integration) and `--test yx_runner` (dual-root corpus guard), then
  `yaoxiang test --json --parallel` runs hierarchical real tests—the default mode (language corpus)
  and explicit `src/std/tests` (library layer) each produce a report; the summary table (total /
  passed / failed / skipped / time_secs and by_kind) is written to the job summary, failed files
  print `kind` / `exit_code` / `stderr` for forensics, and any non-zero exit in either suite is
  judged as red

### 2. yaoxiang.toml Configuration

Placed under `[tool.test]`, conforming to RFC-015's `[tool.*]` third-party extension convention:

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # matched ones are removed from the discovery set (--list removes them too)
parallel = true                   # execute in parallel (ORed with the --parallel flag)
```

- Default `patterns = ["tests/**/*.yx"]` — zero-config, works out of the box
- `exclude` has the same form as `patterns` (literal path or `root/**…`, always matched by path
  prefix); excluded means not a test, and fixtures that need runtime behavior verification go
  through `yaoxiang run` directly
- **Single-file mode (`yaoxiang test foo.yx`) runs directly without reading config** — under
  explicit paths, neither `exclude` nor `parallel` config keys take effect (except for flags)
- It may be split into a separate repository in the future (the `[tool.test]` location remains
  unchanged)

### 3. std.test Module (Pure YaoXiang)

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library (value-semantics standard form, landed 2026-09-03)
// First dogfooding library: YaoXiang's test library is written in YaoXiang.

use std.result

assert_eq: (a: Any, b: Any) -> Result(Void, String) = (a, b) => {
    if a == b { return result.ok(void) }
    return result.err(f"Expected {b}, got {a}")
}

assert_ne: (a: Any, b: Any) -> Result(Void, String) = (a, b) => {
    if a != b { return result.ok(void) }
    return result.err(f"Expected not equal to {b}, got {a}")
}

assert_true: (cond: Bool) -> Result(Void, String) = (cond) => {
    if cond { return result.ok(void) }
    return result.err(f"Expected true, got {cond}")
}

// assert_not and assert_false are the same body; assert_err / assert_err_code see §8.1
```

- Assertion functions are **value-semantics**: they return `Result(Void, String)`, with failures
  expressed as `Err(diagnostic info)`, without aborting the process—the §7 suite uses this to
  collect per-test verdicts. The process-level abort semantics of `std.assert.assert` is reserved
  for runtime guards and does not enter the test assertion path. The Ok payload is `Void` (the
  spec's unit per type-system.md; `()` is an empty Tuple, the two are not mixed—finalized
  2026-09-03)
- **Function family of 7 (delivered 2026-09-03, abort transitional version removed)**: value-typed
  `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not` (same body as
  assert_false, reserved for the `!assert` form) + `assert_err` (§8.1)
  - `assert_err_code` (§8.1 error code assertion)
  - `assert_approx_eq(a: Float, b: Float, eps: Float)` (Phase 3, delivered 2026-09-07):
    `|a - b| <= eps` verdict, with `eps` **explicitly given** by the caller—tolerance is part of the
    test contract, no hidden default; negative eps yields Err at the declaration site, NaN always
    yields Err
- `assert_eq` / `assert_ne` use **Any-annotated parameters**—`==`/`!=` and f-string interpolation
  work normally on Any, not depending on the generics system. Note that parameters **must be
  explicitly annotated**: unannotated parameters fail the native generics `&Result(T, E)` call check
  (R1 probe empirical evidence)
- `assert_false` / `assert_not` use `cond == false` to express negation (the `not` unary syntax has
  not landed, migration is possible after stabilization; the `!assert` unary form has the same
  constraint, see §8.1)
- Block body + explicit `return` form: the type of the then arm of the if expression is discarded
  during checking; the if expression of both-arm Result is a checking blind spot, and the
  implementation works around it
- `std.test` does not depend on any native code, implemented in pure YaoXiang

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedding into the Binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) are embedded into the
binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source text
    // more in the future
];
```

The module system (RFC-029, fully landed 2026-08-02) provides the entry point: the Registry holds
both native modules and source modules, and the orchestrator is responsible for multi-file
orchestration. The resolution order for `use std.test`:

1. First check the Rust native modules (existing mechanism, e.g., `std.assert`)
2. If not found, check the embedded `STD_YX_FILES`—if matched, inject the **virtual path** (e.g.,
   `<std>/test.yx`) as a seed module into the orchestrator, going through the normal frontend
   pipeline (parse → typecheck → IR)
3. If not found, go to the file system for discovery (user modules)

The `use std.assert` inside an embedded source module is normally resolved by the resolver to the
native registry—native and source modules coexist in the Registry, and cross-kind dependencies
naturally hold. Embedded modules are **compiled on demand**: they enter the pipeline only when
imported.

Advantages:

- `use std.test` also works in single-file mode
- The standard library version is strictly bound to the binary, no version mismatch
- No need for users to configure the standard library path

**Future: File System Standard Library**

When the YaoXiang project mode matures, the standard library will be changed to file-system form.
See updates to RFC-014 for details.

### 5. Discovery and Execution

**Prerequisite (2026-08-02 review decision)**: CLI `run` is wired to the orchestrator. The current
CLI `run` goes through the single-file pipeline (`run_file_with_diagnostics`), which cannot resolve
user module imports; while the subprocess model of `yaoxiang test` inherits CLI capabilities, and
test files importing project modules is a core scenario. Therefore, Phase 1 first delegates the CLI
`Run` source branch to `run_project` (orchestrator, directory-recursive discovery); on-demand
discovery along use is then layered as a pure performance optimization. Single files without imports
behave equivalently through the orchestrator, and the bytecode branch remains unchanged.

**Discovery phase**:

1. If `[PATHS]` is specified, use the specified paths directly
2. Otherwise, read `[tool.test].patterns` from `yaoxiang.toml`
3. If not configured, default to `tests/**/*.yx`
4. Apply `--filter` filtering (filename contains)
5. The discovery scope is the test layering (§9): default patterns only cover language-availability
   corpus; the library test layer (e.g., `src/std/tests/`) is discovered via explicit paths or
   package configuration, and is not mixed into the default scan

**Execution phase**:

1. For each file, dispatch execution based on the header directive (directive grammar see §8.2,
   parsed via `src/util/test_markers.rs`, shared with yx_runner):
   - Behavior test: `yaoxiang run <file>` subprocess (runtime errors by default carry source
     location and stack frames—debug_map is generated by default, stack trace outputs
     `file:line:col`); `// mode:` declares the subprocess `--runtime` mode
   - Compile-time rejection: single-step `yaoxiang check <file>`
   - Runtime failure: two steps of `check` (must pass) + `run` (must fail)
2. Files with `// skip: <reason>` skip execution and are counted as skipped in the report
3. The verdict and expected code comparison follows the §8.2 verdict matrix; directive parse failure
   skips execution and FAILs directly (construction-time rejection)
4. Capture stdout/stderr for the report
5. Default serial; `--parallel` (or `[tool.test].parallel`) starts a worker pool based on available
   cores, with each file still being an independent subprocess—skip/invalid are handled first in
   discovery order, execution results are streamed in completion order, and JSON is sorted by path
   (Phase 3, delivered 2026-09-07)
6. If `--fail-fast`, stop scheduling new files immediately upon the first FAIL; in parallel mode,
   in-flight files finish running and are counted

### 6. Test Isolation

Test isolation is naturally achieved through process-level boundaries:

- Each test file runs in an independent subprocess
- Each subprocess has an independent Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional independent Heap context mechanism is required
- **Parallel execution (Phase 3) does not extend the isolation boundary**: subprocesses share the
  working directory (CWD), so parallel tests must not occupy files at the same path within CWD—file
  I/O tests should use independent file names and clean up at the end

### 7. Suites and Multiple Tests (Value-typed Model)

A test file can contain multiple tests. The in-file organization (landed 2026-09-03):

```yaoxiang
// tests/list_test.yx
use std.list
use std.result
use std.test

push_grows_len: () -> Result(Void, String) = () => {
    xs = [1]
    extended = list.push(xs, 2)
    test.assert_eq(list.len(extended), 2)
}

pop_returns_last: () -> Result(Void, String) = () => {
    mut xs = [1, 2]
    last = list.pop(xs)
    test.assert_eq(last, 2)
}

main: () -> Void = {
    test.suite([
        ("push_grows_len", () => push_grows_len()),
        ("pop_returns_last", () => pop_returns_last()),
    ])
}
```

- Each test is a zero-parameter function returning `Result(Void, String)`; assertion failures are
  expressed as `Err` (§3 value-semantics assertion family), without interrupting the
  process—subsequent tests run as usual
- `test.suite` calls and collects one by one: a non-Ok test records the name and diagnostic, Ok is
  silent; after all are run, if any Err exists, it aborts via `std.assert.assert` with failure
  details (`N of M test(s) failed` + each item `[FAIL] name: diagnostic`)—the file's exit code is
  non-zero (§5 verdict unchanged). The abort here is a runtime guard of the test binary, not the
  assertion path; all-Ok silently exits with 0
- Top-level test functions are enqueued **in closure form** (`("name", () => test_fn())`):
  referencing a top-level function name as a value is not yet supported (IR-level limitation,
  `E3006`)—the closure body calling a global function is unaffected
- The runner only sees files, does not do function-level scanning: per-test verdicts come entirely
  from the in-suite collection, and the file's internal structure is transparent to the runner—the
  zero-compiler-change principle is unaffected
- Explicitly not adopted: in-process catch boundaries (17-keyword iron rule); runner calling
  functions one by one (only for internal scenarios like §8.2 compile failures)
- The API form has been finalized (2026-09-03):
  `suite(tests: List((String, () -> Result(Void, String)))) -> Void`; duplicate names are not
  detected (name is only used for report display); `--filter` filters by filename, does not perceive
  test names within the suite

### 8. Three-Layer Design for Negative Tests (Expected Failures)

Negative tests are split by the layer at which failure occurs, with each layer in its proper place:

#### 8.1 Value-Level Reverse (Generic, User-Facing)

The operation under test returns `Result`, and the test expresses the expected failure with ordinary
assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// or a one-liner wrapper (the code only exists on the std Error carrier, E nailed as Error):
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` have been delivered with the value-semantics
  family (2026-09-03, §3); the `!assert` unary form will be provided after the `not` syntax lands
  (same constraint as `assert_false`'s `cond == false`)
- Error code assertion depends on the `Error` value carrying a machine-readable `code` field—landed:
  `Error = { code, message }` (native `error_new(code, message)`), read via `result.unwrap_err(r)`
  to get the carrier + `result.code(e)` / `result.message(e)` accessors (the design-phase estimated
  `error_new_with_code` naming, code constant exports, and `err.code` field access are all not
  adopted—the language has no Struct field access, code constants are not exported)
- As Result-ification progresses, fallible operations return `Result` one by one, and the file-level
  negative markers in the corpus migrate to in-file assertions accordingly

#### 8.2 File-Level Negative Directives (For Language Designers Only)

Compilation is all-or-nothing per file; you cannot express "this line should not compile" within a
file; runtime failures also need file-level expression (e.g., suites contain tests that must fail).
The file header declares expectations via **structured directives**, and the runner dispatches
verdicts by expected category (dispatch finalized 2026-09-03; directive grammar finalized and landed
2026-09-06).

Header directive grammar (`// key: value`, within the first 16 lines, strict token matching):

```text
// expect: compile-error E1002 [E1003 ...]   compile-time rejection test
// expect: runtime-error E6003 [...]         runtime failure test
// skip: <reason>                             skip execution, counted as skipped
// mode: embedded|standard|full               subprocess --runtime mode (consumed only by the run step)
```

- No `expect:` directive = behavior test. `expect:` is the **only declaration** of expectation—as of
  2026-09-06, the `[test:error]` boolean marker and the Chinese `预期:` prose code-scrubbing have
  been deprecated: a boolean marker and an expectation line are two loosely-coupled facts, and
  keeping them consistent by discipline is bound to drift; the strict English token grammar lets the
  runner parse mechanically (kind + codes are all fixed tokens, any extra tokens after the code
  means parse failure), and parse failure = skip execution and FAIL directly—there is no silent
  degradation channel for directive declaration errors (construction-time rejection)
- **Compile error category**: single-step `check`—must fail and the output must contain all
  `[EXXXX]`; if compilation passes = FAIL (expected error not reported), if rejected but codes don't
  match = FAIL. Syntax errors (E1xxx parse phase) and semantic errors (E2xxx+) do not have
  independent categories—the expected code itself nails down the phase
- **Runtime error category**: two-step verdict—`check` must **pass** (innocent at compile time),
  `run` must fail and the output must contain all `[EXXXX]`. Compile-time explosion = FAIL (the key
  verdict opposite to the compile error category, preventing the "compilation unexpectedly passes,
  runtime accidentally fails" miss)
- Isomorphic with the industry: Rust compiletest `//~ ERROR`, Go `// ERROR "regexp"`, GCC
  `dg-error`, Clang `expected-error` all declare expectations within fixture comments and let the
  harness compare bidirectionally; they use line-level anchoring because multi-diagnostic compilers
  need to distinguish multiple expectations in the same file. Our compiler stops at the first error
  with one diagnostic per file, so file-level is isomorphic with the compiler's reality—after error
  recovery lands, the line-anchored form can be added to the grammar (Cranelift filetests' file
  header directive + function-level expectation is a similar mixed form)
- Known rendering debt: parse-phase diagnostics are currently output in Debug form (`code: "E0012"`
  instead of `[E0012]`), and code scanning accepts both forms; the strict form will be reinstated
  after diagnostic rendering is unified
- **Only serves this repository's corpus, not part of the user-facing test framework**; the
  dual-runner verdict convention has been closed off (2026-09-03): yx_runner (cargo test) and
  `yaoxiang test` share `src/util/test_markers.rs` for parsing header directives, and the
  `06-compile-errors` directory convention is deprecated. The reporting layer provides category
  counts: the human summary has a `Categories:` line, the JSON summary has `by_kind` (behavior /
  compile-error / runtime-error / invalid, skipped counted separately), and each file has a `kind`

#### 8.3 Runtime Hard Failure (Folded into Result-ification)

No independent mechanism is set up—operations that can fail return `Result` in line with the
language direction, and tests uniformly go through §8.1. Process-level aborts (such as assertion
violations, runtime parameter mismatches) gradually converge to values as Result-ification
progresses, and the test framework does not provide dedicated semantics for them. (Note: the
"runtime error category" marker verdict in §8.2 is the runner's file-level verification channel for
**operations not yet Result-ifiable**, and does not contradict the semantic direction of this
section—the latter is the destination, the former is the migration-period channel)

### 9. Test System Layering: Language Corpus and Library Tests (Finalized 2026-09-03)

Tests are divided into two layers by **the object under test**, each with its own owner and
maintainer; the marker system (§8.2) and assertion library (§3) are shared at both levels:

**Layer 1: Language Availability Corpus (`tests/yaoxiang/`)**

- The object under test is **the language itself**—parser, type system, modules, concurrency,
  ownership, compile-time rejection, runtime semantics; directories are organized by language
  specification chapters
- std in the corpus is only used as an **assertion tool** (`std.assert` / `std.test`), never
  tested—the library's API behavior does not belong to language availability
- Within the corpus, three categories of verdicts are dispatched per §8: behavior tests /
  compile-time rejection tests / runtime failure tests

**Layer 2: Library Tests (Travel with the Library)**

- The object under test is **the public API contract of the library** (e.g., `list.push` behavior,
  `result.code` semantics)
- Tests are written **within the library's own package**: std's package is `src/std/`, and its
  yx-level tests go to `src/std/tests/` (co-located with the implementation body; the directory
  coexists with Rust unit tests, with non-overlapping file types); `std.test`'s own tests are also
  here (using std.test to test std.test, completing the self-hosting loop)
- Future user packages follow the same convention: tests are in-package, discovered via the
  package's `[tool.test]` (RFC-014's package management test layout is previewed by this)
- Discovery does not enter the default patterns (the default `tests/**/*.yx` only covers the
  language layer): the library test layer is discovered via explicit paths
  (`yaoxiang test src/std/tests`) or package configuration; CI runs in layers

Migration note: **Already migrated (2026-09-06)**—the 19 files in the original
`tests/yaoxiang/07-std/` were individually screened and all turned out to be library tests (the
objects under test are all std module API contracts; the roles of language features such as `?`
propagation, automatic borrowing, and generics instantiation within them are carriers rather than
the object under test), and the entire set was migrated into `src/std/tests/` with the `07-std`
directory removed; yx_runner changed to dual-root discovery (`tests/yaoxiang/` + `src/std/tests/`),
and the default patterns do not include the library layer (integration tests cement this contract).
The language corpus has zero std-API tests from then on.

## Relationship with Existing Systems

| Item                                                 | Relationship                                                                                                         |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Unchanged, compiler internal tests continue to use Rust                                                              |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                           |
| `std.assert.assert(cond)`                            | Reserved for runtime guards; the `std.test` value-semantics assertion family is based on `std.result` (§3, §7)       |
| Module system (RFC-029)                              | Embedded source modules are wired via Registry/orchestrator; CLI `run` connected to orchestrator is the prerequisite |
| Corpus refactoring (`io.println` → `assert.assert`)  | Fully aligned in direction with `yaoxiang test`                                                                      |
| `@` annotation                                       | Not used, no `@test` introduced                                                                                      |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project`
  (multi-file run prerequisite)
- `src/main.rs` — add `Test` subcommand
- `src/std/test.yx` — add pure YaoXiang module
- `build.rs` — embed `std/*.yx` into the binary
- orchestrator / Registry — support loading `.yx` modules from embedded sources via virtual paths
- RFC-015 config parsing — `[tool.test]` section
- Subprocess execution + reporting

Deliverables:

- `yaoxiang test` basically usable
- 4 assertion functions in `std.test`
- Default `tests/**/*.yx` discovery
- Serial execution + default output format

### Phase 2: Polish

- `--filter` / `--fail-fast` / `--verbose` parameters
- `--json` output (CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: Advanced (Delivered 2026-09-07)

- `--parallel` parallel execution (worker pool + each file as independent subprocess;
  `[tool.test].parallel` config key has the same effect)
- `[tool.test].exclude` config (prefix-match removal, `--list` removes too)
- `assert_approx_eq` (Float with explicit eps assertion, §3)

## Risks and Mitigation

| Risk                                                               | Probability | Mitigation                                                                                                                                                                                      |
| ------------------------------------------------------------------ | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` interpolation on Any fails                                | None        | Empirically verified 2026-08-02 (Int/String both work normally)                                                                                                                                 |
| `yaoxiang.toml` config parsing not in current CLI                  | Low         | Simple extension, does not affect core functionality                                                                                                                                            |
| CLI run connected to orchestrator introduces behavioral regression | Low         | Import-less single-file path is equivalent; integration tests already cover orchestrator                                                                                                        |
| Embedding `.yx` source files into binary increases size            | Low         | `.yx` source files are tiny, negligible                                                                                                                                                         |
| Test loop time grows with corpus                                   | High        | Main cost is full recompilation per file (11.3s measured at 185 files), not subprocess startup; `--parallel` only mitigates the process side, compilation cost requires test loop cache slicing |

## Open Questions

- [x] Can the `use std.assert` reference in `std/test.yx` be resolved correctly? —**Resolved
      (2026-08-02)**. After the module system (RFC-029) landed, native and source modules coexist in
      the Registry, the resolver unifies resolution, and cross-kind dependencies naturally hold
- [x] Does `f"..."` in test output introduce new type constraints via generic `to_string`?
      —**Resolved (2026-08-02)**. Empirically, `==`/`!=` and f-string interpolation both work on
      unannotated parameters (Any) (Int/String verified), introducing no new constraints
- [x] Feasibility of `?` generic parameters? —**Resolved (2026-08-02)**: the `?` type syntax does
      not currently exist (and is silently swallowed, tracked as a separate issue); Phase 1
      assertion functions use unannotated parameters, not depending on the generics system

## Design Decision Records

| Decision                         | Decision                                                                                                                                                                                                                                                                                                                                  | Date                      | Rationale                                                                                                                                                                                                                                                                                                                                                              |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Test marker method               | No `@test` annotation, test files are ordinary `.yx`                                                                                                                                                                                                                                                                                      | 2026-07-26                | Zero compiler changes, subprocess is isolation                                                                                                                                                                                                                                                                                                                         |
| Assertion method                 | `std.test` module as pure YaoXiang function                                                                                                                                                                                                                                                                                               | 2026-07-26                | Self-hosted, no native code                                                                                                                                                                                                                                                                                                                                            |
| Test execution model             | Subprocess `yaoxiang run <file>` + exit code                                                                                                                                                                                                                                                                                              | 2026-07-26                | Process-level isolation, zero compiler changes                                                                                                                                                                                                                                                                                                                         |
| Standard library loading         | Currently embed into binary, file system in the future                                                                                                                                                                                                                                                                                    | 2026-07-26                | Version binding, single-file usable                                                                                                                                                                                                                                                                                                                                    |
| Assertion parameter type         | Unannotated parameters (Any), not depending on generics system                                                                                                                                                                                                                                                                            | 2026-08-02                | `?` type syntax does not exist; Any empirically comparable and interpolatable                                                                                                                                                                                                                                                                                          |
| Multi-file run                   | CLI `run` delegates to `run_project` (orchestrator) as prerequisite                                                                                                                                                                                                                                                                       | 2026-08-02                | Subprocess model inherits CLI capabilities; on-demand discovery along use degenerates to pure performance optimization                                                                                                                                                                                                                                                 |
| Report source location           | Runtime errors carry by default                                                                                                                                                                                                                                                                                                           | 2026-09-07                | debug_map is generated by default, stack trace outputs `file:line:col`; frame attribution via embedded modules (std.test) is not guaranteed here, belongs to RFC-034                                                                                                                                                                                                   |
| Negative test layering           | Value-level reverse generic / compile-failure runner structured markers (internal only) / hard failure folded into Result-ification                                                                                                                                                                                                       | 2026-09-02                | Value-typed model finalized; replaces implicit [test:error] convention                                                                                                                                                                                                                                                                                                 |
| Multiple tests per file          | Value-typed standard model: test functions return Result, suite collects per-test verdicts                                                                                                                                                                                                                                                | 2026-09-02                | No catch, no entry calls (entries only for internal scenarios)                                                                                                                                                                                                                                                                                                         |
| Error code                       | Error adds machine-readable `code` field                                                                                                                                                                                                                                                                                                  | 2026-09-02                | Supports error code assertion; compile-time codes go through runner comparison                                                                                                                                                                                                                                                                                         |
| Assertion library form           | Value-semantics family of 7 functions landed, `Result(Void, String)` contract; abort transitional version removed                                                                                                                                                                                                                         | 2026-09-03                | Void is the spec's unit (`()` is an empty Tuple, not mixed); nested position Any is rigid, unannotated parameters fail the native generics check—parameters must be explicitly annotated (R1 probe empirical evidence)                                                                                                                                                 |
| Test system layering             | Language corpus (`tests/yaoxiang/`) and library tests (travel with library, std → `src/std/tests/`) split into two layers; std in corpus only as assertion tool                                                                                                                                                                           | 2026-09-03                | Object under test determines ownership and maintainer; library test layout within package previews RFC-014                                                                                                                                                                                                                                                             |
| Negative marker dispatch verdict | Dispatched by expected category: compile error category's `check` must fail, runtime error category's `check` must pass + `run` must fail; report provides category counts                                                                                                                                                                | 2026-09-03 (landed 09-06) | Category mixing causes "compilation unexpectedly passes, runtime accidentally fails" misses; expected code nails down phase, syntax errors do not have a separate category                                                                                                                                                                                             |
| Header directive grammar         | Expectations declared via English structured directives (`// expect:` / `// skip:` / `// mode:`, strict token grammar, parse failure FAILs directly); deprecate `[test:error]` boolean markers and Chinese `预期:` prose code-scrubbing                                                                                                   | 2026-09-06                | Expectations are properties of fixture content, in-fixture declaration is isomorphic with the industry (compiletest / Go / GCC / Clang all do this), the central manifest will rot; boolean marker + expectation line being two facts coupled by discipline is a defect surface; structured grammar lets the runner make mechanical verdicts without human involvement |
| Parallel execution model         | `--parallel` starts a per-core worker pool (each file is still an independent subprocess), skip/invalid handled first in discovery order, execution results streamed in completion order, JSON sorted by path; `--fail-fast` stops scheduling (in-flight files finish and are counted); CWD still shared, isolation boundary not extended | 2026-09-07                | Under the subprocess model, parallelism = OS thread scheduling spawn, no yx-layer concurrency needed; the main time cost is full recompilation per file (risk table), parallelism only mitigates the process side—test loop cache slicing is the main mitigation                                                                                                       |

## References

- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — Standard library
  directory structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]` config
  section
- [RFC-030: assert Assertion Mechanism](./030-assert-mechanism.md) — Underlying dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — Reference
  design
