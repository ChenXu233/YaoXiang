---
title: 'Test Writing Standards'
description:
  YaoXiang project test writing hard rules, defining standards for unit tests, integration tests, benchmark tests, documentation tests, and property-based tests
---

# Test Writing Standards

This document defines the hard rules for test writing in the YaoXiang project. All contributors must adhere to the following rules; violations will require changes during Code Review.

---

## Table of Contents

- [General Principles](#general-principles)
- [Yx Corpus and Library Test Hierarchy](#yx-corpus-and-library-test-hierarchy)
- [Unit Test Standards](#unit-test-standards)
- [Integration Test Standards](#integration-test-standards)
- [Benchmark Test Standards](#benchmark-test-standards)
- [Documentation Test Standards](#documentation-test-standards)
- [Property Test Standards](#property-test-standards)
- [Coverage Requirements](#coverage-requirements)
- [Appendix](#appendix)

---

## General Principles

### Scope of Application

These standards apply to all Rust test code in the YaoXiang project, including:

| Test Type       | Location               | Framework                        |
| --------------- | ---------------------- | -------------------------------- |
| Unit Tests      | `src/<module>/tests/`  | `#[test]` + `#[cfg(test)]`       |
| Integration Tests | `tests/`            | `#[test]`                        |
| Benchmark Tests | `benches/`             | Criterion.rs                     |
| Doc Tests       | API documentation      | `cargo test --doc`               |
| Property Tests  | Any test location      | proptest / quickcheck            |

### Core Principles

**Principle 0: The specification is the authoritative source for tests, not code.**
This is the most important principle in this document. Tests verify that code conforms to the specification, not that code "passes with the current implementation." When a test reveals that code behavior differs from the specification, **fix the code, never fix the test.**

Specification files are located at:

- `docs/src/design/language-spec.md` —— Core language specification
- `docs/src/design/rfc/accepted/` —— Accepted RFC design documents

Each test file must declare the corresponding specification section at the top (see Rule 2.1). Any developer should be able to take the specification document, compare it against the tests, and verify the correctness of the implementation. Conversely—if a piece of code has no corresponding specification description, it should not exist, and should not be tested.

```rust
// 🟢 Good — test directly references the spec, verifying code conforms to spec
//! Literal Tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integer Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String Interpolation

#[test]
fn test_decimal_literal_parsing() {
    // Spec §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 Garbage — test accommodates current code's implementation, not verifying spec
#[test]
fn test_literal_1() {
    // No idea which section of the spec this corresponds to
    // If parse_literal returns the wrong value, this test will "pass"
    // because it only verifies the function doesn't panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and find that the code's behavior doesn't match the spec. You have two choices:

| Wrong Approach                      | Correct Approach                   |
| ----------------------------------- | ---------------------------------- |
| Modify the test to "pass"           | Modify the code to match the spec  |
| Add `#[ignore]` to the test         | Fix the code implementation now    |
| Add special condition branches in the test to accommodate code | Remove branches, let the test expose the problem directly |

Remember: **Red light = code is wrong, not the test.** (Unless your test itself has a bug, which is a different matter.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of the code under test by reading the tests, without additional comments or external documentation.

```rust
// 🟢 Good — test name says what's being tested and what's expected
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 Garbage — no one knows what this tests
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for random failures.**
Tests must be repeatable in any environment. Tests that depend on random numbers, system time, or thread scheduling order must use fixed seeds or use mocks instead.

**Principle 3: One test, one thing.** If a test name needs "and" to connect multiple behaviors, split it into multiple tests.

```rust
// 🟢 Good — each test verifies only one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Garbage — one test crammed with too much unrelated content
#[test]
fn test_parser() {
    // Tests tokenize, parse, typecheck, codegen...
}
```

**Principle 4: Test behavior, not implementation.**
Refactoring internal implementation should not cause tests to fail. If changing one line of implementation code causes 10 tests to fail, your tests are wrong.

But here's a key distinction: **the definition of "behavior" comes from the specification, not from how the current code behaves.**
If the code changes behavior (i.e., new behavior that contradicts the specification), the tests must fail. If you can't achieve this, your tests are "code-accommodating tests"—they let bugs walk right in.

```
Specification (language-spec.md / RFC)  ──defines──►  Expected Behavior  ──drives──►  Tests
                                                │
Current Code  ──implements──►  Actual Behavior  ──compared──►  Test Results

If Actual Behavior ≠ Expected Behavior:
  Test must fail (red light)  ──►  Fix code  ──►  Test passes (green light)

If Actual Behavior = Expected Behavior (but implementation is poor):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is what Principle 4 means
```

**Principle 5: Don't write fallback/compatibility/test-mode-specific code.** The test environment is one you have full control over. If you need `#[cfg(not(ci))]` to skip a test, the test design has a fundamental problem.

### Terminology Definitions

| Term              | Definition                                                  |
| ----------------- | ----------------------------------------------------------- |
| Unit Test         | Tests behavior of a single function or module, no external dependencies |
| Integration Test  | Tests multiple modules working together through public API or CLI entry |
| Benchmark Test    | Measures code performance, detects performance regressions |
| Doc Test          | Executable code examples embedded in documentation comments |
| Property Test     | Tests that verify invariants based on random inputs         |

### Relationship with Commit Conventions

All test-related commits must use the `:white_check_mark: test:` type, following the [Commit Conventions](./commit-convention.md).

```
:white_check_mark: test(parser): Add Pratt parser infix expression tests
:white_check_mark: test(codegen): Complete switch statement IR generation tests
```

---

## Yx Corpus and Library Test Hierarchy

These standards govern **Rust-side test code**. Tests for the YaoXiang language itself (`.yx` corpus and library tests) are organized in two layers based on what is being tested. Architecture design and decision contracts are governed by RFC-036 (§7 Suite Collection / §8 Three-Tier Negatives / §9 Test Hierarchy), and corpus writing guidelines are in `tests/yaoxiang/TEST_STANDARDS.md`:

- **Language Availability Corpus** (`tests/yaoxiang/`) — the object under test is the language itself; std is only used as assertion tools. Three judgment categories based on failure layer: behavior tests / compile-time rejection tests / runtime failure tests
- **Library Tests** (follow the library) — the object under test is the library's public API contract; std's yx-level tests are located at `src/std/tests/`, and future user package tests are in the package with `[tool.test]` discovery

The `.yx` test file header format, header directives (`// expect:` / `// skip:` / `// mode:`, RFC-036 §8.2) and assertion conventions are as specified in TEST_STANDARDS.md; judgment parsing is implemented by the shared dual-runner `src/util/test_markers.rs` (Rust side, governed by these standards).

---

## Unit Test Standards

### File Organization

**Rule 1.1**: The `tests/` directory for unit tests must be **at the same level** as the `mod.rs` of the module under test. `tests/` does not aggregate upward or cross-level.

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——declares sibling tests/
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——pratt's own tests
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # parser module-level tests (does not include pratt submodules)
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

Key decision criteria: **Whichever directory the `tests/` is in, that directory's `mod.rs` must declare it with `#[cfg(test)] mod tests;`.**

**Rule 1.1 Supplement: Aggregation upward is prohibited.** Subdirectory module tests must be in that subdirectory's own `tests/`, not aggregated to parent-level `tests/`.

| Module Type                    | Test Location            | Example                                        |
| ------------------------------ | ----------------------- | ---------------------------------------------- |
| Directory module (has `mod.rs`) | `tests/` under that directory | `emitter/tests/`, `codes/tests/`         |
| Single-file module (only `.rs`) | Parent's `tests/`      | `session.rs` → `diagnostic/tests/session.rs` |

```text
# ✅ Correct: each directory module's tests are independent
src/util/diagnostic/
├── codes/
│   ├── mod.rs              # #[cfg(test)] mod tests;
│   └── tests/              # ✅ codes' own tests
│       ├── mod.rs
│       └── codes.rs
├── emitter/
│   ├── mod.rs              # #[cfg(test)] mod tests;
│   └── tests/              # ✅ emitter's own tests
│       ├── mod.rs
│       ├── text.rs
│       └── ansi.rs
└── tests/                  # ✅ diagnostic level (single-file module)
    ├── mod.rs
    ├── session.rs
    ├── suggest.rs
    └── collect.rs

# ❌ Wrong: aggregate emitter and codes tests to diagnostic/tests/
src/util/diagnostic/
└── tests/
    ├── mod.rs              # ❌ forced to declare mod emitter; mod codes;
    ├── emitter/            # ❌ should be in emitter/tests/
    └── codes/              # ❌ should be in codes/tests/
```

#### Single-File Module vs Directory Module Test Placement Rules

**Core distinction**: The module's organization form determines test placement.

| Module Type         | Determining Factor              | Test Location              | Example                                              |
| ------------------ | ------------------------------- | -------------------------- | ---------------------------------------------------- |
| **Directory module** | Has independent directory and `mod.rs` | `tests/` under that directory | `inference/tests/`                             |
| **Single-file module** | Only `.rs` file, no independent directory | Parent's `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

**Detailed explanation**:

```
src/frontend/core/typecheck/
├── mod.rs                          # typecheck module's mod.rs
├── checker.rs                      # single-file module
├── environment.rs                  # single-file module
├── overload.rs                     # single-file module
├── type_eval.rs                    # single-file module
├── dead_code.rs                    # single-file module
├── spawn_placement.rs              # single-file module
├── signature.rs                    # single-file module
├── types.rs                        # single-file module
│
├── tests/                          # ✅ typecheck's test directory
│   ├── mod.rs                      # declares single-file module tests
│   ├── checker.rs                  # tests for checker.rs
│   ├── environment.rs              # tests for environment.rs
│   ├── overload.rs                 # tests for overload.rs (single-file module tests go here)
│   ├── type_eval.rs                # tests for type_eval.rs
│   ├── dead_code.rs                # tests for dead_code.rs
│   ├── spawn_placement.rs          # tests for spawn_placement.rs
│   ├── signature.rs                # tests for signature.rs
│   └── types.rs                    # tests for types.rs
│
├── inference/                      # directory module (has mod.rs)
│   ├── mod.rs                      # #[cfg(test)] mod tests; ——declares sibling tests/
│   ├── expressions.rs
│   ├── statements.rs
│   ├── patterns.rs
│   ├── bounds.rs
│   ├── subtyping.rs
│   ├── generics.rs
│   ├── compatibility.rs
│   ├── scope.rs
│   ├── assignment.rs
│   └── tests/                      # ✅ inference's test directory
│       ├── mod.rs
│       ├── expressions.rs          # tests for expressions.rs
│       ├── statements.rs           # tests for statements.rs
│       └── ...
│
└── traits/                         # deleted (logic merged into types/trait_data.rs)
```

**Why do single-file module tests go in the parent's `tests/`?**

Because single-file modules (like `overload.rs`) don't have their own `mod.rs`, they cannot declare `#[cfg(test)] mod tests;`. Per Rust's module system, test files must be declared by a `mod.rs` to compile. Therefore, single-file module tests can only be declared by the parent module's `mod.rs`, placed in the parent's `tests/` directory.

**Decision flowchart**:

```
Encountering a module, where to put its tests?
│
├── Is this module a directory (has mod.rs)?
│   └── Yes → Create tests/ in that directory, declared by that directory's mod.rs
│
├── Is this module a single file (only .rs)?
│   └── Yes → Tests go in parent's tests/, declared by parent's mod.rs
│
└── Uncertain?
    └── Check for independent directory and mod.rs
```

**Common mistakes**:

```
# ❌ Mistake 1: Creating independent tests/ directory for single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ Should not create directory for single-file module
    └── tests/
        └── overload.rs

# ❌ Mistake 2: Declaring #[cfg(test)] mod tests; in single-file module
# overload.rs
#[cfg(test)]                        # ❌ Single-file module cannot declare like this
mod tests;                          # because there's no overload/tests/ directory

# ✅ Correct approach: tests go in parent tests/
src/frontend/core/typecheck/
├── overload.rs                     # source file
└── tests/
    └── overload.rs                 # test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern — don't do this:**

```
# ❌ Wrong: submodules' tests aggregated to parent
src/frontend/core/types/
├── mod.rs              # should only declare base and computation
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ parent tests/ contains submodule tests
    ├── mod.rs          # ❌ forced to declare mod base; mod computation;
    ├── base/           # ❌ should be in base/tests/
    │   └── var.rs
    └── computation/    # ❌ should be in computation/tests/
        └── ...
```

```
# ✅ Correct approach: each module's tests are independent
src/frontend/core/types/
├── mod.rs              # only declares pub mod base; pub mod computation;
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——declares sibling tests/
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; ——declares sibling tests/
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**Why is upward aggregation prohibited?** Because Rust's module system requires `#[cfg(test)] mod tests;` to decide test file compilation at the declaration point. If `types/mod.rs` declares `mod tests;`, then the contents of `types/tests/` are private to the `types` module—it should not cross into `base` or `computation`'s territory. Each module's tests should be internal implementation details of that module, not of the parent module. This rule also applies during module refactoring: when you split `types` into `base` and `computation`, the tests should follow the split modules, not stay put. **Test directories mirror source structure, but follow module boundaries.**

**Rule 1.2**: `tests/mod.rs` only handles module declaration and re-export, no test functions.

```rust
//! Parser core tests — mirrors src/frontend/core/parser/
//!
//! Tests for ast.rs, parser_state.rs, and expression/integration parsing.

mod ast;
mod error_recovery;
mod expressions;
mod integration;
mod parser_state;
```

**Rule 1.3**: Each test file corresponds to only one source file. Tests for multiple source modules must not be mixed in one file.

**Rule 1.4**: Test declarations must use file form `mod tests;` (with semicolon), pointing to sibling `tests/` directory. **Inline form `mod tests { ... }` is prohibited for putting test code directly in source files.**

```rust
// ✅ Correct — file form declaration, test code in separate file
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests;

// 🔴 Forbidden — inline form, test code parasitic in source file
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Test code should not appear in source files
    }
}
```

**Why is inline forbidden?**

1. Single responsibility for source files: source files contain implementation, test files contain tests. Mixed together, editing tests requires scrolling to the bottom, editing implementation requires skipping tests.
2. Clear module boundaries: `tests/` directory is a physical boundary, immediately visible which modules have tests and which don't.
3. Safe refactoring: when splitting modules, `tests/` directories follow; inline tests require manual extraction from source files.
4. Code review: in PR diffs, source changes and test changes are in separate files, not mixed together.

### Module Declaration Standards

**Rule 2.1**: All test files must have module-level doc comments `//!` at the top, explaining the specification source (language specification section number + RFC number). If a test doesn't reference any specification section, that code has no specification basis—it shouldn't exist.

```rust
//! Literal Tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integer Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String Interpolation
```

**Why must specifications be referenced?**
Because test expectations come from the specification, not from "what the current code outputs." If code changes output and tests are updated accordingly, the tests protect nothing. Only specification-anchored tests can distinguish "intentional breaking changes" from "unintentional regressions."

**Rule 2.2**: Test module `use` imports must be precise to specific types/functions, glob imports `use super::*` are prohibited.

```rust
// 🟢 Good — precise imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Garbage — no one knows what you're testing
use super::*;
```

### Naming Conventions

**Rule 3.1**: Test function naming format is `test_<what>_<scenario>`, all lowercase with underscore separators.

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, you should know what's being tested and what's expected. Numeric sequential naming is prohibited.

```rust
// 🟢 Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Garbage — no idea what's being tested
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**Rule 3.3**: Helper functions don't need `test_` prefix; use verbs or nouns to describe their purpose.

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### Test Structure Standards (Arrange-Act-Assert)

**Rule 4.1**: Each test function must follow the three-phase structure: Arrange → Act → Assert, with blank lines between phases.

```rust
#[test]
fn test_parse_binary_addition() {
    // Arrange
    let source = "1 + 2";

    // Act
    let expr = parse_expr(source);

    // Assert
    assert!(matches!(expr, Expr::Binary { op: BinOp::Add, .. }));
}
```

**Rule 4.2**: Simple tests (single call + single assertion) may omit phase comments, but must not exceed 5 lines of logic code. Tests exceeding 5 lines must explicitly mark the three phases.

### Helper Function Standards

**Rule 5.1**: Setup logic that appears 3 or more times must be extracted into helper functions.

```rust
// 🟢 Good — extracted common setup
fn with_state<F>(source: &str, mut f: F)
where
    F: FnMut(&mut ParserState<'_>),
{
    let tokens = tokenize(source).unwrap();
    let mut state = ParserState::new(&tokens);
    f(&mut state);
}

#[test]
fn test_current_returns_first_token() {
    with_state("42", |state| {
        let tok = state.current();
        assert_eq!(&tok.unwrap().kind, &TokenKind::IntLiteral(42));
    });
}
```

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print sufficient context on panic. `unwrap()` may be used directly in test function bodies (`#[test] fn ...`)—on failure, Rust automatically prints line numbers; but on failure inside helper functions, line numbers point to the helper function definition, not the call site.

```rust
// 🟢 Good — helper function failure prints source content
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Garbage — on failure you can't see which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of test files, immediately after `use` imports. If shared across multiple test modules, place in `tests/mod.rs` and `pub(crate)` export.

### Assertion Style

**Rule 6.1**: For enum variant matching, prefer `assert!(matches!(...))`, do not use `if let` + `panic!`.

```rust
// 🟢 Good
assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(42)));

// 🔴 Garbage
if let TokenKind::IntLiteral(v) = tokens[0].kind {
    assert_eq!(v, 42);
} else {
    panic!("Expected IntLiteral");
}
```

**Rule 6.2**: Use `assert_eq!` for precise value comparison, `assert!` for boolean assertions. Do not use `assert!(a == b)` instead of `assert_eq!(a, b)`.

**Rule 6.3**: All assertions must include custom error messages unless the assertion itself fully describes the failure reason.

```rust
// 🟢 Good — on assertion failure, can quickly locate issue
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 Good — assert_eq! automatically prints value diff on failure, no extra message needed
assert_eq!(error_count, 0);

// 🔴 Garbage — on failure, only know "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: Assertion order must be `assert_eq!(actual, expected)`, actual value first, expected second.

### Anti-Pattern Checklist

The following are prohibited and their alternatives:

| Anti-Pattern                              | Problem                                        | Alternative                                                        |
| ----------------------------------------- | ---------------------------------------------- | ------------------------------------------------------------------ |
| `#[cfg(test)] mod tests { ... }` inline tests | Source file bloat, fuzzy module boundaries, refactoring difficulty | Test code in separate `tests/` directory, declared with `mod tests;` (see Rule 1.4) |
| Test accommodating code's wrong behavior  | Hides specification deviations, legitimizes bugs | Fix code to match specification, keep test unchanged             |
| Deriving test expectations from code output | Tests become "recorders of current implementation" | Derive expectations from specification                           |
| Permanent `#[ignore]` markers             | Hides rotting tests                            | Fix or delete                                                      |
| `println!` debug output                   | Pollutes test output                           | Use `assert!` for explicit assertions                              |
| `thread::sleep`                          | Random failures + slow                         | Use synchronization mechanisms or mock                             |
| Operating real filesystem in tests        | Slow and non-repeatable                        | Use `tempfile`                                                     |
| Depending on test execution order         | Random failures                                | Each test has independent setup                                    |
| Single test function exceeding 30 lines   | Unreadable                                     | Split tests or use helper functions                                |
| `unwrap()` without context in helpers     | Hard to locate issues                          | Use `expect("why")` or custom panic (see Rule 5.2)                 |
| Copy-pasting identical setup 3+ times     | High modification cost                         | Extract helper functions                                           |

---

## Integration Test Standards

### Test Organization

**Rule 7.1**: Integration tests go in the project root's `tests/` directory. Entry file `tests/integration.rs` uses `#[path]` attributes to include submodules.

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**Rule 7.2**: Each `tests/integration/*.rs` file corresponds to one test topic (compiler backend, code generation, executor, etc.), no mixing.

**Rule 7.3**: Integration tests must test through the project's public API. Do not directly reference `crate::` internal modules in integration tests. Use `yaoxiang::` public path.

```rust
// 🟢 Good — through public API
use yaoxiang::run;

// 🔴 Garbage — bypasses public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests prefer inline source strings. External fixture files (in `tests/fixtures/`) are only used when source exceeds 30 lines.

```rust
#[test]
fn test_fibonacci() {
    run_ok(
        r#"
        main: () -> Void = {
            mut a = 0
            mut b = 1
            while a < 100 {
                mut next = a + b
                a = b
                b = next
            }
        }
        "#,
    );
}
```

**Rule 8.2**: Fixture files must use `.yx` extension, and file names describe the test intent.

### E2E Coverage Principles

**Rule 9.1**: Integration tests for each language feature must cover three paths:

| Path         | Description                                         |
| ----------- | --------------------------------------------------- |
| Happy path  | Legal input produces expected output                |
| Error path  | Illegal input produces clear error message (not panic) |
| Boundary    | Boundary values (empty input, max value, max nesting depth) |

**Rule 9.2**: Integration tests must not depend on network, system environment variables, or external services.

---

## Benchmark Test Standards

### Criterion.rs Usage Standards

**Rule 10.1**: Benchmark tests go in the `benches/` directory, entry file is `benches/lib.rs`. Split by test topic.

```
benches/
├── lib.rs              # entry, defines criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # cross-language comparison benchmark
├── parser.rs           # parser benchmark
└── codegen.rs          # code generation benchmark
```

**Rule 10.2**: Each benchmark function must include module doc comments `//!` explaining test purpose and measurement metrics.

```rust
//! YaoXiang interpreter performance benchmarks
//!
//! Measurement metric: single iteration time (wall time)
//! Baseline: Rust native implementation
```

### Preventing Compiler Optimization

**Rule 11.1**: All benchmark outputs must be wrapped in `criterion::black_box` to prevent compiler optimization from eliminating them.

```rust
use criterion::{black_box, Criterion};

fn bench_parse(c: &mut Criterion) {
    c.bench_function("parse_fib", |b| {
        b.iter(|| {
            let result = parse(black_box(FIB_SOURCE));
            black_box(result)
        })
    });
}
```

**Rule 11.2**: Benchmark input data must be `const` or `lazy_static`, not dynamically generated inside `iter` closures—otherwise you're measuring data generation + code under test total time.

### Benchmark Grouping and Naming

**Rule 12.1**: Benchmark test naming format is `<module_under_test>_<scenario>`, all lowercase with underscores. Consistent with unit test naming rules.

**Rule 12.2**: Must use `criterion_group!` to logically group related benchmarks. All benchmarks in one group is prohibited.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Documentation Test Standards

### Usage Scenarios

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code example in doc comments. These examples are executed via `cargo test --doc`.

````rust
/// Tokenizes a source string into a sequence of Tokens.
///
/// ```
/// use yaoxiang::frontend::core::lexer::tokenize;
///
/// let tokens = tokenize("42").unwrap();
/// assert_eq!(tokens.len(), 2); // IntLiteral + Eof
/// ```
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    // ...
}
````

**Rule 13.2**: Doc test code examples must compile and assertions must pass. Examples with `ignore` markers are prohibited unless the example demonstrates a compile-time error.

````rust
/// ```ignore
/// // Demonstrating compile-time error — ignore is allowed
/// let x: int = "string";
/// ```
````

### Coverage Requirements

**Rule 14.1**: Doc tests cover API happy path only. Boundary cases and error paths are covered by unit tests.

**Rule 14.2**: Doc test example code must be concise—no more than 10 lines. If an example needs longer context, the API design has problems.

---

## Property Test Standards

### Usage Scenarios

**Rule 15.1**: The following scenarios must use property tests (proptest or quickcheck) instead of manually writing multiple boundary value cases:

| Scenario                        | Example                                       |
| ------------------------------- | --------------------------------------------- |
| Parser round-trip               | `parse(pretty_print(ast)) == ast`             |
| Serialization/deserialization   | `deserialize(serialize(data)) == data`         |
| Mathematical operation identities | `a + b == b + a`                            |
| Compiler optimization preserves semantics | `eval(code) == eval(optimize(code))`   |

**Rule 15.2**: Property tests use `proptest` as the primary framework (already declared in `Cargo.toml` `dev-dependencies`).

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_roundtrip_serialize_deserialize(value: i64) {
        let serialized = serialize(&value);
        let deserialized: i64 = deserialize(&serialized).unwrap();
        prop_assert_eq!(deserialized, value);
    }
}
```

### Property Definition Principles

**Rule 16.1**: Each property test must have a clear property declaration—the invariant being verified must be stated in comments.

```rust
// Property: any integer literal produces the same value after tokenize → tokens_to_string
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**Rule 16.2**: If a property test finds failures, use proptest's regression mechanism—add failing inputs to `proptest-regressions/` directory, don't replace with a manually written regular test.

---

## Coverage Requirements

### New Code Coverage Targets

**Rule 17.1**: Coverage requirements for new code:

| Code Type                                  | Line Coverage | Branch Coverage |
| ------------------------------------------ | ------------- | --------------- |
| Core compiler modules (frontend/middle/backends) | ≥ 85%       | ≥ 80%           |
| Utility/helper modules (util)              | ≥ 75%         | ≥ 70%           |
| Runtime modules (vm/runtime)               | ≥ 80%         | ≥ 75%           |
| Standard library (std)                     | ≥ 75%         | ≥ 70%           |
| Error handling and diagnostics             | ≥ 90%         | ≥ 85%           |

**Rule 17.2**: Error handling paths (all `Err` branches) must be 100% covered. User-visible error messages must be verified by tests.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, author must self-check the following:

- [ ] `cargo test` all pass
- [ ] `cargo test --doc` all pass
- [ ] `cargo bench` no performance regression (if hot path changes involved)
- [ ] New code meets coverage targets
- [ ] Test naming follows naming conventions
- [ ] Each test file declares corresponding specification section (Rule 2.1)
- [ ] Test expectations come from specification definitions, not "what the current code outputs"
- [ ] No `#[ignore]` marked tests (unless with explicit issue number comment)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic messages)
- [ ] Commit messages use `:white_check_mark: test:` type
- [ ] **Did NOT modify test expectations because "code behavior doesn't match specification"—changed the code, not the tests**
- [ ] **No inline tests** (`#[cfg(test)] mod tests { ... }` must be changed to `mod tests;` + separate files, see Rule 1.4)

**Rule 18.2**: Reviewer must reject PRs containing the following issues:

- Only happy path tests, missing error paths
- Tests contain `thread::sleep` or depend on execution order
- Copy-pasted test code more than 3 times without extracting helper functions
- Test names don't follow naming conventions
- Permanent `#[ignore]` marked tests
- **Tests accommodating code's wrong behavior** (when code doesn't match spec, modifying test instead of code)
- **Tests don't declare corresponding specification section** (see Rule 2.1)
- **Test expectations come from code output, not specification** (tests derived backwards equal no tests)
- **Inline tests exist** (`#[cfg(test)] mod tests { ... }` instead of `mod tests;` + separate files, see Rule 1.4)
- Tests only verify "no panic" without asserting specific behavior
- Deleted failing tests that exposed code bugs (instead of fixing code and then seeing it turn green)

---

## Appendix

### A. Test Commands Quick Reference

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test integration

# Run doc tests only
cargo test --doc

# Run specific test (filter by name)
cargo test test_parse_expr

# Run benchmarks
cargo bench

# Show test output (stdout hidden by default)
cargo test -- --nocapture

# Run single-threaded (debug concurrency issues)
cargo test -- --test-threads=1

# Generate coverage report (requires cargo-llvm-cov)
cargo llvm-cov --html
```

### B. Commit Message Template

Test-related commits must follow this template:

```
:white_check_mark: test(<scope>): <brief description>

<Optional: list of scenarios covered>
```

Example:

```
:white_check_mark: test(parser): Add Pratt parser infix operator tests

Covered scenarios:
- Arithmetic operator precedence (+, -, *, /, %)
- Comparison operator chaining (1 < x < 10)
- Logical operator short-circuit
- Assignment operator right-associativity
```

### C. New Test File Checklist

When creating a new test module, ensure the following files are included:

```
# Adding tests under src/<module>/ directory
src/<module>/tests/
├── mod.rs          # module declaration + public helper functions
└── <subject>.rs    # test file, named corresponding to source file under test

# Adding integration tests under tests/ directory
tests/
├── integration.rs   # update: add #[path] declaration
└── integration/
    └── <topic>.rs   # new test file
```

### D. References

- [YaoXiang Language Specification](../reference/language-spec/index.md) —— **Authoritative source for tests**
- [Accepted RFCs](../design/rfc/index.md) —— **Authoritative source for design decisions**
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Conventions](./commit-convention.md)
- [Project Contributing Guide](./contributing.md)

---

> 💡
> **Remember**: Tests don't verify that your code "works"—they verify that your code conforms to the specification. When the specification changes, tests follow the specification. When code is wrong, fix the code, don't fix the tests. **Code serves the specification, tests guard the specification. The moment tests accommodate code, you lose all protection.**