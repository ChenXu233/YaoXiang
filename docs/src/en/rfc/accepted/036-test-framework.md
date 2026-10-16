---
title: 'RFC-036: std.test Test Framework and yaoxiang test Command'
status: 'Accepted'
author: 'Chenxu'
created: '2026-07-26'
updated: '2026-09-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test Test Framework and yaoxiang test Command

## Summary

Introduce the standard test framework `std.test` module and the `yaoxiang test` CLI subcommand for
YaoXiang. Test files are ordinary `.yx` files; the subprocess exit code determines overall
pass/fail; files internally support multiple test functions—assertion failures are expressed as
`Err` values (value semantics), and the suite collects per-test verdicts (§7). The `std.test` module
is implemented in pure YaoXiang, making it the first dogfooding library. `yaoxiang test` is a CLI
tool, not a compiler feature—it involves no changes to the parser, IR, bytecode, or executor.

## Motivation

### Why a Test Framework?

YaoXiang's current test coverage relies on Rust-side `#[test]` and the `tests/` integration tests.
This means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io)
   cannot be written in YaoXiang
2. Unit test coverage of standard library modules is blocked because no test infrastructure is
   available
3. Regression tests for language features (such as the RFC-032 spawn semantics change) lack
   automated means

### Key Constraints

- **17-keyword iron rule**: no new keywords or syntax constructs
- **Zero compiler changes**: don't touch the parser, IR, bytecode, or executor
- **Self-hosting first**: the test library is written in YaoXiang, making it the first dogfooding
  library

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
│  Execution:  per file: yaoxiang run <file>                    │
│              check exit code → serial execution              │
│              │                                               │
│  Reporting:  PASS/FAIL → summary                             │
│              supports --json / --verbose / --fail-fast        │
│                                                              │
│  Assertion:  std.test (pure YaoXiang, self-hosted)            │
│              base: std.assert.assert                          │
│              diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **The test framework is not a compiler feature, it's a CLI tool** — `yaoxiang run` can already
   "execute tests"; `yaoxiang test` just runs all the files for you and shows you a report
2. **Zero compiler changes** — no `@test` annotation scanning, no bytecode metadata segments, no
   special executor entry points
3. **Self-hosting** — the `std.test` module is implemented in pure YaoXiang, with base capabilities
   from `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files** — files run as subprocesses, exit code determines overall
   pass/fail
5. **Assertion failures are values, not process events** — test functions return `Result`, failures
   are expressed as `Err`, the suite collects per-test verdicts one by one (§7); process-level abort
   belongs only to runtime guards, not to test assertions

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Only run test files whose name contains <NAME>
  --fail-fast         Stop on the first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              Only list test files, don't run
  --no-progress       Suppress progress output (header and PASS lines); FAIL details and summary
                      are preserved (CI scenario)
  --json              Output results in JSON format (for CI integration)
  --parallel          Execute in parallel (one worker per core; ORed with [tool.test].parallel)
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
  summary are always emitted, failures cannot be silenced; `--list` outputs one test file path per
  line without execution
- Each file carries `kind` (behavior / compile-error / runtime-error / invalid, §8.2); the summary
  carries `by_kind` execution counts (fixed four keys, not including skipped); the human summary
  carries a `Categories:` distribution line
- Under `--parallel`, the human progress lines stream in **completion order** (whole blocks not
  interleaved); JSON `files` are sorted by `file` path for stable output (CI diff friendly)
- The in-file per-test `tests` array comes from the §7 suite collection, taking effect with the
  value-based model
- The official CI (`.github/workflows/ci.yml` test job) consumes it this way: the cargo side runs
  `--test integration` (CLI integration) and `--test yx_runner` (dual-root corpus guard), then
  `yaoxiang test --json --parallel` runs in layers—default mode (language corpus) and explicit
  `src/std/tests` (library layer) each produce a report; the summary (total / passed / failed /
  skipped / time_secs and by_kind) is written to the job summary, failed files print `kind` /
  `exit_code` / `stderr` for forensics; any non-zero suite exit is red

### 2. yaoxiang.toml Configuration

Placed under `[tool.test]`, conforming to the `[tool.*]` third-party extension convention of
RFC-015:

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # matches are removed from the discovery set (--list removes them too)
parallel = true                   # parallel execution (ORed with --parallel flag)
```

- Default `patterns = ["tests/**/*.yx"]` — zero-config out-of-the-box for users
- `exclude` shares the same form as `patterns` (literal paths or `root/**…`, always matched by path
  prefix); excluded means not a test—fixtures that need runtime behavior validation run directly
  with `yaoxiang run`
- **Single-file mode (`yaoxiang test foo.yx`) runs directly, without reading config**—under explicit
  paths, the `exclude` / `parallel` config keys have no effect (except the flag)
- May be split into a separate repository in the future (the `[tool.test]` location remains
  unchanged)

### 3. std.test Module (Pure YaoXiang)

```yaoxiang
// std/test.yx — pure YaoXiang test assertion library (value-semantics standard form, landed 2026-09-03)
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

- Assertion functions are **value-semantic**: they return `Result(Void, String)`, failure is
  expressed as `Err(diagnostic)`, no process abort—the §7 suite collects per-test verdicts
  accordingly. The process-level abort semantics of `std.assert.assert` are reserved for runtime
  guards and do not enter the test assertion path. The Ok payload is `Void` (unit per
  type-system.md; `()` is an empty Tuple, the two are not mixed—decided 2026-09-03)
- **7 functions in the family (delivered 2026-09-03, abort transition version removed)**:
  value-based `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not` (shares the
  body with assert_false, reserved for the `!assert` form) + `assert_err` (§8.1)
  - `assert_err_code` (§8.1 error code assertion)
  - `assert_approx_eq(a: Float, b: Float, eps: Float)` (Phase 3, delivered 2026-09-07):
    `|a - b| <= eps` verdict, with `eps` provided **explicitly** by the caller—tolerance is part of
    the test contract, no hidden default; a negative `eps` is an Err at the declaration site, NaN is
    always Err
- `assert_eq` / `assert_ne` use **Any-annotated parameters**—`==`/`!=` and f-string interpolation
  work normally on Any, without depending on the generics system. Note that parameters **must be
  annotated explicitly**: unannotated parameters cannot pass the call check against native generic
  `&Result(T, E)` (empirically verified by the R1 probe)
- `assert_false` / `assert_not` use `cond == false` to express negation (the unary `not` syntax is
  not yet landed; it can be migrated once stable; the unary `!assert` form has the same dependency,
  see §8.1)
- Block body + explicit `return` form: the then-arm type of the if expression is dropped during
  checking; a two-arm-Result if expression is a checking blind spot, so the implementation works
  around it
- `std.test` depends on no native code; it's pure YaoXiang

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embed in Binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) is embedded in the
binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // source text
    // more in the future
];
```

The module system (RFC-029, fully landed 2026-08-02) provides the integration point: the Registry
holds both native modules and source modules, and the orchestrator handles multi-file choreography.
The resolution order for `use std.test`:

1. First check Rust native modules (existing mechanism, such as `std.assert`)
2. If missed, check the embedded `STD_YX_FILES`—if matched, inject the module into the orchestrator
   with a **virtual path** (e.g. `<std>/test.yx`) as the seed module, going through the normal
   frontend pipeline (parse → typecheck → IR)
3. If missed, fall through to filesystem discovery (user modules)

`use std.assert` inside an embedded source module is resolved normally by the resolver to the native
registry—native and source modules coexist in the Registry, so cross-kind dependencies work
naturally. Embedded modules are **compiled on demand**: they enter the pipeline only when imported.

Advantages:

- `use std.test` also works in single-file mode
- Standard library version is strictly bound to the binary, no version mismatch
- No need for users to configure the standard library path

**Future: Filesystem Standard Library**

When the YaoXiang project model matures, the standard library will switch to a filesystem form. See
the RFC-014 update for details.

### 5. Discovery and Execution

**Prerequisite (2026-08-02 review decision)**: the CLI `run` integrates with the orchestrator. The
current CLI `run` goes through a single-file pipeline (`run_file_with_diagnostics`), which cannot
resolve user module imports; whereas `yaoxiang test`'s subprocess model inherits the CLI's
capabilities, and test files importing project modules is a core scenario. Therefore Phase 1 first
delegates the CLI `Run`'s source-file branch to `run_project` (orchestrator, recursive directory
discovery); on-demand discovery via `use` then becomes a pure performance optimization layered on
top. Single files without imports behave equivalently under the orchestrator, the bytecode branch is
unchanged.

**Discovery Phase**:

1. If `[PATHS]` is specified, use those paths directly
2. Otherwise, read `[tool.test].patterns` from `yaoxiang.toml`
3. If not configured, default to `tests/**/*.yx`
4. Apply `--filter` (file name contains)
5. The discovery scope is the test layering (§9): default patterns cover only the
   language-availability corpus; the library test layer (e.g. `src/std/tests/`) is discovered via
   explicit paths or package configuration, not mixed into the default scan
6. **Project directories are not loose test sets**: when recursive discovery encounters a directory
   containing `yaoxiang.toml`, that directory is a **project** (the manifest is both the project
   identity marker and the Bin-role determination basis), and only its conventional entry `main.yx`
   is brought into discovery—no recursion, no other `.yx` files in the same directory (those are
   project modules, imported via `use`). Rationale: by definition module files have no `main`, so
   running them as standalone programs must report "missing main binding"; and putting non-test
   `.yx` files (fixtures, library modules) under `tests/` is a **legal layout**—the discovery rule
   must not depend on the invalid assumption that "every `.yx` under `tests/**` is an independently
   runnable test". When a project directory lacks an entry, no files are discovered (a project is
   not a loose test set), and this is not an error boundary
7. **`.yx` files not belonging to any project directory are treated as test targets**—this is the
   definition of the default rule. When you really need to place non-test auxiliary `.yx` files, put
   them in a project directory, or exclude them in `[tool.test].exclude` (existing mechanism in
   Phase 3)

**Execution Phase**:

1. For each file, dispatch by header directive (directive grammar in §8.2, parsed via
   `src/util/test_markers.rs`, shared with yx_runner):
   - Behavior tests: `yaoxiang run <file>` subprocess (runtime errors carry source location and
     stack frame by default—debug_map is generated by default, stack trace outputs `file:line:col`);
     `// mode:` declares the subprocess `--runtime` mode
   - Compile-rejection class: single-step `yaoxiang check <file>`
   - Runtime-failure class: two steps `check` (must pass) + `run` (must fail)
2. Files with `// skip: <reason>` are skipped and counted in the report as skipped
3. Verdict comparison against expected codes follows the §8.2 verdict matrix; a directive parse
   failure means no execution and direct FAIL (construction-time rejection)
4. Capture stdout/stderr for the report
5. Serial by default; `--parallel` (or `[tool.test].parallel`) spawns a worker pool by available
   core count; each file is still an independent subprocess—skip/invalid is processed first in
   discovery order, execution results stream in completion order, JSON is sorted by path (Phase 3,
   delivered 2026-09-07)
6. If `--fail-fast`, stop scheduling new files on the first FAIL; under parallel mode, in-flight
   files finish and are counted

### 6. Test Isolation

Test isolation is naturally achieved through the process-level boundary:

- Each test file runs in its own subprocess
- Each subprocess has its own Heap, Frame, and NativeContext
- A panic in one test file does not affect other test files
- No additional isolated Heap context mechanism is needed
- **Parallel execution (Phase 3) does not extend the isolation boundary**: subprocesses share the
  working directory (CWD); parallel tests must not occupy files at the same path under CWD—file I/O
  tests should use independent file names and clean up on completion

### 7. Suite and Multiple Tests (Value-Based Model)

A test file can contain multiple tests. The internal organization (landed 2026-09-03):

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
  expressed as `Err` (the §3 value-semantic assertion family), without interrupting the
  process—subsequent tests continue to run
- `test.suite` calls and collects one by one: a non-Ok test records the name and diagnostic, Ok is
  silent; after all have run, any Err triggers abort via `std.assert.assert` with the failure
  details attached (`N of M test(s) failed` + each `[FAIL] name: diagnostic`)—the file exits
  non-zero (§5 verdict unchanged). The abort here is the test binary's runtime guard, not the
  assertion path; all Ok exits silently with 0
- Top-level test functions enter the queue in **closure form** (`("name", () => test_fn())`):
  referencing top-level function names as values is not yet supported (IR-layer limitation,
  `E3006`)—calling a global function inside a closure body is not affected
- The runner only sees the file, no function-level scanning: per-test verdicts come entirely from
  in-suite collection; the file's internal structure is transparent to the runner—the
  zero-compiler-change principle is unaffected
- Explicitly not adopted: in-process catch boundaries (17-keyword iron rule); runner entry-point
  per-function calls (limited to internal scenarios such as §8.2 compile failures)
- The API form has been finalized (2026-09-03):
  `suite(tests: List((String, () -> Result(Void, String)))) -> Void`; duplicate names are not
  detected (names are only used for report display); `--filter` filters by file name and is unaware
  of in-suite test names

### 8. Three-Layer Design for Negative Tests (Expected Failure)

Negative tests are split by where the failure occurs, with each layer having its own home:

#### 8.1 Value-Level Reversal (General, User-Facing)

The operation under test returns a `Result`, and the test expresses the expected failure with
ordinary assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// or one packaged call (the code only exists on the std Error carrier, E is hard-pinned to Error):
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` have been delivered with the value-semantic family
  (2026-09-03, §3); the unary `!assert` form awaits the landing of the `not` syntax (same constraint
  as `cond == false` in `assert_false`)
- Error code assertions depend on the `Error` value carrying a machine-readable `code` field—
  landed: `Error = { code, message }` (native `error_new(code, message)`); reading goes through
  `result.unwrap_err(r)` to obtain the carrier + `result.code(e)` / `result.message(e)` accessors
  (the design-stage predicted `error_new_with_code` name, exported code constants, and `err.code`
  field access are all not adopted—the language has no Struct field access, and code constants are
  not exported)
- As Result-ification advances, fallible operations one by one return `Result`, and file-level
  negative markers in the corpus migrate to in-file assertions

#### 8.2 File-Level Negative Directives (Internal Use Only for Language Designers)

Compilation is all-or-nothing for the whole file, so you cannot express "this line shouldn't
compile" within a file; runtime failures also need file-level expression (e.g. when the suite
contains tests that must fail). The file header declares expectations with **structured
directives**, and the runner dispatches by expected category (dispatch decided 2026-09-03; directive
grammar decided and landed 2026-09-06).

Header directive grammar (`// key: value`, within the first 16 lines, strict token matching):

```text
// expect: compile-error E1002 [E1003 ...]   compile-rejection test
// expect: runtime-error E6003 [...]         runtime-failure test
// skip: <reason>                            skip execution, counted as skipped
// mode: embedded|standard|full              subprocess --runtime mode (consumed only by the run step)
```

- No `expect:` directive = behavior test. `expect:` is the **sole declaration** of
  expectation—2026-09-06 decided to deprecate the `[test:error]` boolean marker and the Chinese
  `预期:` prose approach: a boolean marker and an expectation line are two loosely coupled facts,
  and relying on discipline to keep them in sync inevitably drifts; the strict English-token grammar
  lets the runner parse mechanically (kind + codes are fixed tokens, any extra token after a code is
  a parse failure), parse failure = no execution and direct FAIL—no silent degradation path for
  malformed directive declarations (construction-time rejection)
- **Compile-error class**: single-step `check`—must fail and the output must contain all `[EXXXX]`;
  compile passing = FAIL (what should have been reported wasn't), rejection but codes don't match =
  FAIL. Syntax errors (E1xxx parse stage) and semantic errors (E2xxx+) have no independent
  category—the expected code itself pins the stage
- **Runtime-error class**: two-step verdict—`check` must **succeed** (compile-stage innocent), `run`
  must fail and the output must contain all `[EXXXX]`. Compile-stage explosion = FAIL (the
  opposite-direction key verdict versus the compile-error class, preventing "compile accidentally
  passes, run coincidentally fails" miss-detection)
- Co-structured with the industry: Rust compiletest `//~ ERROR`, Go `// ERROR "regexp"`, GCC
  `dg-error`, Clang `expected-error` all declare expectations inside fixture comments and are
  compared bidirectionally by the harness; they use line-level anchoring because multi-diagnostic
  compilers need to distinguish multiple expectations in the same file, but this compiler stops at
  the first error with one diagnostic per file, so file-level is isomorphic to the compiler's
  reality—once error recovery is landed, the line-anchor form can be added to the grammar (Cranelift
  filetests' file-header directives + function-level expectations are the same mixed form)
- Known rendering debt: parse-stage diagnostics currently output in Debug form (`code: "E0012"`
  rather than `[E0012]`); code scanning accepts both forms; strict form is restored after diagnostic
  rendering is unified
- **Serves only this repository's corpus, not part of the user-facing test framework**; the
  dual-runner verdict convention has been closed off (2026-09-03): yx_runner (cargo test) and
  `yaoxiang test` share `src/util/test_markers.rs` to parse header directives, and the
  `06-compile-errors` directory convention is deprecated. The reporting layer gives category counts:
  the human summary has a `Categories:` line, the JSON summary has `by_kind` (behavior /
  compile-error / runtime-error / invalid, skipped counted separately), each file has a `kind`

#### 8.3 Runtime Hard Failure (Folded into Result-ification)

No independent mechanism is set—operations that can fail return `Result` per the language direction,
and tests uniformly go through §8.1 expression. Process-level abort (e.g. assertion violations,
runtime parameter mismatches) is gradually converged into values as Result-ification proceeds, and
the test framework provides no dedicated semantics for them. (Note: the "runtime error class" marker
verdict in §8.2 is the runner's file-level verification channel for **operations that cannot yet be
Result-ified**, which is not in conflict with the semantic direction of this section—the latter is
the destination, the former is the migration channel)

### 9. Test System Layering: Language Corpus and Library Tests (Decided 2026-09-03)

Tests are split into two layers by **the object under test**, each with its own home and maintainer;
the marker system (§8.2) and the assertion library (§3) are shared at both levels:

**Layer 1: Language-Availability Corpus (`tests/yaoxiang/`)**

- The object under test is **the language itself**—parser, type system, modules, concurrency,
  ownership, compile-time rejection, runtime semantics; directories are organized by language spec
  chapter
- std only appears in the corpus as an **assertion tool** (`std.assert` / `std.test`), never under
  test—library API behavior does not belong to language availability
- The corpus is split by §8 into three verdict classes: behavior tests / compile-rejection tests /
  runtime-failure tests

**Two forms of Layer 1**: single-file corpus (one `.yx` is one independently runnable program) and
**project fixtures** (one directory per fixture, with `yaoxiang.toml` + entry `main.yx` + module
files, e.g. `tests/yaoxiang-multifile/`; project-level contract per RFC-039 D48). Module files in
project fixtures are not test targets—default discovery only takes their entry point (§5 item 6);
the project-semantic verdict of a whole fixture is performed by a dedicated runner
(`cargo test --test yx_multifile_runner`).

**Layer 2: Library Tests (Travel with the Library)**

- The object under test is **the library's public API contract** (e.g. `list.push` behavior,
  `result.code` semantics)
- Tests are written **in the library's own package**: the std package is `src/std/`, and its
  yx-level tests go to `src/std/tests/` (co-located with the implementation; the directory coexists
  with Rust unit tests, the file types don't intersect); tests for `std.test` itself also live there
  (using std.test to test std.test, closing the self-hosting loop)
- Future user packages follow the same convention: tests inside the package, discovered by the
  package's `[tool.test]` (RFC-014 package management's test layout is previewed by this)
- Discovery does not enter default patterns (default `tests/**/*.yx` only covers the language
  layer): the library test layer is discovered via explicit paths (`yaoxiang test src/std/tests`) or
  package configuration; CI runs in layers

Migration note: **already migrated (2026-09-06)**—the 19 files in the original
`tests/yaoxiang/07-std/` were triaged one by one and all turned out to be library tests (the object
under test is the API contract of std modules; the role of language features like `?` propagation,
automatic borrowing, and generic instantiation in them is that of carrier rather than the object
under test), the whole directory migrated to `src/std/tests/` and `07-std` was removed; yx_runner
switched to dual-root discovery (`tests/yaoxiang/` + `src/std/tests/`), default patterns don't
include the library layer (integration tests cement this contract). The language corpus has zero
std-API tests from now on.

## Relationship to Existing Systems

| Item                                                 | Relationship                                                                                                             |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Rust `#[test]`                                       | Untouched, compiler-internal tests continue to use Rust                                                                  |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test`                                                                               |
| `std.assert.assert(cond)`                            | Reserved for runtime guards; the `std.test` value-semantic assertion family changes to be based on `std.result` (§3, §7) |
| Module system (RFC-029)                              | Embedded source modules integrate via Registry/orchestrator; CLI `run` integrating with orchestrator is a prerequisite   |
| Corpus refactoring (`io.println` → `assert.assert`)  | Exactly the same direction as `yaoxiang test`                                                                            |
| `@` annotations                                      | Not used, no `@test` introduced                                                                                          |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source-file branch delegates to
  `run_project` (multi-file run prerequisite)
- `src/main.rs` — add `Test` subcommand
- `src/std/test.yx` — add pure YaoXiang module
- `build.rs` — embed `std/*.yx` into the binary
- orchestrator / Registry — support loading `.yx` modules from embedded sources by virtual path
- RFC-015 config parsing — `[tool.test]` section
- Subprocess execution + reporting

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

- `--parallel` parallel execution (worker pool + each file independent subprocess;
  `[tool.test].parallel` config key has the same effect)
- `[tool.test].exclude` config (prefix-match removal, `--list` removes them too)
- `assert_approx_eq` (Float explicit eps assertion, §3)

## Risks and Mitigations

| Risk                                                                 | Probability | Mitigation                                                                                                                                                                                         |
| -------------------------------------------------------------------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` interpolation fails on Any                                  | None        | 2026-08-02 empirically verified (Int/String both work normally)                                                                                                                                    |
| `yaoxiang.toml` config parsing not in current CLI                    | Low         | Simple extension, doesn't affect core functionality                                                                                                                                                |
| CLI run integrating with orchestrator introduces behavior regression | Low         | The no-import single-file path is equivalent; integration tests already cover the orchestrator                                                                                                     |
| Embedding `.yx` source files in binary increases size                | Low         | `.yx` source files are tiny, negligible                                                                                                                                                            |
| Test loop time grows with corpus                                     | High        | The main item is per-file full compilation (11.3s measured for 185 files), not subprocess startup; `--parallel` only mitigates the process side, compilation cost requires test loop cache slicing |

## Open Questions

- [x] Can the `use std.assert` reference in `std/test.yx` be resolved correctly? — **Resolved
      (2026-08-02)**. After the module system (RFC-029) landed, native and source modules coexist in
      the Registry, the resolver handles them uniformly, cross-kind dependencies work naturally
- [x] Does the generic `to_string` of `f"..."` in test output introduce new type constraints? —
      **Resolved (2026-08-02)**. Empirically verified that `==`/`!=` and f-string interpolation both
      work on unannotated parameters (Any) (Int/String verification passed), no new constraints
      introduced
- [x] Feasibility of `?` generic parameters? — **Resolved (2026-08-02)**: the `?` type syntax
      doesn't currently exist (and is silently swallowed, a separate issue tracks it); Phase 1
      assertion functions use unannotated parameters, not depending on the generics system

## Design Decision Record

| Decision                                                | Decision                                                                                                                                                                                                                                                                                                                           | Date                      | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Test marker approach                                    | No `@test` annotation, test files are ordinary `.yx`                                                                                                                                                                                                                                                                               | 2026-07-26                | Zero compiler changes, subprocess is isolation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Assertion approach                                      | `std.test` module pure YaoXiang functions                                                                                                                                                                                                                                                                                          | 2026-07-26                | Self-hosting, no native code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Test execution model                                    | Subprocess `yaoxiang run <file>` + exit code                                                                                                                                                                                                                                                                                       | 2026-07-26                | Process-level isolation, zero compiler changes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Standard library loading                                | Currently embedded in binary, filesystem in the future                                                                                                                                                                                                                                                                             | 2026-07-26                | Version binding, single-file available                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Assertion parameter type                                | Unannotated parameters (Any), doesn't depend on the generics system                                                                                                                                                                                                                                                                | 2026-08-02                | `?` type syntax doesn't exist; Any empirically works for comparison and interpolation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Multi-file execution                                    | CLI `run` delegates to `run_project` (orchestrator) as prerequisite                                                                                                                                                                                                                                                                | 2026-08-02                | Subprocess model inherits CLI capabilities; on-demand discovery via `use` degenerates to a pure performance optimization                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Report source location                                  | Runtime errors carry by default                                                                                                                                                                                                                                                                                                    | 2026-09-07                | debug_map is generated by default, stack trace outputs `file:line:col`; frame attribution via the embedded module (std.test) is not guaranteed under this, belongs to RFC-034                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Negative test layering                                  | Value-level reversal general / compile-failure runner structured markers (internal only) / hard failure folded into Result-ification                                                                                                                                                                                               | 2026-09-02                | Value-based model decided; replaces implicit [test:error] convention                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Multiple in-file tests                                  | Value-based standard model: test functions return Result, suite collects per-test verdicts                                                                                                                                                                                                                                         | 2026-09-02                | No catch, no entry-point calls (entry only for internal scenarios)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Error code                                              | Error adds machine-readable `code` field                                                                                                                                                                                                                                                                                           | 2026-09-02                | Supports error code assertions; compile-time codes go through runner comparison                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Assertion library form                                  | Value-semantic family of 7 functions landed, `Result(Void, String)` contract; abort transition version removed                                                                                                                                                                                                                     | 2026-09-03                | Void is the spec unit (`()` is an empty Tuple, not mixed); nested position is Any-rigid, unannotated parameters can't pass the native generic check—parameters must be annotated explicitly (empirically verified by R1 probe)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Test system layering                                    | Language corpus (`tests/yaoxiang/`) and library tests (with the library, std → `src/std/tests/`) split into two layers; std only appears in the corpus as an assertion tool                                                                                                                                                        | 2026-09-03                | The object under test determines ownership and maintainer; library tests living with the package layout is a preview of RFC-014                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Discovery semantics: project directory ≠ loose test set | When recursive discovery encounters a directory containing `yaoxiang.toml`, only its conventional entry `main.yx` is taken; other `.yx` files in the same directory are project modules, not independent test targets. `.yx` files not belonging to any project directory are treated as test targets (residual port in §5 item 7) | 2026-10-16                | The judgment "this is a test" was originally a pure extension match (`util/test_runner.rs`'s `collect_yx_recursive`), which doesn't hold: putting non-test `.yx` files (fixtures, library modules) under `tests/` is a legal layout. The same rule has already forced two escape routes (library test layer moved out of `tests/`, project fixture layer relies on CI explicit paths to bypass), per D3 the rule should be changed rather than worked around again; industry convention is the same direction (Cargo only treats top-level `.rs` under `tests/` as integration test targets, pytest collects by the `test_*.py` naming convention). Red-then-green: `yaoxiang test` without path arguments went from 321 passed / 10 failed → 321 passed / 0 failed, `tests/yaoxiang` (CI caliber) unchanged at 311/0 |
| Negative marker dispatch verdict                        | Dispatch by expected category: compile-error class `check` must fail, runtime-error class `check` must pass + `run` must fail; report gives category counts                                                                                                                                                                        | 2026-09-03 (landed 09-06) | Mixed-category verdict would let "compile accidentally passes, run coincidentally fails" slip through; the expected code pins the stage, syntax errors don't get their own category                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| Header directive grammar                                | Expectations declared with English structured directives (`// expect:` / `// skip:` / `// mode:`, strict token grammar, parse failure directly FAIL); deprecates `[test:error]` boolean markers and Chinese `预期:` prose approach                                                                                                 | 2026-09-06                | Expectation is a property of the fixture content, in-fixture declaration is co-structured with the industry (compiletest / Go / GCC / Clang all do this), a central list inevitably rots; boolean marker + expectation line as two facts coupled by discipline is a defect surface; structured grammar lets the runner judge mechanically without human involvement                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| Parallel execution model                                | `--parallel` spawns a per-core worker pool (each file still an independent subprocess), skip/invalid handled first in discovery order, execution results stream in completion order, JSON sorted by path; `--fail-fast` stops scheduling (in-flight finish and are counted); CWD remains shared, isolation boundary not extended   | 2026-09-07                | Under the subprocess model, parallelism = OS thread-scheduled spawn, no yx-level concurrency needed; the main time cost is per-file full compilation (risk table), parallelism only mitigates the process side—test loop cache slicing is the main mitigation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |

## References

- [RFC-014: Package Management System Design](014-package-manager.md) — standard library directory
  structure
- [RFC-015: Configuration System](015-configuration-system.md) — `[tool.test]` config section
- [RFC-030: assert Mechanism](030-assert-mechanism.md) — underlying dependency
- [Rust `#[test]` Mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — reference
  design
