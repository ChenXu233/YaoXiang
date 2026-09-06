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

Introduce the standard testing framework `std.test` module and the `yaoxiang test` CLI subcommand
for YaoXiang. Test files are ordinary `.yx` files; overall pass/fail is determined by the subprocess
exit code; a file may contain multiple test functions—assertion failures are expressed as `Err`
values (value semantics), and the suite collects per-test verdicts (§7). The `std.test` module is
implemented in pure YaoXiang and is the first dogfooding library. `yaoxiang test` is a CLI tool, not
a compiler feature—it involves no changes to the parser, IR, bytecode, or executor.

## Motivation

### Why Do We Need a Testing Framework?

Currently, YaoXiang's test coverage depends on Rust-side `#[test]` and the `tests/` integration
tests. This means:

1. The standard library (std.math / std.list / std.dict / std.convert / std.io) cannot be
   unit-tested in YaoXiang
2. `#117 Unit test coverage for each standard library module` is blocked because no usable test
   infrastructure exists
3. Regression tests for language features (such as the RFC-032 spawn semantic changes) lack
   automation

### Key Constraints

- **17-keyword iron rule**: introduce no new keywords or syntactic constructs
- **Zero compiler changes**: do not touch the parser, IR, bytecode, or executor
- **Bootstrap first**: the test library is written in YaoXiang; the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI layer:  yaoxiang test [--filter --fail-fast --json ...] │
│              │                                               │
│  Discovery:  read yaoxiang.toml → [tool.test] patterns        │
│              default: tests/**/*.yx                          │
│              │                                               │
│  Execution:  for each file: yaoxiang run <file>               │
│              check exit code → serial execution              │
│              │                                               │
│  Reporting:  PASS/FAIL → summary                             │
│              supports --json / --verbose / --fail-fast       │
│                                                              │
│  Assertion:  std.test (pure YaoXiang, bootstrap)              │
│              underlying: std.assert.assert                    │
│              diagnostics: f"Expected {expected}, got {actual}" │
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **The test framework is not a compiler feature; it is a CLI tool** — `yaoxiang run` can already
   "execute tests"; `yaoxiang test` simply runs all files for you and shows you the report
2. **Zero compiler changes** — no `@test` annotation scanning, no bytecode metadata sections, no
   special executor entry points
3. **Bootstrap** — the `std.test` module is implemented in pure YaoXiang; the underlying
   capabilities come from `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files** — files run as subprocesses, and the exit code determines
   overall pass/fail
5. **Assertion failures are values, not process events** — test functions return `Result`; failures
   are expressed as `Err`, and the suite collects per-test verdicts one by one (§7); process-level
   aborts are reserved for runtime guards and are not used for test assertions

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Only run tests whose file name contains <NAME>
  --fail-fast         Stop on the first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              List test files only, do not run
  --no-progress       Suppress progress output (header and PASS lines); FAIL details and summary are retained (for CI)
  --json              Output results in JSON format (for CI integration)
```

#### Output Format

**Default output** (per-test verdicts come from the in-file suite collection, see §7):

```
Running 3 test files...

tests/math_test.yx ........................ PASS (0.002s)
tests/list_test.yx ........................ FAIL (0.003s)
  `-- [FAIL] push_grows_len: Expected 3, got 2
  `-- [ ok ] pop_returns_last
Results: 2 files passed, 1 file failed, 0 skipped (0.006s)
Categories: 2 behavior, 0 compile-error, 0 runtime-error
```

**JSON output** (`--json`):

```json
{
  "summary": {
    "total": 3,
    "passed": 2,
    "failed": 1,
    "skipped": 0,
    "by_kind": { "behavior": 3, "compile-error": 0, "runtime-error": 0, "invalid": 0 },
    "time_secs": 0.006
  },
  "files": [
    { "file": "tests/math_test.yx", "kind": "behavior", "passed": true, "time_secs": 0.002 },
    {
      "file": "tests/list_test.yx",
      "kind": "behavior",
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
- `--no-progress` only suppresses progress output (header and PASS lines)—FAIL details and the
  summary are always output; failures cannot be silenced; `--list` outputs one test file path per
  line, without execution
- Each file carries `kind` (behavior / compile-error / runtime-error / invalid, §8.2), the summary
  carries `by_kind` execution counts (fixed four keys, not including skipped); the human summary has
  an `Categories:` distribution line
- The per-file `tests` array comes from the §7 suite collection and takes effect with the value-ized
  model (#319)

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

- Default `patterns = ["tests/**/*.yx"]` — zero-config out of the box
- Single-file mode (`yaoxiang test foo.yx`) runs directly without reading the configuration
- May be split into a separate repository in the future (the `[tool.test]` location remains
  unchanged)

### 3. std.test Module (Pure YaoXiang)

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library (value-semantics standard form, landed 2026-09-03)
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

// assert_not and assert_false are the same; assert_err / assert_err_code see §8.1
```

- Assertion functions use **value semantics**: they return `Result(Void, String)`; failures are
  expressed as `Err(diagnostics)`, and they do not abort the process—the §7 suite collects per-test
  verdicts based on this. The process-level abort semantics of `std.assert.assert` are reserved for
  runtime guards and do not enter the test assertion path. The Ok payload is `Void` (per
  type-system.md, unit; `()` is the empty Tuple, the two are not mixed—finalized 2026-09-03)
- **7-function family (delivered 2026-09-03, abort transitional version removed)**: value-ized
  `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not` (same as assert_false,
  reserved for the `!assert` unary form) + `assert_err` (§8.1)
  - `assert_err_code` (§8.1 error code assertion)
- `assert_eq` / `assert_ne` use **Any-annotated parameters**—`==`/`!=` and f-string interpolation
  work correctly on Any, with no dependency on the generics system. Note that parameters **must be
  explicitly annotated**: parameters without annotations fail the call check against the native
  generic `&Result(T, E)` (verified by the R1 probe)
- `assert_false` / `assert_not` use `cond == false` to express negation (the `not` unary syntax is
  not landed; this can be migrated after stabilization; the `!assert` unary form has the same
  dependency, see §8.1)
- Block body + explicit `return` form: the type of the if expression's then-arm is discarded during
  checking; the if expression with two Result arms is a blind spot for the checker; the
  implementation works around it
- `std.test` depends on no native code and is implemented in pure YaoXiang

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedded in the binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) are embedded into the
binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source text
    // more in the future
];
```

The module system (RFC-029, fully landed 2026-08-02) provides the entry point: the Registry holds
both native and source modules, and the orchestrator is responsible for multi-file orchestration.
The resolution order for `use std.test`:

1. First look up the Rust native module (existing mechanism, e.g., `std.assert`)
2. If not found, look up the embedded `STD_YX_FILES`—if matched, inject the module into the
   orchestrator with a **virtual path** (e.g., `<std>/test.yx`) as the seed module, going through
   the normal frontend pipeline (parse → typecheck → IR)
3. If not found, fall through to filesystem discovery (user modules)

The `use std.assert` inside embedded source modules is resolved by the resolver to the native
registry as usual—native and source modules coexist in the Registry, so cross-kind dependencies work
naturally. Embedded modules are **compiled on demand**: they only enter the pipeline when imported.

Advantages:

- `use std.test` works even in single-file mode
- The standard library version is strictly bound to the binary and cannot be mismatched
- No need for the user to configure standard library paths

**Future: Filesystem standard library**

Once the YaoXiang project mode matures, the standard library will switch to a filesystem form. See
updates to RFC-014 for details.

### 5. Discovery and Execution

**Prerequisite (2026-08-02 review decision)**: CLI `run` connects to the orchestrator. Currently,
CLI `run` uses a single-file pipeline (`run_file_with_diagnostics`) and cannot resolve user module
imports; yet `yaoxiang test`'s subprocess model inherits CLI capabilities, and test files importing
project modules is a core scenario. Therefore, Phase 1 first delegates the CLI `Run` source-code
branch to `run_project` (orchestrator, directory-recursive discovery); #247 (on-demand discovery
along `use`) afterward serves as a pure performance optimization overlay. A single file with no
import behaves identically through the orchestrator, and the bytecode branch is unchanged.

**Discovery phase**:

1. If `[PATHS]` is specified, use the specified paths directly
2. Otherwise, read `[tool.test].patterns` from `yaoxiang.toml`
3. If not configured, default to `tests/**/*.yx`
4. Apply the `--filter` (file name contains)
5. The discovery scope equals the test layering (§9): the default patterns only cover the
   language-availability corpus; the library test layer (e.g., `src/std/tests/`) is discovered via
   explicit paths or package configuration, and is not mixed into the default scan

**Execution phase**:

1. For each file, route execution by the header directives (directive grammar see §8.2, parsed by
   `src/util/test_markers.rs`, shared with yx_runner):
   - Behavior test: `yaoxiang run --debug-info <file>` subprocess (`--debug-info` makes runtime
     errors carry source locations—verified 2026-08-02 that stack trace outputs `file:line:col`);
     `// mode:` declares the subprocess `--runtime` mode
   - Compile-time rejection: single-step `yaoxiang check <file>`
   - Runtime-failure: two-step `check` (must pass) + `run` (must fail)
2. Files with `// skip: <reason>` skip execution and are counted as skipped in the report
3. Verdict-vs-expected-code comparison follows the §8.2 verdict matrix; directive parse failure
   skips execution and goes directly to FAIL (construct-time rejection)
4. Capture stdout/stderr for the report
5. Serial execution only (Phase 1); `--parallel` will be supported in the future
6. If `--fail-fast`, stop immediately on the first FAIL

### 6. Test Isolation

Test isolation is naturally provided by process-level boundaries:

- Each test file runs in an independent subprocess
- Each subprocess has its own Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional isolated Heap context mechanism is required

### 7. Suite and Multi-test (Value-ized Model)

A test file may contain multiple tests. The in-file organization (landed 2026-09-03):

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
  expressed as `Err` (§3 value-semantics assertion family), and they do not interrupt the
  process—subsequent tests still run
- `test.suite` calls them one by one and collects: if a test is not Ok, record its name and
  diagnostics; Ok is silent; after all tests run, if any Err exists, abort with `std.assert.assert`
  and attach failure details (`N of M test(s) failed` + each `[FAIL] name: diagnostics`)—the file
  exit code is non-zero (§5 verdict unchanged). The abort here is a runtime guard of the test
  binary, not the assertion path; all Ok silently exits with 0
- Top-level test functions enter the list as **closures** (`("name", () => test_fn())`): top-level
  function names as value references are not yet supported (IR-layer limitation, `E3006`)—closure
  bodies calling global functions are not affected
- The runner only sees files, and does not scan at the function level: per-test verdicts come
  entirely from the in-suite collection, and the in-file structure is transparent to the runner—the
  zero-compiler-change principle is not affected
- Explicitly not adopted: in-process catch boundaries (17-keyword iron rule); runner per-function
  entry calls (only allowed for internal scenarios like §8.2 compile failure)
- API form finalized (2026-09-03, #319):
  `suite(tests: List((String, () -> Result(Void, String)))) -> Void`; duplicates are not detected
  (names are used only for report display); `--filter` filters by file name and is unaware of
  in-suite test names

### 8. Three-Layer Design for Negative Tests (Expected Failure)

Negative tests are split by the layer where failure occurs, and each layer has its place:

#### 8.1 Value-Level Reverse (General, for Users)

The operation under test returns `Result`, and the test expresses the expected failure with ordinary
assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// Or a one-line wrapper (codes only exist on the std Error carrier, E nailed down as Error):
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` have been delivered along with the value-semantics
  family (2026-09-03, §3); the `!assert` unary form will be provided after the not syntax lands
  (same constraint as `assert_false`'s `cond == false`)
- Error code assertions depend on the `Error` value carrying a machine-readable `code`
  field—delivered by #323 M4: `Error = { code, message }` (native `error_new(code, message)`),
  accessed via `result.unwrap_err(r)` to get the carrier + `result.code(e)` / `result.message(e)`
  accessors (the originally estimated `error_new_with_code` naming, code constant export, and
  `err.code` field access during design were all not adopted—the language has no Struct field
  access, and code constants are not exported)
- As the Result-ization advances (#301, #316), fallible operations return `Result` one by one, and
  file-level negative markers in the corpus are migrated to in-file assertions

#### 8.2 File-Level Negative Directives (Internal Use Only by Language Designers)

Compilation is all-or-nothing per file, so "this line should not compile" cannot be expressed within
a file; runtime failures similarly require file-level expression (e.g., when a suite contains a test
that must fail). The file header declares expectations with **structured directives**, and the
runner **routes verdicts** by expected category (routing finalized 2026-09-03; directive grammar
finalized and landed 2026-09-06).

Header directive grammar (`// key: value`, within the first 16 lines, strict token matching):

```text
// expect: compile-error E1002 [E1003 ...]   compile-time rejection test
// expect: runtime-error E6003 [...]         runtime-failure test
// skip: <reason>                            skip execution, counted as skipped
// mode: embedded|standard|full              subprocess --runtime mode (consumed only by the run step)
```

- No `expect:` directive = behavior test. `expect:` is the **sole declaration** of
  expectation—finalized 2026-09-06 to deprecate the `[test:error]` boolean flag and the Chinese
  `预期:` prose code-scrape: the boolean flag and the expectation line are two loosely coupled
  facts, and keeping them consistent through discipline will inevitably drift; the English strict
  token grammar lets the runner parse mechanically (kind + codes are all fixed tokens; any extra
  token after a code causes parse failure), parse failure = do not execute, go directly to
  FAIL—there is no silent degradation channel for incorrectly declared directives (construct-time
  rejection)
- **Compile-error category**: single-step `check`—must fail and the output must contain all
  `[EXXXX]`; compile success = FAIL (expected to fail but didn't), rejection with mismatched code =
  FAIL. Syntax errors (E1xxx parse stage) and semantic errors (E2xxx+) do not have independent
  categories—the expected code itself nails down the stage
- **Runtime-error category**: two-step verdict—`check` must **succeed** (compile-time innocence),
  `run` must fail and the output must contain all `[EXXXX]`. Compile-time failure = FAIL (the
  verdict direction opposite to the compile-error category, preventing "compile accidentally passes,
  runtime coincidentally fails" from being missed)
- Isomorphic with the industry: Rust compiletest `//~ ERROR`, Go `// ERROR "regexp"`, GCC
  `dg-error`, Clang `expected-error` all declare expectations in fixture comments and compare
  bidirectionally via harness; they adopt line-level anchoring because multi-diagnostic compilers
  need to distinguish multiple expectations in the same file, while this compiler stops on first
  error with one diagnostic per file, and file-level is isomorphic with the compiler reality—after
  error recovery lands, the line-anchored form can be appended to the grammar (Cranelift filetests'
  file-header directive + function-level expectations use the same hybrid form)
- Known rendering debt: parse-stage diagnostics are currently output in Debug form (`code: "E0012"`
  rather than `[E0012]`); the code scanner accepts both forms; the strict form will be reinstated
  after diagnostic rendering is unified
- **Serves only this repository's corpus, not part of the user-facing test framework**; the
  dual-runner verdict convention is closed (2026-09-03, #319): yx_runner (cargo test) and
  `yaoxiang test` share `src/util/test_markers.rs` for parsing header directives, and the
  06-compile-errors directory convention is deprecated. The report layer provides category counts:
  the human summary has a `Categories:` line, the JSON summary has `by_kind` (behavior /
  compile-error / runtime-error / invalid, with skipped counted separately), and each file carries
  `kind`

#### 8.3 Runtime Hard Failure (Folded into Result-ization)

No independent mechanism is provided—operations that may fail return `Result` per the language
direction (#301, #316), and tests uniformly use the §8.1 expression. Process-level aborts (such as
assertion violations, runtime parameter mismatches) are gradually converged into values as
Result-ization progresses, and the test framework provides no special semantics for them. (Note: the
"runtime-error category" mark verdict in §8.2 is the runner's file-level verification channel for
**operations that cannot yet be Result-ized**, and does not contradict the semantic direction of
this section—the latter is the end point, the former is the migration-period channel)

### 9. Test System Layering: Language Corpus and Library Tests (Finalized 2026-09-03)

Tests are divided into two layers by the **object under test**, each with its own home and
maintainer; the marker system (§8.2) and assertion library (§3) are shared by both layers:

**Layer 1: Language-Availability Corpus (`tests/yaoxiang/`)**

- The object under test is the **language itself**—parser, type system, modules, concurrency,
  ownership, compile-time rejection, runtime semantics; the directory is organized by language-spec
  chapter
- std appears in the corpus only as an **assertion tool** (`std.assert` / `std.test`); it is never
  the object under test—the library's API behavior does not belong to language availability
- The corpus is split per §8 into three verdict categories: behavior tests / compile-time rejection
  tests / runtime-failure tests

**Layer 2: Library Tests (Alongside the Library)**

- The object under test is the **library's public API contract** (e.g., `list.push` behavior,
  `result.code` semantics)
- Tests are written **in the library's own package**: std's package is `src/std/`, and its yx-level
  tests belong to `src/std/tests/` (in the same place as the implementation); `std.test`'s own tests
  are also there (using std.test to test std.test, completing the bootstrap loop)
- Future user packages follow the same convention: tests inside the package, discovered via the
  package's `[tool.test]` (RFC-014's package management test layout is rehearsed by this)
- Discovery is not in the default patterns (the default `tests/**/*.yx` covers only the language
  layer): the library test layer is discovered via explicit paths (`yaoxiang test src/std/tests`) or
  package configuration; CI runs by layer

Migration notes: the library tests currently residing at `tests/yaoxiang/07-std/` migrate to
`src/std/tests/` per this model; each is examined during migration—pure API behavior tests migrate,
and tests where the object under test is actually a language boundary (such as native `&T`
auto-borrowing) remain in the corpus layer and are filed under the corresponding spec chapter

## Relationship with Existing Systems

| Item                                                 | Relationship                                                                                                      |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Unchanged; compiler-internal tests continue to use Rust                                                           |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                        |
| `std.assert.assert(cond)`                            | Reserved for runtime guards; `std.test` value-semantics assertion family is based on `std.result` (§3, §7)        |
| Module system (RFC-029)                              | Embedded source modules connect via Registry/orchestrator; CLI `run` connecting to orchestrator is a prerequisite |
| `#200` refactor (`io.println` → `assert.assert`)     | Exactly the same direction as `yaoxiang test`                                                                     |
| `@` annotation                                       | Not used; no `@test` introduced                                                                                   |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source-code branch delegates to
  `run_project` (multi-file run prerequisite)
- `src/main.rs` — add `Test` subcommand
- `src/std/test.yx` — add pure YaoXiang module
- `build.rs` — embed `std/*.yx` into the binary
- orchestrator / Registry — support loading `.yx` modules from embedded sources via virtual paths
- RFC-015 config parsing — `[tool.test]` section
- Subprocess execution (`--debug-info`) + reporting

Deliverables:

- `yaoxiang test` basically usable
- `std.test` 4 assertion functions
- Default `tests/**/*.yx` discovery
- Serial execution + default output format

### Phase 2: Refinement

- `--filter` / `--fail-fast` / `--verbose` parameters
- `--json` output (CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: Advanced

- `--parallel` parallel execution (depends on spawn concurrency model maturity)
- `[tool.test].exclude` configuration
- More assertion functions (e.g., `assert_approx_eq` for Float)

## Risks and Mitigations

| Risk                                                        | Probability | Mitigation                                                                                                                                                                                               |
| ----------------------------------------------------------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` interpolation fails on Any                         | None        | Verified 2026-08-02 (Int/String both work)                                                                                                                                                               |
| `yaoxiang.toml` config parsing not in current CLI           | Low         | Simple extension, doesn't affect core features                                                                                                                                                           |
| CLI run connecting to orchestrator introduces regressions   | Low         | No-import single-file path is equivalent; integration tests already cover orchestrator                                                                                                                   |
| Embedding `.yx` source files into the binary increases size | Low         | `.yx` source files are very small, negligible                                                                                                                                                            |
| Test loop time grows with corpus                            | High        | The main cost is full-file compilation (185 files measured at 11.3s), not subprocess startup; `--parallel` only alleviates the process side; compilation cost needs test-loop caching (#251/#293 slices) |

## Open Questions

- [x] Can the `use std.assert` reference in `std/test.yx` resolve correctly? — **Resolved
      (2026-08-02)**. After the module system (RFC-029) lands, native and source modules coexist in
      the Registry, the resolver unifies resolution, and cross-kind dependencies work naturally
- [x] Does the generic `to_string` of `f"..."` in test output introduce new type constraints? —
      **Resolved (2026-08-02)**. Verified that `==`/`!=` and f-string interpolation work on
      parameters without annotation (Any) (Int/String verified), no new constraints introduced
- [x] Is the `?` generic parameter feasible? — **Resolved (2026-08-02)**: the `?` type syntax does
      not currently exist (and is silently swallowed, tracked in a separate issue); Phase 1
      assertion functions use parameters without annotation, no dependency on the generics system

## Design Decision Log

| Decision                        | Decision                                                                                                                                                                                                                                  | Date                      | Rationale                                                                                                                                                                                                                                                                                                                                                               |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Test marker method              | Do not use `@test` annotations; test files are ordinary `.yx`                                                                                                                                                                             | 2026-07-26                | Zero compiler changes; subprocess equals isolation                                                                                                                                                                                                                                                                                                                      |
| Assertion method                | `std.test` module, pure YaoXiang functions                                                                                                                                                                                                | 2026-07-26                | Bootstrap; no native code                                                                                                                                                                                                                                                                                                                                               |
| Test execution model            | Subprocess `yaoxiang run <file>` + exit code                                                                                                                                                                                              | 2026-07-26                | Process-level isolation; zero compiler changes                                                                                                                                                                                                                                                                                                                          |
| Standard library loading        | Currently embed in binary; filesystem in the future                                                                                                                                                                                       | 2026-07-26                | Version binding; usable in single-file mode                                                                                                                                                                                                                                                                                                                             |
| Assertion parameter type        | Parameters without annotation (Any); no dependency on generics system                                                                                                                                                                     | 2026-08-02                | `?` type syntax doesn't exist; Any verified to be comparable and interpolable                                                                                                                                                                                                                                                                                           |
| Multi-file run                  | CLI `run` delegates to `run_project` (orchestrator) as prerequisite                                                                                                                                                                       | 2026-08-02                | Subprocess model inherits CLI capabilities; #247 degraded to pure performance optimization                                                                                                                                                                                                                                                                              |
| Report source location          | Subprocess with `--debug-info`                                                                                                                                                                                                            | 2026-08-02                | Verified stack trace outputs `file:line:col`; frame attribution via embedded modules (std.test) is not guaranteed here, belongs to #289 + RFC-034                                                                                                                                                                                                                       |
| Negative test layering          | Value-level reverse (general) / compile-failure runner structured markers (internal only) / hard failure folded into Result-ization                                                                                                       | 2026-09-02                | #319 finalized; replaces implicit [test:error] convention                                                                                                                                                                                                                                                                                                               |
| In-file multi-test              | Value-ized standard model: test functions return Result, suite collects per-test verdicts                                                                                                                                                 | 2026-09-02                | No catch, no per-entry call (entry only for internal scenarios)                                                                                                                                                                                                                                                                                                         |
| Error code                      | Error gains machine-readable `code` field                                                                                                                                                                                                 | 2026-09-02                | Supports error code assertions; compile-time codes go through runner comparison                                                                                                                                                                                                                                                                                         |
| Assertion library form          | Value-semantics 7-function family landed, `Result(Void, String)` contract; abort transitional version removed                                                                                                                             | 2026-09-03                | Void is the normative unit (`()` is the empty Tuple, not mixed); nested position is Any-rigid, parameters without annotation fail the native generic check—parameters must be explicitly annotated (verified by R1 probe)                                                                                                                                               |
| Test system layering            | Language corpus (`tests/yaoxiang/`) and library tests (alongside library; std → `src/std/tests/`) in two layers; std in the corpus is only an assertion tool                                                                              | 2026-09-03                | The object under test determines the home and maintainer; library tests following package layout rehearses RFC-014                                                                                                                                                                                                                                                      |
| Negative marker routing verdict | Route by expected category: compile-error category `check` must fail; runtime-error category `check` must pass + `run` must fail; report gives category counts                                                                            | 2026-09-03 (landed 09-06) | Mixed category verdicts allow "compile accidentally passes, runtime coincidentally fails" to be missed; the expected code nails down the stage; syntax errors do not have an independent category                                                                                                                                                                       |
| Header directive grammar        | Expectations declared with English structured directives (`// expect:` / `// skip:` / `// mode:`, strict token grammar, parse failure goes directly to FAIL); deprecate `[test:error]` boolean flag and Chinese `预期:` prose code-scrape | 2026-09-06                | Expectations are properties of fixture content, in-fixture declaration is isomorphic with the industry (compiletest / Go / GCC / Clang all do this), a central list will inevitably rot; boolean flag + expectation line as two facts coupled by discipline is a defect surface; structured grammar lets the runner make mechanical verdicts without human intervention |

## References

- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — Standard library
  directory structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]` config
  section
- [RFC-030: assert Assertion Mechanism](../review/030-assert-mechanism.md) — Underlying dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — Reference
  design
