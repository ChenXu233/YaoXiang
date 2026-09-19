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

Introduce a standard testing framework `std.test` module and `yaoxiang test` CLI subcommand for
YaoXiang. Test files are regular `.yx` files; pass/fail is determined by the child process's exit
code; files support multiple test functions internally—assertion failures are expressed as `Err`
values (value semantics), with per-test verdicts collected by the suite (§7). The `std.test` module
is implemented in pure YaoXiang and is the first dogfooding library. `yaoxiang test` is a CLI tool,
not a compiler feature—it does not involve any changes to the parser, IR, bytecode, or executor.

## Motivation

### Why do we need a testing framework?

YaoXiang's current test coverage relies on Rust-side `#[test]` and `tests/` integration tests. This
means:

1. The standard library (std.math / std.list / std.dict / std.convert / std.io) cannot have unit
   tests written in YaoXiang
2. Unit test coverage for standard library modules is blocked because there is no usable test
   infrastructure
3. Regression tests for language features (e.g., RFC-032 spawn semantic changes) lack automation

### Key Constraints

- **17-keyword iron rule**: No new keywords or syntactic structures introduced
- **Zero compiler changes**: Do not touch the parser, IR, bytecode, or executor
- **Bootstrap-first**: Test library written in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI layer:   yaoxiang test [--filter --fail-fast --json ...]│
│              │                                               │
│  Discovery:   Read yaoxiang.toml → [tool.test] patterns      │
│               Default: tests/**/*.yx                         │
│              │                                               │
│  Execution:   For each file: yaoxiang run <file>             │
│               Check exit code → serial execution             │
│              │                                               │
│  Reporting:   PASS/FAIL → summary                            │
│               Supports --json / --verbose / --fail-fast      │
│                                                              │
│  Assertions:  std.test (pure YaoXiang, self-hosted)          │
│               Lower layer: std.assert.assert                 │
│               Diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **The testing framework is not a compiler feature, it is a CLI tool**—`yaoxiang run` can already
   "execute tests"; `yaoxiang test` just helps you run all the files and shows you a report
2. **Zero compiler changes**—no `@test` annotation scanning, bytecode metadata segments, or executor
   special entry points
3. **Self-hosted**—the `std.test` module is implemented in pure YaoXiang, with lower-level
   capabilities from `std.assert` / `std.result`
4. **Test files are regular `.yx` files**—files run as child processes, pass/fail determined by exit
   code
5. **Assertion failure is a value, not a process event**—test functions return `Result`, assertion
   failures are expressed as `Err`, the suite collects per-test verdicts (§7); process-level abort
   belongs only to runtime guards, not test assertions

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
  --list              List test files only, do not run
  --no-progress       Suppress progress output (header and PASS lines); FAIL details and summary preserved (for CI scenarios)
  --json              Output results in JSON format (for CI integration)
  --parallel          Execute in parallel (one worker per core; OR-ed with [tool.test].parallel)
```

#### Output Format

**Default output** (per-test verdicts from in-file suite collection, see §7):

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

- Failed files additionally carry `exit_code` and `stderr` (child process diagnostics with ANSI
  stripped, for CI forensics); when `--verbose` and `--json` are combined, all files carry `stdout`
  / `stderr`
- `--no-progress` only suppresses progress output (header and PASS lines)—FAIL details and summary
  always output; failures cannot be silently dropped; `--list` outputs one test file path per line,
  does not execute
- Each file carries `kind` (behavior / compile-error / runtime-error / invalid, §8.2), summary
  carries `by_kind` execution counts (fixed four keys, excluding skipped); the human summary
  includes a `Categories:` distribution line
- Under `--parallel`, the human progress lines stream in completion order (whole blocks do not
  interleave); JSON `files` is sorted by `file` path for stable output (CI-diff friendly)
- The in-file per-test `tests` array comes from §7 suite collection, effective with the value-typed
  model
- The official CI (`.github/workflows/ci.yml` test job) consumes this: the cargo side runs
  `--test integration` (CLI integration) and `--test yx_runner` (dual-root corpus guard), then
  `yaoxiang test --json --parallel` runs in layers—default mode (language corpus) and explicit
  `src/std/tests` (library layer) each produce a report; the summary table (total / passed / failed
  / skipped / time_secs and by_kind) is written to the job summary; failed files print `kind` /
  `exit_code` / `stderr` for forensics; any non-zero exit suite is red

### 2. yaoxiang.toml Configuration

Placed under `[tool.test]`, conforming to RFC-015's `[tool.*]` third-party extension convention:

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # Matched entries are removed from the discovery set (--list also removes them)
parallel = true                   # Parallel execution (OR-ed with --parallel flag)
```

- Default `patterns = ["tests/**/*.yx"]`—zero-config out of the box
- `exclude` shares the same form as `patterns` (literal path or `root/**…`, always matched by path
  prefix); excluded means not a test; fixtures that need runtime behavior validation go through
  `yaoxiang run` directly
- **Single-file mode (`yaoxiang test foo.yx`) runs directly without reading config**—with explicit
  paths, both `exclude` and `parallel` config keys are ineffective (except for flags)
- May be split into a separate repository in the future (the `[tool.test]` position remains
  unchanged)

### 3. std.test Module (Pure YaoXiang)

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library (value semantics standard form, landed 2026-09-03)
// First dogfooding library: YaoXiang's test library is written in YaoXiang.

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

- Assertion functions follow **value semantics**: return `Result(Void, String)`, failures expressed
  as `Err(diagnostic info)`, do not abort the process—§7 suites collect per-test verdicts based on
  this. The process-level abort semantics of `std.assert.assert` are reserved for runtime guards and
  do not enter the test assertion path. The Ok payload is `Void` (unit per type-system.md spec; `()`
  is an empty Tuple, the two are not mixed—decided 2026-09-03)
- **Function family of 7 (delivered 2026-09-03, abort transitional version removed)**: value-typed
  `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not` (same body as
  assert_false, reserved for `!assert` reshaping) + `assert_err` (§8.1)
  - `assert_err_code` (§8.1 error code assertion)
  - `assert_approx_eq(a: Float, b: Float, eps: Float)` (Phase 3, delivered 2026-09-07):
    `|a - b| <= eps` judgment, eps is **explicitly given** by the caller—tolerance is part of the
    test contract, no hidden default; negative eps at the declaration site is an Err, NaN is always
    Err
- `assert_eq` / `assert_ne` use **Any-typed parameters**—`==`/`!=` and f-string interpolation work
  normally on Any, not depending on the generics system. Note that parameters **must be explicitly
  annotated**: unannotated parameters fail the native generics `&Result(T, E)` call check (R1 probe
  empirical)
- `assert_false` / `assert_not` use `cond == false` to express negation (`not` unary syntax not
  landed, can migrate when stable; the `!assert` unary form has the same dependency, see §8.1)
- Block body + explicit `return` form: the then-arm type of the if expression is discarded during
  checking; the if expression with both arms as Result is a checking blind spot, implementation
  works around it
- `std.test` does not depend on any native code, implemented in pure YaoXiang

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedded Binary**

`std/test.yx` (and all future YaoXiang-written standard library modules) is embedded in the binary
at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source text
    // more in the future
];
```

The module system (RFC-029, fully landed 2026-08-02) provides the entry point: Registry holds both
native modules and source modules, orchestrator handles multi-file orchestration. The parsing order
of `use std.test`:

1. First check Rust native modules (existing mechanism, e.g., `std.assert`)
2. If not found, check embedded `STD_YX_FILES`—if matched, the **virtual path** (e.g.,
   `<std>/test.yx`) is used as the seed module and injected into the orchestrator, going through the
   normal frontend pipeline (parse → typecheck → IR)
3. If not found, go to file system discovery (user modules)

The `use std.assert` inside an embedded source module is normally resolved by the resolver to the
native registry—native and source modules coexist in the Registry, and cross-type dependencies
naturally hold. Embedded modules **compile on demand**: only enter the pipeline when imported.

Advantages:

- `use std.test` works in single-file mode
- Standard library version is strictly bound to the binary, no version mismatch
- No need for users to configure standard library paths

**Future: File System Standard Library**

When YaoXiang's project mode matures, the standard library will switch to file system form. See
updates to RFC-014.

### 5. Discovery and Execution

**Prerequisites (2026-08-02 review decision)**: CLI `run` accesses the orchestrator. Currently CLI
`run` goes through the single-file pipeline (`run_file_with_diagnostics`), unable to resolve user
module imports; and `yaoxiang test`'s child process model inherits CLI capabilities, with test files
importing project modules being a core scenario. Therefore Phase 1 first delegates the CLI `Run`
source branch to `run_project` (orchestrator, directory recursive discovery); on-demand discovery
along `use` is later layered as a pure performance optimization. A single file with no imports
behaves equivalently through the orchestrator, and the bytecode branch is unchanged.

**Discovery phase**:

1. If `[PATHS]` is specified, use the specified paths directly
2. Otherwise read `[tool.test].patterns` from `yaoxiang.toml`
3. If no config, default `tests/**/*.yx`
4. Apply `--filter` (filename contains)
5. The discovery scope is the test layering (§9): default patterns only cover language-availability
   corpus; library test layer (e.g., `src/std/tests/`) is discovered via explicit path or package
   config, not mixed into the default scan

**Execution phase**:

1. For each file, route execution by header directives (directive grammar see §8.2, parsed via
   `src/util/test_markers.rs`, shared with yx_runner):
   - Behavior test: `yaoxiang run <file>` child process (runtime errors by default carry source
     location and stack frames—debug_map generated by default, stack trace output `file:line:col`);
     `// mode:` declares the child process `--runtime` mode
   - Compile-time rejection: single step `yaoxiang check <file>`
   - Runtime failure: two steps `check` (must pass) + `run` (must fail)
2. Files with `// skip: <reason>` skip execution, counted as skipped in the report
3. Verdict and expected code comparison follows the §8.2 verdict matrix; directive parse failure
   does not execute and directly FAILS (construction-time rejection)
4. Capture stdout/stderr for the report
5. Serial by default; `--parallel` (or `[tool.test].parallel`) starts a worker pool by available
   cores, each file is still an independent child process—skip/invalid processed in discovery order
   first, execution results stream in completion order, JSON sorted by path (Phase 3, delivered
   2026-09-07)
6. If `--fail-fast`, stop scheduling new files at the first FAIL; in parallel mode, in-flight files
   finish and are counted

### 6. Test Isolation

Test isolation is naturally achieved through process-level boundaries:

- Each test file runs in an independent child process
- Each child process has independent Heap, Frame, NativeContext
- A panic in one test file does not affect other test files
- No additional independent Heap context mechanism needed
- **Parallel execution (Phase 3) does not extend isolation boundaries**: child processes share the
  working directory (CWD), parallel tests must not occupy files at the same path within CWD—file I/O
  tests use distinct file names and clean up on finish

### 7. Suite and Multiple Tests (Value-Typed Model)

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

main: () -> Void = {
    test.suite([
        ("push_grows_len", () => push_grows_len()),
        ("pop_returns_last", () => pop_returns_last()),
    ])
}
```

- Each test is a zero-arg function returning `Result(Void, String)`; assertion failures are
  expressed as `Err` (§3 value semantics assertion family), do not interrupt the process—subsequent
  tests run as usual
- `test.suite` calls one by one and collects: if a test is not Ok, record the name and diagnostic,
  Ok is silent; after all complete, if any Err exists, abort with `std.assert.assert` and attach
  failure details (`N of M test(s) failed` + each `[FAIL] name: diagnostic`)—the file's exit code is
  non-zero (§5 verdict unchanged). The abort here is a runtime guard of the test binary, not the
  assertion path; all Ok silently exits 0
- Top-level test functions are enqueued as **closures** (`("name", () => test_fn())`): top-level
  function names as value references are not yet supported (IR-layer limitation, `E3006`)—calling
  global functions in closure bodies is unaffected
- The runner only sees the file, no function-level scanning: per-test verdicts come entirely from
  in-suite collection, the file's internal structure is transparent to the runner—the
  zero-compiler-change principle is unaffected
- Explicitly not adopted: in-process catch boundaries (17-keyword iron rule); runner calling each
  function entry by entry (only for internal scenarios like §8.2 compile failures)
- API form decided (2026-09-03): `suite(tests: List((String, () -> Result(Void, String)))) -> Void`;
  duplicate names are not detected (names are only used for report display); `--filter` filters by
  filename, not aware of in-suite test names

### 8. Three-Layer Design for Negative Tests (Expected Failure)

Negative tests are split by the layer where the failure occurs, each in its proper place:

#### 8.1 Value-Level Reverse (General, User-Facing)

The operation under test returns `Result`, and tests express expected failure with normal
assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// Or a one-liner wrapper (code only exists on std Error carrier, E pinned to Error):
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` delivered with the value semantics family
  (2026-09-03, §3); `!assert` unary form pending `not` syntax landing (same constraint as
  `assert_false`'s `cond == false`)
- Error code assertion depends on `Error` values carrying a machine-readable `code` field—landed:
  `Error = { code, message }` (native `error_new(code, message)`), reading goes through
  `result.unwrap_err(r)` for the carrier + `result.code(e)` / `result.message(e)` accessors (the
  design-phase estimated `error_new_with_code` naming, code constant exports, and `err.code` field
  access were all not adopted—the language has no Struct field access, code constants not exported)
- As Result-ization progresses, operations that can fail return `Result` one by one, and the
  file-level negative markers in the corpus migrate to in-file assertions accordingly

#### 8.2 File-Level Negative Directives (For Language Designers Internal Use Only)

Compilation is all-or-nothing for a file, so "this line should not compile" cannot be expressed
in-file; runtime failures similarly need file-level expression (e.g., a suite contains a test that
must fail). The file header declares expectations with **structured directives**, and the runner
routes verdicts by expected category (decided 2026-09-03 routing; directive grammar decided and
landed 2026-09-06).

Header directive grammar (`// key: value`, within the first 16 lines, strict token matching):

```text
// expect: compile-error E1002 [E1003 ...]   Compile-time rejection test
// expect: runtime-error E6003 [...]         Runtime failure test
// skip: <reason>                            Skip execution, counted as skipped
// mode: embedded|standard|full              Child process --runtime mode (consumed only by run step)
```

- No `expect:` directive = behavior test. `expect:` is the **sole declaration** of
  expectation—2026-09-06 decision to deprecate the `[test:error]` boolean flag and the Chinese
  `预期:` prose code-scraping: the boolean flag and expectation line are two loosely coupled facts,
  relying on discipline to keep them consistent is destined to drift; the strict English token
  grammar lets the runner parse mechanically (kind + codes are all fixed tokens, any extra tokens
  after codes is a parse failure), parse failure = no execution, direct FAIL—the silent degradation
  channel for directive declaration errors does not exist (construction-time rejection)
- **Compile error class**: single-step `check`—must fail and output must contain all `[EXXXX]`;
  compilation pass = FAIL (the expected one was not reported), rejection with mismatched codes =
  FAIL. Syntax errors (E1xxx parse phase) and semantic errors (E2xxx+) are not separate
  categories—the expected code itself pins the phase
- **Runtime error class**: two-step verdict—`check` must **succeed** (innocent at compile time),
  `run` must fail and output must contain all `[EXXXX]`. Compile-time explosion = FAIL (the verdict
  in the opposite direction from the compile error class, preventing "compile accidentally passes,
  runtime luckily fails" missed detection)
- Conforms to industry: Rust compiletest `//~ ERROR`, Go `// ERROR "regexp"`, GCC `dg-error`, Clang
  `expected-error` all declare expectations in fixture comments and the harness compares
  bidirectionally; they use line-level anchoring because multi-diagnostic compilers need to
  distinguish multiple expectations in the same file; our compiler stops at the first error, one
  diagnostic per file, so file-level matches compiler reality—after error recovery lands,
  line-anchor form can be appended to the grammar (Cranelift filetests' file header directives +
  function-level expectations is the same hybrid form)
- Known rendering debt: parse-phase diagnostics currently output in Debug form (`code: "E0012"`
  instead of `[E0012]`), code scanning accepts both forms; the strict form will be reclaimed after
  diagnostic rendering is unified
- **Only serves this repository's corpus, not part of the user test framework**; the dual-runner
  verdict agreement is closed (2026-09-03): yx_runner (cargo test) and `yaoxiang test` share
  `src/util/test_markers.rs` for parsing header directives, the 06-compile-errors directory
  convention is deprecated. The reporting layer gives category counts: human summary includes
  `Categories:` line, JSON summary includes `by_kind` (behavior / compile-error / runtime-error /
  invalid, skipped counted separately), each file carries `kind`

#### 8.3 Runtime Hard Failure (Subsumed into Result-ization)

No independent mechanism—the operations that can fail return `Result` per the language direction,
tests uniformly go through §8.1. Process-level abort (e.g., assertion violations, runtime parameter
misalignment) gradually converges to values with Result-ization, the test framework does not provide
dedicated semantics for it. (Note: the "runtime error class" marker verdict in §8.2 is the runner's
file-level verification channel for **operations that cannot yet be Result-ized**, which is not
contradictory with the semantic direction of this section—the latter is the end state, the former is
the migration-period channel)

### 9. Test System Layering: Language Corpus and Library Tests (Decided 2026-09-03)

Tests are split into two layers by **object under test**, each with its own home and maintainer; the
marker system (§8.2) and assertion library (§3) are shared at both levels:

**Layer 1: Language Availability Corpus (`tests/yaoxiang/`)**

- Object under test is the **language itself**—parser, type system, modules, concurrency, ownership,
  compile-time rejection, runtime semantics; directories organized by language specification
  chapters
- std in the corpus is only used as an **assertion tool** (`std.assert` / `std.test`), never
  tested—the library's API behavior does not belong to language availability
- The corpus is split into three verdict categories per §8: behavior test / compile-time rejection
  test / runtime failure test

**Layer 2: Library Tests (Live with the Library)**

- Object under test is the **library's public API contract** (e.g., `list.push` behavior,
  `result.code` semantics)
- Tests are written in the **library's own package**: std's package is `src/std/`, its yx-level
  tests belong in `src/std/tests/` (co-located with the implementation body; the directory coexists
  with Rust unit tests, file types do not intersect); `std.test`'s own tests are also there (using
  std.test to test std.test, self-hosting closed loop)
- Future user packages follow the same convention: tests in the package, discovered via the
  package's `[tool.test]` (RFC-014 package management's test layout is previewed here)
- Discovery does not enter default patterns (default `tests/**/*.yx` only covers the language
  layer): the library test layer is discovered via explicit path (`yaoxiang test src/std/tests`) or
  package config; CI runs in layers

Migration note: **Migrated (2026-09-06)**—the original 19 files in `tests/yaoxiang/07-std/` were
screened one by one and all turned out to be library tests (the objects under test are all std
module API contracts; language features like `?` propagation, automatic borrowing, generics
instantiation play the role of carriers rather than objects under test); all moved into
`src/std/tests/` and the 07-std directory was removed; yx_runner changed to dual-root discovery
(`tests/yaoxiang/` + `src/std/tests/`), default patterns do not include the library layer
(integration tests solidify this contract). Language corpus has zero std-API tests since then.

## Relationship with Existing Systems

| Item                                                 | Relationship                                                                                                     |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched, compiler-internal tests continue to use Rust                                                          |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                       |
| `std.assert.assert(cond)`                            | Reserved for runtime guards; `std.test` value semantics assertion family is based on `std.result` (§3, §7)       |
| Module system (RFC-029)                              | Embedded source modules accessed via Registry/orchestrator; CLI `run` accessing orchestrator is the prerequisite |
| Corpus refactoring (`io.println` → `assert.assert`)  | Direction fully consistent with `yaoxiang test`                                                                  |
| `@` annotation                                       | Not used, no `@test` introduced                                                                                  |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project`
  (multi-file run prerequisite)
- `src/main.rs` — Add `Test` subcommand
- `src/std/test.yx` — Add pure YaoXiang module
- `build.rs` — Embed `std/*.yx` into the binary
- orchestrator / Registry — Support loading `.yx` modules from embedded sources via virtual paths
- RFC-015 config parsing — `[tool.test]` section
- Child process execution + reporting

Deliverables:

- `yaoxiang test` basically usable
- `std.test` 4 assertion functions
- Default `tests/**/*.yx` discovery
- Serial execution + default output format

### Phase 2: Polish

- `--filter` / `--fail-fast` / `--verbose` parameters
- `--json` output (CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: Advanced (Delivered 2026-09-07)

- `--parallel` parallel execution (worker pool + independent child process per file;
  `[tool.test].parallel` config key has the same effect)
- `[tool.test].exclude` config (prefix match removal, `--list` also removes)
- `assert_approx_eq` (Float explicit eps assertion, §3)

## Risks and Mitigation

| Risk                                                            | Probability | Mitigation                                                                                                                                                                                 |
| --------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `f"..."` interpolation on Any fails                             | None        | Empirically verified 2026-08-02 (Int/String both work)                                                                                                                                     |
| `yaoxiang.toml` config parsing not in current CLI               | Low         | Simple extension, does not affect core functionality                                                                                                                                       |
| CLI run accessing orchestrator introduces behavioral regression | Low         | No-import single-file path equivalent; integration tests already cover orchestrator                                                                                                        |
| Embedding `.yx` source files in binary increases size           | Low         | `.yx` source files are very small, negligible                                                                                                                                              |
| Test loop time grows with corpus                                | High        | Main cost is per-file full compilation (185 files measured 11.3s), not child process startup; `--parallel` only mitigates the process side, compilation cost needs test loop cache slicing |

## Open Questions

- [x] Can `use std.assert` references in `std/test.yx` be correctly resolved?—**Resolved
      (2026-08-02)**. After the module system (RFC-029) landed, native and source modules coexist in
      the Registry, resolver uniformly resolves, cross-type dependencies naturally hold
- [x] Does generic `to_string` in `f"..."` test output introduce new type constraints?—**Resolved
      (2026-08-02)**. Empirically verified that on unannotated parameters (Any), `==`/`!=` and
      f-string interpolation both work (Int/String verified), no new constraints introduced
- [x] Feasibility of `?` generic parameters?—**Resolved (2026-08-02)**: `?` type syntax does not
      currently exist (and would be silently swallowed, a separate issue tracks this); Phase 1
      assertion functions use unannotated parameters, not depending on the generics system

## Design Decision Log

| Decision                       | Determination                                                                                                                                                                                                                                                                                                                | Date                      | Reason                                                                                                                                                                                                                                                                                                                                              |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Test marker approach           | No `@test` annotation, test files are regular `.yx`                                                                                                                                                                                                                                                                          | 2026-07-26                | Zero compiler changes, child process = isolation                                                                                                                                                                                                                                                                                                    |
| Assertion approach             | `std.test` module pure YaoXiang functions                                                                                                                                                                                                                                                                                    | 2026-07-26                | Self-hosted, no native code                                                                                                                                                                                                                                                                                                                         |
| Test execution model           | Child process `yaoxiang run <file>` + exit code                                                                                                                                                                                                                                                                              | 2026-07-26                | Process-level isolation, zero compiler changes                                                                                                                                                                                                                                                                                                      |
| Standard library loading       | Currently embed binary, file system in the future                                                                                                                                                                                                                                                                            | 2026-07-26                | Version binding, single-file usable                                                                                                                                                                                                                                                                                                                 |
| Assertion parameter type       | Unannotated parameters (Any), not depending on generics                                                                                                                                                                                                                                                                      | 2026-08-02                | `?` type syntax does not exist; Any empirically comparable, interpolatable                                                                                                                                                                                                                                                                          |
| Multi-file running             | CLI `run` delegates to `run_project` (orchestrator) as prerequisite                                                                                                                                                                                                                                                          | 2026-08-02                | Child process model inherits CLI capability; on-demand discovery along `use` degrades to pure performance optimization                                                                                                                                                                                                                              |
| Reporting source location      | Runtime errors carry by default                                                                                                                                                                                                                                                                                              | 2026-09-07                | debug_map generated by default, stack trace output `file:line:col`; frame attribution via embedded modules (std.test) is not in this guarantee, belongs to RFC-034                                                                                                                                                                                  |
| Negative test layering         | Value-level reverse general / compile failure runner structured marker (internal only) / hard failure into Result-ization                                                                                                                                                                                                    | 2026-09-02                | Value-typed model decided; replaces implicit [test:error] convention                                                                                                                                                                                                                                                                                |
| In-file multiple tests         | Value-typed standard model: test functions return Result, suite collects per-test verdicts                                                                                                                                                                                                                                   | 2026-09-02                | No catch, no entry call (entries only for internal scenarios)                                                                                                                                                                                                                                                                                       |
| Error code                     | Error adds machine-readable `code` field                                                                                                                                                                                                                                                                                     | 2026-09-02                | Supports error code assertion; compile-time codes go through runner comparison                                                                                                                                                                                                                                                                      |
| Assertion library form         | Value semantics family of 7 functions landed, `Result(Void, String)` contract; abort transitional version removed                                                                                                                                                                                                            | 2026-09-03                | Void is the spec unit (`()` is empty Tuple, not mixed); nesting position Any rigid, unannotated parameters fail native generics check—parameters must be explicitly annotated (R1 probe empirical)                                                                                                                                                  |
| Test system layering           | Language corpus (`tests/yaoxiang/`) and library tests (live with library, std → `src/std/tests/`) split into two layers; std in corpus only as assertion tool                                                                                                                                                                | 2026-09-03                | Object under test determines home and maintainer; library tests live with package layout, previewing RFC-014                                                                                                                                                                                                                                        |
| Negative marker routed verdict | Route by expected category: compile error class `check` must fail, runtime error class `check` must pass + `run` must fail; report gives category counts                                                                                                                                                                     | 2026-09-03 (landed 09-06) | Category mixing causes "compile accidentally passes, runtime luckily fails" missed detection; expected code pins phase, syntax errors do not get a separate category                                                                                                                                                                                |
| Header directive grammar       | Expectations declared as English structured directives (`// expect:` / `// skip:` / `// mode:`, strict token grammar, parse failure directly FAIL); deprecate `[test:error]` boolean flag and Chinese `预期:` prose code-scraping                                                                                            | 2026-09-06                | Expectation is a property of fixture content, in-fixture declaration conforms to industry (compiletest / Go / GCC / Clang all do this), central manifest is destined to rot; boolean flag + expectation line dual fact coupled by discipline is a defect surface; structured grammar lets the runner mechanically judge without human participation |
| Parallel execution model       | `--parallel` starts per-core worker pool (each file still independent child process), skip/invalid processed first in discovery order, execution results stream in completion order, JSON sorted by path; `--fail-fast` stops scheduling (in-flight finishes and counted); CWD still shared, isolation boundary not extended | 2026-09-07                | Under child process model, parallel = OS thread scheduling spawn, no yx-layer concurrency needed; main time cost is per-file full compilation (risk table), parallel only mitigates the process side—test loop cache slicing is the main mitigation                                                                                                 |

## References

- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — Standard library
  directory structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]` config
  section
- [RFC-030: assert Assertion Mechanism](../review/030-assert-mechanism.md) — Lower-level dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — Reference
  design
