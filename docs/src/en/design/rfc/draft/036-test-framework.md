---
title: "RFC-036: std.test Testing Framework and yaoxiang test Command"
status: "Draft"
author: "Chenxu"
created: "2026-07-26"
updated: "2026-07-26"
issue: "#94, #95, #221"
---

# RFC-036: std.test Testing Framework and yaoxiang test Command

## Summary

Introduce the standard testing framework `std.test` module and the `yaoxiang test` CLI subcommand for YaoXiang. Test files are ordinary `.yx` files that pass or fail based on `std.assert.assert` and exit codes. The `std.test` module is implemented in pure YaoXiang and serves as the first dogfooding library. `yaoxiang test` is a CLI tool, not a compiler feature—no changes to the parser, IR, bytecode, or executor are involved.

## Motivation

### Why do we need a testing framework?

YaoXiang's current test coverage relies on Rust-side `#[test]` and `tests/` integration tests. This means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io) cannot be written in YaoXiang
2. `#117 Unit test coverage for each std module` is blocked because no usable test infrastructure exists
3. Regression tests for language features (e.g., RFC-032 spawn semantic changes) lack automated means

### Key Constraints

- **17 keyword rule**: Do not introduce any new keywords or syntax constructs
- **Zero compiler changes**: Do not touch the parser, IR, bytecode, or executor
- **Self-bootstrap first**: The test library is written in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI layer:     yaoxiang test [--filter --fail-fast --json...]│
│                     │                                        │
│  Discovery:     Read yaoxiang.toml → [tool.test] patterns    │
│                 Default: tests/**/*.yx                       │
│                     │                                        │
│  Execution:     For each file: yaoxiang run <file>           │
│                 Check exit code → serial execution           │
│                     │                                        │
│  Reporting:     PASS/FAIL → summary                          │
│                 Supports --json / --verbose / --fail-fast     │
│                                                              │
│  Assertion:     std.test (pure YaoXiang, self-bootstrapped)  │
│                 Underlying: std.assert.assert                │
│                 Diagnostics: f"Expected {expected}, got {actual}" │
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **The testing framework is not a compiler feature, but a CLI tool** — `yaoxiang run` can already "execute tests"; `yaoxiang test` just helps you run all the files and view the report
2. **Zero compiler changes** — No introduction of `@test` annotation scanning, bytecode metadata sections, or special executor entry points
3. **Self-bootstrap** — The `std.test` module is implemented in pure YaoXiang, calling `std.assert.assert` underneath
4. **Test files are ordinary `.yx` files** — Pass or fail is determined by the exit code

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Only run tests whose file names contain <NAME>
  --fail-fast         Stop on the first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              List test files only, do not run
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
      "file": "tests/string_test.yx", "passed": false, "time_secs": 0.003,
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
# 未来可扩展:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- Default `patterns = ["tests/**/*.yx"]` — works out of the box with zero configuration
- Single-file mode (`yaoxiang test foo.yx`) runs directly without reading configuration
- May be split into a separate repository in the future (the `[tool.test]` position remains unchanged)

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
- The `?` generic parameter of `assert_eq` / `assert_ne` depends on the generics system
- `std.test` does not depend on any native code, implemented in pure YaoXiang

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedded in Binary**

`std/test.yx` (and all future std modules written in YaoXiang) is embedded in the binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // Source code text
    // More in the future
];
```

When the module loader resolves `use std.test`:
1. First, check Rust native modules (existing mechanism, e.g., `std.assert`)
2. If not found, check the embedded `STD_YX_FILES` for the source code of `std/test.yx`
3. Compile the source code and register it into the module system

Advantages:
- `use std.test` works in single-file mode
- The standard library version is strictly bound to the binary, no version mismatch
- No need for users to configure the std path

**Future: File System Standard Library**

When the YaoXiang project mode matures, the standard library will transition to a file system form. See the update to RFC-014 for details.

### 5. Discovery and Execution

**Discovery phase**:
1. If `[PATHS]` is specified, use the specified path directly
2. Otherwise, read `patterns` from `[tool.test]` in `yaoxiang.toml`
3. If no configuration exists, default to `tests/**/*.yx`
4. Apply the `--filter` filter (file name contains the pattern)

**Execution phase**:
1. For each file: launch a subprocess with `yaoxiang run <file>`
2. Check the exit code: 0 means PASS, non-zero means FAIL
3. Capture stdout/stderr for reporting
4. Serial execution only (Phase 1); `--parallel` supported in the future
5. If `--fail-fast` is set, stop immediately on the first FAIL

### 6. Test Isolation

Test isolation is naturally achieved through process-level boundaries:
- Each test file runs in an independent subprocess
- Each subprocess has its own Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional independent Heap context mechanism is required

## Relationship with Existing Systems

| Item | Relationship |
|------|------|
| Rust `#[test]` | Untouched; compiler internal tests continue to use Rust |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test` |
| `std.assert.assert(cond)` | Retained; `std.test` depends on it underneath |
| `#200` refactoring (`io.println` → `assert.assert`) | Direction fully aligned with `yaoxiang test` |
| `@` annotations | Not used; no `@test` introduced |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:
- `src/main.rs` — Add the `Test` subcommand
- `src/std/test.yx` — Add the pure YaoXiang module
- `build.rs` — Embed `std/*.yx` into the binary
- Module loader — Support loading `.yx` modules from embedded sources
- RFC-015 config parsing — `[tool.test]` section
- Subprocess execution + reporting

Deliverables:
- `yaoxiang test` basically usable
- 4 assertion functions in `std.test`
- Default `tests/**/*.yx` discovery
- Serial execution + default output format

### Phase 2: Refinement

- `--filter` / `--fail-fast` / `--verbose` parameters
- `--json` output (for CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: Advanced

- `--parallel` parallel execution (depends on maturation of the spawn concurrency model)
- `[tool.test].exclude` configuration
- More assertion functions (e.g., `assert_approx_eq` for Float)

## Risks and Mitigations

| Risk | Probability | Mitigation |
|------|------|------|
| `f"..."` interpolation fails on generic types | Low | Verified that basic types work in `std.assert.assert` |
| Subprocess startup overhead affects test speed | Medium | Phase 1 serial execution is acceptable; Phase 3 parallel execution mitigates this |
| `yaoxiang.toml` config parsing not in current CLI | Low | Simple extension, does not affect core functionality |
| Generic `?` unavailable in `std.test` | Low | Can fall back to `Any` type or type specialization |
| Embedding `.yx` source files into binary increases size | Low | `.yx` source files are tiny, negligible |

## Open Questions

- [ ] Can the `use std.assert` reference in `std/test.yx` be correctly resolved in the module loader? Need to verify dependency relationships between embedded source modules
- [ ] Will the generic `to_string` in `f"..."` for test output introduce new type constraints? Needs verification

## Design Decision Record

| Decision | Resolution | Date | Reason |
|------|------|------|------|
| Test marking method | Do not use `@test` annotations; test files are ordinary `.yx` | 2026-07-26 | Zero compiler changes, subprocess equals isolation |
| Assertion method | `std.test` module as pure YaoXiang functions | 2026-07-26 | Self-bootstrap, no native code |
| Test execution model | Subprocess `yaoxiang run <file>` + exit code | 2026-07-26 | Process-level isolation, zero compiler changes |
| Standard library loading | Currently embedded in binary, file system in the future | 2026-07-26 | Version binding, works in single-file mode |
| Generic assertions | Depend on `?` generic parameter | 2026-07-26 | No specialization introduced, trust the generics system |

## References

- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — Standard library directory structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]` configuration section
- [RFC-030: assert Mechanism](../review/030-assert-mechanism.md) — Underlying dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — Reference design