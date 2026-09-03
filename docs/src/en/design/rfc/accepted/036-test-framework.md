---
title: 'RFC-036: std.test Framework and yaoxiang test Command'
status: 'Accepted'
author: '晨煦 (Chenxu)'
created: '2026-07-26'
updated: '2026-09-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test Framework and yaoxiang test Command

## Summary

Introduces the standard test framework `std.test` module and the `yaoxiang test` CLI subcommand for
YaoXiang. Test files are ordinary `.yx` files whose overall pass/fail status is determined by the
subprocess exit code; within a file, multiple test functions are supported — assertion failures are
expressed as `Err` values (value semantics), and per-test verdicts are collected by the suite (§7).
The `std.test` module is implemented in pure YaoXiang and is the first dogfooding library.
`yaoxiang test` is a CLI tool, not a compiler feature — it involves no changes to the parser, IR,
bytecode, or executor.

## Motivation

### Why a test framework?

YaoXiang's current test coverage depends on Rust-side `#[test]` and `tests/` integration tests. This
means:

1. The standard library (std.math / std.list / std.dict / std.convert / std.io) cannot have unit
   tests written in YaoXiang
2. `#117 Unit test coverage for each std module` is blocked because no test infrastructure is
   available
3. Regression tests for language features (such as the RFC-032 spawn semantics change) lack
   automated means

### Key constraints

- **17 keyword iron rule**: no new keywords or syntactic constructs introduced
- **Zero compiler changes**: no touches to parser, IR, bytecode, or executor
- **Bootstrap-first**: the test library is written in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI layer:  yaoxiang test [--filter --fail-fast --json ...] │
│              │                                               │
│  Discovery:  Read yaoxiang.toml → [tool.test] patterns       │
│              Default: tests/**/*.yx                          │
│              │                                               │
│  Execution:  For each file: yaoxiang run <file>              │
│              Check exit code → serial execution              │
│              │                                               │
│  Reporting:  PASS/FAIL → summary                             │
│              Supports --json / --verbose / --fail-fast       │
│                                                              │
│  Assertion:  std.test (pure YaoXiang, bootstrap)             │
│              Lower layer: std.assert.assert                   │
│              Diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core principles

1. **The test framework is not a compiler feature; it is a CLI tool** — `yaoxiang run` can already
   "execute tests"; `yaoxiang test` simply helps you run all files and shows you the report
2. **Zero compiler changes** — no `@test` annotation scanning, no bytecode metadata sections, no
   special executor entry points
3. **Bootstrap** — the `std.test` module is implemented in pure YaoXiang, with lower-level
   capabilities from `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files** — a file is run as a subprocess, and the exit code
   determines overall pass/fail
5. **Assertion failures are values, not process events** — test functions return `Result`, assertion
   failures are expressed as `Err`, and the suite collects per-test verdicts (§7); process-level
   abort belongs only to runtime guards, not to test assertions

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, else tests/)

Options:
  --filter <NAME>     Only run tests whose file name contains <NAME>
  --fail-fast         Stop on the first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              Only list test files, do not run them
  --no-progress       Suppress progress output (header and PASS lines); FAIL details and summary are kept (CI scenario)
  --json              Output results in JSON format (for CI integration)
```

#### Output format

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
- `--no-progress` only suppresses progress output (header and PASS lines) — FAIL details and summary
  are always output, failures cannot be silenced; `--list` outputs one test file path per line and
  does not execute
- The in-file per-test `tests` array comes from §7 suite collection and takes effect with the
  value-based model (#319)

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

- Default `patterns = ["tests/**/*.yx"]` — zero-config out of the box for users
- Single-file mode (`yaoxiang test foo.yx`) runs directly, without reading configuration
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

- Assertion functions are **value-semantic**: they return `Result((), String)`, failures are
  expressed as `Err(diagnostic)`, and they do not abort the process — §7's suite uses this to
  collect per-test verdicts. The process-level abort semantics of `std.assert.assert` are preserved
  for runtime guards and do not enter the test assertion path
- Transition note: the 4 functions delivered in Phase 1 are based on `std.assert.assert` (abort
  semantics) as a bootstrap transitional implementation; the value-semantic family is the standard
  form of this RFC and will replace it after landing (#319)
- `assert_eq` / `assert_ne` use **untyped parameters** (`Any`) — empirically verified on 2026-08-02:
  `==`/`!=` and f-string interpolation work correctly on Any (verified with Int/String), and **do
  not depend on the generics system**. Type annotations can be added once generics are ready
- `assert_false` uses `cond == false` to express negation (`not` unary syntax is not yet
  implemented; the `!assert` unary form has the same dependency — see §8.1)
- Error code assertions (`assert(err.code == "E3017")`) depend on the `Error` value carrying a
  machine-readable `code` field (§8.1)
- `std.test` does not depend on any native code; it is implemented in pure YaoXiang

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedded binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) is embedded into the
binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source text
    // more in the future
];
```

The module system (RFC-029, fully landed on 2026-08-02) provides the entry point: the Registry holds
both native modules and source modules, and the orchestrator handles multi-file arrangement. The
resolution order for `use std.test`:

1. First check Rust native modules (existing mechanism, e.g. `std.assert`)
2. If not found, check the embedded `STD_YX_FILES` — on hit, inject as a seed module under a
   **virtual path** (e.g. `<std>/test.yx`) into the orchestrator, going through the normal frontend
   pipeline (parse → typecheck → IR)
3. If not found, fall through to filesystem discovery (user modules)

The `use std.assert` inside an embedded source module is resolved normally by the resolver to the
native registry — native and source modules coexist in the Registry, and cross-kind dependencies
work naturally. Embedded modules are **compiled on demand**: they only enter the pipeline when
imported.

Advantages:

- `use std.test` works in single-file mode
- Standard library version is strictly bound to the binary; no version mismatch
- No need for users to configure standard library paths

**Future: Filesystem standard library**

When the YaoXiang project mode matures, the standard library will move to filesystem form. See the
RFC-014 update for details.

### 5. Discovery and Execution

**Prerequisite (2026-08-02 review decision)**: CLI `run` integrates with the orchestrator. Currently
CLI `run` goes through the single-file pipeline (`run_file_with_diagnostics`) and cannot resolve
user module imports; but `yaoxiang test`'s subprocess model inherits CLI capabilities, and test
files importing project modules is a core scenario. Therefore, Phase 1 first delegates the CLI `Run`
source branch to `run_project` (orchestrator, directory-recursive discovery); #247 (on-demand
discovery along `use`) becomes a pure performance optimization layered on top afterwards. Single
files without imports behave equivalently through the orchestrator; the bytecode branch is
unchanged.

**Discovery phase**:

1. If `[PATHS]` is specified, use those paths directly
2. Otherwise read `[tool.test].patterns` from `yaoxiang.toml`
3. If no configuration, default to `tests/**/*.yx`
4. Apply `--filter` (file name contains)

**Execution phase**:

1. For each file: launch a subprocess with `yaoxiang run --debug-info <file>` (`--debug-info` makes
   runtime errors carry source locations — empirically verified on 2026-08-02 that stack trace
   output includes `file:line:col`); the header `[test:runtime]` declares the subprocess's
   `--runtime` mode (closed on 2026-09-03)
2. Files with header `[test:ignore]: <reason>` skip execution and are counted as skipped in the
   report (closed on 2026-09-03)
3. Check exit code: 0 is PASS, non-zero is FAIL; `[test:error]` files are inversely judged and
   compared against the expected code per §8.2
4. Capture stdout/stderr for the report
5. Execute serially only (Phase 1); `--parallel` supported in the future
6. If `--fail-fast`, stop immediately on the first FAIL

### 6. Test Isolation

Test isolation is naturally provided by process-level boundaries:

- Each test file runs in an independent subprocess
- Each subprocess has its own Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional isolated Heap context mechanism is needed

### 7. Suite and Multiple Tests (Value-based Model)

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
  expressed as `Err` (the value-semantic assertion family in §3) and do not interrupt the process —
  subsequent tests continue to run
- `test.suite` invokes them one by one and collects: if a test is not Ok, print that test's name and
  diagnostic; Ok is silent
- File exit code: suite all Ok → 0; any Err → non-zero (the exit code judgment in §5 is unchanged)
- The runner only sees files, not function-level scanning: per-test verdicts come entirely from
  in-suite collection, the internal structure of the file is transparent to the runner — the
  zero-compiler-changes principle is not affected
- Explicitly not adopted: in-process catch boundaries (17 keyword iron rule); runner calling
  function entries one by one (only for internal scenarios such as §8.2 compile failures)
- The specific API shape of `test.suite` (signature, duplicate-name handling, interaction between
  `--filter` and suite names) is an implementation detail, finalized in #319 at landing

### 8. Three-Layer Design for Negative Testing (Expected Failures)

Negative testing is split by the layer at which the failure occurs, and each layer is placed
appropriately:

#### 8.1 Value-level Inversion (Generic, User-facing)

The operation under test returns a `Result`, and the test uses ordinary assertions to express
expected failure:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
test.assert_eq(result.err_code(r), "E3017")
```

- `std.test` adds the `assert_not` / `assert_err` function family; the `!assert` unary form is
  provided once the `not` syntax lands (same constraint as `assert_false`'s `cond == false`)
- Error code assertions depend on extending the `Error` value with a machine-readable `code` field:
  from `Struct { message }` to `{ code, message }` (native `error_new_with_code`, std exports code
  constants), so that `err.code == "E3017"` can be asserted
- As the Result-ification advances (#301, #316), fallible operations gradually return `Result`, and
  the file-level negative markers in corpus migrate to in-file assertions

#### 8.2 Compile Failure (Only for Language Designers' Internal Use)

Compilation is all-or-nothing per file and cannot express "this line should not compile" in-file.
File-level special markers are retained:

- The `[test:error]` marker is read by the runner and inversely judged (run exit code non-zero =
  PASS)
- The marker is upgraded to a **structured expected code**: the runner parses the header
  `预期: 编译错误 EXXXX` and compares it with the `[EXXXX]` actually emitted by the compiler on
  stderr; mismatch = FAIL (lightweight version of the trybuild stderr snapshot idea, zero compiler
  changes)
- **Only serves this repository's corpus, not part of the user test framework**; the dual-runner
  judgment convention has been closed (2026-09-03, #319): `yx_runner` (cargo test) and
  `yaoxiang test` share `src/util/test_markers.rs` to parse header markers
  (`[test:error]`/`[test:ignore]`/`[test:runtime]`/`预期: EXXXX`, first 16 lines); the
  06-compile-errors directory convention is deprecated

#### 8.3 Runtime Hard Failure (Rolled into Result-ification)

No dedicated mechanism is provided — operations that can fail return `Result` per the language
direction (#301, #316), and tests uniformly take the §8.1 path. Process-level abort (such as
assertion violations, runtime parameter mismatches) is gradually converged to values as
Result-ification advances, and the test framework does not provide dedicated semantics for it.

## Relationship with Existing Systems

| Item                                                 | Relationship                                                                                                      |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched; compiler-internal tests continue to use Rust                                                           |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                        |
| `std.assert.assert(cond)`                            | Preserved for runtime guards; `std.test`'s value-semantic assertion family is based on `std.result` (§3, §7)      |
| Module system (RFC-029)                              | Embedded source modules integrate via Registry/orchestrator; CLI `run` integrating orchestrator is a prerequisite |
| `#200` refactor (`io.println` → `assert.assert`)     | Same direction as `yaoxiang test`                                                                                 |
| `@` annotation                                       | Not used; no `@test` introduced                                                                                   |

## Implementation Strategy

### Phase 1: Core functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project`
  (prerequisite for multi-file running)
- `src/main.rs` — Add `Test` subcommand
- `src/std/test.yx` — Add pure YaoXiang module
- `build.rs` — Embed `std/*.yx` into the binary
- orchestrator / Registry — Support loading `.yx` modules from embedded sources via virtual paths
- RFC-015 configuration parsing — `[tool.test]` section
- Subprocess execution (`--debug-info`) + reporting

Deliverables:

- `yaoxiang test` basically usable
- `std.test` with 4 assertion functions
- Default `tests/**/*.yx` discovery
- Serial execution + default output format

### Phase 2: Polish

- `--filter` / `--fail-fast` / `--verbose` options
- `--json` output (CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: Advanced

- `--parallel` execution (depends on the spawn concurrency model being complete)
- `[tool.test].exclude` configuration
- More assertion functions (e.g. `assert_approx_eq` for Float)

## Risks and Mitigations

| Risk                                                              | Probability | Mitigation                                                                                                                                                                                         |
| ----------------------------------------------------------------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` interpolation failure on Any                             | None        | Empirically verified on 2026-08-02 (Int/String both work)                                                                                                                                          |
| `yaoxiang.toml` configuration parsing not in current CLI          | Low         | Simple extension, does not affect core functionality                                                                                                                                               |
| CLI run integrating orchestrator introduces behavioral regression | Low         | Single-file path without imports is equivalent; integration tests already cover the orchestrator                                                                                                   |
| Embedding `.yx` source files in binary increases size             | Low         | `.yx` source files are very small, negligible                                                                                                                                                      |
| Test loop time grows with corpus                                  | High        | Main cost is full per-file compilation (185 files measured at 11.3s), not subprocess startup; `--parallel` only helps the process side; compilation cost needs test loop caching (#251/#293 slice) |

## Open Questions

- [x] Can `use std.assert` in `std/test.yx` resolve correctly? — **Resolved (2026-08-02)**. After
      the module system (RFC-029) landed, native and source modules coexist in the Registry; the
      resolver unifies resolution, and cross-kind dependencies work naturally
- [x] Does the generic `to_string` of `f"..."` in test output introduce new type constraints? —
      **Resolved (2026-08-02)**. Empirically, `==`/`!=` and f-string interpolation work on untyped
      parameters (Any) (verified with Int/String); no new constraints introduced
- [x] Is the `?` generic parameter feasible? — **Resolved (2026-08-02)**: the `?` type syntax does
      not currently exist (and would be silently swallowed; a separate issue tracks this); Phase 1
      assertion functions use untyped parameters and do not depend on the generics system

## Design Decision Records

| Decision                 | Decision                                                                                                                   | Date       | Rationale                                                                                                                                                        |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Test marker approach     | No `@test` annotations; test files are ordinary `.yx`                                                                      | 2026-07-26 | Zero compiler changes, subprocess is the isolation                                                                                                               |
| Assertion approach       | `std.test` module in pure YaoXiang functions                                                                               | 2026-07-26 | Bootstrap, no native code                                                                                                                                        |
| Test execution model     | Subprocess `yaoxiang run <file>` + exit code                                                                               | 2026-07-26 | Process-level isolation, zero compiler changes                                                                                                                   |
| Standard library loading | Currently embedded binary, filesystem in the future                                                                        | 2026-07-26 | Version binding, single-file usable                                                                                                                              |
| Assertion parameter type | Untyped parameters (Any), does not depend on the generics system                                                           | 2026-08-02 | `?` type syntax does not exist; Any empirically comparable and interpolable                                                                                      |
| Multi-file running       | CLI `run` delegates to `run_project` (orchestrator) as a prerequisite                                                      | 2026-08-02 | Subprocess model inherits CLI capability; #247 downgraded to pure performance optimization                                                                       |
| Report source location   | Subprocess with `--debug-info`                                                                                             | 2026-08-02 | Empirically stack trace output includes `file:line:col`; frame attribution through embedded modules (std.test) is not guaranteed here, belongs to #289 + RFC-034 |
| Negative test layering   | Value-level inversion generic / compile-failure runner structured marker (internal only) / hard failure → Result-ification | 2026-09-02 | #319 finalized; supersedes the implicit `[test:error]` convention                                                                                                |
| Multiple tests in file   | Value-based standard model: test functions return `Result`, suite collects per-test verdicts                               | 2026-09-02 | No catch, no entry-point calls (entry only for internal scenarios)                                                                                               |
| Error code               | Error gains machine-readable `code` field                                                                                  | 2026-09-02 | Supports error code assertions; compile-time codes go through runner comparison                                                                                  |

## References

- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — Standard library
  directory structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]`
  configuration section
- [RFC-030: assert Mechanism](../review/030-assert-mechanism.md) — Underlying dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — Reference
  design
