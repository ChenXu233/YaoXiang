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
YaoXiang. Test files are ordinary `.yx` files; the overall pass/fail is determined by the subprocess
exit code. Multiple test functions are supported within a file—assertion failures are expressed as
`Err` values (value semantics), and the suite collects per-test verdicts (§7). The `std.test` module
is implemented in pure YaoXiang and is the first dogfooding library. `yaoxiang test` is a CLI tool,
not a compiler feature—no changes to the parser, IR, bytecode, or executor are involved.

## Motivation

### Why a test framework?

Currently, YaoXiang's test coverage relies on the Rust-side `#[test]` and `tests/` integration
tests. This means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io)
   cannot be written in YaoXiang
2. Unit test coverage for each standard library module is blocked because no test infrastructure is
   available
3. Regression tests for language features (such as the RFC-032 spawn semantic change) lack automated
   means

### Key Constraints

- **The 17-keyword iron law**: do not introduce any new keywords or syntax structures
- **Zero compiler changes**: do not touch the parser, IR, bytecode, or executor
- **Self-hosting first**: the test library is written in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI Layer:    yaoxiang test [--filter --fail-fast --json ...]│
│              │                                               │
│  Discovery:    Read yaoxiang.toml → [tool.test] patterns     │
│                Default: tests/**/*.yx                         │
│              │                                               │
│  Execution:    For each file: yaoxiang run <file>             │
│                Check exit code → Serial execution             │
│              │                                               │
│  Reporting:    PASS/FAIL → Aggregate                          │
│                Supports --json / --verbose / --fail-fast      │
│              │                                               │
│  Assertion:    std.test (pure YaoXiang, self-hosted)          │
│                Underlying: std.assert.assert                  │
│                Diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **The test framework is not a compiler feature, it is a CLI tool** — `yaoxiang run` can already
   "execute tests"; `yaoxiang test` just helps you run all files and shows you the report
2. **Zero compiler changes** — no `@test` annotation scanning, bytecode metadata sections, or
   special executor entry points
3. **Self-hosting** — the `std.test` module is implemented in pure YaoXiang, with underlying
   capabilities from `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files** — files run as subprocesses; the exit code determines
   overall pass/fail
5. **Assertion failures are values, not process events** — test functions return `Result`, assertion
   failures are expressed as `Err`, the suite collects per-test verdicts one by one (§7);
   process-level abort belongs only to runtime guards, not test assertions

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Only run tests whose filename contains <NAME>
  --fail-fast         Stop on the first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              List test files only, do not run
  --no-progress       Do not show progress output (header and PASS lines); FAIL details and summary are preserved (CI scenario)
  --json              Output results in JSON format (for CI integration)
  --parallel          Run in parallel (one worker per core; OR with [tool.test].parallel)
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
  stripped, for CI forensics); when `--verbose` and `--json` are combined, all files carry `stdout`
  / `stderr`
- `--no-progress` only suppresses progress output (header and PASS lines)—FAIL details and summary
  are always emitted; failures cannot be silenced; `--list` prints one test file path per line,
  without executing
- Each file carries `kind` (behavior / compile-error / runtime-error / invalid, §8.2); the summary
  carries `by_kind` execution counts (fixed four keys, not including skipped); the human summary
  carries a `Categories:` distribution line
- Under `--parallel`, the human progress lines stream out in **completion order** (whole blocks do
  not interleave); the JSON `files` are sorted by `file` path to ensure stable output (CI-diff
  friendly)
- The in-file per-test `tests` array comes from the §7 suite collection, in effect with the
  value-ified model
- The official CI (`.github/workflows/ci.yml` test job) consumes exactly this: the cargo side runs
  `--test integration` (CLI integration) and `--test yx_runner` (dual-root corpus guard), then
  `yaoxiang test --json --parallel` runs in layers—default mode (language corpus) and explicit
  `src/std/tests` (library layer) each produce a report; the summary table (total / passed / failed
  / skipped / time_secs and by_kind) is written to the job summary, failed files print `kind` /
  `exit_code` / `stderr` for forensics, and any non-zero exit from either suite turns the job red

### 2. yaoxiang.toml Configuration

Placed under `[tool.test]`, conforming to RFC-015's `[tool.*]` third-party extension convention:

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # matched entries are excluded from the discovery set (--list excludes them too)
parallel = true                   # parallel execution (OR with the --parallel flag)
```

- Default `patterns = ["tests/**/*.yx"]` — zero-config out of the box
- `exclude` and `patterns` share the same shape (literal paths or `root/**…`, all matched by path
  prefix); excluded means not a test—fixtures that need runtime behavior validation are run directly
  via `yaoxiang run`
- **Single-file mode (`yaoxiang test foo.yx`) runs directly without reading config**—under explicit
  paths, neither `exclude` nor `parallel` config keys take effect (except for the flag)
- May be split into a separate repository in the future (the `[tool.test]` position is unchanged)

### 3. std.test Module (Pure YaoXiang)

```yaoxiang
// std/test.yx — pure YaoXiang test assertion library (standard value-semantics form, landed 2026-09-03)
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

// assert_not shares a body with assert_false; see §8.1 for assert_err / assert_err_code
```

- The assertion functions are **value-semantics**: they return `Result(Void, String)`, with failures
  expressed as `Err(diagnostic message)`; they do not abort the process—§7 suites collect per-test
  verdicts accordingly. The process-level abort semantics of `std.assert.assert` are reserved for
  runtime guards and do not enter the test assertion path. The Ok payload is `Void` (per
  type-system.md, unit; `()` is an empty Tuple; the two are not interchangeable—finalized
  2026-09-03)
- **The function family is 7 functions (delivered 2026-09-03, the abort transitional version was
  removed)**: value-ified `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not`
  (shares a body with assert_false, reserved as the home for `!assert`) + `assert_err` (§8.1)
  - `assert_err_code` (§8.1, error-code assertion)
  - `assert_approx_eq(a: Float, b: Float, eps: Float)` (Phase 3, delivered 2026-09-07): judges
    `|a - b| <= eps`; `eps` is **explicitly supplied by the caller**—tolerance is part of the test
    contract, no hidden default; negative `eps` is an Err at the declaration site, NaN is always an
    Err
- `assert_eq` / `assert_ne` use **Any as the parameter annotation**—`==` / `!=` and f-string
  interpolation work fine on Any, and don't depend on the generics system. Note that the parameters
  **must be explicitly annotated**: parameters without annotations cannot pass the call check for
  native generics `&Result(T, E)` (verified by the R1 probe)
- `assert_false` / `assert_not` use `cond == false` to express negation (the `not` unary syntax is
  not yet landed; can be migrated once stable; the `!assert` unary form depends on the same
  constraint, see §8.1)
- The block body + explicit `return` form: in the if expression, the type of the then arm is
  discarded during checking, and an if expression with Result-typed arms is a checking blind
  spot—the implementation works around it
- `std.test` does not depend on any native code; it is pure YaoXiang

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedded Binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) is embedded into the
binary at build time:

```rust
// build.rs or a build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source code text
    // more in the future
];
```

The module system (RFC-029, fully landed 2026-08-02) provides the integration point: the Registry
holds both native modules and source modules, and the orchestrator handles multi-file orchestration.
The resolution order for `use std.test` is:

1. First, look up the Rust native module (the existing mechanism, e.g. `std.assert`)
2. If not found, look up the embedded `STD_YX_FILES`—if matched, inject the orchestrator with a
   **virtual path** (e.g. `<std>/test.yx`) as the seed module, going through the normal front-end
   pipeline (parse → typecheck → IR)
3. If not found, fall back to file-system discovery (user modules)

The `use std.assert` inside an embedded source module is resolved normally by the resolver to the
native registry—native and source modules coexist in the Registry, so cross-kind dependencies just
work. Embedded modules are **compiled on demand**: they only enter the pipeline when imported.

Advantages:

- `use std.test` also works in single-file mode
- The standard library version is strictly bound to the binary, no version mismatch
- No need for users to configure the standard library path

**Future: File-System Standard Library**

Once the YaoXiang project mode matures, the standard library will switch to file-system form. See
the updates to RFC-014 for details.

### 5. Discovery and Execution

**Prerequisite (2026-08-02 review decision)**: the CLI `run` is wired into the orchestrator.
Currently the CLI `run` takes the single-file pipeline (`run_file_with_diagnostics`), and cannot
resolve user-module imports; meanwhile, the subprocess model of `yaoxiang test` inherits CLI
capabilities, and test files importing project modules is a core scenario. So in Phase 1, the source
branch of CLI `Run` is first delegated to `run_project` (orchestrator, recursive directory
discovery); on-demand discovery along `use` is layered on later as a pure performance optimization.
Single files without imports are behaviorally equivalent via orchestrator, and the bytecode branches
are unchanged.

**Discovery Phase**:

1. If `[PATHS]` is specified, use the specified paths directly
2. Otherwise, read `[tool.test].patterns` from `yaoxiang.toml`
3. If not configured, default to `tests/**/*.yx`
4. Apply the `--filter` filter (filename contains)
5. The discovery scope is the test layering (§9): the default patterns only cover the
   language-availability corpus; the library test layer (e.g. `src/std/tests/`) is discovered via
   explicit paths or package configuration, not mixed into the default scan

**Execution Phase**:

1. For each file, dispatch and execute by the header directive (directive grammar in §8.2, parsed
   via `src/util/test_markers.rs`, shared with `yx_runner`):
   - Behavior tests: `yaoxiang run <file>` subprocess (runtime errors carry the source location and
     stack frame by default—`debug_map` is generated by default, stack trace outputs
     `file:line:col`); `// mode:` declares the subprocess `--runtime` mode
   - Compile-time rejection: single-step `yaoxiang check <file>`
   - Runtime failure: two steps—`check` (must pass) + `run` (must fail)
2. Files with `// skip: <reason>` skip execution and are counted as skipped in the report
3. The verdict is compared against the expected code via the §8.2 verdict matrix; if directive
   parsing fails, do not execute and FAIL directly (construction-time rejection)
4. Capture stdout/stderr for the report
5. Serial by default; `--parallel` (or `[tool.test].parallel`) starts a worker pool sized to the
   available cores, and each file is still an independent subprocess—skip/invalid are processed
   first in discovery order, execution results stream out in completion order, and JSON is sorted by
   path (Phase 3, delivered 2026-09-07)
6. With `--fail-fast`, stop scheduling new files on the first FAIL; in parallel mode, in-flight
   files finish running and are counted

### 6. Test Isolation

Test isolation is naturally achieved through the process boundary:

- Each test file runs in an independent subprocess
- Each subprocess has an independent Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional independent Heap context mechanism is required
- **Parallel execution (Phase 3) does not extend the isolation boundary**: subprocesses share the
  working directory (CWD); parallel tests must not occupy files at the same path under
  CWD—file-I/O-style tests should use independent file names and clean up when done

### 7. Suites and Multi-Tests (Value Model)

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

main: () -> Void = {
    test.suite([
        ("push_grows_len", () => push_grows_len()),
        ("pop_returns_last", () => pop_returns_last()),
    ])
}
```

- Each test is a zero-argument function returning `Result(Void, String)`; assertion failures are
  expressed as `Err` (§3 value-semantics assertion family), without interrupting the
  process—subsequent tests run as usual
- `test.suite` calls each one in turn and collects: a non-Ok test records the name and diagnostic,
  Ok stays silent; after all are run, any Err aborts via `std.assert.assert` with the failure detail
  appended (`N of M test(s) failed` + each `[FAIL] name: diagnostic`)—the file's exit code is
  non-zero (§5 verdict unchanged). The abort here is a runtime guard of the test binary, not the
  assertion path; all-Ok exits 0 silently
- Top-level test functions are enqueued as **closures** (`("name", () => test_fn())`): top-level
  function names are not yet supported as value references (IR-level limitation, `E3006`); calling
  global functions from a closure body is unaffected
- The runner only sees the file and does not do function-level scanning: per-test verdicts come
  entirely from the in-suite collection, and the in-file structure is transparent to the runner—the
  zero-compiler-change principle is not affected
- Explicitly not adopted: in-process catch boundaries (the 17-keyword iron law); runner calling each
  function via an entry point (limited to internal scenarios like §8.2 compile failures)
- The API shape has been finalized (2026-09-03):
  `suite(tests: List((String, () -> Result(Void, String)))) -> Void`; duplicate names are not
  detected (names are only used for report display); `--filter` filters by filename and is unaware
  of the names inside the suite

### 8. Three-Layer Design for Negative Tests (Expected Failures)

Negative tests are split by the layer where the failure occurs, and each layer is assigned to its
proper place:

#### 8.1 Value-Level Negation (General, User-Facing)

The operation under test returns a `Result`, and the test expresses the expected failure with
ordinary assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// or a one-line wrapper (the code only lives on the std Error carrier; E is hard-pinned as Error):
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` were delivered with the value-semantics family
  (2026-09-03, §3); the `!assert` unary form will be provided once the `not` syntax lands (sharing
  the same constraint as `cond == false` in `assert_false`)
- Error-code assertion depends on the `Error` value carrying a machine-readable `code` field—already
  landed: `Error = { code, message }` (native `error_new(code, message)`), read via
  `result.unwrap_err(r)` to get the carrier, and accessors `result.code(e)` / `result.message(e)`
  (the design-phase predicted `error_new_with_code` name, exported code constants, and `err.code`
  field access were not adopted—the language has no Struct field access, and code constants are not
  exported)
- As Result-ification progresses, fallible operations return `Result` one by one, and the file-level
  negative markers in the corpus are migrated in-file to assertions accordingly

#### 8.2 File-Level Negative Directives (For Internal Use by Language Designers Only)

Compilation is all-or-nothing for the whole file, and you can't express "this line should not
compile" within a file; the same applies to runtime failures (e.g. a suite containing a test that
must fail). The expectation is declared at the head of the file as a **structured directive**, and
the runner **dispatches and judges** by the expected category (dispatch finalized 2026-09-03;
directive grammar finalized and landed 2026-09-06).

Header directive grammar (`// key: value`, within the first 16 lines, strict token matching):

```text
// expect: compile-error E1002 [E1003 ...]   compile-time rejection test
// expect: runtime-error E6003 [...]         runtime-failure test
// skip: <reason>                            skip execution, counted as skipped
// mode: embedded|standard|full              subprocess --runtime mode (only consumed by the run step)
```

- No `expect:` directive = behavior test. `expect:` is the **sole declaration of expectation**—on
  2026-09-06 we finalized the deprecation of the `[test:error]` boolean marker and the
  Chinese-language `预期:` prose code-grabbing: a boolean marker and an expectation line are two
  loosely coupled facts, kept consistent only by discipline and inevitably drift; English
  strict-token grammar lets the runner parse mechanically (kind and codes are all fixed tokens, any
  extra token after the code is a parse failure), and parse failure = do not execute, FAIL
  directly—there is no silent-degradation channel for an erroneous directive declaration
  (construction-time rejection)
- **Compile-error class**: single-step `check`—must fail and the output must contain all `[EXXXX]`;
  compile passing = FAIL (should have failed but didn't), rejected but code mismatch = FAIL. No
  separate category for syntax errors (E1xxx parse phase) vs semantic errors (E2xxx+)—the expected
  code itself pins the phase
- **Runtime-error class**: two-step verdict—`check` must **succeed** (compile-time is innocent),
  `run` must fail and the output must contain all `[EXXXX]`. Compile-time explosion = FAIL (this is
  the verdict that goes in the opposite direction from the compile-error class, preventing the
  "compile unexpectedly passes, runtime coincidentally fails" miss-detection)
- Aligned with industry conventions: Rust compiletest's `//~ ERROR`, Go's `// ERROR "regexp"`, GCC's
  `dg-error`, Clang's `expected-error`—all declare expectations in fixture comments and let the
  harness compare in both directions; they adopt line-level anchoring because multi-diagnostic
  compilers need to distinguish multiple expectations in the same file, whereas our compiler stops
  at the first error and emits one diagnostic per file, so file-level is isomorphic to the
  compiler's reality—line-anchored form can be added to the grammar once error recovery lands
  (Cranelift filetests' file-head directive + function-level expectations are the same kind of
  hybrid form)
- Known rendering debt: parse-phase diagnostics are currently output in Debug form (`code: "E0012"`
  rather than `[E0012]`); code scanning accepts both forms; once diagnostic rendering is unified,
  the strict form will be tightened
- **Only serves the corpus of this repository, not part of the user-facing test framework**; the
  dual-runner judgment convention has been wrapped up (2026-09-03): `yx_runner` (cargo test) and
  `yaoxiang test` share `src/util/test_markers.rs` for parsing header directives, and the
  06-compile-errors directory convention is deprecated. The reporting layer emits category counts:
  the human summary carries a `Categories:` line; the JSON summary carries `by_kind` (behavior /
  compile-error / runtime-error / invalid, with skipped counted separately); each file carries
  `kind`

#### 8.3 Runtime Hard Failures (Folding into Result-ification)

No separate mechanism is provided—operations that can fail return `Result` per the language
direction, and tests uniformly follow the §8.1 expression. Process-level aborts (such as assertion
violations, runtime argument misalignments) are gradually collapsed into values as Result-ification
progresses, and the test framework does not provide dedicated semantics for them. (Note: the
"runtime-error class" marker judgment in §8.2 is the runner's file-level verification channel for
**operations that cannot yet be Result-ified**, and does not contradict the semantic direction of
this section—the latter is the endpoint, the former is the migration-phase channel)

### 9. Test System Layering: Language Corpus and Library Tests (Decided 2026-09-03)

Tests are split into two layers by **the object under test**, each with its own ownership and
maintainer; the marker system (§8.2) and assertion library (§3) are shared at both levels:

**Layer One: Language Usability Corpus (`tests/yaoxiang/`)**

- The object under test is **the language itself**—parser, type system, module, concurrency,
  ownership, compile-time rejection, runtime semantics; the directory is organized by language spec
  chapter
- std is used in the corpus only as an **assertion tool** (`std.assert` / `std.test`), and is never
  tested—library API behavior does not belong to language usability
- Within the corpus, three verdict classes are dispatched per §8: behavior tests / compile-time
  rejection tests / runtime failure tests

**Layer Two: Library Tests (Live with the Library)**

- The object under test is the **public API contract of the library** (e.g. `list.push` behavior,
  `result.code` semantics)
- Tests live in **the library's own package**: the std package is `src/std/`, and its yx-level tests
  belong to `src/std/tests/` (co-located with the implementation; the directory coexists with Rust
  unit tests, file types don't cross); the test of `std.test` itself is also here (test std.test
  with std.test, a self-hosting closed loop)
- Future user packages will follow the same convention: tests inside the package, discovered via the
  package's `[tool.test]` (this previews RFC-014 package management's test layout)
- Discovery is not in the default patterns (the default `tests/**/*.yx` only covers the language
  layer): the library test layer is discovered via explicit paths (`yaoxiang test src/std/tests`) or
  package configuration; CI runs in layers

Migration note: **Already migrated (2026-09-06)**—after one-by-one triage, all 19 files in the
original `tests/yaoxiang/07-std/` turned out to be library tests (the object under test in each is
the API contract of an std module; the role of language features like `?` propagation, automatic
borrowing, and generic instantiation in them is the carrier, not the object under test); the whole
directory was moved into `src/std/tests/` and the 07-std directory was removed; `yx_runner` was
changed to dual-root discovery (`tests/yaoxiang/` + `src/std/tests/`), and the default patterns do
not include the library layer (the integration test pins this contract). The language corpus has
zero std-API tests from then on.

## Relationship with Existing Systems

| Item                                                 | Relationship                                                                                                   |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched; compiler-internal tests continue using Rust                                                         |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                     |
| `std.assert.assert(cond)`                            | Reserved for runtime guards; the `std.test` value-semantics assertion family is based on `std.result` (§3, §7) |
| Module system (RFC-029)                              | Embedded source modules enter via Registry/orchestrator; prerequisite is CLI `run` wiring orchestrator         |
| Corpus refactor (`io.println` → `assert.assert`)     | Direction fully aligned with `yaoxiang test`                                                                   |
| `@` annotations                                      | Not used; `@test` is not introduced                                                                            |

## Implementation Strategy

### Phase 1: Core Features

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project`
  (multi-file run prerequisite)
- `src/main.rs` — add the `Test` subcommand
- `src/std/test.yx` — add the pure YaoXiang module
- `build.rs` — embed `std/*.yx` into the binary
- orchestrator / Registry — support loading `.yx` modules from the embedded source under a virtual
  path
- RFC-015 config parsing — the `[tool.test]` section
- Subprocess execution + reporting

Deliverables:

- `yaoxiang test` basically works
- `std.test` 4 assertion functions
- Default `tests/**/*.yx` discovery
- Serial execution + default output format

### Phase 2: Refinement

- `--filter` / `--fail-fast` / `--verbose` parameters
- `--json` output (for CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: Advanced (Delivered 2026-09-07)

- `--parallel` parallel execution (worker pool + per-file independent subprocess; the
  `[tool.test].parallel` config key has the same effect)
- `[tool.test].exclude` config (prefix-match exclusion; `--list` also excludes)
- `assert_approx_eq` (Float assertion with explicit eps, §3)

## Risks and Mitigation

| Risk                                                           | Probability | Mitigation                                                                                                                                                                                                       |
| -------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` interpolation failure on Any                          | None        | Verified on 2026-08-02 (Int/String both work)                                                                                                                                                                    |
| `yaoxiang.toml` config parsing not in the current CLI          | Low         | Simple extension, doesn't affect core features                                                                                                                                                                   |
| CLI `run` wiring orchestrator introduces behavioral regression | Low         | Path equivalence for single files without imports; integration tests already cover the orchestrator                                                                                                              |
| Embedding `.yx` source files into the binary increases size    | Low         | `.yx` source files are tiny, negligible                                                                                                                                                                          |
| Test loop runtime grows with the corpus                        | High        | The main cost is per-file full compilation (185 files measured at 11.3s), not subprocess startup; `--parallel` only alleviates the process-side cost; test-loop cache slicing is needed for the compilation cost |

## Open Questions

- [x] Can `use std.assert` inside `std/test.yx` be resolved correctly?—**Resolved (2026-08-02)**.
      After the module system (RFC-029) landed, native and source modules coexist in the Registry;
      the resolver is unified, and cross-kind dependencies just work
- [x] Does the generic `to_string` of `f"..."` in test output introduce new type
      constraints?—**Resolved (2026-08-02)**. Verified that on untyped parameters (Any), both `==` /
      `!=` and f-string interpolation work (verified for Int/String), no new constraints are
      introduced
- [x] Is `?` feasible as a generic parameter?—**Resolved (2026-08-02)**: the `?` type syntax does
      not currently exist (and is silently swallowed; a separate issue is being tracked), so the
      Phase 1 assertion functions use untyped parameters and do not depend on the generics system

## Design Decision Log

| Decision                           | Determination                                                                                                                                                                                                                                                                                                                                              | Date                      | Reason                                                                                                                                                                                                                                                                                                                                                                                               |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Test marker approach               | Do not use `@test` annotations; test files are ordinary `.yx`                                                                                                                                                                                                                                                                                              | 2026-07-26                | Zero compiler changes, subprocess is isolation                                                                                                                                                                                                                                                                                                                                                       |
| Assertion approach                 | `std.test` module, pure YaoXiang functions                                                                                                                                                                                                                                                                                                                 | 2026-07-26                | Self-hosting, no native code                                                                                                                                                                                                                                                                                                                                                                         |
| Test execution model               | Subprocess `yaoxiang run <file>` + exit code                                                                                                                                                                                                                                                                                                               | 2026-07-26                | Process-level isolation, zero compiler changes                                                                                                                                                                                                                                                                                                                                                       |
| Standard library loading           | Currently embedded binary, file-system in the future                                                                                                                                                                                                                                                                                                       | 2026-07-26                | Version binding, single-file usable                                                                                                                                                                                                                                                                                                                                                                  |
| Assertion parameter types          | Untyped parameters (Any), does not depend on the generics system                                                                                                                                                                                                                                                                                           | 2026-08-02                | The `?` type syntax does not exist; Any is verified to be comparable and interpolatable                                                                                                                                                                                                                                                                                                              |
| Multi-file run                     | CLI `run` delegates to `run_project` (orchestrator) as a prerequisite                                                                                                                                                                                                                                                                                      | 2026-08-02                | Subprocess model inherits CLI capabilities; on-demand discovery along `use` degrades into a pure performance optimization                                                                                                                                                                                                                                                                            |
| Report source location             | Runtime errors carry by default                                                                                                                                                                                                                                                                                                                            | 2026-09-07                | `debug_map` is generated by default, stack trace outputs `file:line:col`; frame attribution transit through embedded modules (std.test) is not guaranteed here, belongs to RFC-034                                                                                                                                                                                                                   |
| Negative-test layering             | Value-level negation for general use / compile-failure structured marker on the runner (internal only) / hard failures fold into Result-ification                                                                                                                                                                                                          | 2026-09-02                | Value-ified model finalized; replaces the implicit `[test:error]` convention                                                                                                                                                                                                                                                                                                                         |
| Multi-tests within a file          | Value-ified standard model: test functions return Result, suite collects per-test verdicts                                                                                                                                                                                                                                                                 | 2026-09-02                | No catch, no entry-point calling (entry points only for internal scenarios)                                                                                                                                                                                                                                                                                                                          |
| Error code                         | Error gains a machine-readable `code` field                                                                                                                                                                                                                                                                                                                | 2026-09-02                | Supports error-code assertion; compile-time codes go through runner comparison                                                                                                                                                                                                                                                                                                                       |
| Assertion library shape            | Value-semantics family of 7 functions landed, `Result(Void, String)` contract; abort transitional version removed                                                                                                                                                                                                                                          | 2026-09-03                | Void is the spec unit (`()` is an empty Tuple, don't mix); nested-position Any is rigid, untyped parameters cannot pass the native generics call check—parameters must be explicitly annotated (R1 probe verified)                                                                                                                                                                                   |
| Test system layering               | Language corpus (`tests/yaoxiang/`) and library tests (live with the library, std → `src/std/tests/`) split into two layers; std is used in the corpus only as an assertion tool                                                                                                                                                                           | 2026-09-03                | The object under test determines ownership and maintainer; library tests living with the package previews RFC-014                                                                                                                                                                                                                                                                                    |
| Negative-marker dispatched verdict | Dispatch by expected category: compile-error class `check` must fail, runtime-error class `check` must pass + `run` must fail; report emits category counts                                                                                                                                                                                                | 2026-09-03 (landed 09-06) | Mixing categories causes the "compile unexpectedly passes, runtime coincidentally fails" miss-detection; the expected code pins the phase; syntax errors are not given a separate category                                                                                                                                                                                                           |
| Header directive grammar           | Expectations declared in English structured directives (`// expect:` / `// skip:` / `// mode:`, strict-token grammar, parse failure means direct FAIL); deprecate `[test:error]` boolean marker and Chinese-language `预期:` prose code-grabbing                                                                                                           | 2026-09-06                | The expectation is a property of the fixture content; in-fixture declaration is isomorphic to industry conventions (compiletest / Go / GCC / Clang all do this), a centralized list will inevitably rot; a boolean marker + an expectation line are two facts coupled only by discipline, which is a defect surface; structured grammar lets the runner judge mechanically with no human in the loop |
| Parallel execution model           | `--parallel` starts a per-core worker pool (each file is still an independent subprocess), skip/invalid are processed first in discovery order, execution results stream out in completion order, JSON is sorted by path; `--fail-fast` stops scheduling (in-flight files finish and are counted); CWD is still shared, isolation boundary is not extended | 2026-09-07                | Under the subprocess model, parallelism is just OS-thread-scheduled spawn, no yx-layer concurrency required; the main time cost is per-file full compilation (risk table), and parallelism only alleviates the process-side cost—test-loop cache slicing is the main mitigation                                                                                                                      |

## References

- [RFC-014: Package Management System Design](014-package-manager.md) — standard library directory
  structure
- [RFC-015: Configuration System](015-configuration-system.md) — the `[tool.test]` config section
- [RFC-030: assert Mechanism](030-assert-mechanism.md) — underlying dependency
- [Rust `#[test]` Mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — reference
  design
