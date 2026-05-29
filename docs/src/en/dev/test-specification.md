---
title: "Testing Writing Standards"
description: YaoXiang project testing hard rules, defining standards for unit tests, integration tests, benchmark tests, documentation tests, and property-based tests
---

# Testing Writing Standards

This document defines the hard rules for writing tests in the YaoXiang project. All contributors must adhere to the following rules; violators will be requested to make changes during Code Review.

---

## Table of Contents

- [General Principles](#general-principles)
- [Unit Test Standards](#unit-test-standards)
- [Integration Test Standards](#integration-test-standards)
- [Benchmark Test Standards](#benchmark-test-standards)
- [Documentation Test Standards](#documentation-test-standards)
- [Property-Based Test Standards](#property-based-test-standards)
- [Coverage Requirements](#coverage-requirements)
- [Appendix](#appendix)

---

## General Principles

### Scope of Application

These standards apply to all Rust test code in the YaoXiang project, including:

| Test Type | Location | Framework |
|-----------|----------|-----------|
| Unit Tests | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| Integration Tests | `tests/` | `#[test]` |
| Benchmark Tests | `benches/` | Criterion.rs |
| Documentation Tests | API documentation comments | `cargo test --doc` |
| Property-Based Tests | Any test location | proptest / quickcheck |

### Core Principles

**Principle 0: The canonical source for tests is the specification, not the code.** This is the most important principle in this document. Tests verify that code conforms to the specification, not that the code "passes with the current implementation." When tests discover that code behavior differs from the specification, **fix the code, never fix the tests**.

Specification files are located at:
- `docs/src/design/language-spec.md` —— Language core specification
- `docs/src/design/rfc/accepted/` —— Accepted RFC design documents

Each test file must declare the corresponding specification section at the top (see Rule 2.1). Any developer should be able to take the specification document and compare it against the tests to verify implementation correctness. Conversely—if a piece of code has no corresponding specification description, it should not exist, and it should not be tested.

```rust
// ✅ Good — test directly references the specification and verifies code compliance
//! Literal tests — based on Language Specification §2.6
//!
//! §2.6.1: Integer Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation

#[test]
fn test_decimal_literal_parsing() {
    // Specification §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 Bad — test accommodates current code implementation behavior rather than verifying specification
#[test]
fn test_literal_1() {
    // Can't tell which section of the specification this corresponds to
    // If parse_literal returns a wrong value, this test will "pass in green"
    // because it only verifies the function doesn't panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and discover the code behavior differs from the specification. You have two choices:

| Wrong Approach | Correct Approach |
|----------------|------------------|
| Modify the test to "pass" | Modify the code to match the specification |
| Add `#[ignore]` to the test | Immediately fix the code implementation |
| Add special conditional branches in the test to accommodate the code | Remove branches so the test directly exposes the problem |

Remember: **Red light = code is wrong, not test is wrong.** (Unless your test itself has a bug, which is a different matter.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of the code under test by reading the tests, without additional comments or external documentation.

```rust
// ✅ Good — test name states what is being tested and what is expected
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 Bad — nobody knows what's being tested
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for random failures.** Tests must be repeatable in any environment. Tests that depend on random numbers, system time, or thread scheduling order must use fixed seeds or be replaced with mocks.

**Principle 3: One test, one thing.** If a test name requires "and" to connect multiple behaviors, split it into multiple tests.

```rust
// ✅ Good — each test verifies only one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Bad — one test crammed with too much unrelated content
#[test]
fn test_parser() {
    // Test tokenize, test parse, test typecheck, test codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring internal implementation should not cause tests to fail. If you change one line of implementation code and 10 tests fail, your tests are written incorrectly.

But here's a critical distinction: **the definition of "behavior" comes from the specification, not from how the current code behaves.** If code changes behavior (i.e., new behavior that doesn't match the specification), tests must fail. If you can't achieve this, your tests are "tests that accommodate the code"—they let bugs pass right through.

```
Specification (language-spec.md / RFC)  ──defines──►  Expected behavior  ──drives──►  Tests
                                           │
Current code  ──implements──►  Actual behavior  ──compared against──►  Test results

If actual behavior ≠ expected behavior:
  Test must fail (red light)  ──►  Fix code  ──►  Test passes (green light)
  
If actual behavior = expected behavior (but implementation is poor):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is what Principle 4 means
```

**Principle 5: Don't write fallback/compatibility/mode-specific test code.** The test environment is one you have full control over. If you need `#[cfg(not(ci))]` to skip a test, the test design has a fundamental problem.

### Terminology Definitions

| Term | Definition |
|------|------------|
| Unit Test | Tests the behavior of a single function or module without external system dependencies |
| Integration Test | Tests multiple modules collaborating through public APIs or command-line entry points |
| Benchmark Test | Measures code performance and detects performance regressions |
| Documentation Test | Executable code examples embedded in documentation comments |
| Property-Based Test | Tests that verify invariants (properties) using random inputs |

### Relationship with Commit Conventions

All test-related commits must use the `:white_check_mark: test:` type, as specified in [Commit Conventions](./commit-convention.md).

```
:white_check_mark: test(parser): add infix expression tests for Pratt parser
:white_check_mark: test(codegen): complete switch statement IR generation tests
```

---

## Unit Test Standards

### File Organization

**Rule 1.1**: The `tests/` directory for unit tests must be at the **same level** as the `mod.rs` of the module under test. `tests/` does not aggregate upward or cross-level.

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——declares tests/ at the same level
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

Key judgment criterion: **Whichever directory `tests/` is placed in, that directory's `mod.rs` must declare it with `#[cfg(test)] mod tests;`.**

#### Test Placement Rules for Single-File Modules vs Directory Modules

**Core distinction**: The module's organization form determines where tests are placed.

| Module Type | Judgment Basis | Test Location | Example |
|-------------|----------------|---------------|---------|
| **Directory Module** | Has its own directory and `mod.rs` | `tests/` under that directory | `inference/tests/` |
| **Single-File Module** | Only has `.rs` file, no independent directory | Parent module's `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

**Detailed Explanation**:

```
src/frontend/core/typecheck/
├── mod.rs                          # typecheck module's mod.rs
├── checker.rs                      # single-file module
├── environment.rs                  # single-file module
├── overload.rs                    # single-file module
├── type_eval.rs                    # single-file module
├── dead_code.rs                    # single-file module
├── spawn_placement.rs              # single-file module
├── signature.rs                    # single-file module
├── types.rs                        # single-file module
│
├── tests/                          # ✅ typecheck's test directory
│   ├── mod.rs                      # declares tests for single-file modules
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
│   ├── mod.rs                      # #[cfg(test)] mod tests; ——declares tests/ at same level
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
└── traits/                         # directory module (has mod.rs)
    ├── mod.rs                      # #[cfg(test)] mod tests; ——declares tests/ at same level
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
        ├── solver.rs               # tests for solver.rs
        ├── impl_check.rs           # tests for impl_check.rs
        └── ...
```

**Why are single-file module tests placed in the parent `tests/`?**

Because single-file modules (like `overload.rs`) don't have their own `mod.rs`, they cannot declare `#[cfg(test)] mod tests;`. According to Rust's module system, test files must be declared by a `mod.rs` to be compiled. Therefore, tests for single-file modules can only be declared by the parent module's `mod.rs` and placed in the parent's `tests/` directory.

**Decision Flow**:

```
Encounter a module, where should tests go?
│
├── Is this module a directory (has mod.rs)?
│   └── Yes → Create tests/ in that directory, declared by that directory's mod.rs
│
├── Is this module a single file (only has .rs)?
│   └── Yes → Tests go in the parent's tests/ directory, declared by the parent's mod.rs
│
└── Not sure?
    └── Check if there's an independent directory and mod.rs
```

**Common Mistakes**:

```
# ❌ Mistake 1: Creating an independent tests/ directory for a single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ Should not create a directory for a single-file module
    └── tests/
        └── overload.rs

# ❌ Mistake 2: Declaring #[cfg(test)] mod tests; inside a single-file module
# overload.rs
#[cfg(test)]                        # ❌ Single-file module cannot declare like this
mod tests;                          # because there's no overload/tests/ directory

# ✅ Correct approach: tests go in parent tests/
src/frontend/core/typecheck/
├── overload.rs                     # source file
└── tests/
    └── overload.rs                 # test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern — Don't do this:**

```
# ❌ Wrong: Child module tests centralized to parent
src/frontend/core/types/
├── mod.rs              # should only declare base and computation
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ Parent tests/ contains child module tests
    ├── mod.rs          # ❌ forced to declare mod base; mod computation;
    ├── base/           # ❌ this should be in base/tests/
    │   └── var.rs
    └── computation/    # ❌ this should be in computation/tests/
        └── ...
```

```
# ✅ Correct approach: each module's tests are independent
src/frontend/core/types/
├── mod.rs              # only declares pub mod base; pub mod computation;
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——declares tests/ at same level
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; ——declares tests/ at same level
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**Why can't tests aggregate upward?** Because Rust's module system requires that `#[cfg(test)] mod tests;` determines test file compilation at declaration time. If `types/mod.rs` declares `mod tests;`, then the contents of `types/tests/` are private details of the `types` module—it should not cross into the territories of `base` or `computation`. Each module's tests should be internal implementation details of that module, not of the parent module. This rule also applies during module refactoring: when you split `types` into `base` and `computation`, the tests should follow the split modules, not stay in place. **Test directories don't mirror source structure; they follow module boundaries.**

**Rule 1.2**: `tests/mod.rs` is only responsible for module declaration and re-export, not for test functions.

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

**Rule 1.4**: `#[cfg(test)]` may only appear in two locations—in `lib.rs` declaring `mod tests`, or inline declaring `#[cfg(test)] mod tests;` inside the source file under test. Must not be used elsewhere.

```rust
// src/frontend/core/parser/mod.rs or lib.rs
#[cfg(test)]
mod tests;
```

### Module Declaration Standards

**Rule 2.1**: All test files must have module-level doc comments `//!` at the top, stating the specification source (language specification section numbers + RFC numbers). If a test doesn't reference any specification section, it means this code has no specification basis—it should not exist.

```rust
//! Literal tests — based on Language Specification §2.6
//!
//! §2.6.1: Integer Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation
```

**Why must specification be referenced?** Because test expectations come from the specification, not from "the current code's output." If someday the code changes its output and the tests are updated accordingly, the tests protect nothing. Only specification-anchored tests can distinguish "intentional breaking changes" from "unintentional regressions."

**Rule 2.2**: Test module `use` imports must be precise to specific types/functions; glob imports `use super::*` are prohibited.

```rust
// ✅ Good — precise imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Bad — nobody knows what you're testing
use super::*;
```

### Naming Conventions

**Rule 3.1**: Test function naming format is `test_<what>_<scenario>`, all lowercase with underscore separation.

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, you should know what's being tested and what's expected. Numeric ordinal naming is prohibited.

```rust
// ✅ Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Bad — completely unclear what's being tested
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

**Rule 4.1**: Each test function must follow a three-phase structure: Arrange → Act → Assert, with blank lines between the three phases.

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
// ✅ Good — extract common setup
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

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print sufficient context on panic. In test function bodies (`#[test] fn ...`), you may directly `unwrap()`—on failure, Rust automatically prints the line number; but when a helper function fails, the line number points to the helper function definition, and the context at the call site is not visible.

```rust
// ✅ Good — helper function failure prints source content
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Bad — on failure you can't tell which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, right after `use` imports. If shared across multiple test modules, place them in `tests/mod.rs` and export with `pub(crate)`.

### Assertion Style

**Rule 6.1**: Enum variant matching should use `assert!(matches!(...))`, not `if let` + `panic!`.

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

**Rule 6.2**: Use `assert_eq!` for precise value comparison, and `assert!` for boolean assertions. Using `assert!(a == b)` to replace `assert_eq!(a, b)` is prohibited.

**Rule 6.3**: All assertions must include custom error messages unless the assertion itself fully describes the failure reason.

```rust
// ✅ Good — when assertion fails, can quickly locate the issue
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// ✅ Good — assert_eq! failure automatically prints value differences, no extra message needed
assert_eq!(error_count, 0);

// 🔴 Bad — on failure, only know "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: Assertion order must be `assert_eq!(actual, expected)`, with actual value first, expected value second.

### Anti-Pattern List

The following practices are prohibited and their alternatives are provided:

| Anti-Pattern | Problem | Alternative |
|--------------|---------|-------------|
| Tests accommodate code's erroneous behavior | Hides specification deviations, legitimizes bugs | Fix code according to specification, keep tests unchanged |
| Deriving test expectations from code output | Tests become "recorders of current implementation" | Derive expectations from specification |
| Permanent `#[ignore]` markers | Hides rotting tests | Fix or delete |
| `println!` debug output | pollutes test output | Use `assert!` for clear assertions |
| `thread::sleep` | Random failures + slow | Use synchronization mechanisms or mocks |
| Operating real filesystem in tests | Slow and non-repeatable | Use `tempfile` |
| Relying on test execution order | Random failures | Each test has independent setup |
| Single test function exceeding 30 lines of logic | Unmaintainable | Split tests or use helper functions |
| `unwrap()` without context in helper functions | Hard to locate issues | Use `expect("why")` or custom panic (see Rule 5.2) |
| copy-paste same setup 3+ times | High modification cost | Extract helper functions |

---

## Integration Test Standards

### Test Organization

**Rule 7.1**: Integration tests are placed in the `tests/` directory at the project root. Entry file `tests/integration.rs` uses `#[path]` attributes to include submodules.

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**Rule 7.2**: Each `tests/integration/*.rs` file corresponds to one test topic (compiler backend, code generation, executor, etc.) and must not be mixed.

**Rule 7.3**: Integration tests must test through the project's public APIs. Direct references to `crate::` internal modules are prohibited in integration tests. Use the `yaoxiang::` public path.

```rust
// ✅ Good — through public API
use yaoxiang::run;

// 🔴 Bad — bypasses public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests prefer inline source code strings. External fixture files are only used when source code exceeds 30 lines (placed in `tests/fixtures/`).

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
| Boundary | Boundary values (empty input, max values, nested depth limits) |

**Rule 9.2**: Integration tests must not depend on network, system environment variables, or external services.

---

## Benchmark Test Standards

### Criterion.rs Usage Standards

**Rule 10.1**: Benchmark tests are unified in the `benches/` directory, with entry file `benches/lib.rs`. Organized by test topic.

```
benches/
├── lib.rs              # entry, defines criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # cross-language comparison benchmarks
├── parser.rs           # parser benchmarks
└── codegen.rs          # code generation benchmarks
```

**Rule 10.2**: Each benchmark function must include module-level doc comments `//!` describing the test purpose and measurement metrics.

```rust
//! YaoXiang interpreter performance benchmarks
//!
//! Measurement metric: single iteration elapsed time (wall time)
//! Baseline: Rust native implementation
```

### Preventing Compiler Optimization

**Rule 11.1**: All benchmark test outputs must be passed through `criterion::black_box` to prevent compiler optimization from eliminating them.

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

**Rule 11.2**: Benchmark test input data must be `const` or `lazy_static`, not dynamically generated inside the `iter` closure—otherwise you're measuring the total time of data generation + tested logic.

### Benchmark Grouping and Naming

**Rule 12.1**: Benchmark test naming format is `<module_under_test>_<scenario>`, all lowercase with underscore separation. Consistent with unit test naming rules.

**Rule 12.2**: `criterion_group!` must be used to logically group related benchmarks. All benchmarks must not be crammed into one group.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Documentation Test Standards

### Usage Scenarios

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code example in doc comments. This example is executed via `cargo test --doc`.

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

**Rule 13.2**: Documentation test code examples must compile successfully and assertions must pass. Examples with `ignore` markers are not allowed, unless the example demonstrates a compile-time error.

```rust
/// ```ignore
/// // Demonstrating compile-time error — ignore is allowed
/// let x: int = "string";
/// ```
```

### Coverage Requirements

**Rule 14.1**: Documentation tests only need to cover the API's happy path. Boundary cases and error paths are covered by unit tests.

**Rule 14.2**: Example code in documentation tests must be concise—no more than 10 lines. If an example needs longer context, the API design has a problem.

---

## Property-Based Test Standards

### Usage Scenarios

**Rule 15.1**: The following scenarios must use property-based tests (proptest or quickcheck) instead of manually writing multiple boundary value cases:

| Scenario | Example |
|----------|---------|
| Parser round-trip | `parse(pretty_print(ast)) == ast` |
| Serialization/deserialization | `deserialize(serialize(data)) == data` |
| Mathematical identities | `a + b == b + a` |
| Compiler optimization preserves semantics | `eval(code) == eval(optimize(code))` |

**Rule 15.2**: Property-based tests use `proptest` as the primary framework (already declared in `Cargo.toml` under `dev-dependencies`).

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

**Rule 16.1**: Each property-based test must have an explicit property declaration—a comment stating the invariant being verified.

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

**Rule 16.2**: If a property-based test discovers a failure, the regression mechanism of `proptest` must be used—add the failing input to the `proptest-regressions/` directory. Do not manually write a regular test to replace it.

---

## Coverage Requirements

### New Code Coverage Targets

**Rule 17.1**: Coverage requirements for new code:

| Code Type | Line Coverage | Branch Coverage |
|-----------|--------------|----------------|
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
- [ ] Test naming conforms to naming conventions
- [ ] Each test file declares corresponding specification sections (Rule 2.1)
- [ ] Test expectations come from specification definitions, not "current code output"
- [ ] No `#[ignore]` marked tests (unless accompanied by explicit issue number comment)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic messages)
- [ ] Commit message uses `:white_check_mark: test:` type
- [ ] **No modification of test expectations due to "code behavior not matching specification"—modify the code, not the tests**

**Rule 18.2**: Reviewers must reject PRs containing the following issues:

- Only happy path tests, missing error paths
- Tests contain `thread::sleep` or depend on execution order
- Copy-pasted test code exceeds 3 times without extracting helper functions
- Test names don't conform to naming conventions
- Permanent `#[ignore]` tests exist
- **Tests accommodate code's erroneous behavior** (when code doesn't match specification, modifying tests rather than code)
- **Tests don't declare corresponding specification sections** (see Rule 2.1)
- **Test expectations derived from code output rather than specification** (backward-derived tests equal no testing)
- Tests only verify "no panic" without asserting specific behavior
- Deleted failing tests that exposed code bugs (rather than fixing the code and seeing it turn green)

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

# Run only documentation tests
cargo test --doc

# Run specific test (filter by name)
cargo test test_parse_expr

# Run benchmark tests
cargo bench

# Show test output (stdout hidden by default)
cargo test -- --nocapture

# Single-threaded execution (investigate concurrency issues)
cargo test -- --test-threads=1

# Generate coverage report (requires cargo-llvm-cov)
cargo llvm-cov --html
```

### B. Commit Message Template

Test-related commits must follow this template:

```
:white_check_mark: test(<scope>): <short description>

<Optional: list of scenarios covered>
```

Example:

```
:white_check_mark: test(parser): add infix operator tests for Pratt parser

Covered scenarios:
- Arithmetic operator precedence (+, -, *, /, %)
- Comparison operator chaining (1 < x < 10)
- Logical operator short-circuiting
- Assignment operator right-associativity
```

### C. New Test File Checklist

When creating a new test module, ensure the following files are included:

```
# Add tests under src/<module>/ directory
src/<module>/tests/
├── mod.rs          # module declaration + public helper functions
└── <subject>.rs    # test file, named to correspond with source file under test

# Add integration tests under tests/
tests/
├── integration.rs   # Update: add #[path] declarations
└── integration/
    └── <topic>.rs   # new test file
```

### D. References

- [YaoXiang Language Specification](../../design/language-spec.md) —— **Canonical source for tests**
- [Accepted RFCs](../../design/rfc/accepted/) —— **Canonical source for design decisions**
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Conventions](./commit-convention.md)
- [Project Contributing Guide](./contributing.md)

---

> 💡 **Remember**: Tests don't verify that your code "works"—they verify that your code conforms to the specification. When the specification changes, tests follow the specification. When code is wrong, fix the code, don't fix the tests. **Code serves the specification; tests guard the specification. The moment tests accommodate the code, you lose all protection.**