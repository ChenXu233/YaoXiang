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
for YaoXiang. Test files are ordinary `.yx` files, and overall pass/fail is determined by the
subprocess exit code; a file may contain multiple test functions—assertion failures are expressed as
`Err` values (value semantics), and the suite collects per-test verdicts (§7). The `std.test` module
is implemented in pure YaoXiang and is the first dogfooding library. `yaoxiang test` is a CLI tool,
not a compiler feature—no changes to the parser, IR, bytecode, or executor are involved.

## Motivation

### Why a testing framework is needed

Currently, YaoXiang's test coverage relies on Rust-side `#[test]` and the `tests/` integration
tests. This means:

1. Unit tests for the standard library (`std.math` / `std.list` / `std.dict` / `std.convert` /
   `std.io`) cannot be written in YaoXiang
2. `#117 Unit test coverage for std lib modules` is blocked because no testing infrastructure is
   available
3. Regression tests for language features (such as the spawn semantics change in RFC-032) lack
   automated means

### Key constraints

- **17-keyword iron rule**: no new keywords or syntactic constructs are introduced
- **Zero compiler changes**: no touching the parser, IR, bytecode, or executor
- **Bootstrap first**: the test library is written in YaoXiang—the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI layer: yaoxiang test [--filter --fail-fast --json ...]  │
│              │                                               │
│  Discovery: read yaoxiang.toml → [tool.test] patterns        │
│              default: tests/**/*.yx                          │
│              │                                               │
│  Execution: per file: yaoxiang run <file>                    │
│              check exit code → serial execution              │
│              │                                               │
│  Reporting: PASS/FAIL → summary                              │
│              supports --json / --verbose / --fail-fast       │
│                                                              │
│  Assertions: std.test (pure YaoXiang, bootstrapped)          │
│              underlying: std.assert.assert                   │
│              diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core principles

1. **The testing framework is not a compiler feature—it is a CLI tool** — `yaoxiang run` can already
   "execute tests"; `yaoxiang test` simply runs all files for you and shows a report
2. **Zero compiler changes** — no `@test` annotation scanning, bytecode metadata segments, or
   special executor entry points
3. **Bootstrapped** — the `std.test` module is implemented in pure YaoXiang, with underlying
   capabilities from `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files** — files run as subprocesses, and the exit code determines
   overall pass/fail
5. **Assertion failure is a value, not a process event** — test functions return `Result`, and
   assertion failures are expressed as `Err`; the suite collects per-test verdicts (§7);
   process-level aborts belong only to runtime guards and are not used for test assertions

## Detailed Design

### 1. CLI design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml,
                   otherwise tests/)

Options:
  --filter <NAME>     Only run tests whose filename contains <NAME>
  --fail-fast         Stop on the first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              Only list test files; do not run
  --no-progress       Suppress progress output (header and PASS lines); FAIL details and
                       summary are preserved (for CI scenarios)
  --json              Output results in JSON format (for CI integration)
```

#### Output format

**Default output** (per-test verdicts come from in-file suite collection; see §7):

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

- Failed files additionally carry `exit_code` and `stderr` (ANSI-stripped subprocess diagnostics,
  for CI forensics); when `--verbose` is combined with `--json`, all files carry `stdout` / `stderr`
- `--no-progress` only suppresses progress output (header and PASS lines)—FAIL details and the
  summary are always output; failures cannot be silenced; `--list` outputs one test file path per
  line without execution
- Each file carries `kind` (behavior / compile-error / runtime-error / invalid, §8.2); summary
  carries `by_kind` execution counts (four fixed keys, excluding skipped); the human summary carries
  a `Categories:` distribution line
- The per-file `tests` array comes from §7 suite collection, effective as the value-typed model
  lands (#319)

### 2. yaoxiang.toml configuration

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
- Single-file mode (`yaoxiang test foo.yx`) runs directly without reading configuration
- May be split into a separate repository in the future (the `[tool.test]` location stays)

### 3. std.test module (pure YaoXiang)

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library (value-semantics standard form,
// landed 2026-09-03)
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

// assert_not shares a body with assert_false; assert_err / assert_err_code see §8.1
```

- Assertion functions are **value-semantic**: they return `Result(Void, String)`; failure is
  expressed as `Err(diagnostic_info)`, without aborting the process—§7 suites collect per-test
  verdicts based on this. The process-level abort semantics of `std.assert.assert` are preserved for
  runtime guards and do not enter the test assertion path. The Ok payload is `Void` (per
  type-system.md the canonical unit; `()` is the empty Tuple—the two are not mixed—decided
  2026-09-03)
- **7 function family (delivered 2026-09-03, abort transitional version removed)**: value-typed
  `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not` (same body as
  `assert_false`, reserved for the `!assert` unary form) + `assert_err` (§8.1)
  - `assert_err_code` (§8.1 error code assertion)
- `assert_eq` / `assert_ne` use **Any-typed parameters**—`==`/`!=` and f-string interpolation work
  on Any without depending on the generics system. Note that **parameters must be explicitly
  annotated**: unannotated parameters fail the native generics `&Result(T, E)` call check (R1 probe
  verified)
- `assert_false` / `assert_not` use `cond == false` to express negation (the `not` unary syntax has
  not landed; it can be migrated once stable; the `!assert` unary form shares this dependency, see
  §8.1)
- Block body + explicit `return` form: the type of the then-branch in an if expression is discarded
  during checking; the if expression with both branches being Result is a checking blind spot—the
  implementation works around it
- `std.test` depends on no native code; it is pure YaoXiang

### 4. Standard library loading mechanism (key design)

**Phase 1: Embedded binary**

`std/test.yx` (and all future stdlib modules written in YaoXiang) are embedded into the binary at
build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // Source text
    // More in the future
];
```

The module system (RFC-029, fully landed 2026-08-02) provides the integration point: the Registry
holds both native modules and source modules, and the orchestrator handles multi-file orchestration.
Resolution order for `use std.test`:

1. First check Rust native modules (existing mechanism, e.g., `std.assert`)
2. If not found, check embedded `STD_YX_FILES`—on a hit, inject the module into the orchestrator via
   a **virtual path** (e.g., `<std>/test.yx`) as the seed module, going through the normal frontend
   pipeline (parse → typecheck → IR)
3. If not found, fall through to filesystem discovery (user modules)

`use std.assert` inside embedded source modules is resolved normally by the resolver to the native
registry—native and source modules coexist in the Registry, so cross-kind dependencies naturally
hold. Embedded modules are **compiled on demand**: they only enter the pipeline when imported.

Advantages:

- `use std.test` works in single-file mode
- The stdlib version is strictly bound to the binary—no version mismatch
- No need for the user to configure stdlib paths

**Future: Filesystem stdlib**

Once the YaoXiang project model matures, the stdlib will switch to a filesystem form. See updates to
RFC-014 for details.

### 5. Discovery and execution

**Prerequisite (2026-08-02 review decision)**: CLI `run` is wired into the orchestrator. The current
CLI `run` goes through the single-file pipeline (`run_file_with_diagnostics`) and cannot resolve
user module imports; but the subprocess model of `yaoxiang test` inherits CLI capability, and test
files importing project modules is a core scenario. Therefore Phase 1 first delegates the source
branch of CLI `Run` to `run_project` (the orchestrator, which does directory-recursive discovery);
#247 (on-demand discovery along `use`) afterwards layers on as a pure performance optimization.
Single files without imports behave equivalently through the orchestrator, and the bytecode branch
is unchanged.

**Discovery phase**:

1. If `[PATHS]` is specified, use it directly
2. Otherwise read `[tool.test].patterns` from `yaoxiang.toml`
3. If no configuration, default to `tests/**/*.yx`
4. Apply `--filter` (filename substring)
5. The discovery scope is the test layering (§9): the default patterns only cover language usability
   corpus; the library test layer (e.g., `src/std/tests/`) is discovered via explicit paths or
   package configuration, and is not mixed into the default scan

**Execution phase**:

1. For each file, branch execution based on header directives (directive grammar in §8.2, parsed via
   `src/util/test_markers.rs`, shared with yx_runner):
   - Behavior test: `yaoxiang run --debug-info <file>` subprocess (`--debug-info` makes runtime
     errors carry source locations—2026-08-02 verified stack trace outputs `file:line:col`);
     `// mode:` declares the subprocess `--runtime` mode
   - Compile-rejection kind: single-step `yaoxiang check <file>`
   - Runtime-failure kind: two-step `check` (must pass) + `run` (must fail)
2. Files with `// skip: <reason>` are skipped and counted as skipped in the report
3. Verdict and expected code comparison follow the §8.2 verdict matrix; directive parse failure does
   not execute and directly FAILs (construction-time rejection)
4. Capture stdout/stderr for the report
5. Serial execution only (Phase 1); `--parallel` support in the future
6. If `--fail-fast`, stop on the first FAIL

### 6. Test isolation

Test isolation is naturally achieved via the process-level boundary:

- Each test file runs in an independent subprocess
- Each subprocess has its own Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional isolated Heap context mechanism is required

### 7. Suite and multiple tests (value-typed model)

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

- Each test is a zero-argument function returning `Result(Void, String)`; assertion failures are
  expressed as `Err` (§3 value-semantics assertion family), and do not interrupt the
  process—subsequent tests continue to run
- `test.suite` calls and collects one by one: a non-Ok test records the name and diagnostic, Ok is
  silent; after all run, if any Err exists, it aborts via `std.assert.assert` with the failure
  details attached (`N of M test(s) failed` + each `[FAIL] name: diagnostic`)—the file exit code is
  non-zero (§5 verdict unchanged). The abort here is the test binary's runtime guard, not the
  assertion path; all-Ok silently exits 0
- Top-level test functions are **enqueued as closures** (`("name", () => test_fn())`): top-level
  function names as value references are not yet supported (IR-layer limitation, `E3006`)—calling
  global functions inside closure bodies is unaffected
- The runner only sees files, and does not do function-level scanning: per-test verdicts come
  entirely from in-suite collection, and the in-file structure is transparent to the runner—the
  zero-compiler-change principle is not affected
- Explicitly not adopted: in-process catch boundaries (17-keyword iron rule); runner entry-point
  call per function (limited to internal scenarios such as §8.2 compile failures)
- API form is decided (2026-09-03, #319):
  `suite(tests: List((String, () -> Result(Void, String)))) -> Void`; duplicates are not detected
  (the name is only used for report display); `--filter` filters by filename and is unaware of
  in-suite test names

### 8. Negative tests (expected failure) — three-layer design

Negative tests are split by the layer at which the failure occurs, and each layer has its place:

#### 8.1 Value-level reversal (general, user-facing)

The operation under test returns a `Result`; the test expresses the expected failure with normal
assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// Or a one-line wrapper (the code only exists on the std Error carrier; E is pinned to Error):
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` are delivered together with the value-semantics
  family (2026-09-03, §3); the `!assert` unary form awaits the landing of the `not` syntax (shares
  the same constraint as `assert_false`'s `cond == false`)
- Error code assertions depend on the `Error` value carrying a machine-readable `code` field—already
  delivered by #323 M4: `Error = { code, message }` (native `error_new(code, message)`); reading
  goes through `result.unwrap_err(r)` to obtain the carrier + `result.code(e)` / `result.message(e)`
  accessors (the design-phase estimated `error_new_with_code` naming, code constant export, and
  `err.code` field access were not adopted—the language has no Struct field access, and code
  constants are not exported)
- As Result-ification advances (#301, #316), failing operations one by one return `Result`, and the
  file-level negative markers in the corpus migrate to in-file assertions

#### 8.2 File-level negative directives (language designers' internal use only)

Compilation is all-or-nothing per file, so "this line shouldn't compile" cannot be expressed
in-file; runtime failures likewise need file-level expression (e.g., suites containing tests that
must fail). The file header declares expectations via **structured directives**, and the runner
**branches verdicts** by expected category (branching decided 2026-09-03; directive grammar decided
and landed 2026-09-06).

Header directive grammar (`// key: value`, within the first 16 lines, strict token matching):

```text
// expect: compile-error E1002 [E1003 ...]   compile-rejection test
// expect: runtime-error E6003 [...]         runtime-failure test
// skip: <reason>                            skip execution, counted as skipped
// mode: embedded|standard|full              subprocess --runtime mode (consumed by the run step only)
```

- No `expect:` directive = behavior test. `expect:` is the **sole declaration** of expectation—on
  2026-09-06 the decision was made to abandon the `[test:error]` boolean marker and the Chinese
  `预期:` prose grep: the boolean marker and the expectation line are two loosely coupled facts, and
  staying in sync purely by discipline is bound to drift; strict English token grammar lets the
  runner parse mechanically (kind + codes are all fixed tokens; any extra tokens after a code is a
  parse failure), parse failure = do not execute and directly FAIL—there is no silent degradation
  channel for incorrectly declared directives (construction-time rejection)
- **Compile-error kind**: single-step `check`—must fail and the output must contain all `[EXXXX]`;
  successful compilation = FAIL (expected error not reported), rejection but mismatched codes =
  FAIL. Syntax errors (E1xxx parse phase) and semantic errors (E2xxx+) are not given separate
  categories—the expected codes themselves pin down the phase
- **Runtime-error kind**: two-step verdict—`check` must **succeed** (no compile-time guilt), `run`
  must fail and the output must contain all `[EXXXX]`. Compile-time crash = FAIL (a key verdict in
  the opposite direction to the compile-error kind, preventing "compilation accidentally passing,
  runtime failure by chance" misses)
- Convergent with industry: Rust compiletest's `//~ ERROR`, Go's `// ERROR "regexp"`, GCC's
  `dg-error`, and Clang's `expected-error` all declare expectations in fixture comments and use the
  harness for two-way comparison; they use line-level anchoring because multi-diagnostic compilers
  need to distinguish multiple expectations within a file; this compiler halts at the first error
  with one diagnostic per file, so file-level is isomorphic to the compiler's reality—after error
  recovery lands, a line-anchored form can be added to the grammar (Cranelift filetests' file-header
  directives + function-level expectations are the same kind of mixed form)
- Known rendering debt: parse-phase diagnostics are currently output in Debug form (`code: "E0012"`
  rather than `[E0012]`); the code scanner accepts both forms; the strict form is reinstated after
  diagnostic rendering is unified
- **Serves only this repository's corpus; it is not part of the user-facing testing framework**; the
  dual-runner verdict convention is closed (2026-09-03, #319): yx_runner (cargo test) and
  `yaoxiang test` share `src/util/test_markers.rs` to parse header directives; the 06-compile-errors
  directory convention is deprecated. The reporting layer produces category counts: the human
  summary carries a `Categories:` line; JSON summary carries `by_kind` (behavior / compile-error /
  runtime-error / invalid, skipped counted separately); each file carries `kind`

#### 8.3 Runtime hard failures (subsumed into Result-ification)

No independent mechanism is established—failing operations return `Result` per the language
direction (#301, #316), and tests uniformly take the §8.1 expression. Process-level aborts (such as
assertion violations, runtime parameter misalignment) gradually converge to values as
Result-ification proceeds, and the testing framework provides no dedicated semantics for them.
(Note: the "runtime-error kind" marker verdict in §8.2 is the runner's file-level verification
channel for **operations not yet Result-ifiable**, which is not in tension with the semantic
direction of this section—the latter is the endpoint; the former is the migration-period channel)

### 9. Test system layering: language corpus and library tests (decided 2026-09-03)

Tests are split into two layers by **the object under test**, each with its own home and maintainer;
the marker system (§8.2) and assertion library (§3) are shared between the two layers:

**Layer 1: Language usability corpus (`tests/yaoxiang/`)**

- The object under test is **the language itself**—parser, type system, modules, concurrency,
  ownership, compile-time rejection, runtime semantics; directories are organized by language spec
  chapter
- std only appears in the corpus as an **assertion tool** (`std.assert` / `std.test`); it is never
  the object under test—library API behavior does not belong to language usability
- Within the corpus, verdicts are split per §8 into three categories: behavior test /
  compile-rejection test / runtime-failure test

**Layer 2: Library tests (live with the library)**

- The object under test is **the library's public API contract** (e.g., `list.push` behavior,
  `result.code` semantics)
- Tests live **within the library's own package**: for std the package is `src/std/`, and its
  yx-level tests go to `src/std/tests/` (in the same place as the implementation; the directory
  coexists with Rust unit tests, and the file types do not intersect); the tests of `std.test`
  itself also live there (using std.test to test std.test—the bootstrapping closed loop)
- Future user packages follow the same convention: tests live in the package, discovered via the
  package's `[tool.test]` (the test layout of RFC-014 package management is previewed by this)
- Discovery is not included in the default patterns (default `tests/**/*.yx` only covers the
  language layer): the library test layer is discovered via explicit paths
  (`yaoxiang test src/std/tests`) or package configuration; CI runs them in layers

Migration note: **Migrated (2026-09-06)**—after one-by-one triage, all 19 files in the original
`tests/yaoxiang/07-std/` turned out to be library tests (the object under test in each was the API
contract of a std module; the role of `?` propagation, auto-borrow, generic instantiation, etc. in
them is a vehicle, not the object under test), and the whole set migrated into `src/std/tests/`; the
07-std directory was removed; yx_runner was changed to dual-root discovery (`tests/yaoxiang/` +
`src/std/tests/`), and the default patterns do not include the library layer (integration tests
cement this contract). The language corpus is now zero std-API tests.

## Relationship with existing systems

| Item                                                 | Relationship                                                                                                        |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                       | Untouched; compiler-internal tests continue to use Rust                                                             |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                          |
| `std.assert.assert(cond)`                            | Reserved for runtime guards; `std.test` value-semantics assertion family is based on `std.result` (§3, §7)          |
| Module system (RFC-029)                              | Embedded source modules integrate via Registry/orchestrator; wiring CLI `run` to the orchestrator is a prerequisite |
| `#200` refactor (`io.println` → `assert.assert`)     | Shares the exact same direction as `yaoxiang test`                                                                  |
| `@` annotation                                       | Not used; no `@test` is introduced                                                                                  |

## Implementation strategy

### Phase 1: Core functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project`
  (multi-file run prerequisite)
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

### Phase 2: Polish

- `--filter` / `--fail-fast` / `--verbose` flags
- `--json` output (CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: Advanced

- `--parallel` concurrent execution (depends on spawn concurrency model being complete)
- `[tool.test].exclude` configuration
- More assertion functions (e.g., `assert_approx_eq` for Float)

## Risks and mitigations

| Risk                                                  | Probability | Mitigation                                                                                                                                                                                          |
| ----------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` interpolation on Any fails                   | None        | Verified 2026-08-02 (Int/String both work)                                                                                                                                                          |
| `yaoxiang.toml` config parsing not in current CLI     | Low         | Simple extension; doesn't affect core functionality                                                                                                                                                 |
| CLI run wiring to orchestrator introduces regressions | Low         | Single-file path without imports is equivalent; integration tests already cover the orchestrator                                                                                                    |
| Embedding `.yx` source files increases binary size    | Low         | `.yx` source files are tiny; negligible                                                                                                                                                             |
| Test loop latency grows with corpus                   | High        | The main cost is per-file full compilation (185 files measured 11.3s), not subprocess startup; `--parallel` only helps the process side; compilation cost needs a test loop cache (#251/#293 slice) |

## Open questions

- [x] Can `use std.assert` references in `std/test.yx` be resolved correctly? —**Resolved
      (2026-08-02)**. After the module system (RFC-029) landed, native and source modules coexist in
      the Registry, and the resolver handles them uniformly; cross-kind dependencies naturally hold
- [x] Does `f"..."` generic `to_string` in test output introduce new type constraints? —**Resolved
      (2026-08-02)**. Verified that `==`/`!=` and f-string interpolation work on unannotated
      parameters (Any) (Int/String verified), and no new constraints are introduced
- [x] Is `?` generic parameter feasible? —**Resolved (2026-08-02)**: the `?` type syntax does not
      currently exist (and is silently swallowed; tracked in a separate issue); Phase 1 assertion
      functions use unannotated parameters, not depending on the generics system

## Design decision log

| Decision                         | Determination                                                                                                                                                                                                              | Date                      | Rationale                                                                                                                                                                                                                                                                                                                                                     |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Test marker method               | No `@test` annotation; test files are ordinary `.yx`                                                                                                                                                                       | 2026-07-26                | Zero compiler changes; subprocesses provide isolation                                                                                                                                                                                                                                                                                                         |
| Assertion method                 | Pure YaoXiang functions in the `std.test` module                                                                                                                                                                           | 2026-07-26                | Bootstrapped; no native code                                                                                                                                                                                                                                                                                                                                  |
| Test execution model             | Subprocess `yaoxiang run <file>` + exit code                                                                                                                                                                               | 2026-07-26                | Process-level isolation; zero compiler changes                                                                                                                                                                                                                                                                                                                |
| Stdlib loading                   | Currently embedded in binary; filesystem in the future                                                                                                                                                                     | 2026-07-26                | Version binding; works in single-file mode                                                                                                                                                                                                                                                                                                                    |
| Assertion parameter types        | Unannotated parameters (Any); no dependency on generics                                                                                                                                                                    | 2026-08-02                | `?` type syntax does not exist; Any verified to be comparable and interpolable                                                                                                                                                                                                                                                                                |
| Multi-file run                   | CLI `run` delegates to `run_project` (orchestrator) as a prerequisite                                                                                                                                                      | 2026-08-02                | Subprocess model inherits CLI capability; #247 degrades to a pure performance optimization                                                                                                                                                                                                                                                                    |
| Report source locations          | Subprocess with `--debug-info`                                                                                                                                                                                             | 2026-08-02                | Verified stack trace outputs `file:line:col`; frame attribution through embedded modules (std.test) is not guaranteed by this and belongs to #289 + RFC-034                                                                                                                                                                                                   |
| Negative test layering           | Value-level reversal general / compile-failure runner structured markers (internal only) / hard failures subsumed into Result-ification                                                                                    | 2026-09-02                | Decided by #319; replaces implicit `[test:error]` convention                                                                                                                                                                                                                                                                                                  |
| Multiple tests in a file         | Value-typed standard model: test functions return Result; suites collect per-test verdicts                                                                                                                                 | 2026-09-02                | No catch; not entry-point call (entry points only for internal scenarios)                                                                                                                                                                                                                                                                                     |
| Error code                       | Error gains a machine-readable `code` field                                                                                                                                                                                | 2026-09-02                | Supports error code assertions; compile-time codes are compared by the runner                                                                                                                                                                                                                                                                                 |
| Assertion library form           | Value-semantics family of 7 functions landed; `Result(Void, String)` contract; abort transitional version removed                                                                                                          | 2026-09-03                | Void is the canonical unit (`()` is the empty Tuple—do not mix); nested site is rigid Any; unannotated parameters fail the native generics check—parameters must be explicitly annotated (R1 probe verified)                                                                                                                                                  |
| Test system layering             | Language corpus (`tests/yaoxiang/`) and library tests (live with the library; std → `src/std/tests/`) split into two layers; std only acts as an assertion tool in the corpus                                              | 2026-09-03                | The object under test determines ownership and maintainer; library test layout living with the package previews RFC-014                                                                                                                                                                                                                                       |
| Negative-marker branched verdict | Branch by expected category: compile-error kind `check` must fail, runtime-error kind `check` must pass + `run` must fail; the report gives category counts                                                                | 2026-09-03 (landed 09-06) | Mixed-category verdicts would let "compilation accidentally passing, runtime failure by chance" slip through; expected codes pin down the phase; syntax errors are not given a separate category                                                                                                                                                              |
| Header directive grammar         | Expectations declared as English structured directives (`// expect:` / `// skip:` / `// mode:`, strict token grammar, parse failure directly FAILS); abandon `[test:error]` boolean markers and Chinese `预期:` prose grep | 2026-09-06                | Expectations are properties of fixture content; in-fixture declaration is convergent with industry (compiletest / Go / GCC / Clang all do this), a central list inevitably rots; boolean marker + expectation line being two facts coupled by discipline is a defect surface; structured grammar lets the runner judge mechanically without human involvement |

## References

- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — Stdlib directory
  structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]`
  configuration section
- [RFC-030: assert Mechanism](../review/030-assert-mechanism.md) — Underlying dependency
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — Reference
  design
