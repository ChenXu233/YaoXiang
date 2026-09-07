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
YaoXiang. Test files are ordinary `.yx` files, judged pass/fail overall by the child process exit
code; files internally support multiple test functions—assertion failures are expressed as `Err`
values (value semantics), collected by the suite as per-test judgments (§7). The `std.test` module
is implemented in pure YaoXiang, making it the first dogfooding library. `yaoxiang test` is a CLI
tool, not a compiler feature—it involves no changes to the parser, IR, bytecode, or executor.

## Motivation

### Why is a testing framework needed?

Current YaoXiang test coverage depends on Rust-side `#[test]` and `tests/` integration tests. This
means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io)
   cannot be written in YaoXiang
2. `#117 Unit test coverage for each std module` is blocked, because no testing infrastructure is
   available
3. Regression tests for language features (e.g., RFC-032 spawn semantic changes) lack automation

### Key constraints

- **17-keyword iron rule**: do not introduce any new keywords or syntactic structures
- **Zero compiler changes**: do not touch the parser, IR, bytecode, or executor
- **Self-hosting first**: the testing library is written in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI layer:  yaoxiang test [--filter --fail-fast --json ...] │
│              │                                               │
│  Discovery:  read yaoxiang.toml → [tool.test] patterns       │
│              default: tests/**/*.yx                          │
│              │                                               │
│  Execution:  for each file: yaoxiang run <file>              │
│              check exit code → serial execution             │
│              │                                               │
│  Reporting:  PASS/FAIL → summary                             │
│              supports --json / --verbose / --fail-fast        │
│                                                              │
│  Assertion:  std.test (pure YaoXiang, self-hosted)           │
│              lower layer: std.assert.assert                  │
│              diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core principles

1. **The testing framework is not a compiler feature, it's a CLI tool** — `yaoxiang run` already can
   "execute tests"; `yaoxiang test` just helps you run all files and shows you the report
2. **Zero compiler changes** — no `@test` annotation scanning, no bytecode metadata segments, no
   executor special entry points
3. **Self-hosting** — the `std.test` module is implemented in pure YaoXiang, with lower-layer
   capabilities from `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files** — files run as child processes, exit code determines
   overall pass/fail
5. **Assertion failures are values, not process events** — test functions return `Result`, assertion
   failures are expressed as `Err`, the suite collects per-test judgments one by one (§7);
   process-level aborts belong only to runtime guards, not to test assertions

## Detailed Design

### 1. CLI design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Only run tests whose file name contains <NAME>
  --fail-fast         Stop on the first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              Only list test files, do not run
  --no-progress       Suppress progress output (header and PASS lines); FAIL details and summary are retained (CI scenario)
  --json              Output JSON format results (for CI integration)
  --parallel          Run in parallel (one worker per core; OR with [tool.test].parallel)
```

#### Output format

**Default output** (per-test judgments come from in-file suite collection, see §7):

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

- Failed files additionally carry `exit_code` and `stderr` (ANSI-stripped child process diagnostics,
  for CI forensics); when `--verbose` is combined with `--json`, all files carry `stdout` / `stderr`
- `--no-progress` only suppresses progress output (header and PASS lines) — FAIL details and summary
  are always output, failures cannot be silenced; `--list` outputs one test file path per line,
  without execution
- Each file carries `kind` (behavior / compile-error / runtime-error / invalid, §8.2), and the
  summary carries `by_kind` execution counts (fixed four keys, not including skipped); the human
  summary has an `Categories:` distribution line
- Under `--parallel`, human progress lines stream in **completion order** (blocks not interleaved),
  and JSON `files` are sorted by `file` path for stable output (CI diff friendly)
- The in-file per-test `tests` array comes from the §7 suite collection, taking effect when the
  value-typed model lands (#319)
- The official CI (`.github/workflows/ci.yml` test job) consumes this: the cargo side runs
  `--test integration` (CLI integration) and `--test yx_runner` (dual-root corpus guard), then
  `yaoxiang test --json --parallel` runs hierarchically—the default mode (language corpus) and
  explicit `src/std/tests` (library layer) each produce a report; the summary table (total / passed
  / failed / skipped / time_secs and by_kind) is written to the job summary, failed files print
  `kind` / `exit_code` / `stderr` for forensics, and any non-zero exit from any suite means red

### 2. yaoxiang.toml configuration

Placed under `[tool.test]`, conforming to RFC-015's `[tool.*]` third-party extension convention:

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # Matches are excluded from the discovery set (--list excludes them too)
parallel = true                   # Parallel execution (OR with --parallel flag)
```

- Default `patterns = ["tests/**/*.yx"]` — zero-config out of the box for users
- `exclude` has the same shape as `patterns` (literal paths or `root/**…`, always matched by path
  prefix); excluded means not a test, fixtures that need runtime behavior verification are run
  directly with `yaoxiang run`
- **Single-file mode (`yaoxiang test foo.yx`) runs directly without reading configuration** — under
  explicit paths, `exclude`/`parallel` configuration keys do not take effect (except flags)
- May be split into a separate repository in the future (`[tool.test]` position remains unchanged)

### 3. std.test module (pure YaoXiang)

```yaoxiang
// std/test.yx — Pure YaoXiang testing assertion library (value-semantics standard form, landed 2026-09-03)
// First dogfooding library: YaoXiang's testing library is written in YaoXiang.

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

- Assertion functions are **value-semantics**: they return `Result(Void, String)`, failures are
  expressed as `Err(diagnostic info)`, without aborting the process — the §7 suite collects per-test
  judgments based on this. The process-level abort semantics of `std.assert.assert` are reserved for
  runtime guards, not used in the test assertion path. The Ok payload is `Void` (the unit per the
  type-system.md spec; `()` is an empty Tuple, the two are not mixed — decision on 2026-09-03)
- **Function family of 7 (delivered on 2026-09-03, abort transitional version removed)**:
  value-typed `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not` (shares body
  with assert_false, reserved for `!assert` aliasing) + `assert_err` (§8.1)
  - `assert_err_code` (§8.1 error code assertion)
  - `assert_approx_eq(a: Float, b: Float, eps: Float)` (Phase 3, delivered 2026-09-07 ):
    `|a - b| <= eps` judgment, with eps **explicitly given by the caller** — tolerance is part of
    the test contract, no hidden default; negative eps is Err at the declaration site, NaN is always
    Err
- `assert_eq` / `assert_ne` use **Any-annotated parameters** — `==`/`!=` and f-string interpolation
  work normally on Any, not depending on the generics system. Note that parameters **must be
  explicitly annotated**: unannotated parameters fail the native generics `&Result(T, E)` call check
  (R1 probe verification)
- `assert_false` / `assert_not` use `cond == false` to express negation (`not` unary syntax is not
  yet landed, can be migrated after stabilization; the `!assert` unary form has the same dependency,
  see §8.1)
- Block body + explicit `return` form: the then-arm type of an if expression is discarded during
  checking, the if expression of two Result arms is a checking blind spot, so the implementation
  works around it
- `std.test` does not depend on any native code, implemented in pure YaoXiang

### 4. Standard library loading mechanism (key design)

**Phase 1: Embedded binary**

`std/test.yx` (and all future std modules written in YaoXiang) are embedded into the binary at build
time:

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

1. First check the Rust native modules (existing mechanism, e.g., `std.assert`)
2. If not found, check the embedded `STD_YX_FILES` — if matched, use a **virtual path** (e.g.,
   `<std>/test.yx`) as the seed module to inject into the orchestrator, going through the normal
   frontend pipeline (parse → typecheck → IR)
3. If not found, fall through to file system discovery (user modules)

The `use std.assert` inside embedded source modules is normally resolved by the resolver to the
native registry — native and source modules coexist in the Registry, so cross-kind dependencies
naturally work. Embedded modules are **compiled on demand**: they only enter the pipeline when
imported.

Advantages:

- `use std.test` works even in single-file mode
- The std version is strictly bound to the binary, no version mismatch
- Users don't need to configure std paths

**Future: file-system std**

Once YaoXiang's project mode matures, the std will switch to file-system form. See updates to
RFC-014.

### 5. Discovery and execution

**Prerequisite (decision on 2026-08-02)**: the CLI `run` hooks into the orchestrator. Currently the
CLI `run` goes through a single-file pipeline (`run_file_with_diagnostics`), unable to resolve user
module imports; and the child-process model of `yaoxiang test` inherits CLI capabilities, with test
files importing project modules being a core scenario. Therefore Phase 1 first delegates the CLI
`Run` source branch to `run_project` (orchestrator, directory-recursive discovery); #247 (on-demand
discovery along use) comes later as a pure performance optimization. A single file with no imports
behaves equivalently through the orchestrator, and the bytecode branch is unchanged.

**Discovery phase**:

1. If `[PATHS]` are specified, use those paths directly
2. Otherwise read `[tool.test].patterns` from `yaoxiang.toml`
3. If no configuration, default to `tests/**/*.yx`
4. Apply `--filter` (file name contains)
5. The discovery scope is the test layer (§9): default patterns only cover language-availability
   corpus; library test layers (e.g., `src/std/tests/`) are discovered via explicit paths or package
   configuration, not mixed into the default scan

**Execution phase**:

1. For each file, dispatch execution based on header directives (directive grammar in §8.2, parsed
   by `src/util/test_markers.rs`, shared with yx_runner):
   - Behavior tests: `yaoxiang run --debug-info <file>` child process (`--debug-info` makes runtime
     errors carry source location — verified on 2026-08-02 that stack trace outputs
     `file:line:col`); `// mode:` declares the child process `--runtime` mode
   - Compile-time rejection kind: single step `yaoxiang check <file>`
   - Runtime-failure kind: two steps — `check` (must pass) + `run` (must fail)
2. Files with `// skip: <reason>` skip execution, counted as skipped in the report
3. Judgment and expected code comparison follow the §8.2 judgment matrix; directive parse failure
   means no execution, direct FAIL (construction-time rejection)
4. Capture stdout/stderr for the report
5. Serial by default; `--parallel` (or `[tool.test].parallel`) spawns a worker pool by available
   cores, each file is still an independent child process — skip/invalid are processed first in
   discovery order, execution results stream in completion order, and JSON is sorted by path (Phase
   3, delivered 2026-09-07)
6. If `--fail-fast`, stop scheduling new files on the first FAIL; in parallel mode, in-flight files
   finish and are counted

### 6. Test isolation

Test isolation is naturally achieved through process-level boundaries:

- Each test file runs in an independent child process
- Each child process has independent Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional independent Heap context mechanism is needed
- **Parallel execution (Phase 3) does not extend isolation boundaries**: child processes share the
  working directory (CWD), so parallel tests must not occupy files at the same path in CWD — file
  I/O tests should use independent file names and clean up on completion

### 7. Suites and multiple tests (value-typed model)

A test file may contain multiple tests. The in-file organization (landed on 2026-09-03):

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

- Each test is a zero-argument function returning `Result(Void, String)`; assertion failures are
  expressed as `Err` (§3 value-semantics assertion family), without interrupting the process —
  subsequent tests continue to run
- `test.suite` calls them one by one and collects: a non-Ok test is recorded with name and
  diagnostic, Ok is silent; after all have run, if any Err exists, abort with `std.assert.assert`
  and attach failure details (`N of M test(s) failed` + each `[FAIL] name: diagnostic`) — the file's
  exit code is non-zero (§5 judgment unchanged). This abort is a runtime guard of the test binary,
  not part of the assertion path; all Ok exits silently with 0
- Top-level test functions enter as **closures** (`("name", () => test_fn())`): using the top-level
  function name as a value reference is not yet supported (IR-layer limitation, `E3006`) — closures
  calling global functions are not affected
- The runner only sees the file, no function-level scanning: per-test judgments come entirely from
  the in-suite collection, the in-file structure is transparent to the runner — the
  zero-compiler-change principle is unaffected
- Explicitly not adopted: in-process catch boundaries (17-keyword iron rule); runner calling
  functions one by one (only for internal scenarios like §8.2 compile failures)
- API form decided (2026-09-03, #319):
  `suite(tests: List((String, () -> Result(Void, String)))) -> Void`; no duplicate name detection
  (names are only used for report display); `--filter` filters by file name, not aware of in-suite
  test names

### 8. Three-layer design for negative tests (expected failure)

Negative tests are split by failure layer, with each layer in its proper place:

#### 8.1 Value-level reverse (general, user-facing)

The tested operation returns `Result`, and the test expresses the expected failure with ordinary
assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// Or a one-line wrapper (the code only exists on the std Error carrier, E is nailed to Error):
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` were delivered with the value-semantics family
  (2026-09-03, §3); the `!assert` unary form awaits the not syntax to land (same constraint as
  `assert_false`'s `cond == false`)
- Error code assertions depend on the `Error` value carrying a machine-readable `code` field —
  already landed via #323 M4: `Error = { code, message }` (native `error_new(code, message)`), read
  via `result.unwrap_err(r)` to get the carrier + `result.code(e)` / `result.message(e)` accessors
  (the design-phase estimated `error_new_with_code` naming, code constant exports, and `err.code`
  field access were all not adopted — the language has no Struct field access, code constants are
  not exported)
- As Result-ification progresses (#301, #316), operations that can fail return `Result` one by one,
  and file-level negative markers in the corpus migrate to in-file assertions accordingly

#### 8.2 File-level negative directives (only for language designers, internal use)

Compilation is all-or-nothing for a whole file, so "this line should not compile" cannot be
expressed in-file; runtime failure also needs file-level expression (e.g., a suite must contain a
failing test). The file header declares expectations with **structured directives**, and the runner
dispatches judgment by expected category (dispatch decision on 2026-09-03; directive grammar decided
and landed on 2026-09-06).

Header directive grammar (`// key: value`, within the first 16 lines, strict token match):

```text
// expect: compile-error E1002 [E1003 ...]   Compile-time rejection test
// expect: runtime-error E6003 [...]         Runtime-failure test
// skip: <reason>                            Skip execution, counted as skipped
// mode: embedded|standard|full              Child process --runtime mode (consumed only by the run step)
```

- No `expect:` directive = behavior test. `expect:` is the **sole declaration** of expectations —
  decided on 2026-09-06 to deprecate the `[test:error]` boolean marker and Chinese `预期:` prose
  code-picking: boolean markers and expectation lines are two loosely-coupled facts, kept consistent
  by discipline they will inevitably drift; the English strict token grammar lets the runner parse
  mechanically (kind + codes are fixed tokens, any extra tokens after a code means parse failure),
  parse failure = no execution, direct FAIL — no silent degradation channel for directive
  declaration errors (construction-time rejection)
- **Compile error kind**: single step `check` — must fail and output must contain all `[EXXXX]`;
  compilation success = FAIL (what should have been reported wasn't), rejected but codes don't match
  = FAIL. Syntax errors (E1xxx parse stage) and semantic errors (E2xxx+) are not given independent
  categories — the expected code itself nails down the stage
- **Runtime error kind**: two-step judgment — `check` must **succeed** (compile-time innocent),
  `run` must fail and output must contain all `[EXXXX]`. Compile-time explosion = FAIL (the opposite
  direction from the compile-error kind's key judgment, preventing "compile accidentally passes,
  runtime coincidentally fails" missed detection)
- Conforms to industry conventions: Rust compiletest `//~ ERROR`, Go `// ERROR "regexp"`, GCC
  `dg-error`, Clang `expected-error` all declare expectations within fixture comments and the
  harness bidirectionally compares; they use line-level anchoring because multi-diagnostic compilers
  need to distinguish multiple expectations in the same file, while this compiler stops on first
  error, one file one diagnostic, so file-level is isomorphic with compiler reality — after error
  recovery lands, the grammar can be extended with line-anchor form (Cranelift filetests'
  file-header directive + function-level expectation is the same mixed form)
- Known rendering debt: parse-stage diagnostics are currently output in Debug form (`code: "E0012"`
  rather than `[E0012]`), code scanning accepts both forms; strict form will be reinstated after
  diagnostic rendering is unified
- **Only serves this repository's corpus, not part of the user testing framework**; the dual-runner
  judgment convention is closed (2026-09-03, #319): yx_runner (cargo test) and `yaoxiang test` share
  `src/util/test_markers.rs` to parse header directives, the 06-compile-errors directory convention
  is deprecated. The report layer gives category counts: the human summary has a `Categories:` line,
  JSON summary carries `by_kind` (behavior / compile-error / runtime-error / invalid, skipped
  counted separately), each file carries `kind`

#### 8.3 Runtime hard failures (folded into Result-ification)

No independent mechanism — operations that can fail return `Result` per the language direction
(#301, #316), and tests uniformly go through §8.1. Process-level aborts (e.g., assertion violation,
runtime parameter mismatch) converge to values as Result-ification progresses, and the testing
framework provides no dedicated semantics for them. (Note: the "runtime error kind" marker judgment
in §8.2 is the runner's file-level verification channel for **operations that cannot yet be
Result-ified**, not contradicting the semantic direction of this section — the latter is the end
state, the former is the migration-period channel)

### 9. Test system layering: language corpus and library tests (decided 2026-09-03)

Tests are divided into two layers by **the object under test**, each with its own ownership and
maintainer; the marker system (§8.2) and assertion library (§3) are shared across both layers:

**Layer 1: Language-availability corpus (`tests/yaoxiang/`)**

- The object under test is **the language itself** — parser, type system, modules, concurrency,
  ownership, compile-time rejection, runtime semantics; directories are organized by language spec
  chapters
- std in the corpus only serves as an **assertion tool** (`std.assert` / `std.test`), never tested —
  library API behavior does not belong to language availability
- The corpus is dispatched per §8 into three judgment kinds: behavior tests / compile-time rejection
  tests / runtime-failure tests

**Layer 2: Library tests (live with the library)**

- The object under test is **the library's public API contract** (e.g., `list.push` behavior,
  `result.code` semantics)
- Tests are written **in the library's own package**: std's package is `src/std/`, and its yx-level
  tests go in `src/std/tests/` (co-located with the implementation; the directory coexists with Rust
  unit tests, file types don't intersect); `std.test`'s own tests are also there (using std.test to
  test std.test, self-hosting loop closure)
- Future user packages follow the same convention: tests are in the package, discovered via the
  package's `[tool.test]` (RFC-014 package management test layout is thus previewed)
- Discovery does not enter default patterns (default `tests/**/*.yx` only covers the language
  layer): library test layers are discovered via explicit paths (`yaoxiang test src/std/tests`) or
  package configuration; CI runs in layers

Migration notes: **Migrated (2026-09-06)** — the 19 files originally in `tests/yaoxiang/07-std/`
were screened one by one and all turned out to be library tests (the objects under test are all std
module API contracts; language features like `?` propagation, automatic borrowing, generics
instantiation in them serve as carriers rather than the objects under test), and the whole directory
was migrated into `src/std/tests/` and the 07-std directory removed; yx_runner changed to dual-root
discovery (`tests/yaoxiang/` + `src/std/tests/`), default patterns don't include the library layer
(integration tests cement this contract). The language corpus contains zero std-API tests from this
point on.

## Relationship with existing systems

| Item                                                 | Relationship                                                                                                           |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched, compiler-internal tests continue to use Rust                                                                |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                             |
| `std.assert.assert(cond)`                            | Reserved for runtime guards; the `std.test` value-semantics assertion family is based on `std.result` (§3, §7)         |
| Module system (RFC-029)                              | Embedded source modules are hooked in via Registry/orchestrator; CLI `run` hooking into orchestrator is a prerequisite |
| `#200` refactor (`io.println` → `assert.assert`)     | Exactly the same direction as `yaoxiang test`                                                                          |
| `@` annotation                                       | Not used, `@test` is not introduced                                                                                    |

## Implementation strategy

### Phase 1: Core functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project`
  (multi-file run prerequisite)
- `src/main.rs` — add `Test` subcommand
- `src/std/test.yx` — new pure YaoXiang module
- `build.rs` — embed `std/*.yx` into the binary
- orchestrator / Registry — support loading `.yx` modules from embedded sources via virtual paths
- RFC-015 config parsing — `[tool.test]` section
- Child process execution (`--debug-info`) + reporting

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

### Phase 3: Advanced (delivered 2026-09-07)

- `--parallel` parallel execution (worker pool + independent child process per file;
  `[tool.test].parallel` config key has the same effect)
- `[tool.test].exclude` config (prefix-match exclusion, `--list` also excludes)
- `assert_approx_eq` (Float explicit eps assertion, §3)

## Risks and mitigations

| Risk                                                             | Probability | Mitigation                                                                                                                                                                                                  |
| ---------------------------------------------------------------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` interpolation on Any fails                              | None        | Verified on 2026-08-02 (Int/String work normally)                                                                                                                                                           |
| `yaoxiang.toml` config parsing not in current CLI                | Low         | Simple extension, doesn't affect core functionality                                                                                                                                                         |
| CLI run hooking into orchestrator introduces behavior regression | Low         | No-import single-file path is equivalent; integration tests cover the orchestrator                                                                                                                          |
| Embedding `.yx` source files into binary increases size          | Low         | `.yx` source files are tiny, negligible                                                                                                                                                                     |
| Test loop time grows with corpus                                 | High        | The main cost is full compilation per file (185 files measured at 11.3s), not subprocess startup; `--parallel` only mitigates the process side, compilation cost needs test loop caching (#251/#293 slices) |

## Open questions

- [x] Can `use std.assert` in `std/test.yx` be resolved correctly? — **Resolved (2026-08-02)**.
      After the module system (RFC-029) landed, native and source modules coexist in the Registry,
      the resolver unifies resolution, cross-kind dependencies naturally work
- [x] Does the generics `to_string` in `f"..."` in test output introduce new type constraints? —
      **Resolved (2026-08-02)**. Verified that on unannotated parameters (Any) `==`/`!=` and
      f-string interpolation both work (Int/String verified), no new constraints introduced
- [x] `?` generics parameter feasibility? — **Resolved (2026-08-02)**: the `?` type syntax does not
      currently exist (and is silently swallowed, tracked in a separate issue), Phase 1 assertion
      functions use unannotated parameters, not depending on the generics system

## Design decision log

| Decision                          | Decision                                                                                                                                                                                                                                                                                                                    | Date                      | Rationale                                                                                                                                                                                                                                                                                                                                                  |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Test marker style                 | No `@test` annotation, test files are ordinary `.yx`                                                                                                                                                                                                                                                                        | 2026-07-26                | Zero compiler changes, subprocess as isolation                                                                                                                                                                                                                                                                                                             |
| Assertion style                   | `std.test` module of pure YaoXiang functions                                                                                                                                                                                                                                                                                | 2026-07-26                | Self-hosting, no native code                                                                                                                                                                                                                                                                                                                               |
| Test execution model              | Subprocess `yaoxiang run <file>` + exit code                                                                                                                                                                                                                                                                                | 2026-07-26                | Process-level isolation, zero compiler changes                                                                                                                                                                                                                                                                                                             |
| Std loading                       | Currently embed in binary, file system in the future                                                                                                                                                                                                                                                                        | 2026-07-26                | Version binding, single-file usable                                                                                                                                                                                                                                                                                                                        |
| Assertion parameter type          | Unannotated parameters (Any), not depending on the generics system                                                                                                                                                                                                                                                          | 2026-08-02                | `?` type syntax doesn't exist; Any is verified to be comparable and interpolable                                                                                                                                                                                                                                                                           |
| Multi-file run                    | CLI `run` delegates to `run_project` (orchestrator) as a prerequisite                                                                                                                                                                                                                                                       | 2026-08-02                | Subprocess model inherits CLI capabilities; #247 degrades to a pure performance optimization                                                                                                                                                                                                                                                               |
| Reporting source location         | Subprocess carries `--debug-info`                                                                                                                                                                                                                                                                                           | 2026-08-02                | Verified stack trace outputs `file:line:col`; frame attribution relayed through embedded modules (std.test) is not guaranteed here, belongs to #289 + RFC-034                                                                                                                                                                                              |
| Negative test layering            | Value-level reverse general / compile-failure runner structured marker (internal only) / hard failure folded into Result-ification                                                                                                                                                                                          | 2026-09-02                | #319 decided; replaces implicit [test:error] convention                                                                                                                                                                                                                                                                                                    |
| In-file multiple tests            | Value-typed standard model: test functions return Result, suite collects per-test judgments                                                                                                                                                                                                                                 | 2026-09-02                | No catch, no entry-point calls (entries only for internal scenarios)                                                                                                                                                                                                                                                                                       |
| Error code                        | Error adds machine-readable `code` field                                                                                                                                                                                                                                                                                    | 2026-09-02                | Supports error code assertion; compile-time codes go through runner comparison                                                                                                                                                                                                                                                                             |
| Assertion library form            | Value-semantics family of 7 functions landed, `Result(Void, String)` contract; abort transitional version removed                                                                                                                                                                                                           | 2026-09-03                | Void is the spec'd unit (`()` is empty Tuple, not mixed); nesting is rigid Any, unannotated parameters fail native generics check — parameters must be explicitly annotated (R1 probe verified)                                                                                                                                                            |
| Test system layering              | Language corpus (`tests/yaoxiang/`) and library tests (live with library, std → `src/std/tests/`) in two layers; std in the corpus only as an assertion tool                                                                                                                                                                | 2026-09-03                | Object under test determines ownership and maintainer; library tests living with the package previews RFC-014                                                                                                                                                                                                                                              |
| Negative marker dispatch judgment | Dispatch by expected category: compile-error kind `check` must fail, runtime-error kind `check` must pass + `run` must fail; report gives category counts                                                                                                                                                                   | 2026-09-03 (landed 09-06) | Mixed-kind judgment lets "compile accidentally passes, runtime coincidentally fails" slip through; expected codes nail down the stage, syntax errors don't get a separate category                                                                                                                                                                         |
| Header directive grammar          | Expectations declared with English structured directives (`// expect:` / `// skip:` / `// mode:`, strict token grammar, parse failure means direct FAIL); deprecate `[test:error]` boolean marker and Chinese `预期:` prose code-picking                                                                                    | 2026-09-06                | Expectations are properties of fixture content, in-fixture declaration is isomorphic with industry (compiletest / Go / GCC / Clang all do this), central lists inevitably rot; boolean marker + expectation line as two facts coupled by discipline is a defect surface; structured grammar lets the runner judge mechanically without human participation |
| Parallel execution model          | `--parallel` spawns per-core worker pool (each file still an independent child process), skip/invalid processed first in discovery order, execution results stream in completion order, JSON sorted by path; `--fail-fast` stops scheduling (in-flight finish and count); CWD still shared, isolation boundary not extended | 2026-09-07                | Subprocess model parallelism = OS thread scheduling spawn, no need for yx-level concurrency; the main time cost is full compilation per file (risk table), parallel only mitigates the process side — #293 caching slicing is the main mitigation                                                                                                          |

## References

- [RFC-014: Package management system design](../accepted/014-package-manager.md) — std directory
  structure
- [RFC-015: Configuration system](../accepted/015-configuration-system.md) — `[tool.test]` config
  section
- [RFC-030: assert mechanism](../review/030-assert-mechanism.md) — underlying dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — reference
  design
