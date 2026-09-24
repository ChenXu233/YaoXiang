---
title: 'RFC-036: std.test Testing Framework and yaoxiang test Command'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-07-26'
updated: '2026-09-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test Testing Framework and yaoxiang test Command

## Summary

Introduce the standard testing framework `std.test` module and the `yaoxiang test` CLI subcommand for YaoXiang. Test files are ordinary `.yx` files, with overall pass/fail determined by subprocess exit code; multiple test functions are supported within a file—assertion failures are expressed as `Err` values (value semantics), with per-test judgments collected by the suite (§7). The `std.test` module is implemented in pure YaoXiang, the first dogfooding library. `yaoxiang test` is a CLI tool, not a compiler feature—no changes to parser, IR, bytecode, or executor.

## Motivation

### Why is a testing framework needed?

Currently, YaoXiang's test coverage relies on Rust's `#[test]` and `tests/` integration tests. This means:

1. Unit tests for the standard library (std.math / std.list / std.dict / std.convert / std.io) cannot be written in YaoXiang
2. Unit test coverage for standard library modules is blocked because no testing infrastructure is available
3. Regression tests for language features (such as RFC-032 spawn semantics changes) lack automation

### Key Constraints

- **17 keyword iron law**: No new keywords or syntactic constructs introduced
- **Zero compiler changes**: Parser, IR, bytecode, and executor untouched
- **Dogfooding first**: Test library written in YaoXiang, the first dogfooding library

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI Layer:   yaoxiang test [--filter --fail-fast --json ...]│
│              │                                               │
│  Discovery:  Read yaoxiang.toml → [tool.test] patterns       │
│              Default: tests/**/*.yx                          │
│              │                                               │
│  Execution:  For each file: yaoxiang run <file>              │
│              Check exit code → Serial execution               │
│              │                                               │
│  Reporting:  PASS/FAIL → Summary                             │
│              Supports --json / --verbose / --fail-fast        │
│                                                              │
│  Assertion:  std.test (pure YaoXiang, dogfooding)            │
│              Base: std.assert.assert                         │
│              Diagnostics: f"Expected {expected}, got {actual}"│
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **Test framework is not a compiler feature, it's a CLI tool** — `yaoxiang run` can already "execute tests", `yaoxiang test` just helps you run all files and shows you the report
2. **Zero compiler changes** — No `@test` annotation scanning, bytecode metadata sections, or executor special entry points introduced
3. **Dogfooding** — `std.test` module implemented in pure YaoXiang, base capabilities from `std.assert` / `std.result`
4. **Test files are ordinary `.yx` files** — Files run as subprocesses, exit code determines overall pass/fail
5. **Assertion failures are values, not process events** — Test functions return `Result`, assertion failures expressed as `Err`, suites collect per-test judgments one by one (§7); process-level abort reserved for runtime guards only, not used for test assertions

## Detailed Design

### 1. CLI Design

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      Specify test files or directories (default: read from yaoxiang.toml, otherwise tests/)

Options:
  --filter <NAME>     Run only tests whose filename contains <NAME>
  --fail-fast         Stop on first failure
  --verbose, -v       Show detailed stdout/stderr for each test
  --list              List test files only, don't run
  --no-progress       Don't show progress output (header and PASS lines); FAIL details and summary preserved (CI scenarios)
  --json              Output JSON format results (for CI integration)
  --parallel          Parallel execution (one worker per core; OR'd with [tool.test].parallel)
```

#### Output Format

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
  "summary": { "total": 3, "passed": 2, "failed": 1, "skipped": 0, "by_kind": { "behavior": 3, "compile-error": 0, "runtime-error": 0, "invalid": 0 }, "time_secs": 0.006 },
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

- Failed files additionally carry `exit_code` and `stderr` (ANSI-stripped subprocess diagnostics for CI forensics); when `--verbose` and `--json` are combined, all files carry `stdout`/`stderr`
- `--no-progress` suppresses only progress output (header and PASS lines)—FAIL details and summary always output, failures cannot be silent; `--list` outputs one test file path per line, doesn't execute
- Each file carries `kind` (behavior / compile-error / runtime-error / invalid, §8.2), summary carries `by_kind` execution counts (fixed four keys, not including skipped); human summary includes `Categories:` category distribution line
- Under `--parallel`, human progress lines stream in **completion order** (whole blocks don't interleave), JSON `files` sorted by `file` path for stable output (CI diff friendly)
- In-file per-test `tests` array comes from §7 suite collection, effective when value-based model lands
- Official CI (`.github/workflows/ci.yml` test job) consumes exactly this: cargo side runs `--test integration` (CLI integration) and `--test yx_runner` (dual-root corpus guards), then `yaoxiang test --json --parallel` runs hierarchically—default mode (language corpus) and explicit `src/std/tests` (library layer) each produce a report; summary table (total / passed / failed / skipped / time_secs and by_kind) written to job summary, failed files print `kind`/`exit_code`/`stderr` for forensics, any suite with non-zero exit marks as red

### 2. yaoxiang.toml Configuration

Placed under `[tool.test]`, compliant with RFC-015's `[tool.*]` third-party extension convention:

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # Matched paths removed from discovery set (also removed from --list)
parallel = true                   # Parallel execution (OR'd with --parallel flag)
```

- Default `patterns = ["tests/**/*.yx"]` — zero-config out-of-box
- `exclude` and `patterns` share the same pattern syntax (literal paths or `root/**…`, all matched by path prefix); excluded means not a test, fixtures needing runtime behavior verification go through `yaoxiang run` directly
- **Single file mode (`yaoxiang test foo.yx`) runs directly, doesn't read config**—with explicit paths, `exclude`/`parallel` config keys have no effect (flags excepted)
- Future possible split into independent repo (`[tool.test]` location unchanged)

### 3. std.test Module (Pure YaoXiang)

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library (value semantics canonical form, shipped 2026-09-03)
// First dogfooding library: YaoXiang's test library written in YaoXiang.

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

// assert_not and assert_false are the same body; assert_err / assert_err_code see §8.1
```

- Assertion functions are **value semantics**: return `Result(Void, String)`, failures expressed as `Err(diagnostic message)`, no process abort—§7 suite collects per-test judgments based on this. The process-level abort semantics of `std.assert.assert` are reserved for runtime guards and don't enter test assertion paths. Ok payload is `Void` (unit type as per type-system.md spec; `()` is empty Tuple, they don't mix—2026-09-03 decision)
- **Function family of 7 (shipped 2026-09-03, abort transition version deleted)**: value-based
  `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not`
  (same body as assert_false, reserved for `!assert` costume) + `assert_err` (§8.1)
  - `assert_err_code` (§8.1 error code assertion)
  - `assert_approx_eq(a: Float, b: Float, eps: Float)` (Phase 3, shipped 2026-09-07): `|a - b| <= eps` judgment, eps **explicitly provided** by caller—tolerance is part of test contract, no hidden default; negative eps Err at declaration site, NaN always Err
- `assert_eq` / `assert_ne` use **Any type annotations on parameters**—`==`/`!=` and f-string interpolation work fine on Any, don't depend on generics system. Note parameters **must be explicitly annotated**: unannotated parameters fail native generic `&Result(T, E)` call checking (R1 probe confirmed)
- `assert_false` / `assert_not` express negation with `cond == false` (`not` unary syntax not yet landed, will migrate when stable; `!assert` unary form has same dependency, see §8.1)
- Block body + explicit `return` form: if expression's then-branch type is discarded during checking, if expressions with both branches Result are a blind spot in checking, implementation avoids this
- `std.test` has no native code dependencies, pure YaoXiang implementation

### 4. Standard Library Loading Mechanism (Key Design)

**Phase 1: Embedded in Binary**

`std/test.yx` (and all future standard library modules written in YaoXiang) are embedded in the binary at build time:

```rust
// build.rs or build script, auto-generated
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // Source code text
    // Future more
];
```

Module system (RFC-029, fully shipped 2026-08-02) provides entry point: Registry holds both native modules and source modules, orchestrator handles multi-file orchestration. Resolution order for `use std.test`:

1. First check Rust native modules (existing mechanism, such as `std.assert`)
2. If not found, check embedded `STD_YX_FILES`—if found, inject into orchestrator with **virtual path** (e.g., `<std>/test.yx`) as seed module, go through normal frontend pipeline (parse → typecheck → IR)
3. If not found, fall through to filesystem discovery (user modules)

Internal `use std.assert` within embedded source modules resolves normally to native registry via resolver—native and source modules coexist in Registry, cross-kind dependencies naturally work. Embedded modules **compiled on-demand**: enter pipeline only when imported.

Advantages:

- `use std.test` works in single-file mode too
- Standard library version tightly bound to binary, no version mismatch possible
- No user configuration of standard library path needed

**Future: Filesystem Standard Library**

When YaoXiang project mode matures, standard library will switch to filesystem form. See updated RFC-014 for details.

### 5. Discovery and Execution

**Prerequisite (2026-08-02 review decision)**: CLI `run` connects to orchestrator. Current CLI `run` uses single-file pipeline (`run_file_with_diagnostics`), cannot resolve user module imports; `yaoxiang test`'s subprocess model inherits CLI capabilities, test file importing project modules is a core use case. Therefore Phase 1 first delegates CLI `Run`'s source branch to `run_project` (orchestrator, directory recursive discovery); along-use on-demand discovery layered later as pure performance optimization. Import-free single files behave identically through orchestrator, bytecode branch unchanged.

**Discovery phase**:

1. If `[PATHS]` specified, use specified paths directly
2. Otherwise read `[tool.test].patterns` from `yaoxiang.toml`
3. If not configured, default to `tests/**/*.yx`
4. Apply `--filter` filtering (filename contains)
5. Discovery scope determines test layering (§9): default patterns only cover language availability corpus; library test layer (such as `src/std/tests/`) discovered via explicit path or package config, not mixed into default scan

**Execution phase**:

1. For each file, route by header directive (directive grammar in §8.2, parsed via `src/util/test_markers.rs`, shared with yx_runner):
   - Behavior tests: `yaoxiang run <file>` subprocess
     (runtime errors carry source location and stack frames by default—debug_map generated by default,
     stack trace outputs `file:line:col`); `// mode:` declaration subprocess `--runtime` mode
   - Compile-time rejection class: single-step `yaoxiang check <file>`
   - Runtime failure class: two-step `check` (must pass) + `run` (must fail)
2. Files with `// skip: <reason>` skip execution, counted in report's skipped
3. Judgment compared to expected code per §8.2 decision matrix; directive parse failure doesn't execute, directly FAIL (construction-period rejection)
4. Capture stdout/stderr for reporting
5. Serial by default; `--parallel` (or `[tool.test].parallel`) starts worker pool by available cores,
   each file still an independent subprocess—skip/invalid processed first in discovery order, execution results stream in completion order, JSON sorted by path (Phase 3, shipped 2026-09-07)
6. If `--fail-fast`, stop scheduling new files on first FAIL; in parallel mode, in-flight files complete and are counted

### 6. Test Isolation

Test isolation is naturally achieved through process-level boundaries:

- Each test file runs in an independent subprocess
- Each subprocess has independent Heap, Frame, NativeContext
- A panic in one test file doesn't affect other test files
- No additional separate Heap context mechanism needed
- **Parallel execution (Phase 3) doesn't expand isolation boundary**: working directories (CWD) shared across subprocesses; parallel tests must not occupy same paths in CWD—file I/O tests use independent filenames and clean up on exit

### 7. Suites and Multiple Tests (Value-based Model)

A test file may contain multiple tests. In-file organization (shipped 2026-09-03):

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

- Each test is a zero-argument function returning `Result(Void, String)`; assertion failures expressed as `Err` (§3 value semantics assertion family), don't interrupt process—subsequent tests run normally
- `test.suite` calls and collects one by one: if any test is not Ok, record name and diagnostic; Ok silently ignored; after all run, if any Err exists, abort with `std.assert.assert` carrying failure details (`N of M test(s) failed` + each `[FAIL] name: diagnostic`)—file exit code non-0 (§5 decision unchanged). This abort is the runtime guard for the test binary, not the assertion path; all Ok exits silently with 0
- Top-level test functions enter the suite as **closures** (`("name", () => test_fn())`): top-level function names as value references not yet supported (IR layer limitation, `E3006`)—closure bodies calling global functions unaffected
- Runner sees only files, no function-level scanning: per-test judgments come entirely from in-suite collection, file internal structure transparent to runner—zero compiler changes principle unaffected
- Explicitly not adopted: in-process catch boundaries (17 keyword iron law); runner calling each function entry point (only for §8.2 compile failure etc. internal scenarios)
- API shape finalized (2026-09-03): `suite(tests: List((String, () -> Result(Void, String)))) -> Void`; duplicate names not detected (names only for report display); `--filter` filters by filename, not aware of in-suite test names

### 8. Negative Testing (Expected Failure) Three-Layer Design

Negative testing splits by failure layer, each layer in its proper place:

#### 8.1 Value-Level Reverse (General, User-Facing)

Operations under test return `Result`, tests express expected failure with ordinary assertions:

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// Or one-liner wrapper (code only exists on std Error carrier, E locked as Error):
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` shipped with value semantics family (2026-09-03, §3); `!assert` unary form to be provided after `not` syntax lands (same constraint as `assert_false`'s `cond == false`)
- Error code assertion depends on `Error` values carrying machine-readable `code` field—shipped: `Error = { code, message }` (native `error_new(code, message)`), reading via `result.unwrap_err(r)` to get carrier + `result.code(e)` / `result.message(e)` accessors (the `error_new_with_code` naming, code constant export, and `err.code` field access estimated during design phase were all not adopted—language has no Struct field access, code constants not exported)
- Following Resultification rollout, operations that can fail return `Result` one by one, file-level negative markers in corpus migrate to in-file assertions

#### 8.2 File-Level Negative Directives (Language Designers Internal Use Only)

Compilation is all-or-nothing per file, cannot express "this line shouldn't compile" within a file; runtime failures similarly need file-level expression (such as suite containing must-fail tests). File header declares expectation via **structured directive**, runner **routes judgment** by expected category (routing finalized 2026-09-03; directive grammar finalized and shipped 2026-09-06).

Header directive grammar (`// key: value`, within first 16 lines, strict token matching):

```text
// expect: compile-error E1002 [E1003 ...]   Compile-time rejection test
// expect: runtime-error E6003 [...]         Runtime failure test
// skip: <reason>                            Skip execution, counted as skipped
// mode: embedded|standard|full              Subprocess --runtime mode (consumed only by run step)
```

- No `expect:` directive = behavior test. `expect:` is the **sole declaration** of expectation—2026-09-06 decision abandons `[test:error]` boolean flag and Chinese `预期:` prose extraction: boolean flag and expectation line are two loosely coupled facts, drift inevitable with discipline; English strict token grammar lets runner parse mechanically (kind + code all fixed tokens, anything extra after code causes parse failure)—parse failure = don't execute, directly FAIL—silent degradation path for incorrect directive declarations doesn't exist (construction-period rejection)
- **Compile error class**: single-step `check`—must fail and output contains all `[EXXXX]`; passing compilation = FAIL (missed what should have been reported), rejected but wrong code = FAIL. Syntax errors (E1xxx parse phase) and semantic errors (E2xxx+) don't have separate categories—expected code itself locks the phase
- **Runtime error class**: two-step judgment—`check` must **succeed** (compilation innocent), `run` must fail and output contains all `[EXXXX]`. Exploding at compile time = FAIL (key judgment in opposite direction from compile error class, prevents "compilation accidentally passed, runtime lucky failed" misses)
- Isomorphic with industry: Rust compiletest `//~ ERROR`, Go `// ERROR "regexp"`, GCC `dg-error`, Clang `expected-error` all declare expectations in fixture comments with harness two-way comparison; they use line-level anchors because multi-diagnostic compilers need to distinguish multiple expectations in same file, but this compiler stops at first error, one diagnosis per file, file-level is isomorphic with compiler reality—after error recovery lands, line anchor form can be added to grammar (Cranelift filetests file-header-directive + function-level expectation is the same hybrid form)
- Known rendering debt: parse-period diagnostics currently output in Debug form (`code: "E0012"` instead of `[E0012]`), code scanning accepts both forms; after diagnostic rendering unified, strict form will be enforced
- **Serves only this repo's corpus, not part of user testing framework**; dual runner judgment contract closed (2026-09-03): yx_runner (cargo test) and `yaoxiang test` share `src/util/test_markers.rs` for header directive parsing, 06-compile-errors directory convention abandoned. Reporting layer provides category counts: human summary includes `Categories:` line, JSON summary includes `by_kind` (behavior / compile-error / runtime-error / invalid, skipped separate), each file includes `kind`

#### 8.3 Runtime Hard Failure (Absorbed into Resultification)

No separate mechanism—operations that will fail return `Result` per language direction, tests uniformly express via §8.1. Process-level abort (such as assertion violations, runtime parameter mismatches) gradually converges to values as Resultification progresses, test framework provides no special semantics for it. (Note: "runtime error class" marking in §8.2 is the runner's file-level verification channel for **operations not yet Resultified**, not contradictory to this section's direction—latter is destination, former is migration-period channel)

### 9. Test System Layering: Language Corpus and Library Tests (Finalized 2026-09-03)

Tests split into two layers by **test subject**, each with its own ownership and maintainer; marking system (§8.2) and assertion library (§3) shared across layers:

**Layer 1: Language Availability Corpus (`tests/yaoxiang/`)**

- Test subject is **the language itself**—parser, type system, modules, concurrency, ownership, compile-time rejection, runtime semantics; directory organized by language spec sections
- std appears in corpus only as **assertion tools** (`std.assert` / `std.test`), never tested—library API behavior is not language availability
- Within corpus, routed by §8 into behavior tests / compile-time rejection tests / runtime failure tests three judgment categories

**Layer 2: Library Tests (With the Library)**

- Test subject is **library's public API contract** (such as `list.push` behavior, `result.code` semantics)
- Tests written **in the library's own package**: std's package is `src/std/`, its yx-level tests go to `src/std/tests/` (co-located with implementation; directory coexists with Rust unit tests, file types don't intersect); `std.test`'s own tests also there (using std.test to test std.test, dogfooding closed loop)
- Future user packages follow same convention: tests within package, discovered with package's `[tool.test]` (test layout rehearsed by RFC-014 package management)
- Not discovered by default patterns (default `tests/**/*.yx` only covers language layer): library test layer discovered via explicit path (`yaoxiang test src/std/tests`) or package config; CI runs in layers

Migration note: **Already migrated (2026-09-06)**—19 files in original `tests/yaoxiang/07-std/` individually assessed, all identified as library tests (test subjects all std module API contracts; `?` propagation, auto-borrowing, generic instantiation etc. language features' role within them is vehicle, not subject), migrated entirely to `src/std/tests/` and 07-std directory removed; yx_runner changed to dual-root discovery (`tests/yaoxiang/` + `src/std/tests/`), default patterns don't include library layer (integration tests cement this contract). Language corpus now has zero std-API tests.

## Relationship with Existing Systems

| Item                                          | Relationship                              |
| --------------------------------------------- | ----------------------------------------- |
| Rust `#[test]`                                | Unchanged, compiler internal tests continue using Rust   |
| Existing `.yx` integration tests (`tests/yaoxiang/`) | Discovered and executed by `yaoxiang test` |
| `std.assert.assert(cond)`                     | Kept for runtime guards; `std.test` value semantics assertion family now based on `std.result` (§3, §7) |
| Module system (RFC-029)                        | Embedded source modules connect via Registry/orchestrator; CLI `run` connecting to orchestrator is prerequisite |
| Corpus refactoring (`io.println` → `assert.assert`) | Fully aligned direction with `yaoxiang test` |
| `@` annotations                               | Not used, no `@test` introduced          |

## Implementation Strategy

### Phase 1: Core Functionality

Scope of changes:

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` source branch delegates to `run_project` (multi-file execution prerequisite)
- `src/main.rs` — New `Test` subcommand
- `src/std/test.yx` — New pure YaoXiang module
- `build.rs` — Embed `std/*.yx` into binary
- orchestrator / Registry — Support loading `.yx` modules from embedded sources with virtual paths
- RFC-015 config parsing — `[tool.test]` section
- Subprocess execution + reporting

Deliverables:

- `yaoxiang test` basic functionality
- `std.test` 4 assertion functions
- Default `tests/**/*.yx` discovery
- Serial execution + default output format

### Phase 2: Polish

- `--filter` / `--fail-fast` / `--verbose` arguments
- `--json` output (CI integration)
- `--list` option
- `--no-progress` option

### Phase 3: Advanced (Shipped 2026-09-07)

- `--parallel` parallel execution (worker pool + each file independent subprocess; `[tool.test].parallel` config key has same effect)
- `[tool.test].exclude` config (prefix-match removal, also removed from `--list`)
- `assert_approx_eq` (Float explicit eps assertion, §3)

## Risks and Mitigations

| Risk                                    | Probability | Mitigation                                        |
| --------------------------------------- | ------------ | ------------------------------------------------- |
| `f"..."` interpolation fails on Any     | None         | Confirmed 2026-08-02 (Int/String both work)      |
| `yaoxiang.toml` config parsing not in current CLI | Low   | Simple extension, doesn't affect core functionality |
| CLI run connecting to orchestrator introduces behavioral regression | Low | Import-free single file paths equivalent; integration tests already cover orchestrator |
| Embedding `.yx` source files into binary increases binary size | Low   | `.yx` source files extremely small, negligible    |
| Test loop time grows with corpus       | High         | Main item is per-file full compilation (185 files measured 11.3s), not subprocess startup; `--parallel` only mitigates process side, compilation cost needs test loop cache slicing |

## Open Questions

- [x] Can `use std.assert` reference in `std/test.yx` resolve correctly? — **Resolved (2026-08-02)**. After module system (RFC-029) landed, native and source modules coexist in Registry, resolver resolves uniformly, cross-kind dependencies naturally work
- [x] Does `f"..."` in test output introduce new type constraints via generics `to_string`? — **Resolved (2026-08-02)**. Confirmed `==`/`!=` and f-string interpolation both work on unannotated parameters (Any) (Int/String verified), no new constraints introduced
- [x] Feasibility of `?` generic parameter? — **Resolved (2026-08-02)**: `?` type syntax doesn't currently exist (and would be silently swallowed, tracked in separate issue), Phase 1 assertion functions use unannotated parameters, don't depend on generics system

## Design Decision Record

| Decision          | Decision                                      | Date       | Reason                       |
| ----------------- | --------------------------------------------- | ---------- | ---------------------------- |
| Test marking method | No `@test` annotation, test files are ordinary `.yx` | 2026-07-26 | Zero compiler changes, subprocess is isolation |
| Assertion method   | `std.test` module pure YaoXiang functions      | 2026-07-26 | Dogfooding, no native code   |
| Test execution model | Subprocess `yaoxiang run <file>` + exit code  | 2026-07-26 | Process-level isolation, zero compiler changes |
| Standard library loading | Currently embedded in binary, filesystem future | 2026-07-26 | Version binding, single file usable |
| Assertion parameter types | Unannotated parameters (Any), no dependency on generics system | 2026-08-02 | `?` type syntax doesn't exist; Any confirmed comparable and interpolatable |
| Multi-file execution | CLI `run` delegates to `run_project` (orchestrator) as prerequisite | 2026-08-02 | Subprocess model inherits CLI capabilities; along-use on-demand discovery degrades to pure performance optimization |
| Report source locations | Runtime errors carry by default | 2026-09-07 | debug_map generated by default, stack trace outputs `file:line:col`; frames from transit through embedded modules (std.test) not guaranteed, RFC-034 |
| Negative testing layering | Value-level reverse general / compile failure runner structured marking (internal only) / hard failure absorbed into Resultification | 2026-09-02 | Value-based model finalized; replaces implicit [test:error] convention |
| Multiple tests in file | Value-based canonical model: test functions return Result, suite collects per-test judgments | 2026-09-02 | No catch, not entry-point calling (entry-point only for internal scenarios) |
| Error codes | Error adds machine-readable `code` field | 2026-09-02 | Powers error code assertions; compile-time codes compared by runner |
| Assertion library shape | Value semantics family of 7 functions shipped, `Result(Void, String)` contract; abort transition version deleted | 2026-09-03 | Void is canonical unit (`()` is empty Tuple, don't mix); nesting position Any is strict, unannotated parameters fail native generic checking—parameters must be explicitly annotated (R1 probe confirmed) |
| Test system layering | Language corpus (`tests/yaoxiang/`) and library tests (with the library, std → `src/std/tests/`) split; std in corpus only as assertion tools | 2026-09-03 | Test subject determines ownership and maintainer; library tests with package layout rehearses RFC-014 |
| Negative marking routing judgment | Route by expected category: compile error class `check` must fail, runtime error class `check` must pass + `run` must fail; report includes category counts | 2026-09-03 (shipped 2026-09-06) | Mixed category judgment would let "compilation accidentally passed, runtime lucky failed" slip through; expected code locks phase, syntax errors don't have separate category |
| Header directive grammar | Expectation declared via English structured directives (`// expect:` / `// skip:` / `// mode:`, strict token grammar, parse failure directly FAIL); abandons `[test:error]` boolean flag and Chinese `预期:` prose extraction | 2026-09-06 | Expectation is attribute of fixture content, in-fixture declaration isomorphic with industry (compiletest / Go / GCC / Clang all do this), central list inevitably rots; boolean flag + expectation line two facts coupled by discipline is defect surface; structured grammar lets runner judge mechanically with no human intervention |
| Parallel execution model | `--parallel` starts per-core worker pool (each file still independent subprocess), skip/invalid processed first in discovery order, execution results stream in completion order, JSON sorted by path; `--fail-fast` stops scheduling (in-flight completes and counted); CWD still shared, isolation boundary not expanded | 2026-09-07 | Under subprocess model, parallel = OS thread scheduling spawns, no yx-layer concurrency needed; main time cost is per-file full compilation (risk table), parallel only mitigates process side—test loop cache slicing is main mitigation |

## References

- [RFC-014: Package Manager Design](../accepted/014-package-manager.md) — Standard library directory structure
- [RFC-015: Configuration System](../accepted/015-configuration-system.md) — `[tool.test]` config section
- [RFC-030: assert Assertion Mechanism](./030-assert-mechanism.md) — Base dependency
- [Rust `#[test]` Mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — Reference design