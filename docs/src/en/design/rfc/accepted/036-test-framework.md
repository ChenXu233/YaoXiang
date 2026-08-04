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

Introduce a standard test framework `std.test` module and the `yaoxiang test` CLI subcommand for
YaoXiang. Test files are plain `.yx` files, and pass/fail is determined via `std.assert.assert` plus
exit code. The `std.test` module is implemented in pure YaoXiang and is the first dogfooding
library. `yaoxiang test` is a CLI tool, not a compiler feature—it involves no changes to the parser,
IR, bytecode, or executor.

## Motivation

### Why a test framework?

Current test coverage in YaoXiang relies on Rust-side `#[test]` and `tests/` integration tests. This
means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io)
   cannot be written in YaoXiang
2. `#117 Unit test coverage for each standard library module` is blocked because there is no usable
   test infrastructure
3. Regression tests for language features (e.g., the RFC-032 spawn semantic change) lack automation

### Key constraints

- **17-keyword iron rule**: no new keywords or syntactic constructs
- **Zero compiler changes**: do not touch the parser, IR, bytecode, or executor
- **Bootstrap-first**: the test library is written in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI layer:   yaoxiang test [--filter --fail-fast --json ...]│
│                  │                                           │
│  Discovery:   read yaoxiang.toml → [tool.test] patterns      │
│                  default: tests/**/*.yx                      │
│                  │                                           │
│  Execution:   for each file: yaoxiang run <file>             │
│                  check exit code → serial execution          │
│                  │                                           │
│  Report:      PASS/FAIL → summary                           │
│                  supports --json / --verbose / --fail-fast   │
│                                                              │
│  Assertion:   std.test (pure YaoXiang, bootstrap)            │
│                  base: std.assert.assert                     │
│                  diagnostic: f"Expected {expected}, got ..."  │
└──────────────────────────────────────────────────────────────┘
```

### Core principles

1. **The test framework is not a compiler feature; it is a CLI tool** — `yaoxiang run` can already
   "execute tests"; `yaoxiang test` just helps you run all the files and gives you a report
2. **Zero compiler changes** — no `@test` annotation scanning, no bytecode metadata segments, no
   executor special entry points
3. **Bootstrap** — the `std.test` module is implemented in pure YaoXiang, with `std.assert.assert`
   as the base
4. **Test files are plain `.yx` files** — pass/fail is determined via exit code

## Detailed design

### 1. CLI design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Only run tests whose filename contains <NAME>
  --fail-fast         Stop on the first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              Only list test files, do not run
  --no-progress       Do not show progress bar (CI scenario)
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

- Default `patterns = ["tests/**/*.yx"]` — zero-config, works out of the box
- Single-file mode (`yaoxiang test foo.yx`) runs directly, does not read config
- May be split into a separate repository in the future (`[tool.test]` position remains unchanged)

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

- 4 assertion functions, all using `f"..."` for diagnostic messages
- `assert_eq` / `assert_ne` use **unannotated parameters** (`Any`) — empirically verified on
  2026-08-02: `==`/`!=` and f-string interpolation work correctly on Any (verified for both Int and
  String), **not dependent on the generics system**. Annotations can be added later when generics
  are ready
- `assert_true` / `assert_false` parameters annotated as `Bool`; `assert_false` uses `cond == false`
  to express negation (the `not` unary syntax is being formalized via #251 and can be migrated once
  stable)
- `std.test` depends on no native code and is implemented in pure YaoXiang

### 4. Standard library loading mechanism (key design)

**Phase 1: Embedded in the binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) are embedded into the
binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source code text
    // more in the future
];
```

The module system (RFC-029, fully landed on 2026-08-02) provides the integration point: the Registry
holds both native modules and source modules, and the orchestrator handles multi-file coordination.
The resolution order for `use std.test`:

1. First, look up Rust native modules (the existing mechanism, e.g., `std.assert`)
2. If not found, look up the embedded `STD_YX_FILES`—if hit, inject as a **virtual path** (e.g.,
   `<std>/test.yx`) seed module into the orchestrator, going through the normal frontend pipeline
   (parse → typecheck → IR)
3. If still not found, fall through to file system discovery (user modules)

A `use std.assert` inside an embedded source module is resolved normally by the resolver to the
native registry—native and source modules coexist in the Registry, so cross-kind dependencies work
naturally. Embedded modules are **compiled on demand**: they only enter the pipeline when imported.

Advantages:

- `use std.test` works even in single-file mode
- Standard library version is strictly bound to the binary, with no version mismatch
- Users do not need to configure standard library paths

**Future: File system standard library**

When the YaoXiang project model matures, the standard library will be moved to a file system form.
See RFC-014's update for details.

### 5. Discovery and execution

**Prerequisite (2026-08-02 review decision)**: the CLI `run` integrates with the orchestrator.
Currently, the CLI `run` uses the single-file pipeline (`run_file_with_diagnostics`), which cannot
resolve user module imports; the `yaoxiang test` subprocess model inherits CLI capabilities, and
test files importing project modules is a core scenario. Therefore, Phase 1 first delegates the
source branch of CLI `Run` to `run_project` (orchestrator, with directory-recursive discovery); #247
(on-demand discovery along `use`) becomes a pure performance optimization later. Single files
without imports behave equivalently through the orchestrator, and the bytecode branch is unchanged.

**Discovery phase**:

1. If `[PATHS]` is specified, use the specified paths directly
2. Otherwise, read `[tool.test].patterns` from `yaoxiang.toml`
3. If no config, default to `tests/**/*.yx`
4. Apply the `--filter` (filename contains)

**Execution phase**:

1. For each file: spawn a subprocess with `yaoxiang run --debug-info <file>` (`--debug-info` makes
   runtime errors carry source locations—empirically verified on 2026-08-02 that the stack trace
   outputs `file:line:col`)
2. Check the exit code: 0 is PASS, non-zero is FAIL
3. Capture stdout/stderr for the report
4. Serial execution only (Phase 1), with `--parallel` support planned
5. If `--fail-fast`, stop immediately on the first FAIL

### 6. Test isolation

Test isolation is achieved naturally through process-level boundaries:

- Each test file runs in an independent subprocess
- Each subprocess has an independent Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional isolated Heap context mechanism is needed

## Relationship with existing systems

| Item                                                 | Relationship                                                                                                               |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched; compiler-internal tests continue to use Rust                                                                    |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                                 |
| `std.assert.assert(cond)`                            | Retained; `std.test` depends on it as its base                                                                             |
| Module system (RFC-029)                              | Embedded source modules connect via Registry/orchestrator; CLI `run` integrating with the orchestrator is the prerequisite |
| `#200` refactor (`io.println` → `assert.assert`)     | Same direction as `yaoxiang test`                                                                                          |
| `@` annotations                                      | Not used; no `@test` introduced                                                                                            |

## Implementation strategy

### Phase 1: Core functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — the CLI `Run` source branch delegates to
  `run_project` (multi-file run prerequisite)
- `src/main.rs` — add the `Test` subcommand
- `src/std/test.yx` — new pure YaoXiang module
- `build.rs` — embed `std/*.yx` into the binary
- orchestrator / Registry — support loading `.yx` modules from embedded sources via virtual path
- RFC-015 config parsing — the `[tool.test]` section
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

- `--parallel` parallel execution (depends on the spawn concurrency model being complete)
- `[tool.test].exclude` config
- More assertion functions (e.g., `assert_approx_eq` for Float)

## Risks and mitigations

| Risk                                                              | Probability | Mitigation                                                                              |
| ----------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------- |
| `f"..."` interpolation failure on Any                             | None        | Empirically verified 2026-08-02 (Int/String both work)                                  |
| Subprocess startup overhead affects test speed                    | Medium      | Phase 1 serial execution, acceptable; Phase 3 parallel mitigation                       |
| `yaoxiang.toml` config parsing not in the current CLI             | Low         | Simple extension; does not affect core functionality                                    |
| CLI `run` integrating with the orchestrator introduces regression | Low         | No-import single-file path equivalent; integration tests already cover the orchestrator |
| Embedding `.yx` source files into the binary increases size       | Low         | `.yx` source files are tiny; negligible                                                 |

## Open questions

- [x] Can the `use std.assert` reference in `std/test.yx` resolve correctly? — **Resolved
      (2026-08-02)**. After the module system (RFC-029) landed, native and source modules coexist in
      the Registry, and the resolver handles them uniformly, so cross-kind dependencies work
      naturally
- [x] Does the generic `to_string` for `f"..."` in test output introduce new type constraints? —
      **Resolved (2026-08-02)**. Empirically, `==`/`!=` and f-string interpolation work on
      unannotated parameters (Any) (verified for Int and String), introducing no new constraints
- [x] Feasibility of the `?` generic parameter? — **Resolved (2026-08-02)**: the `?` type syntax
      does not currently exist (and would be silently swallowed, with a separate issue tracking it);
      Phase 1 assertion functions use unannotated parameters and do not depend on the generics
      system

## Design decision log

| Decision                  | Determination                                                         | Date       | Reason                                                                                            |
| ------------------------- | --------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------- |
| Test marker method        | No `@test` annotation; test files are plain `.yx`                     | 2026-07-26 | Zero compiler changes; subprocess equals isolation                                                |
| Assertion method          | `std.test` module as pure YaoXiang functions                          | 2026-07-26 | Bootstrap; no native code                                                                         |
| Test execution model      | Subprocess `yaoxiang run <file>` + exit code                          | 2026-07-26 | Process-level isolation; zero compiler changes                                                    |
| Standard library loading  | Currently embedded in the binary, file system in the future           | 2026-07-26 | Version binding; works in single-file mode                                                        |
| Assertion parameter types | Unannotated parameters (Any), not dependent on the generics system    | 2026-08-02 | `?` type syntax does not exist; empirically Any is comparable and interpolatable                  |
| Multi-file run            | CLI `run` delegates to `run_project` (orchestrator) as a prerequisite | 2026-08-02 | Subprocess model inherits CLI capabilities; #247 degenerates into a pure performance optimization |
| Report source location    | Subprocess with `--debug-info`                                        | 2026-08-02 | Empirically the stack trace outputs `file:line:col`                                               |

## References

- [RFC-014: Package management system design](../accepted/014-package-manager.md) — Standard library
  directory structure
- [RFC-015: Configuration system](../accepted/015-configuration-system.md) — `[tool.test]` config
  section
- [RFC-030: assert mechanism](../review/030-assert-mechanism.md) — Base dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — Reference
  design
