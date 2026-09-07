---
title: 'RFC-036: std.test Testing Framework and yaoxiang test Command'
status: 'Accepted'
author: '晨煦 (Chenxu)'
created: '2026-07-26'
updated: '2026-09-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test Testing Framework and yaoxiang test Command

## Summary

Introduce the standard testing framework `std.test` module and the `yaoxiang test` CLI subcommand
for YaoXiang. Test files are ordinary `.yx` files, and the overall pass/fail is determined by the
child process exit code; the file internally supports multiple test functions—assertion failures are
expressed as `Err` values (value semantics), and the suite collects per-test judgments (§7). The
`std.test` module is implemented in pure YaoXiang and is the first dogfooding library.
`yaoxiang test` is a CLI tool, not a compiler feature—it involves no changes to the parser, IR,
bytecode, or executor.

## Motivation

### Why Do We Need a Testing Framework?

Currently, YaoXiang's test coverage relies on Rust-side `#[test]` and `tests/` integration tests.
This means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io)
   cannot be written in YaoXiang
2. `#117 Unit test coverage for each std module` is blocked because no testing infrastructure is
   available
3. Regression tests for language features (such as the RFC-032 spawn semantics change) lack
   automated means

### Key Constraints

- **The 17-Keyword Iron Rule**: do not introduce any new keywords or syntactic structures
- **Zero Compiler Changes**: do not touch the parser, IR, bytecode, or executor
- **Bootstrap First**: the test library is written in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI layer:  yaoxiang test [--filter --fail-fast --json ...] │
│              │                                               │
│  Discovery:  read yaoxiang.toml → [tool.test] patterns       │
│              default: tests/**/*.yx                          │
│              │                                               │
│  Execution:  per file: yaoxiang run <file>                   │
│              check exit code → serial execution              │
│              │                                               │
│  Reporting:  PASS/FAIL → summary                             │
│              supports --json / --verbose / --fail-fast       │
│                                                              │
│  Assertions: std.test (pure YaoXiang, self-bootstrapped)      │
│              low-level: std.assert.assert                     │
│              diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **The testing framework is not a compiler feature; it is a CLI tool** — `yaoxiang run` can
   already "execute tests"; `yaoxiang test` simply helps you run all the files and show you a report
2. **Zero compiler changes** — no `@test` annotation scanning, no bytecode metadata segments, no
   special executor entry points
3. **Self-bootstrapping** — the `std.test` module is implemented in pure YaoXiang, with low-level
   capabilities from `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files** — the file is run as a child process, and the overall
   pass/fail is determined by exit code
5. **Assertion failure is a value, not a process event** — test functions return `Result`, assertion
   failures are expressed as `Err`, and the suite collects per-test judgments one by one (§7);
   process-level abort belongs only to runtime guards, not to test assertions

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Only run tests whose filename contains <NAME>
  --fail-fast         Stop on the first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              Only list test files, do not run
  --no-progress       Do not show progress output (header and PASS lines); FAIL details and summary are retained (CI scenario)
  --json              Output results in JSON format (for CI integration)
  --parallel          Run in parallel (one worker per core; ORed with [tool.test].parallel)
```

#### Output Format

**Default output** (per-test judgments are collected from the in-file suite, see §7):

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

- Failed files additionally carry `exit_code` and `stderr` (child process diagnostics with ANSI
  stripped, for CI forensics); when `--verbose` is combined with `--json`, all files carry `stdout`
  / `stderr`
- `--no-progress` only suppresses progress output (header and PASS lines) — FAIL details and summary
  are always output; failures cannot be silenced; `--list` outputs one test file path per line and
  does not execute
- Each file is tagged with `kind` (behavior / compile-error / runtime-error / invalid, see §8.2);
  the summary carries `by_kind` execution counts (fixed four keys, excluding skipped); the human
  summary includes a `Categories:` distribution line
- Under `--parallel`, human progress lines are streamed in **completion order** (without
  interleaving per block); JSON `files` are sorted by `file` path to ensure stable output (CI
  diff-friendly)
- The per-test `tests` array inside files comes from the §7 suite collection, taking effect with the
  value-model landing (#319)

### 2. yaoxiang.toml Configuration

Placed under `[tool.test]`, conforming to RFC-015's `[tool.*]` third-party extension convention:

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # matches are removed from the discovery set (--list removes them as well)
parallel = true                   # Run in parallel (ORed with the --parallel flag)
```

- Default `patterns = ["tests/**/*.yx"]` — zero-config, out-of-the-box
- `exclude` has the same shape as `patterns` (literal path or `root/**…`, always matched by path
  prefix); excluded means not a test — fixtures that need runtime behavior verification are run
  directly with `yaoxiang run`
- **Single-file mode (`yaoxiang test foo.yx`) runs directly without reading config** — under
  explicit paths, neither `exclude` nor `parallel` config keys take effect (except the flag)
- May be split into a separate repository in the future (the `[tool.test]` position remains
  unchanged)

### 3. std.test Module (Pure YaoXiang)

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

// assert_not is the same body as assert_false; assert_err / assert_err_code see §8.1
```

- Assertion functions are **value-semantics**: they return `Result(Void, String)`, failures are
  expressed as `Err(diagnostic info)`, and they do not abort the process — §7 suites collect
  per-test judgments accordingly. The process-level abort semantics of `std.assert.assert` are
  reserved for runtime guards and do not enter the test assertion path. The Ok payload is `Void`
  (per type-system.md as the unit; `()` is an empty Tuple, and the two are not interchangeable —
  decided 2026-09-03)
- **Seven functions in the family (delivered 2026-09-03, abort transition version deleted)**:
  value-semantic `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not` (same
  body as assert_false, reserved for `!assert` packaging) + `assert_err` (§8.1)
  - `assert_err_code` (§8.1 error code assertion)
  - `assert_approx_eq(a: Float, b: Float, eps: Float)` (Phase 3, delivered 2026-09-07): judges
    `|a - b| <= eps`, with eps given **explicitly by the caller** — tolerance is part of the test
    contract, no hidden default; negative eps is Err at the declaration site, NaN is always Err
- `assert_eq` / `assert_ne` use **Any-typed parameters** — `==`/`!=` and f-string interpolation work
  fine on Any, and they don't depend on the generics system. Note that parameters **must be
  explicitly annotated**: unannotated parameters cannot pass the native generics `&Result(T, E)`
  call check (verified by R1 probe)
- `assert_false` / `assert_not` use `cond == false` to express negation (the `not` unary syntax is
  not yet landed; it can migrate after stabilization; the `!assert` unary form has the same
  dependency, see §8.1)
- Block body + explicit `return` form: the then-arm type of the if expression is discarded during
  checking, and the if expression with two Result arms is a checking blind spot — the implementation
  works around this
- `std.test` does not depend on any native code; it is implemented in pure YaoXiang

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedded Binary**

`std/test.yx` (and all future stdlib modules written in YaoXiang) is embedded into the binary at
build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source code text
    // more in the future
];
```

The module system (RFC-029, fully landed 2026-08-02) provides the entry point: the Registry holds
both native modules and source modules, and the orchestrator handles multi-file orchestration. The
resolution order for `use std.test`:

1. First, look up Rust native modules (existing mechanism, such as `std.assert`)
2. If not found, look up the embedded `STD_YX_FILES` — on a hit, inject the orchestrator with a
   **virtual path** (e.g., `<std>/test.yx`) as the seed module, going through the normal frontend
   pipeline (parse → typecheck → IR)
3. If not found, fall back to file system discovery (user modules)

`use std.assert` inside the embedded source module is resolved normally by the resolver to the
native registry — native and source modules coexist in the Registry, and cross-kind dependencies
work naturally. Embedded modules **are compiled on demand**: they only enter the pipeline when
imported.

Advantages:

- `use std.test` works even in single-file mode
- The standard library version is strictly bound to the binary, no version mismatch
- Users do not need to configure the standard library path

**Future: File System Standard Library**

Once the YaoXiang project mode matures, the standard library will switch to a file system form. See
the update in RFC-014 for details.

### 5. Discovery and Execution

**Prerequisite (2026-08-02 review decision)**: CLI `run` must be wired into the orchestrator. The
current CLI `run` uses a single-file pipeline (`run_file_with_diagnostics`) and cannot resolve user
module imports; the `yaoxiang test` child process model inherits CLI capabilities, and test files
importing project modules is a core scenario. So in Phase 1, the CLI `Run` source branch is first
delegated to `run_project` (orchestrator, directory-recursive discovery); #247 (on-demand discovery
along use) is then layered on as a pure performance optimization. Single files without imports
behave equivalently through the orchestrator, and the bytecode branch is unchanged.

**Discovery phase**:

1. If `[PATHS]` is specified, use the specified paths directly
2. Otherwise, read `[tool.test].patterns` from `yaoxiang.toml`
3. If not configured, default to `tests/**/*.yx`
4. Apply `--filter` (filename contains)
5. The discovery scope is the test layering (§9): default patterns only cover language-availability
   corpora; library test layers (e.g., `src/std/tests/`) are discovered via explicit paths or
   package configuration, and are not mixed into the default scan

**Execution phase**:

1. For each file, branch and execute based on the header directive (directive grammar in §8.2,
   parsed via `src/util/test_markers.rs`, shared with yx_runner):
   - Behavior tests: `yaoxiang run --debug-info <file>` child process (`--debug-info` makes runtime
     errors carry source locations — verified 2026-08-02 that stack trace outputs `file:line:col`);
     `// mode:` declares the child process `--runtime` mode
   - Compile-time rejection class: single-step `yaoxiang check <file>`
   - Runtime-failure class: two-step `check` (must pass) + `run` (must fail)
2. Files with `// skip: <reason>` skip execution and are counted as skipped in the report
3. Judgment and expected code comparison follow the §8.2 judgment matrix; directive parse failure
   means no execution and direct FAIL (construction-time rejection)
4. Capture stdout/stderr for the report
5. Serial by default; `--parallel` (or `[tool.test].parallel`) spawns a worker pool sized to
   available cores — each file is still an independent child process — skip/invalid are processed
   first in discovery order, execution results are streamed in completion order, JSON is sorted by
   path (Phase 3, delivered 2026-09-07)
6. With `--fail-fast`, stop scheduling new files on the first FAIL; in parallel mode, in-flight
   files run to completion and are counted

### 6. Test Isolation

Test isolation is naturally achieved through process-level boundaries:

- Each test file runs in an independent child process
- Each child process has its own Heap, Frame, NativeContext
- A panic in one test file does not affect other test files
- No additional isolated Heap context mechanism is needed
- **Parallel execution (Phase 3) does not expand the isolation boundary**: the working directory
  (CWD) is shared between child processes; parallel tests must not occupy files with the same path
  inside the CWD — file-I/O tests use independent file names and clean up on exit

### 7. Suite and Multiple Tests (Value-Model)

A test file may contain multiple tests. In-file organization (landed 2026-09-03):

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

- Each test is a zero-parameter function returning `Result(Void, String)`; assertion failures are
  expressed as `Err` (the §3 value-semantic assertion family), and do not interrupt the process —
  subsequent tests still run
- `test.suite` invokes each one and collects: a test that is not Ok records the name and diagnostic,
  Ok is silent; after all are done, if any Err exists, it aborts via `std.assert.assert` with
  failure details (`N of M test(s) failed` + each `[FAIL] name: diagnostic`) — the file exit code is
  non-zero (§5 judgment unchanged). The abort here is a runtime guard of the test binary, not an
  assertion path; all Ok means silent exit 0
- Top-level test functions are **enqueued in closure form** (`("name", () => test_fn())`): top-level
  function names as value references are not yet supported (IR-level limitation, `E3006`) —
  closure-body calls to global functions are unaffected
- The runner only sees the file, and does not perform function-level scanning: per-test judgments
  come entirely from in-suite collection, and the in-file structure is transparent to the runner —
  the zero-compiler-change principle is preserved
- Explicitly not adopted: in-process catch boundary (17-keyword iron rule); runner calling function
  entry points one by one (limited to internal scenarios such as §8.2 compile failure)
- The API form is decided (2026-09-03, #319):
  `suite(tests: List((String, () -> Result(Void, String)))) -> Void`; duplicate names are not
  detected (names are only used for report display); `--filter` filters by filename and is unaware
  of in-suite test names

### 8. Negative Testing (Expected Failure) Three-Layer Design

Negative tests are split by the layer where the failure occurs, each with its place:

#### 8.1 Value-Level Reverse (General, User-Facing)

The operation under test returns `Result`, and the test expresses expected failure with ordinary
assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// or in one encapsulated form (the code only exists on the std Error carrier, E is nailed down to Error):
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` have been delivered with the value-semantic family
  (2026-09-03, §3); the `!assert` unary form will be provided after the `not` syntax lands (same
  constraint as `assert_false`'s `cond == false`)
- Error code assertion depends on the `Error` value carrying a machine-readable `code` field —
  landed in #323 M4: `Error = { code, message }` (native `error_new(code, message)`), read via
  `result.unwrap_err(r)` to get the carrier + `result.code(e)` / `result.message(e)` accessors (the
  previously estimated `error_new_with_code` naming, exported code constants, and `err.code` field
  access were all not adopted — the language has no Struct field access, and code constants are not
  exported)
- As the Result-ization progresses (#301, #316), operations that can fail return `Result` one by
  one, and file-level negative markers in the corpus migrate to in-file assertions

#### 8.2 File-Level Negative Directives (For Internal Use by Language Designers Only)

Compilation is all-or-nothing per file, so "this line should not compile" cannot be expressed within
a file; runtime failure likewise needs file-level expression (e.g., a suite contains tests that must
fail). The file header declares expectations with **structured directives**, and the runner
**branches judgment** by expected category (branching decided 2026-09-03; directive grammar decided
and landed 2026-09-06).

Header directive grammar (`// key: value`, within the first 16 lines, strict token matching):

```text
// expect: compile-error E1002 [E1003 ...]   Compile-time rejection test
// expect: runtime-error E6003 [...]         Runtime-failure test
// skip: <reason>                            Skip execution, counted as skipped
// mode: embedded|standard|full              Child process --runtime mode (consumed by the run step only)
```

- No `expect:` directive = behavior test. `expect:` is the **sole declaration of expectation** —
  decided 2026-09-06 to deprecate the `[test:error]` boolean flag and the Chinese `预期:` prose
  extraction: the boolean flag and the expectation line are two loosely-coupled facts, and
  consistency by discipline is bound to drift; the strict English token grammar lets the runner
  parse mechanically (kind + codes are all fixed tokens, and any extra token after a code makes the
  parse fail), and parse failure = no execution, direct FAIL — there is no silent-degradation
  channel for incorrect directive declarations (construction-time rejection)
- **Compile-error class**: single-step `check` — must fail and output must contain all `[EXXXX]`;
  compile pass = FAIL (the expected error was not reported), rejection but mismatching codes = FAIL.
  Syntax errors (E1xxx parser phase) and semantic errors (E2xxx+) do not have separate categories —
  the expected code itself nails down the phase
- **Runtime-error class**: two-step judgment — `check` must **succeed** (compile is innocent), `run`
  must fail and output must contain all `[EXXXX]`. Compile-time explosion = FAIL (the
  opposite-direction key check versus the compile-error class, preventing leaks from "compile
  accidentally passes, runtime luckily fails")
- Convergent with industry: Rust compiletest `//~ ERROR`, Go `// ERROR "regexp"`, GCC `dg-error`,
  Clang `expected-error` all declare expectations inside fixture comments and are compared
  bidirectionally by the harness; they use line-level anchoring because multi-diagnostic compilers
  need to distinguish multiple expectations within a file. This compiler stops at the first error,
  with one diagnostic per file, so file-level is isomorphic to the compiler's reality — once error
  recovery lands, line-anchor form can be added to the grammar (Cranelift filetests' file-header
  directives + function-level expectations are the same hybrid form)
- Known rendering debt: parse-phase diagnostics are currently output in Debug form (`code: "E0012"`
  instead of `[E0012]`); code scanning accepts both forms; the strict form will be reinstated after
  diagnostic rendering is unified
- **Only serves this repository's corpus; it is not part of the user-facing testing framework**; the
  dual-runner judgment convention is closed (2026-09-03, #319): yx_runner (cargo test) and
  `yaoxiang test` both use `src/util/test_markers.rs` to parse header directives, and the
  06-compile-errors directory convention is deprecated. The report layer gives category counts:
  human summary includes a `Categories:` line, JSON summary includes `by_kind` (behavior /
  compile-error / runtime-error / invalid, skipped counted separately), and each file carries `kind`

#### 8.3 Runtime Hard Failures (Folded into Result-ization)

No independent mechanism is set up — operations that can fail return `Result` per the language
direction (#301, #316), and tests uniformly follow §8.1 to express expectations. Process-level
aborts (such as assertion violations, runtime parameter mismatches) gradually converge into values
with Result-ization, and the testing framework does not provide special semantics for them. (Note:
the "runtime-error class" marker judgment in §8.2 is the runner's file-level verification channel
for **operations that cannot yet be Result-ized**, which is not contradictory to the semantic
direction of this section — the latter is the destination, the former is the migration-period
channel)

### 9. Test System Layering: Language Corpus and Library Tests (Decided 2026-09-03)

Tests are split into two layers by **the object under test**, each with its own owner and
maintainer; the marker system (§8.2) and the assertion library (§3) are shared across both layers:

**Layer 1: Language Availability Corpus (`tests/yaoxiang/`)**

- The object under test is **the language itself** — parser, type system, modules, concurrency,
  ownership, compile-time rejection, runtime semantics; directories are organized by language
  specification chapters
- std in the corpus acts only as an **assertion tool** (`std.assert` / `std.test`), and is never
  tested — the library's API behavior does not belong to language availability
- Within the corpus, per §8 it is branched into three judgment classes: behavior tests /
  compile-time rejection tests / runtime-failure tests

**Layer 2: Library Tests (Travel with the Library)**

- The object under test is **the library's public API contract** (e.g., `list.push` behavior,
  `result.code` semantics)
- Tests are written in **the library's own package**: the std package is `src/std/`, and its
  yx-level tests go to `src/std/tests/` (co-located with the implementation; the directory coexists
  with Rust unit tests, and the file types do not overlap); `std.test`'s own tests are also there
  (using std.test to test std.test, closing the bootstrap loop)
- Future user packages follow the same convention: tests inside the package, discovered via the
  package's `[tool.test]` (the test layout of RFC-014 package management is previewed by this)
- Discovery does not enter the default patterns (the default `tests/**/*.yx` only covers the
  language layer): the library test layer is discovered via explicit paths
  (`yaoxiang test src/std/tests`) or package configuration; CI runs in layers

Migration note: **Already migrated (2026-09-06)** — the 19 files from the original
`tests/yaoxiang/07-std/` were each assessed and all turned out to be library tests (the objects
under test are all std module API contracts; language features like `?` propagation, automatic
borrowing, and generic instantiation play a supporting role in them, not the object under test);
they were migrated in their entirety to `src/std/tests/` and the 07-std directory was removed;
yx_runner changed to dual-root discovery (`tests/yaoxiang/` + `src/std/tests/`), the default
patterns do not include the library layer (integration tests solidify this contract). The language
corpus has zero std-API tests from this point on.

## Relationship with Existing Systems

| Item                                                 | Relationship                                                                                                    |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched; compiler-internal tests continue to use Rust                                                         |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                      |
| `std.assert.assert(cond)`                            | Reserved for runtime guards; `std.test`'s value-semantic assertion family is now based on `std.result` (§3, §7) |
| Module system (RFC-029)                              | Embedded source modules enter via Registry/orchestrator; CLI `run` wiring into orchestrator is a prerequisite   |
| `#200` refactor (`io.println` → `assert.assert`)     | Same direction as `yaoxiang test`                                                                               |
| `@` annotation                                       | Not used; `@test` is not introduced                                                                             |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project`
  (multi-file run prerequisite)
- `src/main.rs` — add the `Test` subcommand
- `src/std/test.yx` — add the pure YaoXiang module
- `build.rs` — embed `std/*.yx` into the binary
- orchestrator / Registry — support loading `.yx` modules from embedded sources via virtual paths
- RFC-015 config parsing — `[tool.test]` section
- Child process execution (`--debug-info`) + reporting

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

- `--parallel` parallel execution (worker pool + per-file independent child process;
  `[tool.test].parallel` config key has the same effect)
- `[tool.test].exclude` config (prefix-match removal, `--list` removes them as well)
- `assert_approx_eq` (Float with explicit eps assertion, §3)

## Risks and Mitigations

| Risk                                                          | Probability | Mitigation                                                                                                                                                                                                 |
| ------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` interpolation fails on Any                           | None        | Verified 2026-08-02 (Int/String both work)                                                                                                                                                                 |
| `yaoxiang.toml` config parsing not in current CLI             | Low         | Simple extension, does not affect core functionality                                                                                                                                                       |
| CLI run wiring to orchestrator introduces behavior regression | Low         | Single-file paths without imports are equivalent; integration tests cover orchestrator                                                                                                                     |
| Embedding `.yx` source files into binary increases size       | Low         | `.yx` source files are tiny, negligible                                                                                                                                                                    |
| Test loop time grows with corpus                              | High        | Main cost is full per-file compilation (185 files measured at 11.3s), not child process startup; `--parallel` only mitigates the process side; compilation cost needs test-loop caching (#251/#293 slices) |

## Open Questions

- [x] Can `use std.assert` inside `std/test.yx` be resolved correctly? — **Resolved (2026-08-02)**.
      After the module system (RFC-029) lands, native and source modules coexist in the Registry,
      the resolver parses uniformly, and cross-kind dependencies work naturally
- [x] Does the generic `to_string` of `f"..."` in test output introduce new type constraints? —
      **Resolved (2026-08-02)**. Verified that `==`/`!=` and f-string interpolation on unannotated
      parameters (Any) both work (verified for Int/String), no new constraints are introduced
- [x] Feasibility of `?` generic parameters? — **Resolved (2026-08-02)**: the `?` type syntax does
      not currently exist (and would be silently swallowed; a separate issue is tracking this);
      Phase 1 assertion functions use unannotated parameters, and do not depend on the generics
      system

## Design Decision Record

| Decision                           | Decision                                                                                                                                                                                                                                                                                                                                                                  | Date                      | Reason                                                                                                                                                                                                                                                                                                                                                   |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Test marking method                | Do not use `@test` annotations; test files are ordinary `.yx`                                                                                                                                                                                                                                                                                                             | 2026-07-26                | Zero compiler changes; child processes provide isolation                                                                                                                                                                                                                                                                                                 |
| Assertion method                   | Pure YaoXiang functions in the `std.test` module                                                                                                                                                                                                                                                                                                                          | 2026-07-26                | Self-bootstrapping, no native code                                                                                                                                                                                                                                                                                                                       |
| Test execution model               | Child process `yaoxiang run <file>` + exit code                                                                                                                                                                                                                                                                                                                           | 2026-07-26                | Process-level isolation, zero compiler changes                                                                                                                                                                                                                                                                                                           |
| Standard library loading           | Currently embed in binary, file system in the future                                                                                                                                                                                                                                                                                                                      | 2026-07-26                | Version binding, single-file usability                                                                                                                                                                                                                                                                                                                   |
| Assertion parameter type           | Unannotated parameters (Any), do not depend on the generics system                                                                                                                                                                                                                                                                                                        | 2026-08-02                | `?` type syntax does not exist; Any verified to be comparable and interpolatable                                                                                                                                                                                                                                                                         |
| Multi-file run                     | CLI `run` delegates to `run_project` (orchestrator) as a prerequisite                                                                                                                                                                                                                                                                                                     | 2026-08-02                | Child process model inherits CLI capabilities; #247 degrades to pure performance optimization                                                                                                                                                                                                                                                            |
| Reporting source location          | Child process carries `--debug-info`                                                                                                                                                                                                                                                                                                                                      | 2026-08-02                | Verified that stack trace outputs `file:line:col`; frames transitively attributable via embedded modules (std.test) are not guaranteed here, falling under #289 + RFC-034                                                                                                                                                                                |
| Negative test layering             | Value-level reverse general / compile-failure runner structured marker (internal only) / hard failures folded into Result-ization                                                                                                                                                                                                                                         | 2026-09-02                | Decided by #319; supersedes the implicit [test:error] convention                                                                                                                                                                                                                                                                                         |
| In-file multiple tests             | Value-model standard form: test functions return Result, suite collects per-test judgments                                                                                                                                                                                                                                                                                | 2026-09-02                | No catch, no entry-call (entry only for internal scenarios)                                                                                                                                                                                                                                                                                              |
| Error code                         | Add a machine-readable `code` field to Error                                                                                                                                                                                                                                                                                                                              | 2026-09-02                | Supports error code assertion; compile-time codes go through runner comparison                                                                                                                                                                                                                                                                           |
| Assertion library form             | 7 value-semantic functions landed, `Result(Void, String)` contract; abort transition version deleted                                                                                                                                                                                                                                                                      | 2026-09-03                | Void is the canonical unit (`()` is an empty Tuple, do not conflate); the nested slot is rigidly Any, unannotated parameters cannot pass the native generics check — parameters must be explicitly annotated (verified by R1 probe)                                                                                                                      |
| Test system layering               | Language corpus (`tests/yaoxiang/`) and library tests (with the library, std → `src/std/tests/`) split into two layers; std in the corpus acts only as an assertion tool                                                                                                                                                                                                  | 2026-09-03                | The object under test determines ownership and maintainer; library tests with the package layout preview RFC-014                                                                                                                                                                                                                                         |
| Negative marker branching judgment | Branch by expected category: compile-error class `check` must fail, runtime-error class `check` must pass + `run` must fail; report gives category counts                                                                                                                                                                                                                 | 2026-09-03 (landed 09-06) | Mixed category judgment would let "compile accidentally passes, runtime luckily fails" leaks slip through; expected codes nail down the phase, syntax errors do not get a separate category                                                                                                                                                              |
| Header directive grammar           | Expectations declared with English structured directives (`// expect:` / `// skip:` / `// mode:`, strict token grammar, parse failure means direct FAIL); deprecate the `[test:error]` boolean flag and the Chinese `预期:` prose extraction                                                                                                                              | 2026-09-06                | Expectation is a property of fixture content, in-fixture declaration is convergent with industry (compiletest / Go / GCC / Clang all do this), a central list is bound to rot; a boolean flag + expectation line are two facts coupled by discipline — a defect surface; structured grammar lets the runner judge mechanically without human involvement |
| Parallel execution model           | `--parallel` spawns a per-core worker pool (each file is still an independent child process), skip/invalid are processed first in discovery order, execution results are streamed in completion order, JSON is sorted by path; `--fail-fast` stops scheduling (in-flight files run to completion and are counted); CWD remains shared, isolation boundary is not expanded | 2026-09-07                | Under the child process model, parallelism = OS-thread-scheduled spawn, no yx-layer concurrency needed; the main time cost is per-file full compilation (risk table), parallelism only mitigates the process side — #293 caching slice is the main mitigation                                                                                            |

## References

- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — standard library
  directory structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]`
  configuration section
- [RFC-030: assert Assertion Mechanism](../review/030-assert-mechanism.md) — low-level dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — reference
  design
