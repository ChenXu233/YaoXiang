---
title: 'RFC-036: std.test testing framework and yaoxiang test command'
status: 'Accepted'
author: 'Chenxu'
created: '2026-07-26'
updated: '2026-09-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test testing framework and yaoxiang test command

## Summary

Introduce a standard test framework `std.test` module and a `yaoxiang test` CLI subcommand for
YaoXiang. Test files are ordinary `.yx` files whose overall pass/fail is determined by the child
process exit code; inside a file, multiple test functions are supported—assertion failures are
expressed as `Err` values (value semantics), and the suite collects per-test judgments (§7). The
`std.test` module is implemented in pure YaoXiang and is the first dogfooding library.
`yaoxiang test` is a CLI tool, not a compiler feature—it involves no changes to the parser, IR,
bytecode, or executor.

## Motivation

### Why do we need a test framework?

YaoXiang's current test coverage depends on Rust-side `#[test]` and `tests/` integration tests. This
means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io)
   cannot be written in YaoXiang
2. Unit test coverage of standard library modules is blocked, because there is no available test
   infrastructure
3. Regression tests for language features (such as the spawn semantics change in RFC-032) lack
   automated means

### Key constraints

- **17 keyword iron rule**: do not introduce any new keywords or syntactic constructs
- **Zero compiler changes**: do not touch the parser, IR, bytecode, or executor
- **Bootstrap first**: the test library is written in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI layer:  yaoxiang test [--filter --fail-fast --json ...]  │
│              │                                               │
│  Discovery:   read yaoxiang.toml → [tool.test] patterns      │
│              default: tests/**/*.yx                           │
│              │                                               │
│  Execution:   for each file: yaoxiang run <file>              │
│              check exit code → serial execution              │
│              │                                               │
│  Reporting:   PASS/FAIL → summary                             │
│              supports --json / --verbose / --fail-fast        │
│                                                              │
│  Assertion:   std.test (pure YaoXiang, bootstrap)             │
│              low level: std.assert.assert                     │
│              diagnostics: f"Expected {expected}, got {actual}" │
└──────────────────────────────────────────────────────────────┘
```

### Core principles

1. **The test framework is not a compiler feature, it is a CLI tool** — `yaoxiang run` can already
   "execute tests", `yaoxiang test` just helps you run all files and show you a report
2. **Zero compiler changes** — no introduction of `@test` annotation scanning, bytecode metadata
   sections, or special executor entry points
3. **Bootstrap** — the `std.test` module is implemented in pure YaoXiang, with low-level
   capabilities from `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files** — the file runs as a child process, exit code determines
   overall pass/fail
5. **Assertion failure is a value, not a process event** — the test function returns `Result`,
   assertion failure is expressed as `Err`, and the suite collects per-test judgments one by one
   (§7); process-level abort belongs only to runtime guards, not to test assertions

## Detailed design

### 1. CLI design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     only run tests whose filename contains <NAME>
  --fail-fast         stop on the first failure
  --verbose, -v       show detailed stdout/stderr for each test
  --list              only list test files, do not run
  --no-progress       do not show progress output (header and PASS lines); FAIL details and summary are retained (CI scenario)
  --json              output JSON format results (for CI integration)
  --parallel          execute in parallel (one worker per core; OR with [tool.test].parallel)
```

#### Output format

**Default output** (per-test judgment comes from in-file suite collection, see §7):

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
  for CI forensics); when `--verbose` and `--json` are combined, all files carry `stdout` / `stderr`
- `--no-progress` only suppresses progress output (header and PASS lines)—FAIL details and summary
  are always output; failure cannot be silent; `--list` outputs one test file path per line, without
  execution
- Each file carries a `kind` (behavior / compile-error / runtime-error / invalid, §8.2), the summary
  carries `by_kind` execution counts (fixed four keys, not including skipped); the human summary has
  a `Categories:` distribution line appended
- Under `--parallel`, human progress lines are streamed in **completion order** (whole blocks not
  interleaved), JSON `files` is sorted by `file` path to ensure stable output (CI diff friendly)
- The per-test `tests` array inside a file comes from the §7 suite collection, taking effect with
  the value-model landing
- Official CI (`.github/workflows/ci.yml` test job) consumes this: cargo-side runs
  `--test integration` (CLI integration) and `--test yx_runner` (dual-root corpus guard), then
  `yaoxiang test --json --parallel` runs layered execution—default mode (language corpus) and
  explicit `src/std/tests` (library layer) each produce a report; the summary table (total / passed
  / failed / skipped / time_secs and by_kind) is written to the job summary, failed files print
  `kind` / `exit_code` / `stderr` forensics, and any non-zero suite exit code is judged red

### 2. yaoxiang.toml configuration

Placed under `[tool.test]`, conforming to the `[tool.*]` third-party extension convention of
RFC-015:

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # hits are removed from the discovery set (--list also removes)
parallel = true                   # parallel execution (OR with the --parallel flag)
```

- Default `patterns = ["tests/**/*.yx"]` — zero-config out of the box
- `exclude` has the same form as `patterns` (literal paths or `root/**…`, all matched by path
  prefix); excluded means not a test, fixtures that need runtime behavior verification are run
  directly via `yaoxiang run`
- **Single-file mode (`yaoxiang test foo.yx`) runs directly, does not read configuration**—under
  explicit paths, neither `exclude` nor `parallel` config keys take effect (except the flag)
- May be split into a separate repository in the future (`[tool.test]` position unchanged)

### 3. std.test module (pure YaoXiang)

```yaoxiang
// std/test.yx — pure YaoXiang test assertion library (value-semantics standard form, landed 2026-09-03)
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

- Assertion functions are **value semantics**: return `Result(Void, String)`, failure is expressed
  as `Err(diagnostic info)`, no process abort—the §7 suite collects per-test judgments based on
  this. The process-level abort semantics of `std.assert.assert` are reserved for runtime guards and
  do not enter the test assertion path. The Ok payload is `Void` (the canonical unit per
  type-system.md; `()` is an empty Tuple, the two are not mixed—decided 2026-09-03)
- **Function family of 7 (delivered 2026-09-03, abort transitional version removed)**: value-style
  `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not` (same body as
  assert_false, reserved for the `!assert` form) + `assert_err` (§8.1)
  - `assert_err_code` (§8.1 error code assertion)
  - `assert_approx_eq(a: Float, b: Float, eps: Float)` (Phase 3, delivered 2026-09-07): judges
    `|a - b| <= eps`, with eps **given explicitly** by the caller—tolerance is part of the test
    contract, no hidden default; negative eps is Err at the declaration site, NaN is always Err
- `assert_eq` / `assert_ne` use **Any-annotated parameters**—`==`/`!=` and f-string interpolation
  work fine on Any, not relying on the generics system. Note that parameters **must be annotated
  explicitly**: unannotated parameters cannot pass the native generic `&Result(T, E)` invocation
  check (empirically verified by R1 probe)
- `assert_false` / `assert_not` use `cond == false` to express negation (the `not` unary syntax is
  not landed, may be migrated once stable; the `!assert` unary form has the same dependency, see
  §8.1)
- Block body + explicit `return` form: the type of the then-arm of the if expression is dropped
  during checking; the if expression with both arms as Result is a blind spot for checking, the
  implementation works around it
- `std.test` does not depend on any native code, it is pure YaoXiang

### 4. Standard library loading mechanism (key design)

**Phase 1: Embed binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) is embedded in the
binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source code text
    // more in the future
];
```

The module system (RFC-029, fully landed 2026-08-02) provides the entry point: the Registry holds
both native modules and source modules, the orchestrator is responsible for multi-file
orchestration. The resolution order of `use std.test`:

1. First check Rust native modules (existing mechanism, such as `std.assert`)
2. If not found, check the embedded `STD_YX_FILES`—on hit, the seed module is injected into the
   orchestrator with a **virtual path** (e.g. `<std>/test.yx`), going through the normal frontend
   pipeline (parse → typecheck → IR)
3. If not found, go through filesystem discovery (user modules)

The `use std.assert` inside the embedded source module is normally resolved by the resolver to the
native registry—native and source modules coexist in the Registry, cross-kind dependencies are
naturally valid. Embedded modules are **compiled on demand**: they only enter the pipeline when
imported.

Advantages:

- `use std.test` works in single-file mode
- The standard library version is strictly bound to the binary, no version mismatch
- Users do not need to configure the standard library path

**Future: Filesystem standard library**

When the YaoXiang project mode matures, the standard library will change to filesystem form. See
updates to RFC-014 for details.

### 5. Discovery and execution

**Prerequisite (2026-08-02 review decision)**: the CLI `run` connects to the orchestrator. The
current CLI `run` goes through the single-file pipeline (`run_file_with_diagnostics`) and cannot
resolve user module imports; yet the child process model of `yaoxiang test` inherits CLI capability,
and test files importing project modules is a core scenario. Therefore, Phase 1 first delegates the
source branch of CLI `Run` to `run_project` (orchestrator, recursive directory discovery); lazy
on-use discovery later is layered as a pure performance optimization. Single files without imports
are behaviorally equivalent through the orchestrator, the bytecode branch is unchanged.

**Discovery phase**:

1. If `[PATHS]` are specified, use the specified paths directly
2. Otherwise, read the `[tool.test].patterns` from `yaoxiang.toml`
3. If not configured, default to `tests/**/*.yx`
4. Apply the `--filter` filter (filename contains)
5. The discovery scope is the test layering (§9): default patterns only cover language-availability
   corpus; the library test layer (e.g. `src/std/tests/`) is discovered via explicit paths or
   package configuration, not mixed into the default scan

**Execution phase**:

1. For each file, branch execution by header directives (directive grammar see §8.2, parsed via
   `src/util/test_markers.rs`, shared with yx_runner):
   - Behavior tests: `yaoxiang run <file>` child process (runtime errors carry source location and
     stack frames by default—debug_map is generated by default, stack trace outputs
     `file:line:col`); `// mode:` declares the child process `--runtime` mode
   - Compile-time rejection type: single step `yaoxiang check <file>`
   - Runtime failure type: two steps `check` (must pass) + `run` (must fail)
2. Files with `// skip: <reason>` skip execution and are counted as skipped in the report
3. Judgment and expected code comparison follow the §8.2 judgment matrix; directive parse failure
   means no execution and direct FAIL (construction-time rejection)
4. Capture stdout/stderr for the report
5. Serial by default; `--parallel` (or `[tool.test].parallel`) spawns a worker pool by available
   core count, each file is still an independent child process—skip/invalid are handled first in
   discovery order, execution results are streamed in completion order, JSON is sorted by path
   (Phase 3, delivered 2026-09-07)
6. If `--fail-fast`, stop scheduling new files on the first FAIL; under parallel mode, in-flight
   files run to completion and are counted

### 6. Test isolation

Test isolation is naturally achieved through process-level boundaries:

- Each test file runs in an independent child process
- Each child process has independent Heap, Frame, NativeContext
- A panic in one test file does not affect other test files
- No additional independent Heap context mechanism is required
- **Parallel execution (Phase 3) does not extend the isolation boundary**: the working directory
  (CWD) is shared between child processes, parallel tests must not occupy files at the same path in
  CWD—file I/O tests should use independent filenames and clean up at the end

### 7. Suites and multiple tests (value model)

A test file can contain multiple tests. In-file organization (landed 2026-09-03):

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

- Each test is a zero-arg function returning `Result(Void, String)`; assertion failure is expressed
  as `Err` (§3 value-semantics assertion family), and does not interrupt the process—subsequent
  tests continue to run
- `test.suite` calls one by one and collects: if a test is not Ok, the name and diagnostic are
  recorded; Ok is silent; after all have run, if any is Err, it is aborted with `std.assert.assert`
  and failure details are appended (`N of M test(s) failed` + each `[FAIL] name: diagnostic`)—the
  file exit code is non-zero (§5 judgment unchanged). The abort here is a runtime guard of the test
  binary, not the assertion path; all Ok exits silently with 0
- Top-level test functions are enqueued as **closures** (`("name", () => test_fn())`): referencing
  top-level function names as values is not currently supported (IR layer restriction, `E3006`);
  calling global functions in a closure body is unaffected
- The runner only sees the file, does not perform function-level scanning: per-test judgment comes
  entirely from the in-suite collection, the file's internal structure is transparent to the
  runner—the zero-compiler-change principle is not affected
- Explicitly not adopted: in-process catch boundary (17 keyword iron rule); runner calling test
  functions one by one (only for internal scenarios like §8.2 compile failures)
- The API form has been decided (2026-09-03):
  `suite(tests: List((String, () -> Result(Void, String)))) -> Void`; duplicate names are not
  detected (name is only used for report display); `--filter` filters by filename and does not
  perceive test names within suites

### 8. Three-layer design for negative tests (expected failure)

Negative tests are split by the layer at which failure occurs, each with its own placement:

#### 8.1 Value-level reverse (generic, for users)

The operation under test returns `Result`, the test expresses expected failure with normal
assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// or a one-liner wrapper (code only exists on the std Error carrier, E nailed down to Error):
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` have been delivered along with the value-semantics
  family (2026-09-03, §3); the `!assert` unary form will be provided after the `not` syntax lands
  (same constraint as `assert_false`'s `cond == false`)
- Error code assertion depends on the `Error` value carrying a machine-readable `code` field—already
  landed: `Error = { code, message }` (native `error_new(code, message)`), read via
  `result.unwrap_err(r)` to get the carrier + `result.code(e)` / `result.message(e)` accessors (the
  design-phase-estimated `error_new_with_code` naming, exported code constants, and `err.code` field
  access were not adopted—the language has no Struct field access, and code constants were not
  exported)
- As Result-ization progresses, operations that can fail will return `Result` one by one, and
  file-level negative markers in the corpus will migrate to in-file assertions

#### 8.2 File-level negative directives (only for internal use by language designers)

Compilation is all-or-nothing for the whole file, so you cannot express "this line should not
compile" within a file; runtime failure also requires file-level expression (e.g. suites must
contain tests that must fail). The file header declares expectations with **structured directives**,
the runner **branches and judges** by expected category (branching decided 2026-09-03; directive
grammar decided and landed 2026-09-06).

Header directive grammar (`// key: value`, within the first 16 lines, strict token matching):

```text
// expect: compile-error E1002 [E1003 ...]   compile-time rejection test
// expect: runtime-error E6003 [...]         runtime failure test
// skip: <reason>                            skip execution, counted as skipped
// mode: embedded|standard|full              child process --runtime mode (consumed only by the run step)
```

- No `expect:` directive = behavior test. `expect:` is the **sole declaration** of
  expectations—decided 2026-09-06 to deprecate the `[test:error]` boolean marker and the Chinese
  `预期:` prose code scraping: the boolean marker and the expectation line are two loosely coupled
  facts, and relying on discipline to keep them consistent will inevitably drift; the strict English
  token grammar lets the runner parse mechanically (kind + codes are all fixed tokens, any extra
  token after the code causes parse failure), and parse failure = no execution and direct FAIL—there
  is no silent degradation channel for directive declaration errors (construction-time rejection)
- **Compile error type**: single step `check`—must fail and output must contain all `[EXXXX]`;
  compile pass = FAIL (expected error not reported), reject but code mismatch = FAIL. Syntax errors
  (E1xxx parse stage) and semantic errors (E2xxx+) are not given independent categories—the expected
  code itself nails down the stage
- **Runtime error type**: two-step judgment—`check` must **pass** (compile-time innocence), `run`
  must fail and output must contain all `[EXXXX]`. Compile-time explosion = FAIL (this is a critical
  judgment opposite to the compile error type, preventing "compile unexpectedly passes, runtime
  coincidentally fails" missed detection)
- Conforms to industry conventions: Rust compiletest `//~ ERROR`, Go `// ERROR "regexp"`, GCC
  `dg-error`, Clang `expected-error` all declare expectations within fixture comments and let the
  harness compare in both directions; they use line-level anchoring because multi-diagnostic
  compilers need to distinguish multiple expectations in the same file; this compiler stops at the
  first error, one diagnostic per file, so file-level matches the compiler's reality—a line-anchored
  form can be added to the grammar after error recovery lands (Cranelift filetests' file header
  directives + function-level expectations are the same hybrid form)
- Known rendering debt: parse-stage diagnostics currently output in Debug form (`code: "E0012"`
  rather than `[E0012]`), code scanning accepts both forms; the strict form will be recovered after
  diagnostic rendering is unified
- **Only serves this repository's corpus, not part of the user's test framework**; the dual-runner
  judgment convention has been closed off (2026-09-03): yx_runner (cargo test) and `yaoxiang test`
  share `src/util/test_markers.rs` to parse header directives, the 06-compile-errors directory
  convention is deprecated. The report layer provides category counts: the human summary has a
  `Categories:` line, the JSON summary has `by_kind` (behavior / compile-error / runtime-error /
  invalid, skipped counted separately), and each file carries `kind`

#### 8.3 Runtime hard failure (folded into Result-ization)

No independent mechanism is established—operations that will fail return `Result` per language
direction, and tests uniformly go through §8.1. Process-level aborts (such as assertion violations,
runtime parameter misalignment) gradually converge to values as Result-ization proceeds, and the
test framework does not provide dedicated semantics for them. (Note: the "runtime error type" marker
judgment in §8.2 is the runner's file-level verification channel for **operations that cannot yet be
Result-ized**, which does not contradict the semantic direction of this section—the latter is the
end point, the former is the migration-period channel)

### 9. Test system layering: language corpus and library tests (decided 2026-09-03)

Tests are split into two layers by the **object under test**, each with its own placement and
maintainer; the marker system (§8.2) and assertion library (§3) are shared across the two levels:

**Layer 1: language availability corpus (`tests/yaoxiang/`)**

- The object under test is the **language itself**—parser, type system, modules, concurrency,
  ownership, compile-time rejection, runtime semantics; the directory is organized by language
  specification chapter
- std is only used in the corpus as an **assertion tool** (`std.assert` / `std.test`), never
  tested—the API behavior of the library does not belong to language availability
- Within the corpus, judgment is branched into behavior tests / compile-time rejection tests /
  runtime failure tests per §8

**Layer 2: library tests (go with the library)**

- The object under test is the **library's public API contract** (e.g. `list.push` behavior,
  `result.code` semantics)
- Tests are written in **the library's own package**: std's package is `src/std/`, and its yx-level
  tests go in `src/std/tests/` (in the same place as the implementation body; the directory coexists
  with Rust unit tests, file types do not overlap); `std.test`'s own tests are also there (testing
  std.test with std.test, bootstrap closed loop)
- Future user packages follow the same convention: tests are in the package, discovered via the
  package's `[tool.test]` (the test layout of RFC-014 package management is thus previewed)
- Discovery does not enter the default patterns (default `tests/**/*.yx` only covers the language
  layer): the library test layer is discovered via explicit paths (`yaoxiang test src/std/tests`) or
  package configuration; CI runs in layers

Migration note: **migrated (2026-09-06)**—the 19 files in the original `tests/yaoxiang/07-std/` were
screened one by one and all turned out to be library tests (the object under test in each is the API
contract of a std module; the role of language features like `?` propagation, automatic borrowing,
and generic instantiation in them is as carriers, not the object under test), and were migrated as a
whole into `src/std/tests/` with the 07-std directory removed; yx_runner switched to dual-root
discovery (`tests/yaoxiang/` + `src/std/tests/`), the default patterns do not include the library
layer (integration tests solidify that contract). Since then, the language corpus contains zero
std-API tests.

## Relationship with existing systems

| Item                                                 | Relationship                                                                                                          |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Unchanged, compiler internal tests continue to use Rust                                                               |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                            |
| `std.assert.assert(cond)`                            | Reserved for runtime guards; the value-semantics assertion family of `std.test` is built on `std.result` (§3, §7)     |
| Module system (RFC-029)                              | Embedded source modules connect via Registry/orchestrator; CLI `run` connecting to the orchestrator is a prerequisite |
| Corpus refactoring (`io.println` → `assert.assert`)  | Same direction as `yaoxiang test`                                                                                     |
| `@` annotation                                       | Not used, no `@test` introduced                                                                                       |

## Implementation strategy

### Phase 1: core functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project`
  (multi-file run prerequisite)
- `src/main.rs` — add `Test` subcommand
- `src/std/test.yx` — add pure YaoXiang module
- `build.rs` — embed `std/*.yx` into the binary
- orchestrator / Registry — support loading `.yx` modules from embedded sources via virtual paths
- RFC-015 configuration parsing — `[tool.test]` section
- Child process execution + reporting

Deliverables:

- `yaoxiang test` basically usable
- 4 assertion functions in `std.test`
- Default `tests/**/*.yx` discovery
- Serial execution + default output format

### Phase 2: refinement

- `--filter` / `--fail-fast` / `--verbose` parameters
- `--json` output (CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: advanced (delivered 2026-09-07)

- `--parallel` parallel execution (worker pool + each file is an independent child process; the
  `[tool.test].parallel` config key has the same effect)
- `[tool.test].exclude` configuration (prefix-match removal, `--list` also removes)
- `assert_approx_eq` (Float explicit eps assertion, §3)

## Risks and mitigations

| Risk                                                              | Probability | Mitigation                                                                                                                                                                                        |
| ----------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` interpolation on Any fails                               | None        | Verified 2026-08-02 (Int/String both work)                                                                                                                                                        |
| `yaoxiang.toml` config parsing not in current CLI                 | Low         | Simple extension, does not affect core functionality                                                                                                                                              |
| CLI run connecting to orchestrator introduces behavior regression | Low         | No-import single-file path is equivalent; integration tests already cover the orchestrator                                                                                                        |
| Embedding `.yx` source files into the binary increases size       | Low         | `.yx` source files are very small, negligible                                                                                                                                                     |
| Test loop time grows with corpus                                  | High        | The main cost is per-file full compilation (185 files measured 11.3s), not child process startup; `--parallel` only mitigates the process side, compilation cost requires test loop cache slicing |

## Open questions

- [x] Can the `use std.assert` reference in `std/test.yx` be resolved correctly?—**resolved
      (2026-08-02)**. After the module system (RFC-029) landed, native and source modules coexist in
      the Registry, the resolver parses them uniformly, and cross-kind dependencies are naturally
      valid
- [x] Does the generic `to_string` of `f"..."` in test output introduce new type
      constraints?—**resolved (2026-08-02)**. Empirically verified that on unannotated parameters
      (Any), `==`/`!=` and f-string interpolation both work (verified with Int/String), no new
      constraints introduced
- [x] Feasibility of `?` generic parameters?—**resolved (2026-08-02)**: the `?` type syntax does not
      currently exist (and is silently swallowed, tracked in a separate issue), Phase 1 assertion
      functions use unannotated parameters, not relying on the generics system

## Design decision record

| Decision                           | Decision                                                                                                                                                                                                                                                                                                                                             | Date                      | Rationale                                                                                                                                                                                                                                                                                                                                                                      |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Test marker method                 | No `@test` annotation, test files are ordinary `.yx`                                                                                                                                                                                                                                                                                                 | 2026-07-26                | Zero compiler change, child process equals isolation                                                                                                                                                                                                                                                                                                                           |
| Assertion method                   | `std.test` module is pure YaoXiang functions                                                                                                                                                                                                                                                                                                         | 2026-07-26                | Bootstrap, no native code                                                                                                                                                                                                                                                                                                                                                      |
| Test execution model               | Child process `yaoxiang run <file>` + exit code                                                                                                                                                                                                                                                                                                      | 2026-07-26                | Process-level isolation, zero compiler change                                                                                                                                                                                                                                                                                                                                  |
| Standard library loading           | Currently embed in binary, filesystem in the future                                                                                                                                                                                                                                                                                                  | 2026-07-26                | Version binding, single-file usable                                                                                                                                                                                                                                                                                                                                            |
| Assertion parameter type           | Unannotated parameters (Any), not relying on the generics system                                                                                                                                                                                                                                                                                     | 2026-08-02                | `?` type syntax does not exist; empirically Any is comparable and interpolable                                                                                                                                                                                                                                                                                                 |
| Multi-file run                     | CLI `run` delegates to `run_project` (orchestrator) as a prerequisite                                                                                                                                                                                                                                                                                | 2026-08-02                | Child process model inherits CLI capability; lazy on-use discovery later degrades to a pure performance optimization                                                                                                                                                                                                                                                           |
| Report source location             | Runtime errors carry by default                                                                                                                                                                                                                                                                                                                      | 2026-09-07                | debug_map is generated by default, stack trace outputs `file:line:col`; frame attribution relayed through embedded modules (std.test) is not guaranteed here, belongs to RFC-034                                                                                                                                                                                               |
| Negative test layering             | Value-level reverse generic / compile-failure runner structured markers (internal only) / hard failure folded into Result-ization                                                                                                                                                                                                                    | 2026-09-02                | Value model decided; replaces implicit `[test:error]` convention                                                                                                                                                                                                                                                                                                               |
| Multiple tests in one file         | Value-standard model: test functions return Result, suite collects per-test judgment                                                                                                                                                                                                                                                                 | 2026-09-02                | No catch, no entry call (entry only for internal scenarios)                                                                                                                                                                                                                                                                                                                    |
| Error code                         | Error adds machine-readable `code` field                                                                                                                                                                                                                                                                                                             | 2026-09-02                | Supports error code assertion; compile-time code goes through runner comparison                                                                                                                                                                                                                                                                                                |
| Assertion library form             | 7 value-semantics functions landed, `Result(Void, String)` contract; abort transitional version removed                                                                                                                                                                                                                                              | 2026-09-03                | Void is the canonical unit (`()` is an empty Tuple, not mixed); nested position is Any-rigid, unannotated parameters cannot pass native generic check—parameters must be annotated explicitly (R1 probe empirical)                                                                                                                                                             |
| Test system layering               | Language corpus (`tests/yaoxiang/`) and library tests (go with library, std → `src/std/tests/`) in two layers; std in the corpus is only used as an assertion tool                                                                                                                                                                                   | 2026-09-03                | The object under test determines placement and maintainer; library tests go with the package layout as a preview of RFC-014                                                                                                                                                                                                                                                    |
| Negative marker branching judgment | Branched by expected category: compile-error type `check` must fail, runtime-error type `check` must pass + `run` must fail; report provides category counts                                                                                                                                                                                         | 2026-09-03 (landed 09-06) | Mixed-category judgment allows "compile unexpectedly passes, runtime coincidentally fails" to be missed; expected code nails down stage, syntax errors not given a separate category                                                                                                                                                                                           |
| Header directive grammar           | Expectations declared with English structured directives (`// expect:` / `// skip:` / `// mode:`, strict token grammar, parse failure means direct FAIL); deprecate `[test:error]` boolean marker and Chinese `预期:` prose code scraping                                                                                                            | 2026-09-06                | Expectation is a property of fixture content, in-fixture declaration conforms to industry convention (compiletest / Go / GCC / Clang all do this), a central list is bound to rot; the boolean marker + expectation line are two facts coupled by discipline, which is a defect surface; the structured grammar lets the runner judge mechanically without human participation |
| Parallel execution model           | `--parallel` spawns a per-core worker pool (each file is still an independent child process), skip/invalid handled first in discovery order, execution results streamed in completion order, JSON sorted by path; `--fail-fast` stops scheduling (in-flight runs to completion and is counted); CWD is still shared, isolation boundary not extended | 2026-09-07                | Under the child process model, parallelism = OS thread scheduling of spawn, no yx-layer concurrency needed; the main time cost is per-file full compilation (risk table), parallelism only mitigates the process side—test loop cache slicing is the main mitigation                                                                                                           |

## References

- [RFC-014: Package manager system design](../accepted/014-package-manager.md) — standard library
  directory structure
- [RFC-015: Configuration system](../accepted/015-configuration-system.md) — `[tool.test]`
  configuration section
- [RFC-030: assert mechanism](../review/030-assert-mechanism.md) — low-level dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — reference
  design
