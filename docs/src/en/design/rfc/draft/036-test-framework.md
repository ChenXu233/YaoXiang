---
title: "RFC-036: std.test Testing Framework and yaoxiang test Command"
status: "Draft"
author: "Chenxu"
created: "2026-07-25"
updated: "2026-07-25"
issue: "#94"
---

# RFC-036: std.test Testing Framework and yaoxiang test Command

## Summary

Introduce the standard testing framework `std.test` module and the `yaoxiang test` CLI subcommand for YaoXiang. Test discovery is based on existing `@test` annotation-marked functions (**zero new syntax**); assertions use pure functions exported by the `std.test` module (`assert_eq`, `assert_ne`, `assert_ok`, `assert_err`); test execution is driven by compiler annotation scanning + runtime scheduling. No new keywords, reserved words, or syntax constructs are introduced.

## Motivation

### Why do we need a testing framework?

Currently, tests in YaoXiang are scattered across Rust-side `#[test]` and `tests/` integration tests, so test coverage for the YaoXiang language itself depends on writing tests in Rust. This means:

1. Unit tests for the YaoXiang standard library (`std.math` / `std.list` / `std.dict` / `std.convert` / `std.io`) cannot be written in YaoXiang
2. Issue `#117: Unit test coverage for std lib modules` is blocked because no usable testing infrastructure exists
3. Regression tests for language features (such as the spawn semantic changes in RFC-032) lack automated means

### Key Constraint: Zero New Syntax

YaoXiang retains only 17 core keywords (`pub`, `use`, `spawn`, `if`, `else`, `match`, `for`, `in`, `while`, `return`, `break`, `continue`, `as`, `ref`, `true`, `false`, `None`), which is an explicit non-negotiable principle in the design manifesto.

**The testing framework must never introduce new keywords (such as the `test` or `assert` keywords) or new syntax constructs (such as `test` code blocks).**

## Proposal

### Core Design

```
┌─────────────────────────────────────────────────────────────────┐
│                    Test Architecture (Three Layers)             │
│                                                                 │
│  ① Discovery Layer: Compiler scans @test annotations             │
│     → generates test manifest                                    │
│     No new syntax; @ annotation already exists                  │
│                                                                 │
│  ② Execution Layer: yaoxiang test subcommand                     │
│     discover → compile → execute in parallel → report            │
│                                                                 │
│  ③ Assertion Layer: Pure functions in std.test module            │
│     assert_eq / assert_ne / assert_ok / assert_err              │
│     Zero new syntax; pure function calls                         │
└─────────────────────────────────────────────────────────────────┘
```

### Usage Example

```yaoxiang
# Test file: math_test.yx
use std.test
use std.math

# @test annotation marks test functions (@ is an existing Token, not new syntax)
@test
fn test_add() -> Void = {
    test.assert_eq(2 + 3, 5)
    test.assert_eq(-1 + 1, 0)
    test.assert_eq(0 + 0, 0)
}

@test
fn test_subtract() -> Void = {
    test.assert_eq(10 - 3, 7)
    test.assert_ne(10 - 3, 8)
}

@test
fn test_divide_by_zero() -> Void = {
    test.assert_err(1 / 0)
}
```

```bash
# Run all tests
yaoxiang test

# Run a single file
yaoxiang test math_test.yx

# Run tests matching a name pattern
yaoxiang test --filter "add"
```

### Syntax Changes

**None.** This proposal introduces no new syntax constructs.

| Item | Status |
|------|--------|
| New keywords | ❌ None |
| New reserved words | ❌ None |
| New syntax constructs | ❌ None |
| `@test` annotation | ✅ Reuses existing `@` Token |
| `std.test` module | ✅ New module, pure functions |

## Detailed Design

### 1. Test Discovery Mechanism

#### 1.1 The `@test` Annotation

`@test` uses YaoXiang's existing `@` annotation syntax. The annotation mechanism is already supported at the parser layer (use of `@block`/`@eager` annotations in RFC-008 demonstrates that this mechanism is usable).

**Annotation format**:

```yaoxiang
@test
fn test_name() -> Void = { ... }
```

**Rules**:
- `@test` immediately precedes the function definition
- The annotated function must have the signature `() -> Void`
- The annotation only marks; it does not alter function behavior — under a normal `run`, `@test` functions remain ordinary callable functions (but are not auto-invoked)
- Test discovery takes effect only under the `yaoxiang test` subcommand

#### 1.2 Test Discovery Flow

```
yaoxiang test
    │
    ▼
┌─────────────────────┐
│ Scan all .yx files  │  ← By default, scans src/ and tests/ directories
│ Collect @test funcs  │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ Filter:             │  ← --filter parameter matches function name
│ --filter pattern    │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ Compile: each test  │  ← Reuses existing compiler; test marks
│ file; inject test    │     embedded in bytecode
│ registration code   │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ Execute: run tests  │  ← Reuses existing spawn concurrency
│ in parallel;        │     model
│ collect results     │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ Report: pass/fail/  │
│ duration; JUnit XML │
│ (optional)          │
└─────────────────────┘
```

### 2. `std.test` Module Design

#### 2.1 Module Structure

```
src/std/test.rs
    ├── assert_eq(actual, expected)     → Void
    ├── assert_ne(actual, expected)     → Void
    ├── assert_ok(value)                → Void
    ├── assert_err(value)               → Void
    ├── assert_true(cond)               → Void
    ├── assert_false(cond)              → Void
    ├── assert_passes()                 → Void
    └── assert_fails()                  → Void (explicit failure)
```

#### 2.2 Assertion Function Specification

All assertion functions throw `ExecutorError::TestAssertionFailed` on failure, including:
- File path (obtained from the source location of the annotated function)
- Line number
- Failure message (actual vs expected)
- Test function name

```yaoxiang
# std.test.assert_eq implementation pseudocode
fn assert_eq(actual, expected) -> Void = {
    if actual == expected {
        return
    }
    raise TestAssertionFailed(
        "assertion failed: {actual} != {expected}",
        file, line, test_name
    )
}
```

#### 2.3 Module Registration

Registered via the existing `StdModule` trait, completely consistent with modules such as `std.io` and `std.math`:

```rust
impl StdModule for TestModule {
    fn module_path(&self) -> &str { "std.test" }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            NativeExport::new("assert_eq",  "std.test.assert_eq",  "(a: T, b: T) -> Void", native_assert_eq),
            NativeExport::new("assert_ne",  "std.test.assert_ne",  "(a: T, b: T) -> Void", native_assert_ne),
            NativeExport::new("assert_ok",  "std.test.assert_ok",  "(r: Result(T)) -> Void", native_assert_ok),
            NativeExport::new("assert_err", "std.test.assert_err", "(r: Result(T)) -> Void", native_assert_err),
            NativeExport::new("assert_true", "std.test.assert_true", "(b: Bool) -> Void", native_assert_true),
            NativeExport::new("assert_false","std.test.assert_false","(b: Bool) -> Void", native_assert_false),
            NativeExport::new("fail",       "std.test.fail",       "(msg: String) -> Void", native_fail),
        ]
    }
}
```

**Key point**: The `T` in `assert_eq` / `assert_ne` requires generics support. The current compiler already has infrastructure for runtime dispatch of generic functions (RFC-011). If generic dispatch is not yet complete, the first version may be limited to explicit overloads for `Int` / `Float` / `String` / `Bool`, with future upgrades.

### 3. The `yaoxiang test` Subcommand

#### 3.1 CLI Arguments

Added to the `Commands` enum:

```rust
/// Run YaoXiang tests
Test {
    /// Source files or directories to test (default: all .yx files in src/ and tests/)
    #[arg(value_name = "PATH", num_args = 0..)]
    paths: Vec<PathBuf>,

    /// Filter test names (substring match)
    #[arg(short, long, value_name = "PATTERN")]
    filter: Option<String>,

    /// Run tests serially (disable parallel execution)
    #[arg(long)]
    serial: bool,

    /// Number of parallel workers (0 = auto)
    #[arg(long, default_value = "0")]
    workers: usize,

    /// Output JUnit XML report
    #[arg(long, value_name = "FILE")]
    junit: Option<PathBuf>,

    /// Stop on first failure
    #[arg(long)]
    fail_fast: bool,
}
```

#### 3.2 Execution Flow

```rust
match command {
    Commands::Test { paths, filter, serial, workers, junit, fail_fast } => {
        // 1. Discover test files
        let test_files = discover_test_files(&paths);

        // 2. Scan @test annotations, collect test functions
        let test_cases = scan_test_functions(&test_files, &filter);

        // 3. Compile each test file (reuses compiler.compile)
        let compiled = compile_tests(&test_cases);

        // 4. Execute tests in parallel (reuses runtime spawn)
        let results = execute_tests(&compiled, serial, workers);

        // 5. Generate report
        print_test_report(&results, junit);
    }
}
```

#### 3.3 Test Isolation

Each test function runs in an independent runtime context:
- Independent Heap (lists/dicts are not polluted between tests)
- Independent register file
- Independent FFI Registry snapshot (standard library state is not polluted between tests)

Implementation: a new `Frame` + `Heap` is created on each `execute_function`, reusing the existing isolation mechanism (see `src/backends/interpreter/executor/tests/execute.rs`).

### 4. Bytecode-Level Changes

#### 4.1 Test Metadata

When the compiler scans `@test` annotations, it appends a test metadata section to the generated bytecode file header:

```
[Bytecode Header]
    ...
[Test Metadata Section]
    test_count: u16
    tests: [
        { name: str, file: str, line: u32, param_count: u8 }
    ]
```

#### 4.2 Executor Changes

The `Executor` adds a test execution entry point:

```rust
impl Executor<'_> {
    /// Execute a function marked as a test
    fn run_test(&mut self, test: &TestMeta) -> TestResult {
        let start = Instant::now();
        match self.execute_function(&self.func_by_name(&test.name), &[]) {
            Ok(RuntimeValue::Unit) => TestResult { name: test.name.clone(), passed: true, elapsed, error: None },
            Ok(_) => TestResult { name: test.name.clone(), passed: false, elapsed, error: Some("test returned non-Void".to_string()) },
            Err(e) if is_assertion_error(&e) => TestResult { name: test.name.clone(), passed: false, elapsed, error: Some(e.to_string()) },
            Err(e) => TestResult { name: test.name.clone(), passed: false, elapsed, error: Some(format!("unexpected: {}", e)) },
        }
    }
}
```

### 5. Output Format

#### 5.1 Default Output

```
Running 5 tests...

PASS test_add (0.002s)
PASS test_subtract (0.001s)
FAIL test_divide_by_zero (0.003s)
  └── assertion failed: 1 / 0
      Expected: Error
      Actual:   ExecutorError: division by zero
PASS test_max_value (0.001s)
PASS test_min_value (0.001s)

Results: 4 passed, 1 failed, 0 skipped (0.007s)
```

#### 5.2 JUnit XML

```xml
<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="math_test" tests="5" failures="1" time="0.007">
    <testcase name="test_add" classname="math_test" time="0.002"/>
    <testcase name="test_divide_by_zero" classname="math_test" time="0.003">
      <failure message="assertion failed: 1 / 0">ExecutorError: division by zero</failure>
    </testcase>
  </testsuite>
</testsuites>
```

## Type System Impact

**None.** The testing framework introduces no new types, type constraints, or type system changes. All assertion functions return `Void`, and parameters use the existing generics infrastructure.

## Runtime Behavior

- Functions annotated with `@test` behave unchanged under `yaoxiang run` — they remain ordinary functions
- Only the `yaoxiang test` subcommand triggers test discovery and automatic invocation
- `TestAssertionFailed` thrown by test failures shares the same error handling pipeline as ordinary `ExecutorError`

## Compiler Changes

| Module | Change |
|--------|--------|
| `src/frontend/core/parser/` | Reuses existing annotation parsing (`@` Token), no changes needed |
| `src/middle/core/ir_gen.rs` | Scans `@test` annotations, marks AST nodes |
| `src/middle/passes/codegen/` | Writes test metadata section into bytecode header |
| `src/backends/interpreter/executor/` | Adds `run_test` entry point |
| `src/std/test.rs` | Adds `std.test` module implementation |
| `src/main.rs` | Adds `Test` subcommand branch |

## Backward Compatibility

- ✅ **Fully backward compatible**: A new `yaoxiang test` subcommand is added without altering the behavior of any existing subcommand
- ✅ **Existing code is unaffected**: `@test` is optional; unannotated functions are completely unaffected
- ✅ **Std module registration is independent**: `std.test` is registered as a new `StdModule` and does not affect existing modules

## Trade-offs

### Advantages

1. **Zero new syntax**: Fully conforms to YaoXiang's 17-keyword constraint
2. **Leverages existing mechanisms**: `@` annotation, `StdModule` trait, `NativeExport`, and the spawn concurrency model are all reused
3. **Aligned with Rust's testing experience**: `#[test]` → `@test`, `assert_eq!` → `test.assert_eq()`
4. **Extensible**: Future annotations such as `@benchmark` and `@ignore` can be added without syntax changes

### Disadvantages

1. `assert_eq(a, b)` is a function call, not a macro — unlike Rust, it cannot print the exact values such as `a = 42, b = 43` (values must be captured inside the native implementation)
2. Generic assertions (`assert_eq<T>`) depend on runtime generics dispatch; if that capability is not complete, the first version needs type specialization
3. Annotation scanning happens at compile time, so tests cannot be registered dynamically

## Alternatives

| Option | Description | Advantages | Disadvantages |
|--------|-------------|------------|---------------|
| **A: This proposal (`@test` + `std.test` module)** | Annotation + pure functions | Zero new syntax; leverages existing mechanisms | Function calls rather than macros; slightly weaker diagnostics |
| **B: Test block syntax `test { }`** | Introduces the new keyword `test` | Concise syntax | ❌ Violates the 17-keyword constraint; major parser changes required |
| **C: Naming convention (`test_*` prefix)** | Pure naming convention; scans functions with `test_` prefix | Zero annotation changes | Namespace pollution; easy to mis-match; no explicit marker |
| **D: External test runner** | A separate Rust tool compiles and executes .yx tests | No compiler changes | Architecture split; two parallel compilation pipelines |

**Choice of A**: Under the zero-new-syntax constraint, the annotation approach is the most explicit way to mark tests while minimizing namespace pollution.

## Implementation Strategy

### Phase 1: Infrastructure (v0.7.8)

- `std.test` module: `assert_eq` / `assert_ne` / `assert_ok` / `assert_err` / `fail`
- The initial version is limited to specialization for `Int` / `Float` / `String` / `Bool`
- Confirm annotation parsing (ensure `@` is correctly propagated to the AST at the parser layer)

### Phase 2: Test Discovery and Execution (v0.7.8)

- IR layer scans `@test` annotations and marks them
- Bytecode test metadata section
- `yaoxiang test` subcommand skeleton
- Basic test execution + default output

### Phase 3: Polish (v0.7.9)

- `assert_true` / `assert_false`
- `--filter` / `--serial` / `--fail-fast` parameters
- JUnit XML output
- Test isolation (independent Heap contexts)
- Generic assertion support (if runtime generics dispatch is complete)

### Dependencies

| Dependency | Status | Note |
|------------|--------|------|
| `@` annotation parsing | ✅ Existing | Lexer already has `TokenKind::At`; parser must confirm propagation |
| `StdModule` trait | ✅ Existing | Full implementation already in `src/std/mod.rs` |
| `NativeExport` | ✅ Existing | Standard library function registration mechanism |
| Runtime generics dispatch | ⚠️ Partially implemented | Affects the `assert_eq<T>` generic version; Phase 1 can use type specialization as a workaround |
| `yaoxiang` CLI (clap) | ✅ Existing | The `Commands` enum can be extended directly |
| Concurrent execution model | ⚠️ Draft | RFC-024 is accepted but implementation is in progress; Phase 2 can use a single-threaded implementation |

### Risks

1. **Incomplete annotation parsing**: If the parser layer's handling of `@test` is incomplete (only `@block`/`@eager` are recognized), annotation parsing must be extended. **Mitigation**: An annotation is essentially `@` + identifier, so extending it into a general annotation parser is a small amount of work
2. **Immature generics dispatch**: `assert_eq<T>` requires runtime generics support. **Mitigation**: Phase 1 uses four explicit functions for `Int` / `Float` / `String` / `Bool`, not depending on generics dispatch
3. **Insufficient test isolation**: If global state of the `std` module is polluted between tests. **Mitigation**: A new `NativeContext` + `Heap` is created on each test execution

## Open Questions

- [ ] Does the `@test` annotation support parameters (e.g. `@test(reason = "flaky")`)? Not supported for now; keep it simple
- [ ] Test module naming convention? Proposed convention: test files share the same name as their source file with a `_test` suffix, e.g. `math.rs` → `math_test.yx`
- [ ] Is the `@ignore` annotation supported for skipping specific tests? Not in Phase 1; can be considered in Phase 3
- [ ] Assertion diagnostic message format? Need to determine how `actual` / `expected` are displayed (the current `RuntimeValue` already supports `format_value_with_prefix`)

## Appendix A: Relationship with Existing Testing Infrastructure

| Item | Existing | This RFC | Relationship |
|------|----------|----------|--------------|
| Rust `#[test]` | `src/**/tests/` directories | Untouched | Compiler-internal tests continue to use Rust |
| YaoXiang `.yx` integration tests | `tests/yaoxiang/` | Untouched | Existing `.yx` regression test files continue to serve as integration tests |
| `std.assert.assert(cond)` | Existing | Untouched | Retained for use in ordinary code |
| `TestAssertionFailed` error | None | New | A new error type, aligned with the existing error code specification (RFC-013) |

## Appendix B: Design Decision Records

| Decision | Resolution | Date | Reason |
|----------|------------|------|--------|
| Test marking method | `@test` annotation | 2026-07-25 | Zero new syntax; `@` already exists |
| Assertion method | `std.test` module functions | 2026-07-25 | Pure functions; no macros; no syntax changes |
| Test execution model | Independent Heap context | 2026-07-25 | Avoid state pollution between tests |
| Generic assertions | Type specialization in Phase 1, generics in Phase 3 | 2026-07-25 | Avoid the risk of incomplete generics dispatch |

## References

- [RFC-008: Runtime Concurrency Model](../accepted/008-runtime-concurrency-model.md) — Reference for the `@` annotation mechanism
- [RFC-013: Error Code Specification](../accepted/013-error-code-specification.md) — `TestAssertionFailed` error encoding
- [RFC-030: assert Mechanism](../review/030-assert-mechanism.md) — Runtime implementation of the existing `assert(cond)`
- [RFC-011: Generic Type System](../accepted/011-generic-type-system.md) — Type constraints for generic assertions
- [Rust `#[test]` mechanism](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — Reference design
- [Go `testing` package](https://pkg.go.dev/testing) — Reference design