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

Introduce a standard testing framework `std.test` module and the `yaoxiang test` CLI subcommand for
YaoXiang. Test files are ordinary `.yx` files, with pass/fail determined via `std.assert.assert` and
exit codes. The `std.test` module is implemented in pure YaoXiang and is the first dogfooding
library. `yaoxiang test` is a CLI tool, not a compiler feature—it does not involve any changes to
the parser, IR, bytecode, or executor.

## Motivation

### Why a testing framework is needed

YaoXiang's current test coverage relies on Rust-side `#[test]` and `tests/` integration tests. This
means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io)
   cannot be written in YaoXiang
2. `#117 Unit test coverage for std lib modules` is blocked because no usable test infrastructure
   exists
3. Regression tests for language features (e.g., the spawn semantic changes in RFC-032) lack
   automation

### Key constraints

- **The 17-keyword iron rule**: no new keywords or syntactic constructs
- **Zero compiler changes**: no touching the parser, IR, bytecode, or executor
- **Bootstrapping first**: the test library is written in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI Layer:  yaoxiang test [--filter --fail-fast --json ...] │
│              │                                               │
│  Discovery:  Read yaoxiang.toml → [tool.test] patterns       │
│              Default: tests/**/*.yx                          │
│              │                                               │
│  Execution:  Per file: yaoxiang run <file>                    │
│              Check exit code → serial execution              │
│              │                                               │
│  Reporting:  PASS/FAIL → Summary                              │
│              Supports --json / --verbose / --fail-fast        │
│                                                              │
│  Assertion:  std.test (pure YaoXiang, self-hosted)           │
│              Underlying: std.assert.assert                   │
│              Diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core principles

1. **The testing framework is a CLI tool, not a compiler feature** — `yaoxiang run` already
   "executes tests"; `yaoxiang test` simply runs all files and shows the report
2. **Zero compiler changes** — no `@test` annotation scanning, bytecode metadata segments, or
   special executor entry points
3. **Self-hosted** — the `std.test` module is implemented in pure YaoXiang, calling
   `std.assert.assert` underneath
4. **Test files are ordinary `.yx` files** — pass/fail is determined via exit codes

## Detailed design

### 1. CLI design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Only run tests whose filename contains <NAME>
  --fail-fast         Stop at the first failure
  --verbose, -v       Show verbose stdout/stderr per test
  --list              Only list test files; do not run
  --no-progress       Suppress progress bar (CI scenario)
  --json              Output results in JSON format (for CI integration)
```

#### Output format

**Default output**:

```
Running 5 tests from 3 files...

tests/math_test.yx ........................ PASS (0.002s)
tests/list_test.yx ........................ PASS (0.001s)
tests/string_test.yx ...................... FAIL (0.003s)
  `-- Expected "hello", got "world"
      at tests/string_test.yx:12:5

Results: 2 passed, 1 failed, 0 skipped (0.006s)
```

**JSON output** (`--json`):

```json
{
  "summary": { "total": 3, "passed": 2, "failed": 1, "skipped": 0, "time_secs": 0.006 },
  "tests": [
    { "file": "tests/math_test.yx", "passed": true, "time_secs": 0.002 },
    {
      "file": "tests/string_test.yx",
      "passed": false,
      "time_secs": 0.003,
      "error": "Expected \"hello\", got \"world\"",
      "exit_code": 1
    }
  ]
}
```

### 2. yaoxiang.toml configuration

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

- Default `patterns = ["tests/**/*.yx"]` — zero-config out of the box
- Single-file mode (`yaoxiang test foo.yx`) runs directly without reading the config
- May be split into a separate repository in the future (location under `[tool.test]` unchanged)

### 3. std.test module (pure YaoXiang)

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library
// First dogfooding library: YaoXiang's test library written in YaoXiang

use std.assert

assert_eq = (a, b) => {
    assert.assert(a == b, f"Expected {b}, got {a}")
}

assert_ne = (a, b) => {
    assert.assert(a != b, f"Expected not equal to {b}, got {a}")
}

assert_true = (cond: Bool) => {
    assert.assert(cond, f"Expected true, got {cond}")
}

assert_false = (cond: Bool) => {
    assert.assert(cond == false, f"Expected false, got {cond}")
}
```

- Four assertion functions, all using `f"..."` for diagnostic messages
- `assert_eq` / `assert_ne` use **unannotated parameters** (`Any`) — empirically verified on
  2026-08-02: `==`/`!=` and f-string interpolation work correctly on Any (Int/String both
  validated), **no dependency on the generics system**. Annotations can be added later when generics
  are ready
- `assert_true` / `assert_false` annotate parameters as `Bool`; `assert_false` expresses negation
  with `cond == false` (the `not` unary syntax is being finalized via #251, and can be migrated once
  stable)
- `std.test` depends on no native code; it is implemented in pure YaoXiang

### 4. Standard library loading mechanism (key design)

**Phase 1: Embedded in the binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) is embedded in the
binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source text
    // more in the future
];
```

The module system (RFC-029, fully landed 2026-08-02) provides the integration point: the Registry
holds both native modules and source modules, and the orchestrator handles multi-file orchestration.
The resolution order for `use std.test`:

1. First check Rust native modules (existing mechanism, e.g., `std.assert`)
2. If miss, check embedded `STD_YX_FILES` — on hit, inject the module into the orchestrator under a
   **virtual path** (e.g., `<std>/test.yx`) as a seed module, going through the normal frontend
   pipeline (parse → typecheck → IR)
3. If miss, fall through to filesystem discovery (user modules)

The `use std.assert` inside an embedded source module is resolved by the resolver to the native
registry normally — native and source modules coexist in the Registry, so cross-kind dependencies
work naturally. Embedded modules are **compiled on demand**: they enter the pipeline only when
imported.

Advantages:

- `use std.test` works in single-file mode
- The standard library version is strictly bound to the binary — no version mismatch
- No user configuration of std lib path is required

**Future: Filesystem standard library**

When the YaoXiang project mode matures, the standard library will move to filesystem form. See
updates in RFC-014 for details.

### 5. Discovery and execution

**Prerequisite (2026-08-02 review decision)**: The CLI `run` is hooked into the orchestrator. The
current CLI `run` goes through the single-file pipeline (`run_file_with_diagnostics`) and cannot
resolve user module imports; the subprocess model of `yaoxiang test` inherits CLI capabilities, and
test files importing project modules is a core scenario. Therefore, Phase 1 first delegates the CLI
`Run` source branch to `run_project` (orchestrator, directory-recursive discovery); #247 (on-demand
discovery along use) becomes a pure performance optimization layered on top later. The orchestrator
behavior is equivalent for single files without imports, so the bytecode branch is unchanged.

**Discovery phase**:

1. If `[PATHS]` is specified, use the given paths directly
2. Otherwise read `patterns` under `[tool.test]` from `yaoxiang.toml`
3. If no config exists, default to `tests/**/*.yx`
4. Apply `--filter` (filename substring)

**Execution phase**:

1. Per file: launch a subprocess with `yaoxiang run --debug-info <file>` (`--debug-info` makes
   runtime errors carry source locations — empirically verified 2026-08-02, stack trace outputs
   `file:line:col`)
2. Check the exit code: 0 is PASS, non-zero is FAIL
3. Capture stdout/stderr for reporting
4. Serial execution only (Phase 1); `--parallel` in the future
5. If `--fail-fast`, stop immediately at the first FAIL

### 6. Test isolation

Test isolation is naturally provided by process-level boundaries:

- Each test file runs in an independent subprocess
- Each subprocess has its own Heap, Frame, and NativeContext
- A panic in one test file does not affect others
- No additional isolated Heap context mechanism is required

## Relationship with existing systems

| Item                                                 | Relationship                                                                                                        |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched; internal compiler tests continue to use Rust                                                             |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                          |
| `std.assert.assert(cond)`                            | Retained; `std.test` depends on it                                                                                  |
| Module system (RFC-029)                              | Embedded source modules integrated via Registry/orchestrator; CLI `run` hooking into orchestrator is a prerequisite |
| `#200` refactor (`io.println` → `assert.assert`)     | Same direction as `yaoxiang test`                                                                                   |
| `@` annotations                                      | Not used; no `@test` introduced                                                                                     |

## Implementation strategy

### Phase 1: Core functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — delegate CLI `Run` source branch to `run_project`
  (prerequisite for multi-file execution)
- `src/main.rs` — add `Test` subcommand
- `src/std/test.yx` — add pure YaoXiang module
- `build.rs` — embed `std/*.yx` into the binary
- orchestrator / Registry — support loading `.yx` modules from embedded source via virtual paths
- RFC-015 config parsing — `[tool.test]` section
- Subprocess execution (`--debug-info`) + reporting

Deliverables:

- `yaoxiang test` functional
- `std.test` with four assertion functions
- Default `tests/**/*.yx` discovery
- Serial execution + default output format

### Phase 2: Polish

- `--filter` / `--fail-fast` / `--verbose` options
- `--json` output (CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: Advanced

- `--parallel` execution (depends on spawn concurrency model maturity)
- `[tool.test].exclude` config
- More assertion functions (e.g., `assert_approx_eq` for Float)

## Risks and mitigations

| Risk                                                    | Probability | Mitigation                                                                       |
| ------------------------------------------------------- | ----------- | -------------------------------------------------------------------------------- |
| `f"..."` interpolation on Any fails                     | None        | Empirically verified 2026-08-02 (Int/String both work)                           |
| Subprocess startup overhead slows tests                 | Medium      | Phase 1 serial execution is acceptable; Phase 3 parallelism mitigates            |
| `yaoxiang.toml` config parsing not in current CLI       | Low         | Simple extension; does not affect core functionality                             |
| CLI run hooking into orchestrator introduces regression | Low         | Import-free single-file path is equivalent; integration tests cover orchestrator |
| Embedding `.yx` sources into binary increases size      | Low         | `.yx` sources are tiny; negligible                                               |

## Open questions

- [x] Can `use std.assert` inside `std/test.yx` be resolved correctly? — **Resolved (2026-08-02)**.
      After the module system (RFC-029) landed, native and source modules coexist in the Registry;
      the resolver handles both uniformly, so cross-kind dependencies work naturally
- [x] Does the generic `to_string` of `f"..."` in test output introduce new type constraints? —
      **Resolved (2026-08-02)**. Empirically verified that `==`/`!=` and f-string interpolation work
      on unannotated parameters (Any) (Int/String validated); no new constraints introduced
- [x] Feasibility of `?` generic parameters? — **Resolved (2026-08-02)**: the `?` type syntax does
      not currently exist (and would be silently swallowed — tracked in a separate issue); Phase 1
      assertion functions use unannotated parameters, no dependency on the generics system

## Design decisions

| Decision                   | Decision                                                                                                                           | Date       | Rationale                                                                                    |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ---------- | -------------------------------------------------------------------------------------------- |
| Test marking               | No `@test` annotation; test files are ordinary `.yx` files                                                                         | 2026-07-26 | Zero compiler changes; subprocess provides isolation                                         |
| Assertion mechanism        | Pure YaoXiang functions in `std.test` module                                                                                       | 2026-07-26 | Self-hosted; no native code                                                                  |
| Test execution model       | Subprocess `yaoxiang run <file>` + exit code                                                                                       | 2026-07-26 | Process-level isolation; zero compiler changes                                               |
| Standard library loading   | Currently embedded in binary; filesystem in the future                                                                             | 2026-07-26 | Version binding; works in single-file mode                                                   |
| Assertion parameter types  | Unannotated parameters (Any); no dependency on the generics system                                                                 | 2026-08-02 | `?` type syntax does not exist; Any empirically supports comparison and interpolation        |
| Multi-file execution       | CLI `run` delegates to `run_project` (orchestrator) as a prerequisite                                                              | 2026-08-02 | Subprocess model inherits CLI capabilities; #247 degrades to a pure performance optimization |
| Source location in reports | Subprocess with `--debug-info`                                                                                                     | 2026-08-02 | Empirically verified stack trace outputs `file:line:col`                                     |
| Negative test layering     | Value-level inverse (general) / compile-failure runner structured markers (internal only) / hard failures consolidated into Result | 2026-09-02 | Finalized by #319; supersedes the implicit [test:error] convention                           |
| Multiple tests per file    | Value-based standard model: test functions return Result, suite collects per-test verdicts                                         | 2026-09-02 | No catch, not entry-call (entry calls limited to internal scenarios)                         |
| Error codes                | Error adds machine-readable `code` field                                                                                           | 2026-09-02 | Supports error code assertions; compile-time codes go through runner comparison              |

## Revision: Test model finalized (2026-09-02, #319)

The following four points were finalized in discussion with the maintainers. Where the main text
above conflicts with this section, this section takes precedence.

### R1. Negative test layering (supersedes the implicit [test:error] convention)

Negative tests ("expected to fail") are split by the layer at which the failure occurs, with each
layer assigned as follows:

1. **Value-level inverse (general, user-facing)**: the operation under test returns `Result`, and
   tests express the expected failure with ordinary assertions (`assert(r.is_err())`, error code
   assertions). As the Result-ification progresses (#301, #316), `[test:error]` files in the corpus
   will gradually migrate to in-file assertions, and the file-header marker will disappear. std.test
   adds `assert_not` / `assert_err` function families first (the `!assert` unary form will be
   provided after the `not` syntax lands; the `!` prefix syntax does not currently exist — see the
   same constraint in §3 where `assert_false` uses `cond == false`).
2. **Compile failures (only for language designers, internal use)**: compilation is all-or-nothing
   per file, so "this line should not compile" cannot be expressed within the file. Keep the
   file-level special marker (`[test:error]`), read by the runner for inverse judgment; this serves
   only the corpus in this repository and **is not part of the user-facing test framework**. The
   marker is upgraded to a structured expected code: the runner parses a header
   `expected: compile error EXXXX` and compares it with the compiler's actual stderr output
   `[EXXXX]`; a code mismatch = FAIL (a lightweight version of trybuild's stderr snapshot idea, with
   zero compiler changes).
3. **Runtime hard failures (consolidated into Result-ification, no standalone mechanism)**:
   operations that can fail return `Result` per the language direction (#301, #316); tests uniformly
   go through layer 1.

### R2. Multiple tests per file: the value-based standard model

A test file may contain multiple tests. The standard model is **value-based**: test functions return
`Result`; assertion failures are expressed as `Err` values rather than aborting the process; the
suite collects each test's `Result` one by one and aggregates per-test verdicts into the report; any
non-`Ok` makes the file's exit code non-zero. The `std.test` assertion family provides corresponding
**value-semantic** forms (returning `Err` diagnostic messages, no abort); the process-level abort
semantics of `std.assert.assert` are retained for runtime guards, not for test assertions. No catch
boundary is adopted (the 17-keyword iron rule), and the runner does not call each function as an
entry point (limited to internal scenarios like compile failures, see R1.2).

### R3. Error values gain a code field

To support error code assertions, `Error` is extended from `Struct { message }` to include a
machine-readable `code` (native `error_new_with_code`, with std exporting code constants), so that
`assert(err.code == "E3017")` works. Compile-time error code checks do not go through this path —
the compile-failure process has not run, so it is handled by the runner comparison in R1.2 (the
compiler's stderr already contains `[EXXXX]`).

### R4. Reporting and performance corrections

- The "Running 5 tests from 3 files" example in the output is a relic of the old model: under the
  file-level model, one file = one test process; after R2 lands, per-test verdicts come from the
  suite-internal collection, not from the runner scanning test functions.
- The main performance cost is the per-file full compilation (empirically 11.3s for 185 files), not
  subprocess startup; Phase 3 `--parallel` does not address the compilation cost — see the test loop
  caching slices in #251 / #293.
- Assertion failure locations pass through embedded modules (empirically
  `at std.test.assert_eq (ip: 9)`, the user cannot get their own file line number) and is not within
  the scope of this RFC — stack frame attribution is the experience issue #289 plus the design scope
  of RFC-034 debug metadata (`get_frames`).

## References

- [RFC-014: Package management system design](../accepted/014-package-manager.md) — standard library
  directory structure
- [RFC-015: Configuration system](../accepted/015-configuration-system.md) — `[tool.test]` config
  section
- [RFC-030: assert mechanism](../review/030-assert-mechanism.md) — underlying dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — reference
  design
