---
title: 'RFC-036: std.test Testing Framework and yaoxiang test Command'
status: 'Draft'
author: '晨煦'
created: '2026-07-26'
updated: '2026-07-26'
issue: '#94, #95, #221'
---

# RFC-036: std.test Testing Framework and yaoxiang test Command

## Summary

Introduce a standard testing framework `std.test` module and `yaoxiang test` CLI subcommand for
YaoXiang. Test files are ordinary `.yx` files, with pass/fail determined via `std.assert.assert` +
exit code. The `std.test` module is implemented in pure YaoXiang and serves as the first dogfooding
library. `yaoxiang test` is a CLI tool, not a compiler feature—no changes to parser, IR, bytecode,
or executor.

## Motivation

### Why a Testing Framework?

Currently, YaoXiang's test coverage relies on Rust-side `#[test]` and integration tests in `tests/`.
This means:

1. Standard library (std.math / std.list / std.dict / std.convert / std.io) unit tests cannot be
   written in YaoXiang
2. Issue #117 "Unit test coverage for standard library modules" is blocked due to lack of available
   testing infrastructure
3. Regression tests for language features (e.g., RFC-032 spawn semantics changes) lack automation

### Key Constraints

- **17 keyword iron law**: Introduce no new keywords or syntactic constructs
- **Zero compiler changes**: Touch no parser, IR, bytecode, or executor
- **Dogfooding first**: Write the test library in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI layer:  yaoxiang test [--filter --fail-fast --json ...]│
│              │                                               │
│  Discovery:  Read yaoxiang.toml → [tool.test] patterns       │
│              Default: tests/**/*.yx                          │
│              │                                               │
│  Execution:  For each file: yaoxiang run <file>              │
│              Check exit code → serial execution              │
│              │                                               │
│  Reporting:  PASS/FAIL → aggregation                         │
│              Supports --json / --verbose / --fail-fast       │
│                                                              │
│  Assertion:  std.test (pure YaoXiang, dogfooding)            │
│              Foundation: std.assert.assert                   │
│              Diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **Test framework is not a compiler feature, it's a CLI tool** — `yaoxiang run` already "executes
   tests"; `yaoxiang test` just runs all files for you and shows you the report
2. **Zero compiler changes** — No `@test` annotation scanning, bytecode metadata sections, or
   executor special entry points
3. **Dogfooding** — The `std.test` module is implemented in pure YaoXiang, using `std.assert.assert`
   as the foundation
4. **Test files are ordinary `.yx` files** — Pass/fail determined via exit code

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Run only tests whose file names contain <NAME>
  --fail-fast         Stop on first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              List test files only, don't run
  --no-progress       Hide progress bar (CI scenarios)
  --json              Output JSON format results (for CI integration)
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
# Future extensibility:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- Default `patterns = ["tests/**/*.yx"]` — zero-config, works out of the box
- Single file mode (`yaoxiang test foo.yx`) runs directly without reading config
- May be split into a separate repository in the future (`[tool.test]` location unchanged)

### 3. std.test Module (Pure YaoXiang)

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library
// First dogfooding library: YaoXiang's test library written in YaoXiang

use std.assert

assert_eq: (a: ?, b: ?) -> Void = (a, b) => {
    assert.assert(a == b, f"Expected {b}, got {a}")
}

assert_ne: (a: ?, b: ?) -> Void = (a, b) => {
    assert.assert(a != b, f"Expected not equal to {b}, got {a}")
}

assert_true: (cond: Bool) -> Void = (cond) => {
    assert.assert(cond, f"Expected true, got {cond}")
}

assert_false: (cond: Bool) -> Void = (cond) => {
    assert.assert(!cond, f"Expected false, got {cond}")
}
```

- 4 assertion functions, all using `f"..."` for diagnostic messages
- `assert_eq` / `assert_ne` use `?` generic parameters depending on the generics system
- `std.test` has no native code dependencies, pure YaoXiang implementation

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedded Binary**

`std/test.yx` (and future YaoXiang-written standard library modules) are embedded in the binary at
build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // Source code text
    // More in the future
];
```

When the module loader parses `use std.test`:

1. First check Rust native modules (existing mechanism, e.g., `std.assert`)
2. If not found, check embedded `STD_YX_FILES`, locate source for `std/test.yx`
3. Compile that source and register in module system

Advantages:

- `use std.test` works in single-file mode too
- Standard library version tightly bound to binary, no version mismatch
- No need for users to configure standard library path

**Future: Filesystem Standard Library**

When YaoXiang project mode matures, the standard library will move to filesystem form. See updated
RFC-014 for details.

### 5. Discovery and Execution

**Discovery phase**:

1. If `[PATHS]` are specified, use those paths directly
2. Otherwise read `[tool.test].patterns` from `yaoxiang.toml`
3. If no config, default to `tests/**/*.yx`
4. Apply `--filter` filtering (file name contains)

**Execution phase**:

1. For each file: `yaoxiang run <file>` spawns subprocess
2. Check exit code: 0 is PASS, non-zero is FAIL
3. Capture stdout/stderr for reporting
4. Serial execution only (Phase 1), future `--parallel` support
5. If `--fail-fast`, stop immediately on first FAIL

### 6. Test Isolation

Test isolation is naturally achieved via process-level boundaries:

- Each test file runs in an independent subprocess
- Each subprocess has independent Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional isolated heap context mechanism needed

## Relationship with Existing Systems

| Item                                                 | Relationship                                           |
| ---------------------------------------------------- | ------------------------------------------------------ |
| Rust `#[test]`                                       | Unchanged, compiler internal tests continue using Rust |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`             |
| `std.assert.assert(cond)`                            | Retained, `std.test` depends on it                     |
| `#200` refactoring (`io.println` → `assert.assert`)  | Fully aligned with `yaoxiang test` direction           |
| `@` annotation                                       | Not used, no `@test` introduced                        |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:

- `src/main.rs` — Add `Test` subcommand
- `src/std/test.yx` — Add pure YaoXiang module
- `build.rs` — Embed `std/*.yx` into binary
- Module loader — Support loading `.yx` modules from embedded sources
- RFC-015 config parsing — `[tool.test]` section
- Subprocess execution + reporting

Deliverables:

- `yaoxiang test` basic functionality
- `std.test` with 4 assertion functions
- Default `tests/**/*.yx` discovery
- Serial execution + default output format

### Phase 2: Polish

- `--filter` / `--fail-fast` / `--verbose` options
- `--json` output (CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: Advanced

- `--parallel` parallel execution (depends on spawn concurrency model maturity)
- `[tool.test].exclude` config
- More assertion functions (e.g., `assert_approx_eq` for Float)

## Risks and Mitigations

| Risk                                               | Probability | Mitigation                                                      |
| -------------------------------------------------- | ----------- | --------------------------------------------------------------- |
| `f"..."` interpolation fails on generic types      | Low         | Basic types verified in `std.assert.assert`                     |
| Subprocess startup overhead affects test speed     | Medium      | Phase 1 serial execution acceptable; Phase 3 parallel mitigates |
| `yaoxiang.toml` config parsing not in current CLI  | Low         | Simple extension, doesn't affect core functionality             |
| Generic `?` unavailable in `std.test`              | Low         | Can degrade to `Any` type or type specialization                |
| Embedding `.yx` source files increases binary size | Low         | `.yx` source files are extremely small, negligible              |

## Open Questions

- [ ] Can `use std.assert` reference in `std/test.yx` be correctly resolved in the module loader?
      Need to verify dependency resolution between embedded source modules
- [ ] Will `f"..."` generic `to_string` in test output introduce new type constraints? Need to
      verify

## Design Decision Records

| Decision                 | Rationale                                            | Date       | Reason                                               |
| ------------------------ | ---------------------------------------------------- | ---------- | ---------------------------------------------------- |
| Test marking method      | No `@test` annotation, test files are ordinary `.yx` | 2026-07-26 | Zero compiler changes, subprocess provides isolation |
| Assertion method         | `std.test` module with pure YaoXiang functions       | 2026-07-26 | Dogfooding, no native code                           |
| Test execution model     | Subprocess `yaoxiang run <file>` + exit code         | 2026-07-26 | Process-level isolation, zero compiler changes       |
| Standard library loading | Currently embedded binary, filesystem in future      | 2026-07-26 | Version binding, works in single-file mode           |
| Generic assertions       | Depends on `?` generic parameters                    | 2026-07-26 | No specialization introduced, trust generics system  |

## References

- [RFC-014: Package Manager Design](../accepted/014-package-manager.md) — Standard library directory
  structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]` config
  section
- [RFC-030: assert Mechanism](../review/030-assert-mechanism.md) — Foundation dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — Reference
  design
