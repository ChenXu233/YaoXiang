---
title: "Test Writing Standards"
description: YaoXiang project hard conventions for writing tests, defining standards for unit tests, integration tests, benchmarks, doc tests, and property tests
---

# Test Writing Standards

This document defines the hard conventions for writing tests in the YaoXiang project. All contributors must follow the rules below; violators will be required to revise in Code Review.

---

## Table of Contents

- [General Rules](#general-rules)
- [Unit Test Conventions](#unit-test-conventions)
- [Integration Test Conventions](#integration-test-conventions)
- [Benchmark Test Conventions](#benchmark-test-conventions)
- [Doc Test Conventions](#doc-test-conventions)
- [Property Test Conventions](#property-test-conventions)
- [Coverage Requirements](#coverage-requirements)
- [Appendix](#appendix)

---

## General Rules

### Scope

These conventions apply to all Rust test code in the YaoXiang project, including:

| Test Type | Location | Framework |
|----------|------|------|
| Unit Test | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| Integration Test | `tests/` | `#[test]` |
| Benchmark Test | `benches/` | Criterion.rs |
| Doc Test | API doc comments | `cargo test --doc` |
| Property Test | Anywhere in tests | proptest / quickcheck |

### Core Principles

**Principle 0: The authoritative source of tests is the specification, not the code.** This is the most important principle in this document. Tests verify whether the code follows the specification, not whether the code "runs with the current implementation." When a test reveals a discrepancy between code behavior and the specification, **fix the code, never fix the test**.

The specification files are located at:
- `docs/src/design/language-spec.md` — Language core specification
- `docs/src/design/rfc/accepted/` — Accepted RFC design documents

Each test file must declare the corresponding specification section at the top (see Rule 2.1). Any developer should be able to take the specification document and cross-check the test against it to verify the correctness of the implementation. Conversely — if a piece of code has no corresponding specification description, it should not exist, much less be tested.

```rust
// 🟢 Good — The test directly references the specification, verifying that the code follows the spec
//! Literal tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floats (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation

#[test]
fn test_decimal_literal_parsing() {
    // Specification §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 Garbage — The test accommodates the current code's implementation behavior, rather than verifying the spec
#[test]
fn test_literal_1() {
    // Don't know which section of the spec this corresponds to
    // If parse_literal returns the wrong value, this test would "pass green"
    // because it only verifies the function does not panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and find that the code behavior does not match the specification. You have two choices:
| Wrong Approach | Correct Approach |
|----------|----------|
| Modify the test to make it "pass" | Modify the code so the behavior matches the spec |
| Add `#[ignore]` to the test | Fix the code implementation immediately |
| Add special conditional branches to the test to accommodate the code | Remove the branch, let the test expose the issue directly |

Remember: **Red light = the code is wrong, not the test.** (Unless your test itself has a bug, that's a different story.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of the code under test by reading the tests, without needing additional comments or external documentation.

```rust
// 🟢 Good — The test name says what is tested and what is expected
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 Garbage — Nobody knows what this tests
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for flaky tests.** Tests must be repeatable in any environment. Tests that depend on random numbers, system time, or thread scheduling order must use fixed seeds or use mocks instead.

**Principle 3: One test, one thing.** If a test name needs to use "and" to connect multiple behaviors, split it into multiple tests.

```rust
// 🟢 Good — Each test verifies only one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Garbage — One test stuffed with too many unrelated things
#[test]
fn test_parser() {
    // Test tokenize, test parse, test typecheck, test codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring the internal implementation should not cause tests to fail. If changing one line of implementation code causes 10 tests to fail, your tests are wrong.

But there is a key distinction here: **the definition of "behavior" comes from the specification, not from the current code's performance.** If the code changes behavior (i.e., new behavior that does not match the spec), the test must fail. If you cannot do this, your test is a "test that accommodates the code" — it lets bugs come in through the front door.

```
Specification (language-spec.md / RFC)  ──defines──►  Expected behavior  ──drives──►  Tests
                                           │
Current code  ──implements──►  Actual behavior  ──compares──►  Test results

If actual behavior ≠ expected behavior:
  Test must fail (red light)  ──►  Fix code  ──►  Test passes (green light)

If actual behavior = expected behavior (but implementation is ugly):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is the meaning of Principle 4
```

**Principle 5: Do not write fallback/compatibility/specific-mode-active test code.** The test environment is one you can fully control. If you need `#[cfg(not(ci))]` to skip a test, it indicates a fundamental problem with the test design.

### Terminology

| Term | Definition |
|------|------|
| Unit Test | Tests the behavior of a single function or module, with no external system dependencies |
| Integration Test | Tests the collaboration of multiple modules, through the public API or command-line entry |
| Benchmark Test | Measures code performance and detects performance regressions |
| Doc Test | Executable code examples embedded in documentation comments |
| Property Test | Tests that verify invariants (properties) based on random input |

### Relationship with Commit Conventions

All test-related commits must use the `:white_check_mark: test:` type, see [Commit Conventions](./commit-convention.md).

```
:white_check_mark: test(parser): Add tests for Pratt parser infix expressions
:white_check_mark: test(codegen): Complete tests for switch statement IR generation
```

---

## Unit Test Conventions

### File Organization

**Rule 1.1**: The `tests/` directory for unit tests must be at the **same level** as the `mod.rs` of the module under test. `tests/` does not aggregate upward or summarize across levels.

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; —— Declares the same-level tests/
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; —— Pratt's own tests
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # Module-level tests for parser (does not include pratt submodule's content)
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

Key criterion: **Whichever directory the `tests/` is placed in, that directory's `mod.rs` must declare it with `#[cfg(test)] mod tests;`.**

**Rule 1.1 Supplement: No upward aggregation.** A submodule's tests must be placed in that submodule's own `tests/`, not aggregated into the parent-level `tests/`.

| Module Type | Test Location | Example |
|----------|----------|------|
| Directory Module (with `mod.rs`) | `tests/` in that directory | `emitter/tests/`, `codes/tests/` |
| Single-file Module (only `.rs`) | Parent-level `tests/` | `session.rs` → `diagnostic/tests/session.rs` |

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

# ❌ Wrong: Aggregate emitter and codes tests into diagnostic/tests/
src/util/diagnostic/
└── tests/
    ├── mod.rs              # ❌ Forced to declare mod emitter; mod codes;
    ├── emitter/            # ❌ Should be in emitter/tests/
    └── codes/              # ❌ Should be in codes/tests/
```

#### Test Placement Rules for Single-file vs Directory Modules

**Core Difference**: The form of the module's organization determines where the tests are placed.

| Module Type | Criterion | Test Location | Example |
|----------|----------|----------|------|
| **Directory Module** | Has independent directory and `mod.rs` | `tests/` in that directory | `inference/tests/` |
| **Single-file Module** | Only `.rs` file, no independent directory | Parent module's `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

**Detailed Explanation**:

```
src/frontend/core/typecheck/
├── mod.rs                          # typecheck module's mod.rs
├── checker.rs                      # Single-file module
├── environment.rs                  # Single-file module
├── overload.rs                     # Single-file module
├── type_eval.rs                    # Single-file module
├── dead_code.rs                    # Single-file module
├── spawn_placement.rs              # Single-file module
├── signature.rs                    # Single-file module
├── types.rs                        # Single-file module
│
├── tests/                          # ✅ typecheck's test directory
│   ├── mod.rs                      # Declare single-file module tests
│   ├── checker.rs                  # checker.rs's tests
│   ├── environment.rs              # environment.rs's tests
│   ├── overload.rs                 # overload.rs's tests (single-file module tests go here)
│   ├── type_eval.rs                # type_eval.rs's tests
│   ├── dead_code.rs                # dead_code.rs's tests
│   ├── spawn_placement.rs          # spawn_placement.rs's tests
│   ├── signature.rs                # signature.rs's tests
│   └── types.rs                    # types.rs's tests
│
├── inference/                      # Directory module (with mod.rs)
│   ├── mod.rs                      # #[cfg(test)] mod tests; —— Declares same-level tests/
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
│       ├── expressions.rs          # expressions.rs's tests
│       ├── statements.rs           # statements.rs's tests
│       └── ...
│
└── traits/                         # Directory module (with mod.rs)
    ├── mod.rs                      # #[cfg(test)] mod tests; —— Declares same-level tests/
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
        ├── solver.rs               # solver.rs's tests
        ├── impl_check.rs           # impl_check.rs's tests
        └── ...
```

**Why are single-file module tests placed in the parent `tests/`?**

Because single-file modules (like `overload.rs`) do not have their own `mod.rs`, they cannot declare `#[cfg(test)] mod tests;`. According to Rust's module system, test files must be declared by some `mod.rs` to be compiled. Therefore, single-file module tests can only be declared by the parent module's `mod.rs` and placed in the parent module's `tests/` directory.

**Judgment Flow**:

```
Encounter a module, decide where to place the test?
│
├── Is the module a directory (has mod.rs)?
│   └── Yes → Create tests/ in that directory, declared by that directory's mod.rs
│
├── Is the module a single file (only .rs)?
│   └── Yes → Tests go in the parent module's tests/ directory, declared by the parent's mod.rs
│
└── Not sure?
    └── Check whether it has an independent directory and mod.rs
```

**Common Errors**:

```
# ❌ Error 1: Create an independent tests/ directory for a single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ Should not create a directory for a single-file module
    └── tests/
        └── overload.rs

# ❌ Error 2: Declare #[cfg(test)] mod tests; inside a single-file module
# overload.rs
#[cfg(test)]                        # ❌ A single-file module cannot declare like this
mod tests;                          # because there is no overload/tests/ directory

# ✅ Correct: Tests go in the parent tests/
src/frontend/core/typecheck/
├── overload.rs                     # Source file
└── tests/
    └── overload.rs                 # Test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern — Do not write like this:**

```
# ❌ Wrong: Submodule tests collected at the parent
src/frontend/core/types/
├── mod.rs              # Should only declare base and computation
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ Parent tests/ contains submodule tests
    ├── mod.rs          # ❌ Forced to declare mod base; mod computation;
    ├── base/           # ❌ This part should be in base/tests/
    │   └── var.rs
    └── computation/    # ❌ This part should be in computation/tests/
        └── ...
```

```
# ✅ Correct: Each module's tests are independent
src/frontend/core/types/
├── mod.rs              # Only declare pub mod base; pub mod computation;
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; —— Declares same-level tests/
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; —— Declares same-level tests/
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**Why no upward aggregation?** Because Rust's module system requires `#[cfg(test)] mod tests;` to decide the compilation of test files at the declaration point. If `types/mod.rs` declares `mod tests;`, then the contents of `types/tests/` are private to the `types` module — they should not cross into `base` or `computation` territory. Each module's tests should be internal implementation details of that module, not the parent module's. This rule also applies to module refactoring: when you split `types` into `base` and `computation`, the tests should also follow the split modules, not stay in place. **The test directory does not mirror the source structure, but follows module boundaries.**

**Rule 1.2**: `tests/mod.rs` is only responsible for module declarations and re-exports; do not place test functions in it.

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

**Rule 1.3**: Each test file corresponds to only one source file. Tests for multiple source modules are not allowed to be mixed into a single file.

**Rule 1.4**: Test declarations must use the file form `mod tests;` (with semicolon), pointing to the same-level `tests/` directory. **It is forbidden to use the inline form `mod tests { ... }` to write test code directly in the source file.**

```rust
// ✅ Correct — file form declaration, test code in independent files
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
        // Test code should not appear in the source file
    }
}
```

**Why forbid inline?**
1. Single responsibility of source files: source files only contain implementation, test files only contain tests. Mixed together, modifying tests requires scrolling to the bottom of the file, modifying implementation requires skipping past the tests.
2. Clear module boundaries: the `tests/` directory is a physical boundary, allowing immediate visibility into which modules have tests and which do not.
3. Refactoring safety: when modules are split, the `tests/` directory follows along; inline tests require manual extraction from the source file.
4. Code review: in PR diffs, source code changes and test changes are in separate files, not mixed together.

### Module Declaration Conventions

**Rule 2.1**: All test files must have a module-level documentation comment `//!` at the top, explaining the specification source covered by the tests (language specification section number + RFC number). If a test does not reference any specification section, it means the code has no specification basis — it should not exist.

```rust
//! Literal tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floats (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation
```

**Why must we reference the specification?** Because the expected values of tests come from the specification, not from "the output of the current code." If one day the code changes its output and the test is updated accordingly, the test is not protecting anything. Only specification-anchored tests can distinguish between "intentional breaking changes" and "unintentional regressions."

**Rule 2.2**: Test module `use` imports must be precise to specific types/functions; glob imports `use super::*` are forbidden.

```rust
// 🟢 Good — Precise imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Garbage — Others don't know what you are testing
use super::*;
```

### Naming Conventions

**Rule 3.1**: Test function naming format is `test_<what>_<scenario>`, all lowercase, separated by underscores.

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, you should know what is tested and what is expected. Numeric sequence names are forbidden.

```rust
// 🟢 Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Garbage — No idea what is tested
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**Rule 3.3**: Helper functions do not need the `test_` prefix; they should use verbs or nouns to describe their purpose.

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### Test Structure Conventions (Arrange-Act-Assert)

**Rule 4.1**: Every test function must follow the three-part structure: Arrange → Act → Assert, with blank lines between the three parts.

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

**Rule 4.2**: Simple tests (single call + single assertion) may omit the section comments, but the logical code may not exceed 5 lines. Tests exceeding 5 lines must explicitly label the three parts.

### Helper Function Conventions

**Rule 5.1**: Setup logic that appears 3 or more times must be extracted into a helper function.

```rust
// 🟢 Good — Extract common setup
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

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print sufficient context on panic. Inside test function bodies (`#[test] fn ...`), `unwrap()` can be used directly — on failure, Rust automatically prints the line number; but when a helper function fails, the line number points to the helper function's definition, and you cannot see the context at the call site.

```rust
// 🟢 Good — Helper function prints source content on failure
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Garbage — On failure, you cannot see which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, immediately after the `use` imports. If shared by multiple test modules, place them in `tests/mod.rs` and export as `pub(crate)`.

### Assertion Style

**Rule 6.1**: Enum variant matching should prefer `assert!(matches!(...))`; `if let` + `panic!` is not allowed.

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

**Rule 6.2**: Use `assert_eq!` for exact value comparison, and `assert!` for boolean assertions. `assert!(a == b)` is forbidden as a substitute for `assert_eq!(a, b)`.

**Rule 6.3**: All assertions must include custom error messages, unless the assertion itself fully describes the failure reason.

```rust
// 🟢 Good — Failures can be quickly located
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 Good — assert_eq! automatically prints value differences on failure, no extra message needed
assert_eq!(error_count, 0);

// 🔴 Garbage — On failure, you only know "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: The assertion order must be `assert_eq!(actual, expected)`, with the actual value first and the expected value second.

### Anti-pattern Checklist

The following are prohibited practices and their replacements:

| Anti-pattern | Problem | Replacement |
|--------|------|----------|
| `#[cfg(test)] mod tests { ... }` inline tests | Source file bloat, blurred module boundaries, difficult refactoring | Test code placed in independent `tests/` directory, declared with `mod tests;` (see Rule 1.4) |
| Tests accommodating code's wrong behavior | Hides specification deviations, legitimizes bugs | Fix code against the spec, keep tests unchanged |
| Deriving test expectations from code output | Test becomes a "recorder of current implementation" | Derive expected values from the spec |
| Permanent `#[ignore]` markings | Hides rotten tests | Fix or delete |
| `println!` debug output | Pollutes test output | Use `assert!` to make explicit assertions |
| `thread::sleep` | Flaky + slow | Use synchronization mechanisms or mocks |
| Operating on real file system in tests | Slow and unrepeatable | Use `tempfile` |
| Relying on test execution order | Flaky | Each test has independent setup |
| A test function exceeds 30 lines of logic | Nobody can understand it | Split tests or use helper functions |
| `unwrap()` in helper functions without context | Difficult to locate | Use `expect("why")` or custom panic (see Rule 5.2) |
| Copy-paste the same setup 3+ times | High modification cost | Extract helper function |

---

## Integration Test Conventions

### Test Organization

**Rule 7.1**: Integration tests go in the `tests/` directory at the project root. The entry file `tests/integration.rs` uses the `#[path]` attribute to include submodules.

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**Rule 7.2**: Each `tests/integration/*.rs` file corresponds to a single test topic (compiler backends, code generation, executor, etc.), and may not be mixed.

**Rule 7.3**: Integration tests must be tested through the project's public API. It is forbidden to directly reference `crate::` internal modules in integration tests. Use the `yaoxiang::` public path.

```rust
// 🟢 Good — Through the public API
use yaoxiang::run;

// 🔴 Garbage — Bypasses the public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests should prefer inline source code strings. External fixture files (placed in `tests/fixtures/`) are only used when the source code exceeds 30 lines.

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

**Rule 8.2**: Fixture files must end with the `.yx` extension, and the filename describes the test intent.

### E2E Coverage Principles

**Rule 9.1**: The integration tests for each language feature must cover three paths:

| Path | Description |
|------|------|
| Happy path | Legal input produces expected output |
| Error path | Illegal input produces clear error message (not a panic) |
| Boundary | Boundary values (empty input, maximum value, nesting depth limit) |

**Rule 9.2**: Integration tests may not depend on the network, system environment variables, or external services.

---

## Benchmark Test Conventions

### Criterion.rs Usage Conventions

**Rule 10.1**: Benchmark tests are uniformly placed in the `benches/` directory, with the entry file being `benches/lib.rs`. Split by test topic.

```
benches/
├── lib.rs              # Entry, defines criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # Cross-language comparison benchmarks
├── parser.rs           # Parser benchmarks
└── codegen.rs          # Code generation benchmarks
```

**Rule 10.2**: Every benchmark function must include a module-level documentation comment `//!` explaining the test purpose and measurement metrics.

```rust
//! YaoXiang interpreter performance benchmarks
//!
//! Measurement metric: single iteration wall time
//! Baseline: native Rust implementation
```

### Preventing Compiler Optimization

**Rule 11.1**: The output of the code under test in all benchmarks must pass through `criterion::black_box` to prevent the compiler from eliminating optimizations.

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

**Rule 11.2**: Benchmark input data must be `const` or `lazy_static`; it may not be generated dynamically inside the `iter` closure — otherwise the measurement covers the total time of data generation + the code under test.

### Benchmark Grouping and Naming

**Rule 12.1**: Benchmark naming format is `<module-under-test>_<scenario>`, all lowercase, separated by underscores. Consistent with unit test naming rules.

**Rule 12.2**: You must use `criterion_group!` to logically group related benchmarks. It is forbidden to cram all benchmarks into one group.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Doc Test Conventions

### Usage Scenarios

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code example in their documentation comments. This example is executed by `cargo test --doc`.

```rust
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
```

**Rule 13.2**: Doc test code examples must compile successfully and pass their assertions. Examples marked with `ignore` are not allowed, unless the example demonstrates a compile-time error.

```rust
/// ```ignore
/// // Demonstrates compile-time error — may be ignored
/// let x: int = "string";
/// ```
```

### Coverage Requirements

**Rule 14.1**: Doc tests only need to cover the API's happy path. Boundary cases and error paths are covered by unit tests.

**Rule 14.2**: Example code in doc tests must be concise — no more than 10 lines. If the example requires longer context, the API design has a problem.

---

## Property Test Conventions

### Usage Scenarios

**Rule 15.1**: The following scenarios must use property tests (proptest or quickcheck) instead of hand-written multiple boundary value cases:

| Scenario | Example |
|------|------|
| Parser round-trip | `parse(pretty_print(ast)) == ast` |
| Serialization/deserialization | `deserialize(serialize(data)) == data` |
| Mathematical operation identity | `a + b == b + a` |
| Compiler optimizations do not change semantics | `eval(code) == eval(optimize(code))` |

**Rule 15.2**: Property tests use `proptest` as the primary property test framework (declared in `dev-dependencies` of `Cargo.toml`).

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

**Rule 16.1**: Every property test must have a clear property declaration — comments should describe the invariant being verified.

```rust
// Property: any integer literal, after tokenize → tokens_to_string, produces the same value
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**Rule 16.2**: If a property test discovers a failure, the `proptest` regression mechanism must be used — add the failing input to the `proptest-regressions/` directory, do not manually write a regular test to replace it.

---

## Coverage Requirements

### New Code Coverage Goals

**Rule 17.1**: Test coverage requirements for new code:

| Code Type | Line Coverage | Branch Coverage |
|----------|----------|----------|
| Core compiler modules (frontend/middle/backends) | ≥ 85% | ≥ 80% |
| Utility/helper modules (util) | ≥ 75% | ≥ 70% |
| Runtime modules (vm/runtime) | ≥ 80% | ≥ 75% |
| Standard library (std) | ≥ 75% | ≥ 70% |
| Error handling and diagnostics | ≥ 90% | ≥ 85% |

**Rule 17.2**: Error handling paths (all `Err` branches) must be 100% covered. Error messages that users can see must be verified by tests.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, the author must self-check the following items:

- [ ] `cargo test` all passes
- [ ] `cargo test --doc` all passes
- [ ] `cargo bench` has no performance regression (if hot path changes are involved)
- [ ] New code meets coverage goals
- [ ] Test names conform to naming conventions
- [ ] Each test file declares the corresponding specification section (Rule 2.1)
- [ ] Test expected values come from the specification definition, not "the output of the current code"
- [ ] No `#[ignore]` marked tests (unless commented with a clear issue number)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic messages)
- [ ] Commit message uses the `:white_check_mark: test:` type
- [ ] **The test expected value has not been modified because "code behavior does not match the spec" — what is changed is the code, not the test**
- [ ] **No inline tests** (`#[cfg(test)] mod tests { ... }` must be changed to `mod tests;` + independent files, see Rule 1.4)

**Rule 18.2**: Reviewers must reject PRs that contain the following issues:

- Only happy path tests, missing error paths
- `thread::sleep` in tests or reliance on execution order
- Copy-pasted test code more than 3 times without extracting helper functions
- Test names do not conform to naming conventions
- Existence of permanently `#[ignore]` tests
- **Tests accommodating the code's wrong behavior** (modifying the test rather than the code when the code does not match the spec)
- **Tests do not declare the corresponding specification section** (see Rule 2.1)
- **Test expected values come from code output rather than specification definition** (a test derived by reverse engineering is equivalent to no test)
- **Existence of inline tests** (`#[cfg(test)] mod tests { ... }` instead of `mod tests;` + independent files, see Rule 1.4)
- Tests only verify "does not panic" without asserting specific behavior
- Deleted failing tests that exposed code bugs (instead of fixing the code and seeing it turn green)

---

## Appendix

### A. Test Command Quick Reference

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration

# Run only doc tests
cargo test --doc

# Run a specific test (filter by name)
cargo test test_parse_expr

# Run benchmarks
cargo bench

# Show test output (stdout hidden by default)
cargo test -- --nocapture

# Run single-threaded (to debug concurrency issues)
cargo test -- --test-threads=1

# Generate coverage report (requires cargo-llvm-cov)
cargo llvm-cov --html
```

### B. Commit Message Template

Test-related commits must follow this template:

```
:white_check_mark: test(<scope>): <short description>

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

When creating a new test module, make sure to include the following files:

```
# Add tests in src/<module>/ directory
src/<module>/tests/
├── mod.rs          # Module declaration + common helper functions
└── <subject>.rs    # Test file, named after the source file under test

# Add integration tests in tests/ directory
tests/
├── integration.rs   # Update: add #[path] declarations
└── integration/
    └── <topic>.rs   # New test file
```

### D. References

- [YaoXiang Language Specification](../../design/language-spec.md) — **The authoritative source for tests**
- [Accepted RFCs](../../design/rfc/accepted/) — **The authoritative source for design decisions**
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Conventions](./commit-convention.md)
- [Project Contribution Guide](./contributing.md)

---

> 💡 **Remember**: Tests do not verify whether your code "can run" — they verify whether your code complies with the specification. The specification changes, and the tests change with it. If the code is wrong, fix the code, not the test. **Code serves the specification; tests guard the specification. The moment a test accommodates the code, you have lost all protection.**