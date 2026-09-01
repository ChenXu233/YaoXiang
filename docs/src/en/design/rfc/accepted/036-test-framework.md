---
title: 'RFC-036: std.test Testing Framework and yaoxiang test Command'
status: 'Accepted'
author: 'Chenxu'
created: '2026-07-26'
updated: '2026-08-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test Testing Framework and yaoxiang test Command

## Summary

Introduce the standard testing framework `std.test` module and the `yaoxiang test` CLI subcommand
for YaoXiang. Test files are ordinary `.yx` files, with pass/fail determined by
`std.assert.assert` + exit code. The `std.test` module is implemented in pure YaoXiang, serving as
the first dogfooding library. `yaoxiang test` is a CLI tool, not a compiler feature—it does not
involve any changes to the parser, IR, bytecode, or executor.

## Motivation

### Why a testing framework?

Currently, YaoXiang's test coverage relies on Rust-side `#[test]` and `tests/` integration tests.
This means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io)
   cannot be written in YaoXiang
2. `#117 Standard library unit test coverage per module` is blocked because no test infrastructure
   is available
3. Regression tests for language features (e.g., RFC-032 spawn semantics changes) lack automation

### Key Constraints

- **17-keyword iron rule**: no new keywords or syntactic constructs introduced
- **Zero compiler changes**: do not touch the parser, IR, bytecode, or executor
- **Bootstrap first**: the test library is written in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI Layer:  yaoxiang test [--filter --fail-fast --json ...]  │
│              │                                               │
│  Discovery:  read yaoxiang.toml → [tool.test] patterns       │
│              default: tests/**/*.yx                          │
│              │                                               │
│  Execution:  per file: yaoxiang run <file>                   │
│              check exit code → run serially                   │
│              │                                               │
│  Reporting:  PASS/FAIL → summary                             │
│              supports --json / --verbose / --fail-fast        │
│                                                              │
│  Assertion:  std.test (pure YaoXiang, bootstrapped)          │
│              underlying: std.assert.assert                    │
│              diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **The testing framework is not a compiler feature, it is a CLI tool** — `yaoxiang run` can
   already "execute tests"; `yaoxiang test` just helps you run all the files and show you the report
2. **Zero compiler changes** — no `@test` annotation scanning, bytecode metadata sections, or
   special executor entry points
3. **Bootstrapped** — the `std.test` module is implemented in pure YaoXiang, with
   `std.assert.assert` as its underlying call
4. **Test files are ordinary `.yx` files** — pass/fail is determined by exit code

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Only run tests whose file name contains <NAME>
  --fail-fast         Stop at the first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              Only list test files, do not run
  --no-progress       Do not show progress bar (CI scenarios)
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

- Default `patterns = ["tests/**/*.yx"]` — zero-config out-of-the-box for users
- Single-file mode (`yaoxiang test foo.yx`) runs directly without reading config
- May be split into a separate repository in the future (the `[tool.test]` location stays the same)

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
    assert.assert(cond == false, f"Expected false, got {cond}")
}
```

- 4 assertion functions, all using `f"..."` for diagnostic messages
- `assert_eq` / `assert_ne` use **unannotated parameters** (`Any`)—verified on 2026-08-02: `==`/`!=`
  and f-string interpolation work correctly on Any (verified for both Int and String), **no
  dependency on the generics system**. Annotations can be added later when generics are ready
- `assert_true` / `assert_false` parameters annotated as `Bool`; `assert_false` uses `cond == false`
  to express negation (the `not` unary syntax is being canonicalized per #251; once stable, it can
  be migrated)
- `std.test` does not depend on any native code, implemented in pure YaoXiang

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedded in binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) are embedded into the
binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source code text
    // more in the future
];
```

The module system (RFC-029, fully landed on 2026-08-02) provides the access point: the Registry
holds both native modules and source modules, with the orchestrator handling multi-file
orchestration. The resolution order for `use std.test`:

1. First check Rust native modules (existing mechanism, e.g., `std.assert`)
2. If not found, check the embedded `STD_YX_FILES`—if hit, inject as a **virtual path** (e.g.,
   `<std>/test.yx`) seed module into the orchestrator, going through the normal frontend pipeline
   (parse → typecheck → IR)
3. If not found, fall through to file system discovery (user modules)

`use std.assert` inside an embedded source module is resolved normally by the resolver to the native
registry—native and source modules coexist in the Registry, so cross-kind dependencies work
naturally. Embedded modules are **compiled on demand**: they only enter the pipeline when imported.

Advantages:

- `use std.test` works in single-file mode
- Standard library version is strictly bound to the binary, no version mismatch
- No need for users to configure standard library paths

**Future: File-system standard library**

When YaoXiang's project mode matures, the standard library will switch to file-system form. See
updates to RFC-014 for details.

### 5. Discovery and Execution

**Prerequisite (2026-08-02 review decision)**: CLI `run` is wired into the orchestrator. The current
CLI `run` uses a single-file pipeline (`run_file_with_diagnostics`) and cannot resolve user module
imports; but the subprocess model of `yaoxiang test` inherits CLI capabilities, and test files
importing project modules is a core scenario. Therefore Phase 1 first delegates the source branch of
CLI `Run` to `run_project` (orchestrator, recursive directory discovery); #247 (on-demand discovery
along `use`) is layered on later as a pure performance optimization. The single-file path without
imports behaves equivalently under the orchestrator; the bytecode branch is unchanged.

**Discovery phase**:

1. If `[PATHS]` is specified, use the specified paths directly
2. Otherwise, read `[tool.test].patterns` from `yaoxiang.toml`
3. If not configured, default to `tests/**/*.yx`
4. Apply `--filter` (file name contains)

**Execution phase**:

1. For each file: launch a subprocess with `yaoxiang run --debug-info <file>` (`--debug-info` makes
   runtime errors include source locations—verified on 2026-08-02 that stack trace output is
   `file:line:col`)
2. Check exit code: 0 is PASS, non-0 is FAIL
3. Capture stdout/stderr for reporting
4. Serial execution only (Phase 1); `--parallel` in the future
5. If `--fail-fast`, stop immediately at the first FAIL

### 6. Test Isolation

Test isolation is naturally achieved through process-level boundaries:

- Each test file runs in an independent subprocess
- Each subprocess has independent Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional independent Heap context mechanism is needed

## Relationship with Existing Systems

| Item                                                 | Relationship                                                                                                |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched; compiler internal tests continue to use Rust                                                     |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                  |
| `std.assert.assert(cond)`                            | Retained; `std.test` depends on it as the underlying call                                                   |
| Module system (RFC-029)                              | Embedded source modules access via Registry/orchestrator; CLI `run` wired to orchestrator is a prerequisite |
| `#200` refactor (`io.println` → `assert.assert`)     | Same direction as `yaoxiang test`                                                                           |
| `@` annotations                                      | Not used; no `@test` introduced                                                                             |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project`
  (multi-file run prerequisite)
- `src/main.rs` — add new `Test` subcommand
- `src/std/test.yx` — add new pure YaoXiang module
- `build.rs` — embed `std/*.yx` into binary
- orchestrator / Registry — support loading `.yx` modules from embedded sources via virtual paths
- RFC-015 config parsing — `[tool.test]` section
- Subprocess execution (`--debug-info`) + reporting

Deliverables:

- `yaoxiang test` basically functional
- `std.test` with 4 assertion functions
- Default `tests/**/*.yx` discovery
- Serial execution + default output format

### Phase 2: Polish

- `--filter` / `--fail-fast` / `--verbose` options
- `--json` output (CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: Advanced

- `--parallel` concurrent execution (depends on spawn concurrency model maturation)
- `[tool.test].exclude` configuration
- More assertion functions (e.g., `assert_approx_eq` for Float)

## Risks and Mitigations

| Risk                                                           | Probability | Mitigation                                                                  |
| -------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------- |
| `f"..."` interpolation on Any fails                            | None        | Verified on 2026-08-02 (Int/String both work normally)                      |
| Subprocess startup overhead affects test speed                 | Medium      | Phase 1 serial execution, acceptable; Phase 3 parallel mitigates            |
| `yaoxiang.toml` config parsing not in current CLI              | Low         | Simple extension, does not affect core functionality                        |
| CLI `run` wired to orchestrator introduces behavior regression | Low         | No-import single-file path equivalent; integration tests cover orchestrator |
| Embedding `.yx` source files into binary increases size        | Low         | `.yx` source files are very small, negligible                               |

## Open Questions

- [x] Can the `use std.assert` reference in `std/test.yx` resolve correctly? — **Resolved
      (2026-08-02)**. After the module system (RFC-029) landed, native and source modules coexist in
      the Registry, the resolver handles them uniformly, and cross-kind dependencies work naturally
- [x] Does `f"..."` generic `to_string` in test output introduce new type constraints? — **Resolved
      (2026-08-02)**. Verified that `==`/`!=` and f-string interpolation work on unannotated
      parameters (Any) (verified for Int/String), no new constraints introduced
- [x] Feasibility of `?` generic parameters? — **Resolved (2026-08-02)**: the `?` type syntax does
      not currently exist (and is silently swallowed, tracked in a separate issue); Phase 1
      assertion functions use unannotated parameters, no dependency on the generics system

## Design Decision Log

| Decision                  | Decision                                                            | Date       | Rationale                                                                                  |
| ------------------------- | ------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------ |
| Test marker mechanism     | No `@test` annotation; test files are ordinary `.yx`                | 2026-07-26 | Zero compiler changes, subprocess equals isolation                                         |
| Assertion mechanism       | `std.test` module as pure YaoXiang functions                        | 2026-07-26 | Bootstrapped, no native code                                                               |
| Test execution model      | Subprocess `yaoxiang run <file>` + exit code                        | 2026-07-26 | Process-level isolation, zero compiler changes                                             |
| Standard library loading  | Currently embedded in binary, file system in the future             | 2026-07-26 | Version binding, single-file usable                                                        |
| Assertion parameter types | Unannotated parameters (Any), no dependency on generics             | 2026-08-02 | `?` type syntax does not exist; Any verified to be comparable and interpolatable           |
| Multi-file run            | CLI `run` delegates to `run_project` (orchestrator) as prerequisite | 2026-08-02 | Subprocess model inherits CLI capabilities; #247 degrades to pure performance optimization |
| Report source location    | Subprocess with `--debug-info`                                      | 2026-08-02 | Verified stack trace output is `file:line:col`                                             |

## References

- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — standard library
  directory structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]` config
  section
- [RFC-030: assert Assertion Mechanism](../review/030-assert-mechanism.md) — underlying dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — reference
  design
