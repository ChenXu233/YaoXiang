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

Introduce a standard testing framework `std.test` module and `yaoxiang test` CLI subcommand for
YaoXiang. Test files are ordinary `.yx` files, and the overall pass/fail verdict is determined by
the subprocess exit code; files internally support multiple test functions—assertion failures are
expressed as `Err` values (value semantics), and the suite collects per-test verdicts (§7). The
`std.test` module is implemented in pure YaoXiang and is the first dogfooding library.
`yaoxiang test` is a CLI tool, not a compiler feature—it involves no changes to the parser, IR,
bytecode, or executor.

## Motivation

### Why is a test framework needed?

Current YaoXiang test coverage relies on Rust-side `#[test]` and `tests/` integration tests. This
means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io)
   cannot be written in YaoXiang
2. `#117 Unit test coverage for each standard library module` is blocked because no usable test
   infrastructure is available
3. Regression tests for language features (such as RFC-032 spawn semantics changes) lack automated
   means

### Key Constraints

- **17-keyword iron rule**: No new keywords or syntactic structures are introduced
- **Zero compiler changes**: No touching the parser, IR, bytecode, or executor
- **Bootstrap priority**: The test library is written in YaoXiang—the first dogfooding library

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
│              check exit code → run serially                  │
│              │                                               │
│  Reporting:  PASS/FAIL → summary                             │
│              supports --json / --verbose / --fail-fast       │
│                                                              │
│  Assertion:  std.test (pure YaoXiang, bootstrap)             │
│              base: std.assert.assert                         │
│              diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **The test framework is not a compiler feature, but a CLI tool** — `yaoxiang run` can already
   "execute tests"; `yaoxiang test` merely helps you run all the files and show you the report
2. **Zero compiler changes** — No introduction of `@test` annotation scanning, bytecode metadata
   segments, or executor special entry points
3. **Bootstrap** — The `std.test` module is implemented in pure YaoXiang, with base capabilities
   provided by `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files** — Files run as subprocesses, and the exit code determines
   overall pass/fail
5. **Assertion failure is a value, not a process event** — Test functions return `Result`, and
   assertion failures are expressed as `Err`; the suite collects per-test verdicts individually
   (§7); process-level aborts only belong to runtime guards and are not used for test assertions

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Only run test files whose name contains <NAME>
  --fail-fast         Stop on the first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              Only list test files, do not run
  --no-progress       Suppress progress output (header and PASS lines); FAIL details and summary are retained (CI scenarios)
  --json              Output JSON format results (for CI integration)
```

#### Output Format

**Default output** (per-test verdicts come from in-file suite collection, see §7):

```
Running 3 test files...

tests/math_test.yx ........................ PASS (0.002s)
tests/list_test.yx ........................ FAIL (0.003s)
  `-- [FAIL] push_grows_len: Expected 3, got 2
  `-- [ ok ] pop_returns_last
Results: 2 files passed, 1 file failed, 0 skipped (0.006s)
```

**JSON output** (`--json`):

```json
{
  "summary": { "total": 3, "passed": 2, "failed": 1, "skipped": 0, "time_secs": 0.006 },
  "files": [
    { "file": "tests/math_test.yx", "passed": true, "time_secs": 0.002 },
    {
      "file": "tests/list_test.yx",
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

- Failed files additionally carry `exit_code` and `stderr` (subprocess diagnostics with ANSI
  stripped, for CI forensics); when `--verbose` is combined with `--json`, all files carry `stdout`
  / `stderr`
- `--no-progress` only suppresses progress output (header and PASS lines)—FAIL details and summary
  are always output; failures cannot be silenced; `--list` outputs one test file path per line,
  without execution
- The in-file per-test `tests` array comes from §7 suite collection, taking effect when the
  value-based model lands (#319)

### 2. yaoxiang.toml Configuration

Placed under `[tool.test]`, conforming to RFC-015's `[tool.*]` third-party extension convention:

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
# Future extensions:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- Default `patterns = ["tests/**/*.yx"]` — zero-config, works out of the box
- Single-file mode (`yaoxiang test foo.yx`) runs directly without reading config
- May be split into a separate repository in the future (the `[tool.test]` location remains
  unchanged)

### 3. std.test Module (Pure YaoXiang)

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library
// First dogfooding library: YaoXiang's test library written in YaoXiang

use std.result

assert_eq = (a, b) => {
    if a == b { result.ok(()) } else { result.err(f"Expected {b}, got {a}") }
}

assert_ne = (a, b) => {
    if a != b { result.ok(()) } else { result.err(f"Expected not equal to {b}, got {a}") }
}

assert_true = (cond: Bool) => {
    if cond { result.ok(()) } else { result.err(f"Expected true, got {cond}") }
}

assert_false = (cond: Bool) => {
    if cond == false { result.ok(()) } else { result.err(f"Expected false, got {cond}") }
}
```

- Assertion functions use **value semantics**: they return `Result((), String)`, with failures
  expressed as `Err(diagnostic info)`, and do not abort the process—the §7 suite collects per-test
  verdicts based on this. The process-level abort semantics of `std.assert.assert` are reserved for
  runtime guards and do not enter the test assertion path
- Transition note: The 4 functions shipped in Phase 1 are based on `std.assert.assert` (abort
  semantics) as a bootstrap transitional implementation; the value-semantics family is the standard
  form of this RFC, replacing them once landed (#319)
- `assert_eq` / `assert_ne` use **unannotated parameters** (`Any`)—empirically verified on
  2026-08-02: `==`/`!=` and f-string interpolation work correctly on Any (verified for both Int and
  String), **not dependent on the generics system**. Annotations can be added later once generics
  are ready
- `assert_false` uses `cond == false` to express negation (the unary `not` syntax has not landed; it
  can be migrated once stable; the unary form `!assert` shares the same dependency, see §8.1)
- Error code assertions (`assert(err.code == "E3017")`) depend on the `Error` value carrying a
  machine-readable `code` field (§8.1)
- `std.test` does not depend on any native code; it is implemented in pure YaoXiang

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedded in the binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) are embedded in the
binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source code text
    // more in the future
];
```

The module system (RFC-029, fully landed on 2026-08-02) provides the access point: the Registry
holds both native modules and source modules, and the orchestrator handles multi-file coordination.
The resolution order for `use std.test`:

1. First check Rust native modules (existing mechanism, such as `std.assert`)
2. If not found, check the embedded `STD_YX_FILES`—if found, inject it into the orchestrator with a
   **virtual path** (e.g. `<std>/test.yx`) as the seed module, and go through the normal frontend
   pipeline (parse → typecheck → IR)
3. If not found, use filesystem discovery (user modules)

`use std.assert` inside an embedded source module is resolved normally by the resolver to the native
registry—native and source modules coexist in the Registry, and cross-kind dependencies work
naturally. Embedded modules are **compiled on demand**: they only enter the pipeline when imported.

Advantages:

- `use std.test` works in single-file mode
- Standard library version is strictly bound to the binary, avoiding version mismatch
- No need for users to configure the standard library path

**Future: Filesystem Standard Library**

Once YaoXiang's project mode matures, the standard library will switch to a filesystem form. See the
updates to RFC-014 for details.

### 5. Discovery and Execution

**Prerequisite (2026-08-02 review decision)**: CLI `run` integrates the orchestrator. Currently, CLI
`run` follows the single-file pipeline (`run_file_with_diagnostics`) and cannot resolve user module
imports; however, the subprocess model of `yaoxiang test` inherits CLI capabilities, and having test
files import project modules is a core scenario. Therefore, in Phase 1 the CLI `Run` source branch
is first delegated to `run_project` (orchestrator, directory recursive discovery); #247 (on-demand
discovery along `use`) comes later as a pure performance optimization overlay. Single files without
imports behave equivalently through the orchestrator, and the bytecode branch is unchanged.

**Discovery phase**:

1. If `[PATHS]` is specified, use the specified paths directly
2. Otherwise, read `[tool.test].patterns` from `yaoxiang.toml`
3. If not configured, default to `tests/**/*.yx`
4. Apply `--filter` filtering (file name contains)

**Execution phase**:

1. For each file: launch a subprocess with `yaoxiang run --debug-info <file>` (`--debug-info` causes
   runtime errors to include source locations—empirically verified on 2026-08-02 that stack traces
   output `file:line:col`)
2. Check exit code: 0 means PASS, non-zero means FAIL
3. Capture stdout/stderr for the report
4. Execute serially only (Phase 1); `--parallel` will be supported in the future
5. If `--fail-fast` is set, stop immediately on the first FAIL

### 6. Test Isolation

Test isolation is naturally achieved through process-level boundaries:

- Each test file runs in an independent subprocess
- Each subprocess has its own Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional independent Heap context mechanism is needed

### 7. Suite and Multi-Test (Value-Based Model)

A test file can contain multiple tests. In-file organization:

```yaoxiang
// tests/list_test.yx
use std.test
use std.list

push_grows_len = () => {
    xs = []
    list.append(xs, 1)
    test.assert_eq(list.len(xs), 1)
}

pop_returns_last = () => {
    xs = [1, 2]
    test.assert_eq(list.pop(xs), 2)
}

main = {
    test.suite([
        ("push_grows_len", push_grows_len),
        ("pop_returns_last", pop_returns_last),
    ])
}
```

- Each test is a zero-parameter function returning `Result((), String)`; assertion failures are
  expressed as `Err` (the §3 value-semantics assertion family) and do not interrupt the
  process—subsequent tests continue to run
- `test.suite` calls each one and collects: if a test is not Ok, print that test's name and
  diagnostic info; Ok is silent
- File exit code: all tests in the suite Ok → 0; any Err → non-zero (the §5 execution phase exit
  code verdict is unchanged)
- The runner only sees files, not function-level scanning: per-test verdicts come entirely from
  in-suite collection, and the in-file structure is transparent to the runner—the
  zero-compiler-change principle is not affected
- Explicitly not adopted: in-process catch boundaries (17-keyword iron rule); runner calling test
  function entries one by one (limited to internal scenarios such as §8.2 compilation failures)
- The specific API form of `test.suite` (signature, duplicate name handling, interaction between
  `--filter` and suite names) is an implementation detail, finalized in #319 during landing

### 8. Three-Layer Design for Negative Tests (Expected Failures)

Negative tests are split by the layer at which the failure occurs, with each layer assigned to its
proper place:

#### 8.1 Value-Level Reversal (General, User-Facing)

The operation under test returns `Result`, and the test expresses the expected failure with ordinary
assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
test.assert_eq(result.err_code(r), "E3017")
```

- std.test adds the `assert_not` / `assert_err` function family; the unary form `!assert` is
  provided once the `not` syntax lands (same constraint as `assert_false`'s `cond == false`)
- Error code assertions depend on the `Error` value being extended with a machine-readable `code`
  field: expand from `Struct { message }` to `{ code, message }` (native `error_new_with_code`, std
  exports code constants), making `err.code == "E3017"` assertable
- As the Result-ification progresses (#301, #316), fallible operations gradually return `Result`,
  and the file-level negative markers in the corpus migrate to in-file assertions accordingly

#### 8.2 Compilation Failure (For Language Designers' Internal Use Only)

Compilation is all-or-nothing for a file, and "this line should not compile" cannot be expressed
within a file. The file-level special markers are retained:

- The `[test:error]` marker is read by the runner and judged in reverse (run exit code non-zero =
  PASS)
- The marker is upgraded to a **structured expected code**: the runner parses the header
  `expected: compile error EXXXX` and compares it against the `[EXXXX]` output by the compiler on
  stderr; a code mismatch = FAIL (a lightweight version of the trybuild stderr snapshot idea, with
  zero compiler changes)
- **Only serves this repository's corpus, not part of the user test framework**; the implementation
  side needs to unify the dual-runner verdict conventions (directory convention vs. header marker),
  see #319

#### 8.3 Runtime Hard Failure (Folded into Result-ification)

No independent mechanism is set up—operations that may fail return `Result` per the language
direction (#301, #316), and tests uniformly follow the §8.1 expression. Process-level aborts (such
as assertion violations, runtime parameter mismatches) are gradually converged into values as
Result-ification progresses, and the test framework provides no dedicated semantics for them.

## Relationship with Existing Systems

| Project                                              | Relationship                                                                                                    |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched; compiler internal tests continue to use Rust                                                         |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                      |
| `std.assert.assert(cond)`                            | Reserved for runtime guards; the `std.test` value-semantics assertion family is based on `std.result` (§3, §7)  |
| Module system (RFC-029)                              | Embedded source modules connect via Registry/orchestrator; CLI `run` integrating orchestrator is a prerequisite |
| `#200` refactor (`io.println` → `assert.assert`)     | Exactly the same direction as `yaoxiang test`                                                                   |
| `@` annotation                                       | Not used; `@test` is not introduced                                                                             |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project`
  (multi-file execution prerequisite)
- `src/main.rs` — add the `Test` subcommand
- `src/std/test.yx` — add the pure YaoXiang module
- `build.rs` — embed `std/*.yx` into the binary
- orchestrator / Registry — support loading `.yx` modules from embedded sources via virtual paths
- RFC-015 config parsing — `[tool.test]` section
- Subprocess execution (`--debug-info`) + reporting

Deliverables:

- `yaoxiang test` basically usable
- 4 assertion functions in `std.test`
- Default `tests/**/*.yx` discovery
- Serial execution + default output format

### Phase 2: Polish

- `--filter` / `--fail-fast` / `--verbose` options
- `--json` output (CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: Advanced

- `--parallel` parallel execution (depends on spawn concurrency model maturation)
- `[tool.test].exclude` configuration
- More assertion functions (such as `assert_approx_eq` for Float)

## Risks and Mitigations

| Risk                                                              | Probability | Mitigation                                                                                                                                                                                          |
| ----------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` interpolation on Any fails                               | None        | Empirically verified on 2026-08-02 (Int/String both work)                                                                                                                                           |
| `yaoxiang.toml` config parsing not in current CLI                 | Low         | Simple extension, doesn't affect core functionality                                                                                                                                                 |
| CLI run integrating orchestrator introduces behavioral regression | Low         | No-import single-file path is equivalent; integration tests already cover orchestrator                                                                                                              |
| Embedding `.yx` source files in the binary increases size         | Low         | `.yx` source files are extremely small, negligible                                                                                                                                                  |
| Test loop time grows with corpus                                  | High        | Main cost is full per-file compilation (185 files measured 11.3s), not subprocess startup; `--parallel` only mitigates the process side, compilation cost needs test loop caching (#251/#293 slice) |

## Open Questions

- [x] Can `use std.assert` references in `std/test.yx` be resolved correctly? — **Resolved
      (2026-08-02)**. After the module system (RFC-029) landed, native and source modules coexist in
      the Registry; the resolver handles them uniformly, and cross-kind dependencies work naturally
- [x] Does the generic `to_string` of `f"..."` in test output introduce new type constraints? —
      **Resolved (2026-08-02)**. Empirically, `==`/`!=` and f-string interpolation work on
      unannotated parameters (Any) (verified for Int/String), no new constraints introduced
- [x] `?` generic parameter feasibility? — **Resolved (2026-08-02)**: The `?` type syntax does not
      currently exist (and would be silently swallowed, tracked in a separate issue); Phase 1
      assertion functions use unannotated parameters, not depending on the generics system

## Design Decision Record

| Decision                  | Determination                                                                                                                            | Date       | Rationale                                                                                                                                                                 |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Test marker approach      | No `@test` annotation; test files are ordinary `.yx`                                                                                     | 2026-07-26 | Zero compiler changes; subprocess provides isolation                                                                                                                      |
| Assertion approach        | `std.test` module in pure YaoXiang functions                                                                                             | 2026-07-26 | Bootstrap, no native code                                                                                                                                                 |
| Test execution model      | Subprocess `yaoxiang run <file>` + exit code                                                                                             | 2026-07-26 | Process-level isolation, zero compiler changes                                                                                                                            |
| Standard library loading  | Currently embedded in binary, filesystem in the future                                                                                   | 2026-07-26 | Version binding, works in single-file mode                                                                                                                                |
| Assertion parameter types | Unannotated parameters (Any), not depending on generics                                                                                  | 2026-08-02 | `?` type syntax does not exist; Any empirically comparable and interpolable                                                                                               |
| Multi-file execution      | CLI `run` delegates to `run_project` (orchestrator) as prerequisite                                                                      | 2026-08-02 | Subprocess model inherits CLI capabilities; #247 degrades to pure performance optimization                                                                                |
| Reporting source location | Subprocess with `--debug-info`                                                                                                           | 2026-08-02 | Empirically stack trace outputs `file:line:col`; frame attribution forwarded through embedded modules (std.test) is not guaranteed under this, falls under #289 + RFC-034 |
| Negative test layering    | Value-level reversal general / compilation failure runner structured markers (internal only) / hard failure folded into Result-ification | 2026-09-02 | Finalized in #319; replaces implicit [test:error] convention                                                                                                              |
| In-file multi-test        | Value-based standard model: test functions return Result, suite collects per-test verdicts                                               | 2026-09-02 | No catch, no entry-point calls (entry-point only for internal scenarios)                                                                                                  |
| Error code                | Error adds machine-readable `code` field                                                                                                 | 2026-09-02 | Supports error code assertions; compile-time code goes through runner comparison                                                                                          |

## References

- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — Standard library
  directory structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]`
  configuration section
- [RFC-030: assert Assertion Mechanism](../review/030-assert-mechanism.md) — Underlying dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — Reference
  design
