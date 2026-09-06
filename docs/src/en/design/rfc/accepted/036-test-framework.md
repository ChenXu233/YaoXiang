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

Introduce the standard testing framework `std.test` module and `yaoxiang test` CLI subcommand for
YaoXiang. Test files are ordinary `.yx` files, and the overall pass/fail is determined by the child
process exit code; files support multiple test functions internally—assertion failures are expressed
as `Err` values (value semantics), and the suite collects per-test verdicts (§7). The `std.test`
module is implemented in pure YaoXiang and is the first dogfooding library. `yaoxiang test` is a CLI
tool, not a compiler feature—it involves no changes to the parser, IR, bytecode, or executor.

## Motivation

### Why a Testing Framework Is Needed

Currently, YaoXiang's test coverage relies on the Rust-side `#[test]` and `tests/` integration
tests. This means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io)
   cannot be written in YaoXiang
2. `#117 Unit test coverage for standard library modules` is blocked because no test infrastructure
   is available
3. Regression tests for language features (such as the RFC-032 spawn semantics change) lack
   automated means

### Key Constraints

- **17-keyword iron rule**: introduce no new keywords or syntactic constructs
- **Zero compiler changes**: do not touch the parser, IR, bytecode, or executor
- **Self-hosting first**: the test library is written in YaoXiang, the first dogfooding library

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
│  Execution:  For each file: yaoxiang run <file>               │
│              Check exit code → serial execution              │
│              │                                               │
│  Reporting:  PASS/FAIL → summary                             │
│              Supports --json / --verbose / --fail-fast       │
│                                                              │
│  Assertion:  std.test (pure YaoXiang, self-hosted)            │
│              Lower layer: std.assert.assert                   │
│              Diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **The testing framework is not a compiler feature, it is a CLI tool**—`yaoxiang run` can already
   "execute tests"; `yaoxiang test` just helps you run all the files and show you a report
2. **Zero compiler changes**—no `@test` annotation scanning, no bytecode metadata sections, no
   special executor entry point
3. **Self-hosted**—the `std.test` module is implemented in pure YaoXiang, with underlying
   capabilities from `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files**—the file runs as a child process, and the exit code
   determines overall pass/fail
5. **Assertion failure is a value, not a process event**—test functions return `Result`, and
   assertion failures are expressed as `Err`, with the suite collecting per-test verdicts one by one
   (§7); process-level abort belongs only to runtime guards and is not used for test assertions

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Only run test files whose name contains <NAME>
  --fail-fast         Stop at the first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              Only list test files, do not run
  --no-progress       Suppress progress output (header and PASS lines); FAIL details and summary are preserved (CI scenario)
  --json              Output results in JSON format (for CI integration)
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

- Failed files additionally carry `exit_code` and `stderr` (child process diagnostics with ANSI
  stripped, for CI forensics); when `--verbose` and `--json` are combined, all files carry `stdout`
  / `stderr`
- `--no-progress` only suppresses progress output (header and PASS lines)—FAIL details and the
  summary are always output; failures cannot be silenced; `--list` outputs one test file path per
  line, without execution
- The in-file per-test `tests` array comes from the §7 suite collection, taking effect with the
  value model landing (#319)

### 2. yaoxiang.toml Configuration

Placed under `[tool.test]`, following the `[tool.*]` third-party extension convention of RFC-015:

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
# Future extensions:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- Default `patterns = ["tests/**/*.yx"]`—users get a zero-config out-of-the-box experience
- Single-file mode (`yaoxiang test foo.yx`) runs directly without reading config
- May be split into a separate repository in the future (the `[tool.test]` location remains
  unchanged)

### 3. std.test Module (Pure YaoXiang)

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library (value semantics standard form, landed 2026-09-03)
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

// assert_not and assert_false share the same body; assert_err / assert_err_code see §8.1
```

- Assertion functions are **value-semantic**: they return `Result(Void, String)`, with failures
  expressed as `Err(diagnostic info)`, without aborting the process—§7 suites use this to collect
  per-test verdicts. The process-level abort semantics of `std.assert.assert` are reserved for
  runtime guards and do not enter the test assertion path. The Ok payload is `Void` (type-system.md
  specifies unit; `()` is an empty Tuple, the two are not mixed—decided 2026-09-03)
- **Function family of 7 (delivered 2026-09-03, abort transitional version removed)**:
  value-semantic `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not` (shares
  body with assert_false, reserved for `!assert` form) + `assert_err` (§8.1)
  - `assert_err_code` (§8.1 error code assertion)
- `assert_eq` / `assert_ne` use **Any-annotated parameters**—`==`/`!=` and f-string interpolation
  work fine on Any, without depending on the generics system. Note that parameters **must be
  explicitly annotated**: unannotated parameters cannot pass the native generics `&Result(T, E)`
  call check (empirically verified by R1 probe)
- `assert_false` / `assert_not` use `cond == false` to express negation (the `not` unary syntax has
  not landed; can be migrated once stable; the `!assert` unary form depends on the same constraint,
  see §8.1)
- Block body + explicit `return` form: the then-arm type of the if expression is dropped during
  checking; the if expression with both arms of Result is a checking blind spot, which the
  implementation works around
- `std.test` does not depend on any native code; it is implemented in pure YaoXiang

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedded Binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) is embedded into the
binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source text
    // more in the future
];
```

The module system (RFC-029, fully landed 2026-08-02) provides the access point: the Registry holds
both native modules and source modules, and the orchestrator handles multi-file orchestration. The
resolution order for `use std.test`:

1. First, check Rust native modules (existing mechanism, such as `std.assert`)
2. If not found, check the embedded `STD_YX_FILES`—if hit, inject it into the orchestrator as a
   **virtual path** (such as `<std>/test.yx`) as the seed module, going through the normal frontend
   pipeline (parse → typecheck → IR)
3. If not found, fall back to file system discovery (user modules)

The internal `use std.assert` of an embedded source module is resolved normally by the resolver to
the native registry—native and source modules coexist in the Registry, so cross-kind dependencies
naturally work. Embedded modules are **compiled on demand**: they only enter the pipeline when
imported.

Advantages:

- `use std.test` works in single-file mode
- The standard library version is strictly tied to the binary, eliminating version mismatch
- No need for users to configure standard library paths

**Future: File System Standard Library**

When the YaoXiang project mode matures, the standard library will switch to file system form. See
updates to RFC-014 for details.

### 5. Discovery and Execution

**Prerequisite (2026-08-02 review decision)**: CLI `run` integrates with the orchestrator. Currently
CLI `run` goes through the single-file pipeline (`run_file_with_diagnostics`), which cannot resolve
user module imports; and the child process model of `yaoxiang test` inherits CLI capabilities, so
test files importing project modules is a core scenario. Therefore, Phase 1 first delegates the CLI
`Run` source branch to `run_project` (orchestrator, directory-recursive discovery); #247 (on-demand
discovery along `use`) then layers on as a pure performance optimization. Single files without
imports behave equivalently through the orchestrator, and the bytecode branch is unchanged.

**Discovery phase**:

1. If `[PATHS]` is specified, use the specified paths directly
2. Otherwise, read `[tool.test].patterns` from `yaoxiang.toml`
3. If no configuration exists, default to `tests/**/*.yx`
4. Apply `--filter` filtering (file name contains)
5. The discovery scope is the test layering (§9): default patterns only cover the language
   availability corpus; the library test layer (such as `src/std/tests/`) is discovered via explicit
   paths or package configuration, not mixed into the default scan

**Execution phase**:

1. For each file: launch a child process with `yaoxiang run --debug-info <file>` (`--debug-info`
   makes runtime errors carry source location—empirically verified 2026-08-02 that stack trace
   outputs `file:line:col`); the header `[test:runtime]` declares the child process `--runtime` mode
   (closed 2026-09-03)
2. Files with header `[test:ignore]: <reason>` skip execution and are counted as skipped in the
   report (closed 2026-09-03)
3. Check the exit code: 0 is PASS, non-zero is FAIL; `[test:error]` files are judged in reverse and
   compared against the expected code per §8.2
4. Capture stdout/stderr for the report
5. Serial execution only (Phase 1); `--parallel` will be supported in the future
6. If `--fail-fast`, stop immediately at the first FAIL

### 6. Test Isolation

Test isolation is naturally achieved through process-level boundaries:

- Each test file runs in an independent child process
- Each child process has its own independent Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional isolated Heap context mechanism is needed

### 7. Suite and Multiple Tests (Value Model)

A test file can contain multiple tests. The in-file organization (landed 2026-09-03):

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
  expressed as `Err` (§3 value-semantic assertion family), without interrupting the
  process—subsequent tests run as usual
- `test.suite` calls each one and collects: if a test is not Ok, record the name and diagnostic; Ok
  is silent; after all are run, if any Err exists, abort with `std.assert.assert` and attach failure
  details (`N of M test(s) failed` + each `[FAIL] name: diagnostic`)—the file exit code is non-zero
  (§5 judgment unchanged). The abort here is a runtime guard of the test binary, not an assertion
  path; all-Ok exits silently with code 0
- Top-level test functions are entered into the list as **closures** (`("name", () => test_fn())`):
  using the top-level function name as a value reference is not yet supported (IR-layer limitation,
  `E3006`)—calling global functions from a closure body is unaffected
- The runner only sees the file and does not perform function-level scanning: per-test verdicts come
  entirely from in-suite collection, the file's internal structure is transparent to the runner—the
  zero-compiler-change principle is not affected
- Explicitly not adopted: in-process catch boundaries (17-keyword iron rule); runner calling
  function-by-function entries (limited to internal scenarios such as §8.2 compile failures)
- The API form has been finalized (2026-09-03, #319):
  `suite(tests: List((String, () -> Result(Void, String)))) -> Void`; duplicate names are not
  detected (names are only used for report display); `--filter` filters by file name, not aware of
  in-suite test names

### 8. Negative Testing (Expected Failure) Three-Layer Design

Negative tests are split by the layer where the failure occurs, each with its own place:

#### 8.1 Value-Level Reversal (General, User-Facing)

The operation under test returns `Result`, and the test expresses the expected failure with ordinary
assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// or a one-line wrapper (the code only exists on the std Error carrier, E is nailed down as Error):
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` have been delivered along with the value-semantic
  family (2026-09-03, §3); the `!assert` unary form will be provided after the `not` syntax lands
  (same constraint as `assert_false`'s `cond == false`)
- Error code assertions depend on the `Error` value carrying a machine-readable `code` field—already
  landed in #323 M4: `Error = { code, message }` (native `error_new(code, message)`), read via
  `result.unwrap_err(r)` to obtain the carrier and `result.code(e)` / `result.message(e)` accessors
  (the design-phase estimated `error_new_with_code` naming, code constant export, and `err.code`
  field access were all not adopted—the language has no Struct field access, and code constants are
  not exported)
- As the Result-ification progresses (#301, #316), fail-able operations gradually return `Result`,
  and the file-level negative markers in the corpus migrate to in-file assertions accordingly

#### 8.2 File-Level Negative Markers (For Language Designers' Internal Use Only)

Compilation is all-or-nothing for a whole file and cannot express "this line should not compile"
within the file; runtime failures likewise need file-level expression (e.g., a suite contains a test
that must fail). File-level special markers are retained, and the runner **diverts judgment** by
expected category (decided 2026-09-03, landing pending):

- The `[test:error]` marker is read by the runner; the `expected: compile error EXXXX` /
  `expected: runtime error EXXXX` line declares the category and expected code
- **Compile error category**: the runner runs `yaoxiang check`—it must fail and the output must
  contain `[EXXXX]`; compilation passing = FAIL, being rejected but with a mismatched code = FAIL.
  Syntax errors (E1xxx parsing segment) and semantic errors (E2xxx+) are not given independent
  categories—the expected code itself nails down the stage
- **Runtime error category**: two-step judgment—`check` must **pass** (compile-time is innocent),
  and `run` must fail and the output must contain `[EXXXX]`. Compile-time explosion = FAIL (the key
  judgment opposite in direction to the compile error category, preventing "compile accidentally
  passes, runtime coincidentally fails" from being missed)
- `[test:error]` without an `expected:` line falls back to exit≠0 judgment (uncategorized form, not
  encouraged for new corpus)
- **Serves only this repository's corpus, not part of the user testing framework**; the dual-runner
  judgment convention is closed (2026-09-03, #319): yx_runner (cargo test) and `yaoxiang test` share
  `src/util/test_markers.rs` to parse header markers
  (`[test:error]`/`[test:ignore]`/`[test:runtime]`/`expected: EXXXX`, first 16 lines), and the
  06-compile-errors directory convention is deprecated. After the diversion judgment lands, the
  report layer synchronously provides category counts (behavior / compile-error / runtime-error /
  skipped)

#### 8.3 Runtime Hard Failures (Subsumed into Result-ification)

No independent mechanism is set—operations that may fail return `Result` per the language direction
(#301, #316), and tests uniformly go through §8.1 to express. Process-level aborts (such as
assertion violations, runtime parameter errors) gradually converge to values as Result-ification
progresses, and the testing framework provides no special semantics for them. (Note: the "runtime
error category" marker judgment in §8.2 is the runner's file-level verification channel for
**operations that are not yet Result-ifiable**, which does not contradict the semantic direction of
this section—the latter is the destination, the former is a migration-period channel)

### 9. Test System Layering: Language Corpus and Library Tests (Decided 2026-09-03)

Tests are divided into two layers by **the object under test**, each with its own ownership and
maintainer; the marker system (§8.2) and assertion library (§3) are shared across both layers:

**Layer 1: Language Availability Corpus (`tests/yaoxiang/`)**

- The object under test is **the language itself**—parser, type system, modules, concurrency,
  ownership, compile-time rejection, runtime semantics; directories are organized by language
  specification chapters
- std in the corpus only acts as an **assertion tool** (`std.assert` / `std.test`), and is never
  tested—the library's API behavior does not belong to language availability
- Within the corpus, judgment is divided into three categories per §8: behavior tests / compile-time
  rejection tests / runtime failure tests

**Layer 2: Library Tests (Go with the Library)**

- The object under test is **the library's public API contract** (such as `list.push` behavior,
  `result.code` semantics)
- Tests are written in **the library's own package**: std's package is `src/std/`, and its yx-level
  tests belong to `src/std/tests/` (in the same place as the implementation body); `std.test`'s own
  tests are also there (using std.test to test std.test, a self-hosting closed loop)
- Future user packages follow the same convention: tests are inside the package, discovered via the
  package's `[tool.test]` (this previews RFC-014's package management test layout)
- Discovery does not enter the default patterns (default `tests/**/*.yx` only covers the language
  layer): the library test layer is discovered via explicit paths (`yaoxiang test src/std/tests`) or
  package configuration; CI runs in layers

Migration note: the library tests currently in `tests/yaoxiang/07-std/` will be migrated to
`src/std/tests/` according to this model; each one should be screened during migration—pure API
behavior is migrated out, and those whose object under test is actually a language boundary (such as
native `&T` auto-borrow) remain in the corpus layer and are assigned to the corresponding
specification chapter

## Relationship with Existing Systems

| Item                                                 | Relationship                                                                                                            |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched, compiler internal tests continue to use Rust                                                                 |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                              |
| `std.assert.assert(cond)`                            | Reserved for runtime guards; `std.test` value-semantic assertion family changed to base on `std.result` (§3, §7)        |
| Module system (RFC-029)                              | Embedded source modules integrated via Registry/orchestrator; CLI `run` integrating with orchestrator is a prerequisite |
| `#200` refactoring (`io.println` → `assert.assert`)  | Same direction as `yaoxiang test`                                                                                       |
| `@` annotation                                       | Not used, no `@test` introduced                                                                                         |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project`
  (multi-file run prerequisite)
- `src/main.rs` — new `Test` subcommand
- `src/std/test.yx` — new pure YaoXiang module
- `build.rs` — embed `std/*.yx` into the binary
- orchestrator / Registry — support loading `.yx` modules from embedded source via virtual paths
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
- More assertion functions (such as `assert_approx_eq` for Float)

## Risks and Mitigation

| Risk                                                                      | Probability | Mitigation                                                                                                                                                                                                        |
| ------------------------------------------------------------------------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` interpolation fails on Any                                       | None        | Empirically verified 2026-08-02 (Int/String both work)                                                                                                                                                            |
| `yaoxiang.toml` config parsing is not in the current CLI                  | Low         | Simple extension, does not affect core functionality                                                                                                                                                              |
| CLI `run` integrating with orchestrator introduces behavioral regressions | Low         | Import-free single-file path is equivalent; orchestrator is already covered by integration tests                                                                                                                  |
| Embedding `.yx` source files into the binary increases size               | Low         | `.yx` source files are tiny, negligible                                                                                                                                                                           |
| Test loop time grows with corpus                                          | High        | The main cost is full-file compilation per file (185 files measured 11.3s), not child process startup; `--parallel` only alleviates the process side, compilation cost needs test loop caching (#251/#293 slices) |

## Open Questions

- [x] Can the `use std.assert` reference in `std/test.yx` be resolved correctly?—**Resolved
      (2026-08-02)**. After the module system (RFC-029) landed, native and source modules coexist in
      the Registry, and the resolver parses them uniformly; cross-kind dependencies naturally work
- [x] Does the generic `to_string` of `f"..."` in test output introduce new type
      constraints?—**Resolved (2026-08-02)**. Empirically verified that `==`/`!=` and f-string
      interpolation both work on unannotated (Any) parameters (Int/String verified), introducing no
      new constraints
- [x] Feasibility of `?` generic parameter?—**Resolved (2026-08-02)**: the `?` type syntax does not
      currently exist (and would be silently swallowed, a separate issue has been opened to track
      it); Phase 1 assertion functions use unannotated parameters, not depending on the generics
      system

## Design Decision Record

| Decision                           | Decision                                                                                                                                                                            | Date       | Reason                                                                                                                                                                                                                  |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Test marker method                 | Do not use `@test` annotations; test files are ordinary `.yx`                                                                                                                       | 2026-07-26 | Zero compiler changes, child process is isolation                                                                                                                                                                       |
| Assertion method                   | `std.test` module as pure YaoXiang functions                                                                                                                                        | 2026-07-26 | Self-hosted, no native code                                                                                                                                                                                             |
| Test execution model               | Child process `yaoxiang run <file>` + exit code                                                                                                                                     | 2026-07-26 | Process-level isolation, zero compiler changes                                                                                                                                                                          |
| Standard library loading           | Currently embedded binary, file system in the future                                                                                                                                | 2026-07-26 | Version binding, single-file usable                                                                                                                                                                                     |
| Assertion parameter type           | Unannotated parameters (Any), not depending on the generics system                                                                                                                  | 2026-08-02 | `?` type syntax does not exist; Any empirically supports comparison and interpolation                                                                                                                                   |
| Multi-file execution               | CLI `run` delegates to `run_project` (orchestrator) as a prerequisite                                                                                                               | 2026-08-02 | Child process model inherits CLI capabilities; #247 degrades to a pure performance optimization                                                                                                                         |
| Report source location             | Child process with `--debug-info`                                                                                                                                                   | 2026-08-02 | Empirically verified stack trace outputs `file:line:col`; frame attribution via embedded modules (std.test) is not guaranteed here, belongs to #289 + RFC-034                                                           |
| Negative test layering             | Value-level reversal general / compile failure runner structured markers (internal only) / hard failure subsumed into Result-ification                                              | 2026-09-02 | #319 decided; replaces the implicit [test:error] convention                                                                                                                                                             |
| In-file multiple tests             | Value-semantic standard model: test functions return Result, suite collects per-test verdicts                                                                                       | 2026-09-02 | No catch, no entry-point calls (entries only for internal scenarios)                                                                                                                                                    |
| Error code                         | Error adds machine-readable `code` field                                                                                                                                            | 2026-09-02 | Supports error code assertion; compile-time codes go through runner comparison                                                                                                                                          |
| Assertion library form             | Value-semantic family of 7 functions landed, `Result(Void, String)` contract; abort transitional version removed                                                                    | 2026-09-03 | Void is the canonical unit (`()` is empty Tuple, not mixed); the nested position is rigidly Any, unannotated parameters cannot pass native generics checks—parameters must be explicitly annotated (R1 probe empirical) |
| Test system layering               | Language corpus (`tests/yaoxiang/`) and library tests (with the library, std → `src/std/tests/`) split into two layers; std in the corpus only acts as an assertion tool            | 2026-09-03 | The object under test determines ownership and maintainer; library tests following the package layout previews RFC-014                                                                                                  |
| Negative marker diversion judgment | `[test:error]` diverted by expected category: compile error category `check` must fail, runtime error category `check` must pass + `run` must fail; report provides category counts | 2026-09-03 | Mixed-category judgment would let "compile accidentally passes, runtime coincidentally fails" slip through; the expected code nails down the stage, syntax errors are not given a separate category                     |

## References

- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — standard library
  directory structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]`
  configuration section
- [RFC-030: assert Assertion Mechanism](../review/030-assert-mechanism.md) — underlying dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — reference
  design
