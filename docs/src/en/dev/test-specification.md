---
title: "Testing Standards"
description: "YaoXiang project testing hard rules, defining standards for unit tests, integration tests, benchmark tests, documentation tests, and property-based tests"
---

# Testing Standards

This document defines the hard rules for writing tests in the YaoXiang project. All contributors must comply with the following rules; violations will require changes during Code Review.

---

## Table of Contents

- [General Principles](#general-principles)
- [Unit Testing Standards](#unit-testing-standards)
- [Integration Testing Standards](#integration-testing-standards)
- [Benchmark Testing Standards](#benchmark-testing-standards)
- [Documentation Testing Standards](#documentation-testing-standards)
- [Property-Based Testing Standards](#property-based-testing-standards)
- [Coverage Requirements](#coverage-requirements)
- [Appendix](#appendix)

---

## General Principles

### Scope of Application

These rules apply to all Rust test code in the YaoXiang project:

| Test Type | Location | Framework |
|-----------|----------|-----------|
| Unit Tests | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| Integration Tests | `tests/` | `#[test]` |
| Benchmark Tests | `benches/` | Criterion.rs |
| Documentation Tests | API documentation comments | `cargo test --doc` |
| Property-Based Tests | Any test location | proptest / quickcheck |

### Core Principles

**Principle 0: The specification is the authoritative source for tests, not the code.** This is the most important principle in this document. Tests verify that code conforms to the specification, not that code "works with the current implementation." When tests find that code behavior differs from the specification, **fix the code, never fix the tests**.

Specification documents are located at:
- `docs/src/design/language-spec.md` —— Language core specification
- `docs/src/design/rfc/accepted/` —— Accepted RFC design documents

Every test file must declare the corresponding specification section at the top (see Rule 2.1). Any developer should be able to take the specification document and compare it against the tests to verify correctness of the implementation. Conversely—if a piece of code has no corresponding specification description, it should not exist, and should not be tested.

```rust
// ✅ Good——Test directly references the specification, verifying code compliance
//! Literal tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation

#[test]
fn test_decimal_literal_parsing() {
    // Specification §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 Bad——Test accommodates current code behavior instead of verifying the specification
#[test]
fn test_literal_1() {
    // Don't know which section of the specification this corresponds to
    // If parse_literal returns the wrong value, this test will "pass"
    // Because it only verifies the function doesn't panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and find that the code behavior doesn't match the specification. You have two choices:

| Wrong Approach | Correct Approach |
|----------------|-------------------|
| Modify the test to make it "pass" | Modify the code so behavior matches the specification |
| Add `#[ignore]` to the test | Fix the code implementation immediately |
| Add special conditional branches to accommodate the code | Delete branches and let the test expose the problem directly |

Remember: **Red light = the code is wrong, not the test.** (Unless your test itself has a bug, which is a different matter.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of the code under test by reading the tests, without needing additional comments or external documentation.

```rust
// ✅ Good——Test name states what is being tested and what is expected
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 Bad——Nobody knows what's being tested
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for random failures.** Tests must be repeatable in any environment. Tests that depend on random numbers, system time, or thread scheduling order must use fixed seeds or be replaced with mocks.

**Principle 3: One test, one thing.** If a test name needs "and" to connect multiple behaviors, split it into multiple tests.

```rust
// ✅ Good——Each test verifies only one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Bad——Too much unrelated content crammed into one test
#[test]
fn test_parser() {
    // Test tokenize, test parse, test typecheck, test codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring internal implementation should not cause tests to fail. If changing one line of implementation code causes 10 tests to fail, your tests are written incorrectly.

But there's a key distinction here: **The definition of "behavior" comes from the specification, not from how the current code currently behaves.** If the code changes behavior (i.e., new behavior that contradicts the specification), tests must fail. If you can't achieve this, your tests are "code-accommodating tests"—they let bugs slip through.

```
Specification (language-spec.md / RFC)  ──defines──►  Expected behavior  ──drives──►  Tests
                                                          │
Current code  ──implements──►  Actual behavior  ──compares──►  Test results

If actual behavior ≠ expected behavior:
  Test must fail (red light)  ──►  Fix code  ──►  Test passes (green light)
  
If actual behavior = expected behavior (but implementation is poor):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is the meaning of Principle 4
```

**Principle 5: Do not write fallback/compatibility/situational test code.** The test environment is one you can fully control. If you need `#[cfg(not(ci))]` to skip a test, there's a fundamental problem with the test design.

### Terminology Definitions

| Term | Definition |
|------|------------|
| Unit Test | Tests behavior of a single function or module without external dependencies |
| Integration Test | Tests collaboration of multiple modules through public APIs or CLI entry points |
| Benchmark Test | Measures code performance and detects performance regressions |
| Documentation Test | Executable code examples embedded in documentation comments |
| Property-Based Test | Tests that verify invariants using random inputs |

### Relationship with Commit Standards

All test-related commits must use the `:white_check_mark: test:` type, as specified in the [commit standards](./commit-convention.md).

```
:white_check_mark: test(parser): Add Pratt parser infix expression tests
:white_check_mark: test(codegen): Complete switch statement IR generation tests
```

---

## Unit Testing Standards

### File Organization

**Rule 1.1**: The `tests/` directory for unit tests must be at the **same level** as the `mod.rs` of the module under test. `tests/` does not aggregate upward or cross-level.

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——declares同级 tests/
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——pratt's own tests
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # parser module-level tests (does not include pratt submodule content)
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

Key judgment criteria: **When `tests/` is placed in which directory, that directory's `mod.rs` must declare it with `#[cfg(test)] mod tests;`.**

**Rule 1.1 Supplement: Aggregation upward is prohibited.** Tests for subdirectory modules must be placed in that subdirectory's own `tests/`, not aggregated to parent-level `tests/`.

| Module Type | Test Location | Example |
|-------------|---------------|---------|
| Directory module (has `mod.rs`) | `tests/` under that directory | `emitter/tests/`, `codes/tests/` |
| Single-file module (only `.rs`) | Parent's `tests/` | `session.rs` → `diagnostic/tests/session.rs` |

```text
# ✅ Correct: Each directory module's tests are independent
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
└── tests/                  # ✅ diagnostic level (single-file modules)
    ├── mod.rs
    ├── session.rs
    ├── suggest.rs
    └── collect.rs

# ❌ Wrong: Aggregating emitter and codes tests to diagnostic/tests/
src/util/diagnostic/
└── tests/
    ├── mod.rs              # ❌ Forced to declare mod emitter; mod codes;
    ├── emitter/            # ❌ Should be in emitter/tests/
    └── codes/              # ❌ Should be in codes/tests/
```

#### Single-File Modules vs Directory Modules: Test Placement Rules

**Core difference**: The module's organizational form determines where tests are placed.

| Module Type | Judgment Criteria | Test Location | Example |
|-------------|-------------------|---------------|---------|
| **Directory module** | Has its own directory and `mod.rs` | `tests/` under that directory | `inference/tests/` |
| **Single-file module** | Only has `.rs` file, no independent directory | Parent's `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

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
│   ├── checker.rs                  # checker.rs tests
│   ├── environment.rs              # environment.rs tests
│   ├── overload.rs                 # overload.rs tests (single-file module test placed here)
│   ├── type_eval.rs                # type_eval.rs tests
│   ├── dead_code.rs                # dead_code.rs tests
│   ├── spawn_placement.rs          # spawn_placement.rs tests
│   ├── signature.rs                # signature.rs tests
│   └── types.rs                    # types.rs tests
│
├── inference/                      # directory module (has mod.rs)
│   ├── mod.rs                      # #[cfg(test)] mod tests; ——declares同级 tests/
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
│       ├── expressions.rs          # expressions.rs tests
│       ├── statements.rs           # statements.rs tests
│       └── ...
│
└── traits/                         # directory module (has mod.rs)
    ├── mod.rs                      # #[cfg(test)] mod tests; ——declares同级 tests/
    ├── solver.rs
    ├── impl_check.rs
    ├── inheritance.rs
    ├── coherence.rs
    ├── auto_derive.rs
    ├── object_safety.rs
    ├── resolution.rs
    ├── std_traits.rs
    ├── gat/
    ├── specialization/
    └── tests/                      # ✅ traits' test directory
        ├── mod.rs
        ├── solver.rs               # solver.rs tests
        ├── impl_check.rs           # impl_check.rs tests
        └── ...
```

**Why are single-file module tests placed in parent `tests/`?**

Because single-file modules (like `overload.rs`) don't have their own `mod.rs`, they cannot declare `#[cfg(test)] mod tests;`. According to Rust's module system, test files must be declared by some `mod.rs` to compile. Therefore, single-file module tests can only be declared by the parent module's `mod.rs`, and are placed in the parent's `tests/` directory.

**Decision flow**:

```
Encounter a module, where to put tests?
│
├── Is this module a directory (has mod.rs)?
│   └── Yes → Create tests/ in that directory, declared by that directory's mod.rs
│
├── Is this module a single file (only .rs)?
│   └── Yes → Tests go in parent's tests/ directory, declared by parent's mod.rs
│
└── Not sure?
    └── Check if it has an independent directory and mod.rs
```

**Common mistakes**:

```
# ❌ Mistake 1: Creating independent tests/ directory for single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ Should not create directory for single-file module
    └── tests/
        └── overload.rs

# ❌ Mistake 2: Declaring #[cfg(test)] mod tests; inside single-file module
# overload.rs
#[cfg(test)]                        # ❌ Single-file modules cannot declare like this
mod tests;                          # because there's no overload/tests/ directory

# ✅ Correct approach: Tests go in parent tests/
src/frontend/core/typecheck/
├── overload.rs                     # source file
└── tests/
    └── overload.rs                 # test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern—don't do this:**

```
# ❌ Wrong: Child module tests aggregated to parent level
src/frontend/core/types/
├── mod.rs              # Should only declare base and computation
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ Parent tests/ contains child module tests
    ├── mod.rs          # ❌ Forced to declare mod base; mod computation;
    ├── base/           # ❌ This part should be in base/tests/
    │   └── var.rs
    └── computation/    # ❌ This part should be in computation/tests/
        └── ...
```

```
# ✅ Correct approach: Each module's tests are independent
src/frontend/core/types/
├── mod.rs              # Only declares pub mod base; pub mod computation;
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——declares同级 tests/
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; ——declares同级 tests/
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**Why can't we aggregate upward?** Because Rust's module system requires `#[cfg(test)] mod tests;` to determine test file compilation at the declaration site. If `types/mod.rs` declares `mod tests;`, then `types/tests/` content is private to the `types` module—it should not cross into `base` or `computation`'s territory. Each module's tests should be internal implementation details of that module, not the parent module's. This rule also applies during module refactoring: when you split `types` into `base` and `computation`, tests should follow the split modules, not stay in place. **Test directories do not mirror source structure; they follow module boundaries.**

**Rule 1.2**: `tests/mod.rs` is only responsible for module declarations and re-exports, not test functions.

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

**Rule 1.4**: `#[cfg(test)]` may only appear in two places—declaring `mod tests` in `lib.rs`, or inline declaration `#[cfg(test)] mod tests;` inside the source file under test. It must not be used elsewhere.

```rust
// src/frontend/core/parser/mod.rs or lib.rs
#[cfg(test)]
mod tests;
```

### Module Declaration Standards

**Rule 2.1**: All test files must have a module-level documentation comment `//!` at the top, explaining the specification source coverage (language specification section number + RFC number). If a test doesn't reference any specification section, it means this code has no specification basis—it should not exist.

```rust
//! Literal tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation
```

**Why must we reference the specification?** Because test expectations come from the specification, not from "the current code's output." If code changes output and tests are updated accordingly, the tests protect nothing. Only specification-anchored tests can distinguish "intentional breaking changes" from "unintentional regressions."

**Rule 2.2**: Test module `use` imports must be precise to specific types/functions; glob imports `use super::*` are prohibited.

```rust
// ✅ Good——Precise imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Bad——Others don't know what's being tested
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
// ✅ Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Bad——Completely unclear what's being tested
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**Rule 3.3**: Helper functions don't need the `test_` prefix; they should use verbs or nouns describing their purpose.

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### Test Structure Standards (Arrange-Act-Assert)

**Rule 4.1**: Each test function must follow the three-phase structure: Arrange → Act → Assert, with blank lines separating the three phases.

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

**Rule 4.2**: Simple tests (single call + single assertion) may omit phase comments but must not exceed 5 lines of logic code. Tests exceeding 5 lines must explicitly mark the three phases.

### Helper Function Standards

**Rule 5.1**: Setup logic that appears 3 or more times must be extracted into helper functions.

```rust
// ✅ Good——Extract common setup
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

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print sufficient context on panic. In test function bodies (`#[test] fn ...`), direct `unwrap()` is fine—when it fails, Rust automatically prints the line number; but when it fails inside a helper function, the line number points to the helper function definition, not the call site.

```rust
// ✅ Good——Helper function prints source content on failure
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Bad——On failure, you can't see which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, immediately after `use` imports. If shared across multiple test modules, place them in `tests/mod.rs` and export with `pub(crate)`.

### Assertion Style

**Rule 6.1**: Enum variant matching should prefer `assert!(matches!(...))`; `if let` + `panic!` is not allowed.

```rust
// ✅ Good
assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(42)));

// 🔴 Bad
if let TokenKind::IntLiteral(v) = tokens[0].kind {
    assert_eq!(v, 42);
} else {
    panic!("Expected IntLiteral");
}
```

**Rule 6.2**: Use `assert_eq!` for exact value comparisons, `assert!` for boolean assertions. `assert!(a == b)` must not be used instead of `assert_eq!(a, b)`.

**Rule 6.3**: All assertions must include custom error messages, unless the assertion itself already fully describes the failure reason.

```rust
// ✅ Good——Can quickly locate the issue when assertion fails
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// ✅ Good——assert_eq! automatically prints value differences on failure, no extra message needed
assert_eq!(error_count, 0);

// 🔴 Bad——On failure, you only know "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: Assertion order must be `assert_eq!(actual, expected)`, actual value first, expected value second.

### Anti-Pattern Checklist

The following are prohibited patterns and their alternatives:

| Anti-Pattern | Problem | Alternative |
|--------------|---------|-------------|
| Test accommodates code's incorrect behavior | Obscures specification deviation, legitimizes bugs | Fix code to match specification, keep test unchanged |
| Derive test expectations from code output | Test becomes "current implementation's tape recorder" | Derive expectations from specification |
| Permanent `#[ignore]` markers | Hides rotting tests | Fix or delete |
| `println!` debug output | Pollutes test output | Use `assert!` for clear assertions |
| `thread::sleep` | Random failures + slow | Use synchronization mechanisms or mocks |
| Operating real filesystem in tests | Slow and non-repeatable | Use `tempfile` |
| Depending on test execution order | Random failures | Each test has independent setup |
| Single test function exceeds 30 lines of logic | Unreadable | Split test or use helper functions |
| `unwrap()` in helper functions without context | Hard to locate | Use `expect("why")` or custom panic (see Rule 5.2) |
| Copy-pasting identical setup 3+ times | High modification cost | Extract helper function |

---

## Integration Testing Standards

### Test Organization

**Rule 7.1**: Integration tests are placed in the project root `tests/` directory. The entry file `tests/integration.rs` uses `#[path]` attributes to include submodules.

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**Rule 7.2**: Each `tests/integration/*.rs` file corresponds to one test topic (compiler backend, code generation, executor, etc.), not mixed.

**Rule 7.3**: Integration tests must test through the project's public API. Direct references to `crate::` internal modules are prohibited in integration tests. Use `yaoxiang::` public paths.

```rust
// ✅ Good——Through public API
use yaoxiang::run;

// 🔴 Bad——Bypasses public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests prefer inline source code strings. Only use external fixture files when source code exceeds 30 lines (placed in `tests/fixtures/`).

```rust
#[test]
fn test_fibonacci() {
    run_ok(
        r#"
        main = {
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

**Rule 8.2**: Fixture files must use `.yx` extension, and filenames describe the test intent.

### E2E Coverage Principles

**Rule 9.1**: Integration tests for each language feature must cover three paths:

| Path | Description |
|------|-------------|
| Happy path | Valid input produces expected output |
| Error path | Invalid input produces clear error messages (not panic) |
| Boundary | Boundary values (empty input, maximum values, nesting depth limits) |

**Rule 9.2**: Integration tests must not depend on network, system environment variables, or external services.

---

## Benchmark Testing Standards

### Criterion.rs Usage Standards

**Rule 10.1**: Benchmark tests are uniformly placed in the `benches/` directory, with `benches/lib.rs` as the entry point. Files are organized by test topic.

```
benches/
├── lib.rs              # Entry, defines criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # Cross-language comparison benchmarks
├── parser.rs           # Parser benchmarks
└── codegen.rs          # Code generation benchmarks
```

**Rule 10.2**: Each benchmark function must include a module documentation comment `//!` describing the test purpose and measurement metrics.

```rust
//! YaoXiang interpreter performance benchmarks
//!
//! Measurement metrics: single iteration time (wall time)
//! Baseline: Rust native implementation
```

### Preventing Compiler Optimizations

**Rule 11.1**: All benchmark test outputs under test must use `criterion::black_box` to prevent compiler optimization from eliminating them.

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

**Rule 11.2**: Benchmark test input data must be `const` or `lazy_static`; it must not be dynamically generated inside the `iter` closure—otherwise you're measuring the total time of data generation + code under test.

### Benchmark Grouping and Naming

**Rule 12.1**: Benchmark test naming format is `<module_under_test>_<scenario>`, all lowercase with underscores. Consistent with unit test naming rules.

**Rule 12.2**: `criterion_group!` must be used to logically group related benchmarks. All benchmarks must not be crammed into one group.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Documentation Testing Standards

### Usage Scenarios

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code example in documentation comments. These examples are executed via `cargo test --doc`.

```rust
/// Tokenizes a source code string into a sequence of Tokens.
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
```

**Rule 13.2**: Documentation test code examples must compile and assertions must pass. Examples with `ignore` markers are not allowed, unless the example demonstrates a compile-time error.

```rust
/// ```ignore
/// // Demonstrating compile-time error——may use ignore
/// let x: int = "string";
/// ```
```

### Coverage Requirements

**Rule 14.1**: Documentation tests cover the API happy path only. Boundary cases and error paths are covered by unit tests.

**Rule 14.2**: Example code in documentation tests must be concise—no more than 10 lines. If an example needs longer context, the API design has issues.

---

## Property-Based Testing Standards

### Usage Scenarios

**Rule 15.1**: The following scenarios must use property-based testing (proptest or quickcheck) instead of manually writing multiple boundary value cases:

| Scenario | Example |
|----------|---------|
| Parser round-trip | `parse(pretty_print(ast)) == ast` |
| Serialization/deserialization | `deserialize(serialize(data)) == data` |
| Mathematical operation identities | `a + b == b + a` |
| Compiler optimization preserves semantics | `eval(code) == eval(optimize(code))` |

**Rule 15.2**: Property-based tests use `proptest` as the primary framework (already declared in `Cargo.toml` `dev-dependencies`).

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

**Rule 16.1**: Each property-based test must have a clear property declaration—write the invariant being verified in comments.

```rust
// Property: Any integer literal produces the same value after tokenize → tokens_to_string
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**Rule 16.2**: If a property-based test fails, the `proptest` regression mechanism must be used—add the failing input to the `proptest-regressions/` directory. Do not replace it with a regular test.

---

## Coverage Requirements

### New Code Coverage Targets

**Rule 17.1**: Coverage requirements for new code:

| Code Type | Line Coverage | Branch Coverage |
|-----------|---------------|-----------------|
| Core compiler modules (frontend/middle/backends) | ≥ 85% | ≥ 80% |
| Utility/helper modules (util) | ≥ 75% | ≥ 70% |
| Runtime modules (vm/runtime) | ≥ 80% | ≥ 75% |
| Standard library (std) | ≥ 75% | ≥ 70% |
| Error handling and diagnostics | ≥ 90% | ≥ 85% |

**Rule 17.2**: Error handling paths (all `Err` branches) must be 100% covered. User-visible error messages must be verified by tests.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, the author must self-check the following items:

- [ ] `cargo test` all pass
- [ ] `cargo test --doc` all pass
- [ ] `cargo bench` no performance regression (if hot path changes are involved)
- [ ] New code meets coverage targets
- [ ] Test names conform to naming conventions
- [ ] Each test file declares the corresponding specification section (Rule 2.1)
- [ ] Test expectations come from specification definitions, not "current code output"
- [ ] No `#[ignore]`-marked tests (unless with explicit issue number comment)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic messages)
- [ ] Commit message uses `:white_check_mark: test:` type
- [ ] **No modification of test expectations because "code behavior differs from specification"—the code is modified, not the test**

**Rule 18.2**: Reviewers must reject PRs containing the following issues:

- Only happy path tests, missing error paths
- Tests contain `thread::sleep` or depend on execution order
- Copy-pasted test code exceeds 3 times without extracting helper functions
- Test names don't conform to naming conventions
- Permanent `#[ignore]` tests exist
- **Tests accommodate code's incorrect behavior** (when code doesn't match specification, test is modified instead of code)
- **Tests don't declare corresponding specification sections** (see Rule 2.1)
- **Test expectations come from code output rather than specification definitions** (back-derived tests are equivalent to no testing)
- Tests only verify "doesn't panic" without asserting specific behavior
- Deleted tests that exposed code bugs (instead of fixing the code and seeing them turn green)

---

## Appendix

### A. Test Commands Quick Reference

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration

# Run only documentation tests
cargo test --doc

# Run specific test (filter by name)
cargo test test_parse_expr

# Run benchmark tests
cargo bench

# Show test output (stdout hidden by default)
cargo test -- --nocapture

# Run single-threaded (troubleshoot concurrency issues)
cargo test -- --test-threads=1

# Generate coverage report (requires cargo-llvm-cov)
cargo llvm-cov --html
```

### B. Commit Message Template

Test-related commits must follow this template:

```
:white_check_mark: test(<scope>): <brief description>

<Optional: list of covered scenarios>
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
# Add tests under src/<module>/ directory
src/<module>/tests/
├── mod.rs          # Module declaration + public helper functions
└── <subject>.rs    # Test file, named corresponding to the source file under test

# Add integration tests under tests/ directory
tests/
├── integration.rs   # Update: add #[path] declarations
└── integration/
    └── <topic>.rs   # New test file
```

### D. References

- [YaoXiang Language Specification](../../design/language-spec.md) —— **Authoritative source for tests**
- [Accepted RFCs](../../design/rfc/accepted/) —— **Authoritative source for design decisions**
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Standards](./commit-convention.md)
- [Project Contributing Guide](./contributing.md)

---

> 💡 **Remember**: Tests don't verify that your code "works"—they verify that your code conforms to the specification. When the specification changes, tests follow the specification. When code is wrong, fix the code, not the tests. **Code serves the specification; tests guard the specification. The moment tests accommodate code, you lose all protection.**