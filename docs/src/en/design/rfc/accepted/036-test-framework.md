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

Introduce a standard testing framework `std.test` module and a `yaoxiang test` CLI subcommand for
YaoXiang. Test files are ordinary `.yx` files, with the overall pass/fail determined by the child
process exit code; a file may contain multiple test functions internally — assertion failures are
expressed as `Err` values (value semantics), and the suite collects per-test verdicts (§7). The
`std.test` module is implemented in pure YaoXiang and is the first dogfooding library.
`yaoxiang test` is a CLI tool, not a compiler feature — it involves no changes to the parser, IR,
bytecode, or executor.

## Motivation

### Why is a testing framework needed?

Currently, YaoXiang's test coverage relies on Rust-side `#[test]` and `tests/` integration tests.
This means:

1. The standard library (std.math / std.list / std.dict / std.convert / std.io) cannot be
   unit-tested in YaoXiang
2. `#117 Per-module unit test coverage of the standard library` is blocked due to the lack of
   available testing infrastructure
3. Regression tests for language features (such as the RFC-032 spawn semantic change) lack automated
   means

### Key Constraints

- **17-keyword iron rule**: No new keywords or syntactic structures are introduced
- **Zero compiler changes**: No touching the parser, IR, bytecode, or executor
- **Bootstrap-first**: The test library is written in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI 层:  yaoxiang test [--filter --fail-fast --json ...]    │
│              │                                               │
│  发现层:    读取 yaoxiang.toml → [tool.test] patterns         │
│              默认: tests/**/*.yx                              │
│              │                                               │
│  执行层:    对每个文件: yaoxiang run <file>                    │
│              检查 exit code → 串行执行                        │
│              │                                               │
│  报告层:    PASS/FAIL → 汇总                                  │
│              支持 --json / --verbose / --fail-fast            │
│                                                              │
│  断言层:    std.test (纯 YaoXiang，自举)                      │
│              底层: std.assert.assert                          │
│              诊断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **The testing framework is not a compiler feature; it is a CLI tool** — `yaoxiang run` can
   already "execute tests"; `yaoxiang test` simply helps you run all files and show you the report
2. **Zero compiler changes** — No `@test` annotation scanning, bytecode metadata sections, or
   special executor entry points are introduced
3. **Bootstrap** — The `std.test` module is implemented in pure YaoXiang; the underlying
   capabilities come from `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files** — Files run as child processes; the exit code determines
   the overall pass/fail
5. **Assertion failure is a value, not a process event** — Test functions return `Result`, assertion
   failures are expressed as `Err`, and the suite collects per-test verdicts individually (§7);
   process-level abort belongs only to runtime guards and is not used for test assertions

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      指定测试文件或目录（默认: 从 yaoxiang.toml 读取，否则 tests/）

Options:
  --filter <NAME>     只跑文件名包含 <NAME> 的测试
  --fail-fast         遇到第一个失败就停止
  --verbose, -v       显示每个测试的详细 stdout/stderr
  --list              只列出测试文件，不跑
  --no-progress       不显示进度条（CI 场景）
  --json              输出 JSON 格式结果（CI 集成用）
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
      "tests": [
        { "name": "push_grows_len", "passed": false, "error": "Expected 3, got 2" },
        { "name": "pop_returns_last", "passed": true }
      ]
    }
  ]
}
```

### 2. yaoxiang.toml Configuration

Placed under `[tool.test]`, conforming to the `[tool.*]` third-party extension convention of
RFC-015:

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
- Single-file mode (`yaoxiang test foo.yx`) runs directly, without reading configuration
- May be split into a separate repository in the future (location of `[tool.test]` remains
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

- Assertion functions follow **value semantics**: they return `Result((), String)`; failures are
  expressed as `Err(diagnostic info)` without aborting the process — §7 suites rely on this to
  collect per-test verdicts. The process-level abort semantics of `std.assert.assert` are reserved
  for runtime guards and do not enter the test assertion path
- Transition note: The 4 functions delivered in Phase 1 are based on `std.assert.assert` (abort
  semantics) as a bootstrap transitional implementation; the value-semantic family is the standard
  form of this RFC and will replace it once delivered (#319)
- `assert_eq` / `assert_ne` use **unannotated parameters** (`Any`) — empirically verified on
  2026-08-02: `==`/`!=` and f-string interpolation work normally on Any (verified for both
  Int/String), **without depending on the generics system**. Annotations can be added later once
  generics are ready
- `assert_false` uses `cond == false` to express negation (the unary `not` syntax is not yet landed;
  can be migrated once stable; the unary `!assert` form is subject to the same dependency, see §8.1)
- Error code assertions (`assert(err.code == "E3017")`) rely on the `Error` value carrying a
  machine-readable `code` field (§8.1)
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

The module system (RFC-029, fully landed on 2026-08-02) provides the entry point: the Registry holds
both native and source modules, and the orchestrator handles multi-file composition. The resolution
order for `use std.test` is:

1. First, look up the Rust native module (existing mechanism, e.g., `std.assert`)
2. If not found, look up the embedded `STD_YX_FILES` — on hit, inject the module into the
   orchestrator as a **virtual path** (e.g., `<std>/test.yx`) as a seed module, and go through the
   normal frontend pipeline (parse → typecheck → IR)
3. If not found, fall back to file system discovery (user modules)

A `use std.assert` inside an embedded source module is resolved normally by the resolver to the
native registry — native and source modules coexist in the Registry, and cross-kind dependencies
work naturally. Embedded modules are **compiled on demand**: they enter the pipeline only when
imported.

Advantages:

- `use std.test` works in single-file mode
- Standard library version is strictly bound to the binary, preventing version mismatches
- No need for users to configure standard library paths

**Future: File-System-Based Standard Library**

Once the YaoXiang project mode matures, the standard library will switch to a file-system form. See
updates to RFC-014 for details.

### 5. Discovery and Execution

**Prerequisite (2026-08-02 review decision)**: CLI `run` integrates the orchestrator. Currently CLI
`run` takes the single-file pipeline (`run_file_with_diagnostics`) and cannot resolve user module
imports; yet `yaoxiang test`'s child-process model inherits CLI capabilities, and test files
importing project modules is a core scenario. Therefore, Phase 1 first delegates the CLI `Run`
source branch to `run_project` (orchestrator, recursive directory discovery); #247 (on-demand
discovery along use) later layers on as a pure performance optimization. A single file with no
imports behaves equivalently through the orchestrator, and the bytecode branch is unchanged.

**Discovery Phase**:

1. If `[PATHS]` is specified, use the specified paths directly
2. Otherwise, read `[tool.test].patterns` from `yaoxiang.toml`
3. If no configuration exists, default to `tests/**/*.yx`
4. Apply `--filter` (file name contains)

**Execution Phase**:

1. For each file: spawn a child process via `yaoxiang run --debug-info <file>` (`--debug-info` makes
   runtime errors carry source location — empirically verified on 2026-08-02 that stack trace output
   is `file:line:col`)
2. Check exit code: 0 = PASS, non-0 = FAIL
3. Capture stdout/stderr for reporting
4. Serial execution only (Phase 1); `--parallel` will be supported in the future
5. If `--fail-fast` is set, stop immediately on the first FAIL

### 6. Test Isolation

Test isolation is naturally achieved through process-level boundaries:

- Each test file runs in an independent child process
- Each child process has an independent Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional independent Heap context mechanism is needed

### 7. Suites and Multiple Tests (Value-Based Model)

A test file may contain multiple tests. The in-file organization is:

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

- Each test is a zero-arg function returning `Result((), String)`; assertion failures are expressed
  as `Err` (§3 value-semantic assertion family) without interrupting the process — subsequent tests
  run as usual
- `test.suite` calls them one by one and collects: when a test is not Ok, the test's name and
  diagnostic info are printed; Ok is silent
- File exit code: all tests Ok → 0; any Err → non-0 (the §5 exit code check in the execution phase
  remains unchanged)
- The runner only sees files and does not perform function-level scanning: per-test verdicts come
  entirely from in-suite collection, and the internal structure of the file is transparent to the
  runner — the zero-compiler-change principle is unaffected
- Explicitly not adopted: in-process catch boundaries (17-keyword iron rule); runner calls each test
  entry individually (limited to internal scenarios such as §8.2 compile failure)
- The concrete API form of `test.suite` (signature, duplicate name handling, interaction between
  `--filter` and suite names) is an implementation detail and will be finalized in #319 during
  landing

### 8. Three-Layer Design for Negative Tests (Expected Failures)

Negative tests are split by the layer at which failure occurs, with each layer assigned
appropriately:

#### 8.1 Value-Level Negation (Generic, User-Facing)

The operation under test returns `Result`; the test expresses expected failure with ordinary
assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
test.assert_eq(result.err_code(r), "E3017")
```

- std.test adds the `assert_not` / `assert_err` function family; the unary `!assert` form will be
  provided once the `not` syntax is landed (same constraint as `cond == false` for `assert_false`)
- Error code assertions depend on extending the `Error` value with a machine-readable `code` field:
  from `Struct { message }` to `{ code, message }` (native `error_new_with_code`, standard library
  exporting code constants), so that `err.code == "E3017"` can be asserted
- As the Result-ification advances (#301, #316), operations that can fail return `Result` one by
  one, and file-level negative markers in the corpus migrate to in-file assertions accordingly

#### 8.2 Compile Failure (Limited to Internal Use by Language Designers)

Compilation is all-or-nothing for the whole file, so "this line should not compile" cannot be
expressed inside a file. File-level special markers are retained:

- The `[test:error]` marker is read by the runner and reverse-judged (run exit code non-0 = PASS)
- The marker is upgraded to a **structured expected code**: the runner parses the header
  `expected: compile error EXXXX` and compares it against the compiler's stderr output `[EXXXX]`;
  code mismatch = FAIL (a lightweight version of the trybuild stderr snapshot idea, with zero
  compiler changes)
- **Serves only the corpus in this repository, not part of the user-facing test framework**; the
  implementation side needs to unify the dual-runner judgment convention (directory convention vs.
  header marker), see #319

#### 8.3 Runtime Hard Failure (Folded into Result-ification)

No independent mechanism is set up — operations that can fail return `Result` per the language
direction (#301, #316), and tests uniformly go through §8.1. Process-level abort (such as assertion
violation, runtime parameter mismatch) gradually converges to values as Result-ification advances;
the testing framework provides no dedicated semantics for it.

## Relationship to Existing Systems

| Item                                                 | Relationship                                                                                                      |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched; compiler-internal tests continue to use Rust                                                           |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                        |
| `std.assert.assert(cond)`                            | Reserved for runtime guards; `std.test`'s value-semantic assertion family is based on `std.result` (§3, §7)       |
| Module system (RFC-029)                              | Embedded source modules enter via Registry/orchestrator; CLI `run` integrating the orchestrator is a prerequisite |
| `#200` refactor (`io.println` → `assert.assert`)     | Same direction as `yaoxiang test`                                                                                 |
| `@` annotations                                      | Not used; no `@test` introduced                                                                                   |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project`
  (multi-file run prerequisite)
- `src/main.rs` — Add new `Test` subcommand
- `src/std/test.yx` — Add new pure YaoXiang module
- `build.rs` — Embed `std/*.yx` into the binary
- orchestrator / Registry — Support loading `.yx` modules from embedded source via virtual paths
- RFC-015 configuration parsing — `[tool.test]` section
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

### Phase 3: Advanced

- `--parallel` parallel execution (depends on the maturation of the spawn concurrency model)
- `[tool.test].exclude` configuration
- More assertion functions (e.g., `assert_approx_eq` for Float)

## Risks and Mitigation

| Risk                                                          | Probability | Mitigation                                                                                                                                                                            |
| ------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` interpolation failure on Any                         | None        | Empirically verified on 2026-08-02 (Int/String both work)                                                                                                                             |
| `yaoxiang.toml` configuration parsing absent from current CLI | Low         | Simple extension, does not affect core functionality                                                                                                                                  |
| Behavior regression from CLI `run` integrating orchestrator   | Low         | No-import single-file path is equivalent; integration tests already cover orchestrator                                                                                                |
| Embedding `.yx` source files into the binary increases size   | Low         | `.yx` source files are extremely small, negligible                                                                                                                                    |
| Test loop time grows with corpus                              | High        | The main cost is full per-file compilation (11.3s for 185 files measured); `--parallel` only alleviates the process side, compilation cost needs test loop caching (#251/#293 slices) |

## Open Questions

- [x] Can the `use std.assert` reference in `std/test.yx` resolve correctly? — **Resolved
      (2026-08-02)**. After the module system (RFC-029) is landed, native and source modules coexist
      in the Registry, and the resolver unifies resolution; cross-kind dependencies work naturally
- [x] Does `to_string` in f-strings in test output introduce new type constraints? — **Resolved
      (2026-08-02)**. Empirically verified: on unannotated parameters (Any), `==`/`!=` and f-string
      interpolation both work (verified for Int/String), introducing no new constraints
- [x] Feasibility of `?` generic parameter? — **Resolved (2026-08-02)**: the `?` type syntax does
      not currently exist (and would be silently swallowed, tracked in a separate issue); Phase 1
      assertion functions use unannotated parameters and do not depend on the generics system

## Design Decision Record

| Decision                 | Determination                                                                                                                       | Date       | Reason                                                                                                                                                                      |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Test marker method       | No `@test` annotations; test files are ordinary `.yx`                                                                               | 2026-07-26 | Zero compiler changes; child process provides isolation                                                                                                                     |
| Assertion method         | `std.test` module with pure YaoXiang functions                                                                                      | 2026-07-26 | Bootstrap; no native code                                                                                                                                                   |
| Test execution model     | Child process `yaoxiang run <file>` + exit code                                                                                     | 2026-07-26 | Process-level isolation, zero compiler changes                                                                                                                              |
| Standard library loading | Currently embed in binary; file system in the future                                                                                | 2026-07-26 | Version binding; single-file usability                                                                                                                                      |
| Assertion parameter type | Unannotated parameters (Any), no dependency on generics system                                                                      | 2026-08-02 | `?` type syntax does not exist; Any empirically comparable and interpolable                                                                                                 |
| Multi-file run           | CLI `run` delegates to `run_project` (orchestrator) as a prerequisite                                                               | 2026-08-02 | Child-process model inherits CLI capabilities; #247 degrades to pure performance optimization                                                                               |
| Report source location   | Child process with `--debug-info`                                                                                                   | 2026-08-02 | Empirically verified stack trace output `file:line:col`; frame attribution relayed through embedded modules (std.test) is not guaranteed by this, belongs to #289 + RFC-034 |
| Negative test layering   | Value-level negation generic / compile-failure runner structured marker (internal only) / hard failure folded into Result-ification | 2026-09-02 | #319 decision; replaces implicit [test:error] convention                                                                                                                    |
| In-file multiple tests   | Value-based standard model: test functions return Result, suite collects per-test verdicts                                          | 2026-09-02 | No catch, no per-entry invocation (entry only for internal scenarios)                                                                                                       |
| Error code               | Error adds machine-readable `code` field                                                                                            | 2026-09-02 | Supports error code assertions; compile-time codes go through runner comparison                                                                                             |

## References

- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — Standard library
  directory structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]`
  configuration section
- [RFC-030: assert Assertion Mechanism](../review/030-assert-mechanism.md) — Underlying dependency
- [Rust `#[test]` Mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — Reference
  design
