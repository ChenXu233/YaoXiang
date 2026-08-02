---
title: 'RFC-036: std.test Test Framework and yaoxiang test Command'
status: 'Accepted'
author: 'Chenxu'
created: '2026-07-26'
updated: '2026-08-02'
accepted: '2026-08-02'
issue: '#94, #95, #221'
---

# RFC-036: std.test Test Framework and yaoxiang test Command

## Summary

Introduce a standard test framework `std.test` module and `yaoxiang test` CLI subcommand for
YaoXiang. Test files are ordinary `.yx` files that use `std.assert.assert` + exit code to determine
pass/fail. The `std.test` module is implemented in pure YaoXiang and is the first dogfooding
library. `yaoxiang test` is a CLI tool, not a compiler feature — it does not involve any changes to
the parser, IR, bytecode, or executor.

## Motivation

### Why do we need a test framework?

Currently, YaoXiang's test coverage relies on Rust-side `#[test]` and `tests/` integration tests.
This means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io)
   cannot be written in YaoXiang
2. `#117 Unit test coverage for standard library modules` is blocked because there is no usable test
   infrastructure
3. Regression tests for language features (such as the RFC-032 spawn semantics change) lack
   automated means

### Key Constraints

- **17-keyword iron rule**: No new keywords or syntactic constructs introduced
- **Zero compiler changes**: No touching of parser, IR, bytecode, or executor
- **Self-hosting first**: The test library is written in YaoXiang — the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI layer:  yaoxiang test [--filter --fail-fast --json ...]  │
│              │                                               │
│  Discovery:  Read yaoxiang.toml → [tool.test] patterns       │
│              Default: tests/**/*.yx                          │
│              │                                               │
│  Execution:  For each file: yaoxiang run <file>              │
│              Check exit code → serial execution              │
│              │                                               │
│  Reporting:  PASS/FAIL → summary                             │
│              Support --json / --verbose / --fail-fast        │
│                                                              │
│  Assertion:  std.test (pure YaoXiang, self-hosted)           │
│              Low-level: std.assert.assert                     │
│              Diagnostic: f"Expected {expected}, got {actual}" │
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **The test framework is not a compiler feature, it's a CLI tool** — `yaoxiang run` can already
   "execute tests"; `yaoxiang test` just helps you run all files and gives you a report
2. **Zero compiler changes** — No `@test` annotation scanning, bytecode metadata segments, or
   special executor entry points
3. **Self-hosting** — The `std.test` module is implemented in pure YaoXiang, calling
   `std.assert.assert` underneath
4. **Test files are ordinary `.yx` files** — Pass/fail is determined by exit code

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
  --no-progress       Do not show progress bar (CI scenario)
  --json              Output results in JSON format (for CI integration)
```

#### Output Format

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

- Default `patterns = ["tests/**/*.yx"]` — zero-config out-of-the-box
- Single-file mode (`yaoxiang test foo.yx`) runs directly, does not read config
- May be split into an independent repository in the future (the `[tool.test]` location remains
  unchanged)

### 3. std.test Module (Pure YaoXiang)

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
    assert.assert(!cond, f"Expected false, got {cond}")
}
```

- 4 assertion functions, all using `f"..."` for diagnostic messages
- `assert_eq` / `assert_ne` use **unannotated parameters** (`Any`) — verified on 2026-08-02:
  `==`/`!=` and f-string interpolation work correctly on Any (Int/String both verified), **does not
  depend on the generics system**. Annotations can be added once generics are ready in the future
- `std.test` does not depend on any native code; it is implemented in pure YaoXiang

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedded in Binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) is embedded in the
binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source code text
    // more in the future
];
```

The module system (RFC-029, fully landed on 2026-08-02) provides the entry point: the Registry holds
both native modules and source modules, and the orchestrator handles multi-file orchestration. The
resolution order for `use std.test`:

1. First check Rust native modules (existing mechanism, e.g. `std.assert`)
2. If not found, check embedded `STD_YX_FILES` — if matched, inject the seed module into the
   orchestrator with a **virtual path** (e.g. `<std>/test.yx`), going through the normal frontend
   pipeline (parse → typecheck → IR)
3. If not found, fall back to file system discovery (user modules)

`use std.assert` inside an embedded source module is resolved normally by the resolver to the native
registry — native and source modules coexist in the Registry, and cross-kind dependencies naturally
work. Embedded modules are **compiled on demand**: they enter the pipeline only when imported.

Advantages:

- `use std.test` works in single-file mode
- Standard library version is strictly bound to the binary, no version mismatch
- No need for users to configure standard library paths

**Future: File-system Standard Library**

Once YaoXiang's project mode matures, the standard library will be moved to a file-system form. See
RFC-014 updates for details.

### 5. Discovery and Execution

**Prerequisite (2026-08-02 review resolution)**: The CLI `run` connects to the orchestrator.
Currently the CLI `run` goes through the single-file pipeline (`run_file_with_diagnostics`), which
cannot resolve user module imports; meanwhile, the subprocess model of `yaoxiang test` inherits CLI
capabilities, and test files importing project modules is a core scenario. Therefore, Phase 1 first
delegates the CLI `Run` source branch to `run_project` (orchestrator, directory-recursive
discovery); #247 (on-demand discovery along `use`) becomes a pure performance optimization layered
on top. For single files without imports, the orchestrator behavior is equivalent, and the bytecode
branch is unchanged.

**Discovery Phase**:

1. If `[PATHS]` is specified, use the specified paths directly
2. Otherwise, read `[tool.test].patterns` from `yaoxiang.toml`
3. If no configuration exists, default to `tests/**/*.yx`
4. Apply `--filter` (filename contains)

**Execution Phase**:

1. For each file: launch a subprocess with `yaoxiang run --debug-info <file>` (`--debug-info` makes
   runtime errors carry source locations — verified on 2026-08-02 that stack traces output
   `file:line:col`)
2. Check exit code: 0 is PASS, non-0 is FAIL
3. Capture stdout/stderr for reporting
4. Serial execution only (Phase 1); `--parallel` will be supported in the future
5. If `--fail-fast` is set, stop immediately on the first FAIL

### 6. Test Isolation

Test isolation is naturally achieved through process-level boundaries:

- Each test file runs in an independent subprocess
- Each subprocess has its own Heap, Frame, and NativeContext
- A panic in one test file will not affect other test files
- No additional isolated Heap context mechanism is needed

## Relationship with Existing Systems

| Item                                                 | Relationship                                                                                                      |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched; compiler-internal tests continue using Rust                                                            |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                        |
| `std.assert.assert(cond)`                            | Retained; `std.test` depends on it underneath                                                                     |
| Module system (RFC-029)                              | Embedded source modules connect via Registry/orchestrator; CLI `run` connecting to orchestrator is a prerequisite |
| `#200` refactor (`io.println` → `assert.assert`)     | Same direction as `yaoxiang test`                                                                                 |
| `@` annotation                                       | Not used; `@test` is not introduced                                                                               |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project`
  (prerequisite for multi-file execution)
- `src/main.rs` — New `Test` subcommand
- `src/std/test.yx` — New pure YaoXiang module
- `build.rs` — Embed `std/*.yx` into the binary
- orchestrator / Registry — Support loading `.yx` modules from embedded sources via virtual paths
- RFC-015 config parsing — `[tool.test]` section
- Subprocess execution (`--debug-info`) + reporting

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

### Phase 3: Advanced

- `--parallel` parallel execution (depends on spawn concurrency model improvements)
- `[tool.test].exclude` configuration
- More assertion functions (e.g. `assert_approx_eq` for Float)

## Risks and Mitigations

| Risk                                                                 | Probability | Mitigation                                                                          |
| -------------------------------------------------------------------- | ----------- | ----------------------------------------------------------------------------------- |
| `f"..."` interpolation on Any fails                                  | None        | Verified on 2026-08-02 (Int/String both work)                                       |
| Subprocess startup overhead impacts test speed                       | Medium      | Phase 1 serial execution, acceptable; Phase 3 parallel mitigates                    |
| `yaoxiang.toml` config parsing not in current CLI                    | Low         | Simple extension, does not affect core functionality                                |
| CLI `run` connecting to orchestrator introduces behavior regressions | Low         | No-import single-file path equivalent; integration tests already cover orchestrator |
| Embedding `.yx` source files into binary increases size              | Low         | `.yx` source files are extremely small, negligible                                  |

## Open Questions

- [x] Can `use std.assert` references in `std/test.yx` resolve correctly? — **Resolved
      (2026-08-02)**. After the module system (RFC-029) landed, native and source modules coexist in
      the Registry, the resolver handles them uniformly, and cross-kind dependencies naturally work
- [x] Does the generic `to_string` for `f"..."` in test output introduce new type constraints? —
      **Resolved (2026-08-02)**. Verified that on unannotated parameters (Any), `==`/`!=` and
      f-string interpolation both work (Int/String verified), no new constraints introduced
- [x] Feasibility of `?` generic parameter? — **Resolved (2026-08-02)**: `?` type syntax does not
      currently exist (and would be silently swallowed, tracked in a separate issue); Phase 1
      assertion functions use unannotated parameters, not depending on the generics system

## Design Decision Log

| Decision                  | Resolution                                                            | Date       | Rationale                                                                                  |
| ------------------------- | --------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------ |
| Test marker approach      | No `@test` annotation; test files are ordinary `.yx`                  | 2026-07-26 | Zero compiler changes, subprocess is isolation                                             |
| Assertion approach        | `std.test` module as pure YaoXiang functions                          | 2026-07-26 | Self-hosting, no native code                                                               |
| Test execution model      | Subprocess `yaoxiang run <file>` + exit code                          | 2026-07-26 | Process-level isolation, zero compiler changes                                             |
| Standard library loading  | Currently embedded in binary, file-system in the future               | 2026-07-26 | Version binding, works in single-file mode                                                 |
| Assertion parameter types | Unannotated parameters (Any), not depending on generics               | 2026-08-02 | `?` type syntax does not exist; Any verified to be comparable and interpolatable           |
| Multi-file execution      | CLI `run` delegates to `run_project` (orchestrator) as a prerequisite | 2026-08-02 | Subprocess model inherits CLI capabilities; #247 degrades to pure performance optimization |
| Reporting source location | Subprocess carries `--debug-info`                                     | 2026-08-02 | Verified stack trace outputs `file:line:col`                                               |

## References

- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — Standard library
  directory structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]`
  configuration section
- [RFC-030: assert Mechanism](../review/030-assert-mechanism.md) — Low-level dependency
- [Rust `#[test]` Mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — Reference
  design
