---
title: 'RFC-036: std.test Framework and yaoxiang test Command'
status: 'Accepted'
author: 'Chenxu'
created: '2026-07-26'
updated: '2026-09-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test Framework and yaoxiang test Command

## Summary

Introduce the standard test framework `std.test` module and the `yaoxiang test` CLI subcommand for
YaoXiang. Test files are ordinary `.yx` files, with overall pass/fail determined by the subprocess
exit code; a file may contain multiple test functions — assertion failures are expressed as `Err`
values (value semantics), and per-test verdicts are collected by the suite (§7). The `std.test`
module is implemented in pure YaoXiang and is the first dogfooding library. `yaoxiang test` is a CLI
tool, not a compiler feature — it involves no changes to the parser, IR, bytecode, or executor.

## Motivation

### Why a test framework?

Currently YaoXiang's test coverage relies on Rust-side `#[test]` and `tests/` integration tests.
This means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io)
   cannot be written in YaoXiang
2. `#117 Standard library per-module unit test coverage` is blocked, because no test infrastructure
   is available
3. Regression tests for language features (e.g., the RFC-032 spawn semantics change) lack automation

### Key Constraints

- **The 17-keyword iron rule**: introduce no new keywords or syntactic constructs
- **Zero compiler changes**: do not touch the parser, IR, bytecode, or executor
- **Self-hosting first**: the test library is written in YaoXiang; the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI layer:    yaoxiang test [--filter --fail-fast --json ...]│
│              │                                               │
│  Discovery:   Read yaoxiang.toml → [tool.test] patterns     │
│              Default: tests/**/*.yx                          │
│              │                                               │
│  Execution:   Per file: yaoxiang run <file>                   │
│              Check exit code → serial execution              │
│              │                                               │
│  Reporting:   PASS/FAIL → summary                            │
│              Supports --json / --verbose / --fail-fast       │
│                                                              │
│  Assertions:  std.test (pure YaoXiang, self-hosted)           │
│              Underlying: std.assert.assert                   │
│              Diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **The test framework is not a compiler feature; it is a CLI tool** — `yaoxiang run` can already
   "execute tests"; `yaoxiang test` simply runs all the files for you and shows a report
2. **Zero compiler changes** — no `@test` annotation scanning, no bytecode metadata sections, no
   special executor entry point
3. **Self-hosted** — the `std.test` module is implemented in pure YaoXiang, with underlying
   capabilities provided by `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files** — a file runs as a subprocess; the exit code determines
   overall pass/fail
5. **Assertion failures are values, not process events** — test functions return `Result`; assertion
   failures are expressed as `Err`, and the suite collects per-test verdicts (§7); process-level
   aborts are reserved for runtime guards and are not used for test assertions

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...       Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>      Only run tests whose file name contains <NAME>
  --fail-fast          Stop at the first failure
  --verbose, -v        Show detailed stdout/stderr for each test
  --list               Only list test files, do not run
  --no-progress        Suppress progress output (header and PASS lines); FAIL details and summary are preserved (CI scenario)
  --json               Output results in JSON format (for CI integration)
  --parallel           Run in parallel (one worker per core; OR'd with [tool.test].parallel)
```

#### Output Format

**Default output** (per-test verdicts are collected by the in-file suite, see §7):

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

- Failing files additionally carry `exit_code` and `stderr` (ANSI-stripped subprocess diagnostics,
  for CI forensics); when `--verbose` and `--json` are combined, every file carries `stdout` /
  `stderr`
- `--no-progress` only suppresses progress output (header and PASS lines) — FAIL details and the
  summary are always emitted; failures cannot be silenced; `--list` outputs one test file path per
  line, without execution
- Each file carries a `kind` (behavior / compile-error / runtime-error / invalid, §8.2); the summary
  carries a `by_kind` execution count (four fixed keys, no skipped); the human-readable summary
  appends a `Categories:` distribution line
- With `--parallel`, human-readable progress lines are streamed in **completion order** (without
  interleaving blocks); JSON `files` is sorted by `file` path to keep output stable (CI-diff
  friendly)
- The in-file per-test `tests` array comes from the §7 suite collection, taking effect when the
  value-based model lands (#319)
- The official CI (`.github/workflows/ci.yml` test job) consumes this as follows: on the cargo side,
  run `--test integration` (CLI integration) and `--test yx_runner` (dual-root corpus guard), then
  `yaoxiang test --json --parallel` runs tests in layers — the default mode (language corpus) and
  the explicit `src/std/tests` (library layer) each produce a report; the summary table (total /
  passed / failed / skipped / time_secs and by_kind) is written to the job summary; failing files
  print `kind` / `exit_code` / `stderr` for forensics, and any non-zero exit from a suite is treated
  as a failure

### 2. yaoxiang.toml Configuration

Placed under `[tool.test]`, in line with the `[tool.*]` third-party extension convention from
RFC-015:

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # Matches are removed from the discovery set (--list removes them as well)
parallel = true                   # Run in parallel (OR'd with the --parallel flag)
```

- Default `patterns = ["tests/**/*.yx"]` — zero-config out of the box
- `exclude` has the same shape as `patterns` (literal path or `root/**…`, matched by path prefix);
  excluded means not a test — fixtures that need runtime behavior validation go through
  `yaoxiang run` directly
- **Single-file mode (`yaoxiang test foo.yx`) runs directly without reading the config** — under
  explicit paths, the `exclude` / `parallel` config keys have no effect (except for flags)
- May be split into an independent repository in the future (the `[tool.test]` location stays the
  same)

### 3. The std.test Module (Pure YaoXiang)

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library (value-semantics standard form, landed 2026-09-03)
// The first dogfooding library: YaoXiang's test library is written in YaoXiang.

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

// assert_not and assert_false share the same body; assert_err / assert_err_code are covered in §8.1
```

- Assertion functions are **value-semantic**: they return `Result(Void, String)`; failure is
  expressed as `Err(diagnostic message)`, not a process abort — the §7 suite collects per-test
  verdicts accordingly. The process-abort semantics of `std.assert.assert` are reserved for runtime
  guards and do not enter the test-assertion path. The Ok payload is `Void` (the unit in
  type-system.md; `()` is the empty Tuple — the two are not mixed, as finalized on 2026-09-03)
- **The function family has 7 members (delivered 2026-09-03, abort-transition version removed)**:
  the value-based `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not` (same
  body as assert_false, reserved for the `!assert` form) + `assert_err` (§8.1)
  - `assert_err_code` (the error-code assertion in §8.1)
  - `assert_approx_eq(a: Float, b: Float, eps: Float)` (Phase 3, delivered 2026-09-07): the judgment
    is `|a - b| <= eps`, with `eps` **explicitly given by the caller** — tolerance is part of the
    test contract, with no hidden default value; negative `eps` is rejected at the declaration site
    as Err, and NaN always yields Err
- `assert_eq` / `assert_ne` use **Any-annotated parameters** — `==` / `!=` and f-string
  interpolation work fine on Any, without depending on the generics system. Note that **parameters
  must be explicitly annotated**: unannotated parameters cannot pass the call check of the native
  generic `&Result(T, E)` (verified by the R1 probe)
- `assert_false` / `assert_not` express negation as `cond == false` (the unary `not` syntax has not
  landed; once stable it can migrate; the unary `!assert` form has the same dependency, see §8.1)
- Block-body with explicit `return` form: the then-arm type of an if expression is discarded during
  checking, so the if expression whose both arms are Result is a checking blind spot — the
  implementation works around it
- `std.test` does not depend on any native code; it is implemented in pure YaoXiang

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedding into the Binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) is embedded into the
binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // Source text
    // More in the future
];
```

The module system (RFC-029, fully landed on 2026-08-02) provides the access point: the Registry
holds both native modules and source modules, and the orchestrator handles multi-file orchestration.
The resolution order of `use std.test` is:

1. First, look up the Rust native modules (the existing mechanism, such as `std.assert`)
2. If not found, look up the embedded `STD_YX_FILES` — if found, the source is injected into the
   orchestrator as a **seed module** with a virtual path (e.g. `<std>/test.yx`) and goes through the
   normal frontend pipeline (parse → typecheck → IR)
3. If still not found, fall back to filesystem discovery (user modules)

The `use std.assert` inside an embedded source module is resolved normally by the resolver to the
native registry — native and source modules coexist in the Registry, so cross-kind dependencies are
naturally satisfied. Embedded modules are **compiled on demand**: they enter the pipeline only when
imported.

Advantages:

- `use std.test` works in single-file mode as well
- The standard library version is strictly bound to the binary, with no version mismatch
- Users do not need to configure a standard library path

**Future: Filesystem Standard Library**

Once the YaoXiang project mode matures, the standard library will be moved to a filesystem form. See
the update of RFC-014 for details.

### 5. Discovery and Execution

**Prerequisite (decision from the 2026-08-02 review)**: the CLI `run` is wired to the orchestrator.
Currently the CLI `run` goes through the single-file pipeline (`run_file_with_diagnostics`) and
cannot resolve user-module imports; but the subprocess model of `yaoxiang test` inherits the CLI's
capabilities, and test files importing project modules is a core scenario. Therefore Phase 1 first
delegates the source branch of CLI `Run` to `run_project` (the orchestrator, with recursive
directory discovery); #247 (on-demand discovery along `use`) then stacks on top as a pure
performance optimization. A single file with no imports behaves identically through the
orchestrator, with the bytecode branch unchanged.

**Discovery Phase**:

1. If `[PATHS]` is specified, use those paths directly
2. Otherwise, read `[tool.test].patterns` from `yaoxiang.toml`
3. If unconfigured, default to `tests/**/*.yx`
4. Apply `--filter` (substring match against file names)
5. The discovery scope is the test layering (§9): the default patterns only cover the
   language-availability corpus; the library-test layer (e.g. `src/std/tests/`) is discovered via
   explicit paths or package configuration and is not mixed into the default scan

**Execution Phase**:

1. For each file, dispatch execution according to the header directives (the directive grammar is in
   §8.2, parsed by `src/util/test_markers.rs`, shared with yx_runner):
   - Behavior tests: `yaoxiang run <file>` subprocess (since #327, runtime errors by default carry
     source positions and stack frames — debug_map is generated by default, and stack trace outputs
     `file:line:col`); `// mode:` declares the subprocess `--runtime` mode
   - Compile-time rejection: single-step `yaoxiang check <file>`
   - Runtime-failure: a two-step `check` (must pass) + `run` (must fail)
2. Files with `// skip: <reason>` are skipped from execution and counted as skipped in the report
3. Verdicts and expected codes are compared according to the §8.2 judgment matrix; a failed
   directive parse does not execute and directly FAILs (construction-time rejection)
4. Capture stdout/stderr for the report
5. Serial by default; with `--parallel` (or `[tool.test].parallel`), spin up a worker pool with one
   worker per available core, where each file still runs as an independent subprocess — skip/invalid
   are processed first in discovery order, execution results stream in completion order, and JSON is
   sorted by path (Phase 3, delivered 2026-09-07)
6. With `--fail-fast`, stop dispatching new files at the first FAIL; in parallel mode, in-flight
   files are allowed to finish and are counted

### 6. Test Isolation

Test isolation is naturally provided by process-level boundaries:

- Each test file runs in an independent subprocess
- Each subprocess has an independent Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional isolated Heap context mechanism is needed
- **Parallel execution (Phase 3) does not extend the isolation boundary**: subprocesses share the
  working directory (CWD); parallel tests must not occupy files at the same path under CWD —
  file-IO-style tests use independent file names and clean up at the end

### 7. Suite and Multiple Tests (Value-Based Model)

A test file can contain multiple tests. In-file organization (landed 2026-09-03):

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

main = {
    test.suite([
        ("push_grows_len", () => push_grows_len()),
        ("pop_returns_last", () => pop_returns_last()),
    ])
}
```

- Each test is a zero-argument function returning `Result(Void, String)`; assertion failures are
  expressed as `Err` (§3 value-semantics assertion family) and do not interrupt the process —
  subsequent tests continue to run
- `test.suite` calls each in turn and collects: a non-Ok test records its name and diagnostic, an Ok
  one is silent; after all tests finish, if any Err exists, it aborts via `std.assert.assert` with
  the failure details appended (`N of M test(s) failed` + each `[FAIL] name: diagnostic`) — the
  file's exit code is non-zero (§5 verdict unchanged). The abort here is a runtime guard of the test
  binary, not an assertion path; when all are Ok, it exits silently with code 0
- Top-level test functions are enqueued as **closures** (`("name", () => test_fn())`): referring to
  a top-level function name as a value is not yet supported (an IR-layer limitation, `E3006`) —
  calling a global function inside a closure body is not affected
- The runner only sees the file, and does not perform function-level scanning: per-test verdicts
  come entirely from the in-file suite collection, and the file's internal structure is transparent
  to the runner — the zero-compiler-change principle is preserved
- Explicitly not adopted: in-process catch boundaries (the 17-keyword iron rule); the runner calling
  each function as a separate entry point (only used in internal scenarios such as the §8.2
  compile-failure case)
- The API shape has been finalized (2026-09-03, #319):
  `suite(tests: List((String, () -> Result(Void, String)))) -> Void`; duplicate names are not
  detected (the name is only used for report display); `--filter` filters by file name and is not
  aware of in-suite test names

### 8. Three-Layer Design for Negative Tests (Expected Failures)

Negative tests are split by the layer at which the failure occurs, with each layer placed in its
proper position:

#### 8.1 Value-Level Negation (General, User-Facing)

The operation under test returns a `Result`; the test expresses the expected failure with ordinary
assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// Or a one-liner wrapper (the code lives only on the std Error carrier; E is fixed as Error):
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` have been delivered along with the value-semantics
  family (2026-09-03, §3); the unary `!assert` form will be provided once the `not` syntax lands
  (with the same kind of constraint as `cond == false` in `assert_false`)
- Error-code assertions depend on the `Error` value carrying a machine-readable `code` field —
  landed in #323 M4: `Error = { code, message }` (native `error_new(code, message)`); reads go
  through `result.unwrap_err(r)` to obtain the carrier and `result.code(e)` / `result.message(e)`
  accessors (during design we had estimated the `error_new_with_code` naming, code-constant export,
  and `err.code` field access — none of these were adopted: the language has no Struct field access,
  and the code constants are not exported)
- As Result-ification advances (#301, #316), failing operations return `Result` one by one, and the
  file-level negative markers in the corpus migrate to in-file assertions

#### 8.2 File-Level Negative Directives (Internal Use Only for Language Designers)

Compilation is all-or-nothing for an entire file, so it is impossible to express "this line should
not compile" within a file; the same goes for runtime-failure (e.g. a suite that must contain a
failing test). The file header declares the expectation with a **structured directive**, and the
runner **dispatches and judges by expected category** (dispatching finalized on 2026-09-03; the
directive grammar finalized and landed on 2026-09-06).

Header directive grammar (`// key: value`, within the first 16 lines, strict token matching):

```text
// expect: compile-error E1002 [E1003 ...]   Compile-time rejection test
// expect: runtime-error E6003 [...]         Runtime-failure test
// skip: <reason>                            Skip execution, counted as skipped
// mode: embedded|standard|full              Subprocess --runtime mode (consumed only by the run step)
```

- No `expect:` directive = behavior test. `expect:` is the **sole declaration** of the expectation —
  on 2026-09-06 the design finalized: the deprecated `[test:error]` boolean flag and
  Chinese-language `预期:` prose code-extraction are abandoned. A boolean flag and an expectation
  line are two loosely-coupled facts, and trying to keep them consistent by discipline inevitably
  drifts; the strict English token grammar lets the runner parse mechanically (kind and codes are
  all fixed tokens, and any extra token after a code makes the parse fail), and parse failure = no
  execution, immediate FAIL — there is no silent-degradation channel for a misdeclared directive
  (construction-time rejection)
- **Compile-error category**: single-step `check` — must fail and the output must contain all
  `[EXXXX]`; compile passing = FAIL (something that should have been reported wasn't), rejection
  with mismatched codes = FAIL. Syntax errors (E1xxx parse phase) and semantic errors (E2xxx+) are
  not given separate categories — the expected code itself nails down the phase
- **Runtime-error category**: two-step judgment — `check` must **pass** (compile-time is innocent),
  and `run` must fail with output containing all `[EXXXX]`. Compile-time failure = FAIL (the reverse
  of the compile-error category — this is a key judgment that prevents leakage where "compile
  accidentally passes, runtime fails by chance")
- Conformance with industry conventions: Rust compiletest's `//~ ERROR`, Go's `// ERROR "regexp"`,
  GCC's `dg-error`, and Clang's `expected-error` all declare expectations inside fixture comments
  and have the harness compare both ways; they use line-level anchoring because multi-diagnostic
  compilers need to distinguish multiple expectations in the same file, whereas our compiler stops
  at the first error and produces one diagnostic per file — file-level matches the compiler's
  reality. Once error recovery lands, a line-anchored form can be appended to the grammar (Cranelift
  filetests' header directives plus per-function expectations are the same kind of mixed form)
- Known rendering debt: parse-phase diagnostics are currently output in Debug form (`code: "E0012"`
  rather than `[E0012]`); the code scanner accepts both forms; once diagnostic rendering is unified,
  the strict form will be reinstated
- **Serves only this repository's corpus, not part of the user test framework**; the dual-runner
  judgment convention has been finalized (2026-09-03, #319): yx_runner (`cargo test`) and
  `yaoxiang test` share `src/util/test_markers.rs` to parse header directives, and the
  `06-compile-errors` directory convention is deprecated. The reporting layer provides category
  counts: the human summary appends a `Categories:` line, and the JSON summary carries `by_kind`
  (behavior / compile-error / runtime-error / invalid, with skipped counted separately), and each
  file carries a `kind`

#### 8.3 Runtime Hard Failures (Folding into Result-ification)

No separate mechanism is established — operations that can fail return `Result` in line with the
language direction (#301, #316), and tests uniformly go through §8.1. Process-level aborts (such as
assertion violations, runtime argument mismatches) are gradually collapsed into values along with
Result-ification, and the test framework provides no dedicated semantics for them. (Note: the §8.2
"runtime-error" marker judgment is a file-level verification channel of the runner for **operations
that cannot yet be Result-ified**; it is not in conflict with the semantic direction of this section
— the latter is the destination, the former is the migration-period channel.)

### 9. Test System Layering: Language Corpus vs. Library Tests (Finalized 2026-09-03)

Tests are divided into two layers by **what is being tested**, each with its own ownership and
maintainer; the marker system (§8.2) and the assertion library (§3) are shared between the two
layers:

**Layer 1: Language-Availability Corpus (`tests/yaoxiang/`)**

- The thing under test is **the language itself** — parser, type system, modules, concurrency,
  ownership, compile-time rejection, runtime semantics; the directories are organized by
  language-spec chapters
- std is used in the corpus only as an **assertion tool** (`std.assert` / `std.test`) and is never
  tested itself — the API behavior of a library is not part of language availability
- Within the corpus, files are dispatched into three verdict categories per §8: behavior tests /
  compile-time rejection tests / runtime-failure tests

**Layer 2: Library Tests (Travel with the Library)**

- The thing under test is **the public API contract of the library** (e.g. the behavior of
  `list.push`, the semantics of `result.code`)
- Tests live in **the library's own package**: std's package is `src/std/`, and its yx-level tests
  are placed under `src/std/tests/` (co-located with the implementation; the directory coexists with
  the Rust unit tests, and the file types do not overlap); the tests for `std.test` itself are also
  placed here (using std.test to test std.test, closing the self-hosting loop)
- Future user packages follow the same convention: tests live in the package and are discovered via
  the package's `[tool.test]` (this previews the test layout from RFC-014's package management)
- Discovery is not part of the default patterns (the default `tests/**/*.yx` only covers the
  language layer): the library-test layer is discovered via explicit paths
  (`yaoxiang test src/std/tests`) or package configuration; CI runs them in layers

Migration note: **Already migrated (2026-09-06)** — the 19 files in the original
`tests/yaoxiang/07-std/` directory were screened one by one and all turned out to be library tests
(the things under test are all API contracts of std modules; the roles of language features such as
`?` propagation, automatic borrowing, and generic instantiation within them are carriers rather than
the things under test), and the entire directory was migrated into `src/std/tests/` and `07-std` was
removed; yx_runner switched to dual-root discovery (`tests/yaoxiang/` + `src/std/tests/`), and the
default patterns do not include the library layer (the integration test solidifies this contract).
The language corpus has zero std-API tests from this point on.

## Relationship to Existing Systems

| Item                                                 | Relationship                                                                                                                 |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched; compiler-internal tests continue to use Rust                                                                      |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                                   |
| `std.assert.assert(cond)`                            | Reserved for runtime guards; the value-semantics assertion family in `std.test` is built on `std.result` (§3, §7)            |
| Module system (RFC-029)                              | Embedded source modules are hooked in via Registry/orchestrator; CLI `run` being wired to the orchestrator is a prerequisite |
| `#200` refactor (`io.println` → `assert.assert`)     | Points in exactly the same direction as `yaoxiang test`                                                                      |
| `@` annotation                                       | Not used; no `@test` is introduced                                                                                           |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — delegate the source branch of CLI `Run` to
  `run_project` (prerequisite for multi-file execution)
- `src/main.rs` — add the `Test` subcommand
- `src/std/test.yx` — add the pure YaoXiang module
- `build.rs` — embed `std/*.yx` into the binary
- orchestrator / Registry — support loading `.yx` modules from embedded sources via virtual paths
- RFC-015 config parsing — the `[tool.test]` section
- Subprocess execution + reporting

Deliverables:

- `yaoxiang test` is basically usable
- `std.test` with 4 assertion functions
- Default `tests/**/*.yx` discovery
- Serial execution + default output format

### Phase 2: Polish

- `--filter` / `--fail-fast` / `--verbose` parameters
- `--json` output (for CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: Advanced (Delivered 2026-09-07)

- `--parallel` parallel execution (worker pool with each file in an independent subprocess; the
  `[tool.test].parallel` config key has the same effect)
- `[tool.test].exclude` config (prefix-match removal; `--list` removes them as well)
- `assert_approx_eq` (Float assertion with explicit eps, §3)

## Risks and Mitigations

| Risk                                                                     | Probability | Mitigation                                                                                                                                                                                                              |
| ------------------------------------------------------------------------ | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` interpolation failure on Any                                    | None        | Verified 2026-08-02 (Int/String both work)                                                                                                                                                                              |
| `yaoxiang.toml` config parsing not in current CLI                        | Low         | Simple extension; does not affect core functionality                                                                                                                                                                    |
| CLI `run` being wired to the orchestrator introduces behavior regression | Low         | The no-imports single-file path is equivalent; integration tests already cover the orchestrator                                                                                                                         |
| Embedding `.yx` source files into the binary increases size              | Low         | `.yx` source files are tiny; negligible                                                                                                                                                                                 |
| Test-loop time grows with the corpus                                     | High        | The main cost is per-file full compilation (measured 11.3s for 185 files), not subprocess startup; `--parallel` only relieves the process side, and the compilation cost requires test-loop caching (#251/#293 slicing) |

## Open Questions

- [x] Can the `use std.assert` reference inside `std/test.yx` be resolved correctly? — **Resolved
      (2026-08-02)**. After the module system (RFC-029) landed, native and source modules coexist in
      the Registry; the resolver handles them uniformly, and cross-kind dependencies are naturally
      satisfied
- [x] Does the generic `to_string` in `f"..."` introduce new type constraints in the test output? —
      **Resolved (2026-08-02)**. Verified that `==` / `!=` and f-string interpolation both work on
      unannotated parameters (Any) (Int/String verified), introducing no new constraints
- [x] Is the `?` generic parameter feasible? — **Resolved (2026-08-02)**: the `?` type syntax does
      not currently exist (and would be silently swallowed — a separate issue has been opened to
      track it); Phase 1 assertion functions use unannotated parameters and do not depend on the
      generics system

## Design Decision Log

| Decision                             | Decision                                                                                                                                                                                                                                                                                                                                                                                 | Date                           | Rationale                                                                                                                                                                                                                                                                                                                                                                                      |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Test marking approach                | No `@test` annotation; test files are ordinary `.yx`                                                                                                                                                                                                                                                                                                                                     | 2026-07-26                     | Zero compiler changes; subprocesses provide isolation                                                                                                                                                                                                                                                                                                                                          |
| Assertion approach                   | `std.test` module as pure YaoXiang functions                                                                                                                                                                                                                                                                                                                                             | 2026-07-26                     | Self-hosted; no native code                                                                                                                                                                                                                                                                                                                                                                    |
| Test execution model                 | Subprocess `yaoxiang run <file>` + exit code                                                                                                                                                                                                                                                                                                                                             | 2026-07-26                     | Process-level isolation; zero compiler changes                                                                                                                                                                                                                                                                                                                                                 |
| Standard library loading             | Currently embed in the binary; filesystem in the future                                                                                                                                                                                                                                                                                                                                  | 2026-07-26                     | Version binding; works in single-file mode                                                                                                                                                                                                                                                                                                                                                     |
| Assertion parameter type             | Unannotated parameters (Any); does not depend on the generics system                                                                                                                                                                                                                                                                                                                     | 2026-08-02                     | The `?` type syntax does not exist; Any verified to support comparison and interpolation                                                                                                                                                                                                                                                                                                       |
| Multi-file execution                 | CLI `run` delegates to `run_project` (orchestrator) as a prerequisite                                                                                                                                                                                                                                                                                                                    | 2026-08-02                     | The subprocess model inherits CLI capabilities; #247 degrades to a pure performance optimization                                                                                                                                                                                                                                                                                               |
| Report source positions              | Runtime errors carry them by default (#327)                                                                                                                                                                                                                                                                                                                                              | 2026-09-07                     | debug_map is generated by default; stack trace outputs `file:line:col`; the frame attribution for transit through embedded modules (std.test) is not guaranteed here; belongs to #289 + RFC-034                                                                                                                                                                                                |
| Negative-test layering               | Value-level negation (general) / structured compile-failure markers for the runner (internal only) / hard failures folded into Result-ification                                                                                                                                                                                                                                          | 2026-09-02                     | Finalized in #319; replaces the implicit `[test:error]` convention                                                                                                                                                                                                                                                                                                                             |
| In-file multiple tests               | Value-based standard model: test functions return Result, the suite collects per-test verdicts                                                                                                                                                                                                                                                                                           | 2026-09-02                     | No catch; no per-function entry invocation (entries are only used in internal scenarios)                                                                                                                                                                                                                                                                                                       |
| Error code                           | Error adds a machine-readable `code` field                                                                                                                                                                                                                                                                                                                                               | 2026-09-02                     | Supports error-code assertions; compile-time codes go through runner comparison                                                                                                                                                                                                                                                                                                                |
| Assertion library shape              | Value-semantics family of 7 functions landed; the `Result(Void, String)` contract; the abort-transition version is removed                                                                                                                                                                                                                                                               | 2026-09-03                     | Void is the canonical unit (`()` is the empty Tuple, not mixed); the nested bit is rigid Any, and unannotated parameters cannot pass the native generic check — parameters must be explicitly annotated (verified by the R1 probe)                                                                                                                                                             |
| Test system layering                 | Language corpus (`tests/yaoxiang/`) and library tests (travel with the library, std → `src/std/tests/`) split into two layers; std is only used as an assertion tool in the corpus                                                                                                                                                                                                       | 2026-09-03                     | What is being tested determines ownership and maintainer; library tests living with the package previews RFC-014                                                                                                                                                                                                                                                                               |
| Negative-marker dispatching judgment | Dispatch by expected category: compile-error `check` must fail, runtime-error `check` must pass + `run` must fail; report provides category counts                                                                                                                                                                                                                                       | 2026-09-03 (landed 2026-09-06) | Mixed-category judgment would let "compile accidentally passes, runtime fails by chance" leak through; expected codes nail down the phase; syntax errors are not given a separate category                                                                                                                                                                                                     |
| Header directive grammar             | Expectations declared via English structured directives (`// expect:` / `// skip:` / `// mode:`, strict token grammar, parse failure immediately FAILs); the `[test:error]` boolean flag and Chinese-language `预期:` prose code-extraction are abandoned                                                                                                                                | 2026-09-06                     | Expectations are a property of the fixture content; in-fixture declaration is in line with industry practice (compiletest / Go / GCC / Clang all do this), and a central list inevitably rots; a boolean flag plus an expectation line is two facts held together by discipline — that's a defect surface; the structured grammar lets the runner judge mechanically without human involvement |
| Parallel execution model             | `--parallel` spins up a worker pool with one worker per core (each file still in an independent subprocess); skip/invalid are processed first in discovery order, execution results stream in completion order, JSON is sorted by path; `--fail-fast` stops dispatching (in-flight files are allowed to finish and counted); CWD is still shared, the isolation boundary is not extended | 2026-09-07                     | Under the subprocess model, parallelism = OS-thread-scheduled spawn, no yx-level concurrency needed; the main time cost is per-file full compilation (risk table), so parallelism only relieves the process side — #293 caching slicing is the main mitigation                                                                                                                                 |

## References

- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — standard library
  directory structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — the `[tool.test]`
  config section
- [RFC-030: assert Mechanism](../review/030-assert-mechanism.md) — underlying dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — reference
  design
