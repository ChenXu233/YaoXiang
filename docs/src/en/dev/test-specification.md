---
title: Test Writing Specification
description: YaoXiang project test writing hard rules, defining standards for unit tests, integration tests, benchmark tests, doc tests, and property-based tests
---

# Test Writing Specification

This document defines the hard rules for test writing in the YaoXiang project. All contributors must comply with the following rules; violators will be required to make changes during Code Review.

---

## Table of Contents

- [General Principles](#general-principles)
- [Unit Test Specification](#unit-test-specification)
- [Integration Test Specification](#integration-test-specification)
- [Benchmark Test Specification](#benchmark-test-specification)
- [Doc Test Specification](#doc-test-specification)
- [Property-Based Test Specification](#property-based-test-specification)
- [Coverage Requirements](#coverage-requirements)
- [Appendices](#appendices)

---

## General Principles

### Scope of Application

This specification applies to all Rust test code in the YaoXiang project, including:

| Test Type | Location | Framework |
|-----------|----------|-----------|
| Unit tests | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| Integration tests | `tests/` | `#[test]` |
| Benchmark tests | `benches/` | Criterion.rs |
| Doc tests | API documentation comments | `cargo test --doc` |
| Property-based tests | Any test location | proptest / quickcheck |

### Core Principles

**Principle 0: The specification is the authoritative source of truth for tests, not the code.** This is the most important principle in this document. Tests verify that code conforms to the specification, not that code "works with the current implementation." When tests reveal that code behavior is inconsistent with the specification, **fix the code, never fix the tests.**

Specification files are located at:
- `docs/src/design/language-spec.md` —— Core language specification
- `docs/src/design/rfc/accepted/` —— Accepted RFC design documents

Every test file must declare the corresponding specification section at the top (see Rule 2.1). Any developer should be able to take the specification document, compare it with the tests, and verify the correctness of the implementation. Conversely——if a piece of code has no corresponding specification description, it should not exist, and it should certainly not be tested.

```rust
// ✅ Good——test directly references the specification, verifying code follows the specification
//! Literal Tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \nrt'"\, \x, \u{})
//! RFC-012: F-String Interpolation

#[test]
fn test_decimal_literal_parsing() {
    // Specification §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// ❌ Trash——test accommodates the current code's implementation behavior instead of verifying the specification
#[test]
fn test_literal_1() {
    // No idea which section of the specification this corresponds to
    // If parse_literal returns a wrong value, this test will "pass"
    // because it only verifies the function doesn't panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and discover that the code behavior doesn't match the specification. You have two choices:

| Wrong Approach | Correct Approach |
|----------------|------------------|
| Modify the test to make it "pass" | Modify the code so behavior matches the specification |
| Add `#[ignore]` to the test | Fix the code implementation immediately |
| Add special conditional branches in the test to accommodate the code | Remove branches and let the test expose the problem directly |

Remember: **Red light = code is wrong, not the test is wrong.** (Unless your test itself has a bug, which is a different matter.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of the code under test by reading the tests, without additional comments or external documentation.

```rust
// ✅ Good——test name states what's being tested and what's expected
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// ❌ Trash——nobody knows what's being tested
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for random failures.** Tests must be repeatable in any environment. Tests that depend on random numbers, system time, or thread scheduling order must use fixed seeds or mock implementations instead.

**Principle 3: One test, one thing.** If a test name requires "and" to connect multiple behaviors, split into multiple tests.

```rust
// ✅ Good——each test verifies only one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// ❌ Trash——one test crammed with too much unrelated content
#[test]
fn test_parser() {
    // Test tokenize, test parse, test typecheck, test codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring internal implementation should not cause test failures. If you changed one line of implementation code and 10 tests broke, your tests are written wrong.

But there's a critical distinction here: **the definition of "behavior" comes from the specification, not from how the current code currently behaves.** If the code changes behavior (i.e., new behavior that doesn't conform to the specification), tests must fail. If you can't achieve this, your tests are "code-accommodating tests"——they let bugs march right in.

```
Specification (language-spec.md / RFC)  ──defines──►  Expected Behavior  ──drives──►  Tests
                                           │
Current Code  ──implements──►  Actual Behavior  ──compared──►  Test Results

If actual behavior ≠ expected behavior:
  Test must fail (red light)  ──►  Fix code  ──►  Test passes (green light)
  
If actual behavior = expected behavior (but implementation is poor):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is what Principle 4 means
```

**Principle 5: Do not write fallback/compatibility/pattern-specific test code.** The test environment is one you have full control over. If you need `#[cfg(not(ci))]` to skip a test, the test design has a fundamental problem.

### Terminology Definitions

| Term | Definition |
|------|------------|
| Unit test | Tests behavior of a single function or module without external system dependencies |
| Integration test | Tests multiple modules collaborating via public API or command-line entry points |
| Benchmark test | Measures code performance and detects performance regressions |
| Doc test | Executable code examples embedded in documentation comments |
| Property-based test | Tests that verify invariants based on random inputs |

### Relationship with Commit Conventions

All test-related commits must use the `:white_check_mark: test:` type, referring to the [Commit Conventions](./commit-convention.md).

```
:white_check_mark: test(parser): Add Pratt parser infix expression tests
:white_check_mark: test(codegen): Complete switch statement IR generation tests
```

---

## Unit Test Specification

### File Organization

**Rule 1.1**: The `tests/` directory for unit tests must be at the **same level** as the `mod.rs` of the module under test. `tests/` does not aggregate upward or cross-level aggregate.

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——declares tests/ at same level
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——pratt's own tests
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # parser module-level tests (does not contain pratt submodule content)
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

Key judgment criteria: **When `tests/` is placed in which directory, that directory's `mod.rs` must declare it with `#[cfg(test)] mod tests;`.**

#### Single-File Module vs. Directory Module Test Placement Rules

**Core distinction**: The organization form of a module determines where tests are placed.

| Module Type | Judgment Basis | Test Location | Example |
|-------------|----------------|----------------|---------|
| **Directory module** | Has its own directory and `mod.rs` | `tests/` under that directory | `inference/tests/` |
| **Single-file module** | Only has `.rs` files, no independent directory | `tests/` in parent module | `overload.rs` → `typecheck/tests/overload.rs` |

**Detailed explanation**:

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
├── tests/                          # ✅ typecheck's tests directory
│   ├── mod.rs                      # Declares tests for single-file modules
│   ├── checker.rs                  # Tests for checker.rs
│   ├── environment.rs              # Tests for environment.rs
│   ├── overload.rs                 # Tests for overload.rs (single-file module tests go here)
│   ├── type_eval.rs                # Tests for type_eval.rs
│   ├── dead_code.rs                # Tests for dead_code.rs
│   ├── spawn_placement.rs          # Tests for spawn_placement.rs
│   ├── signature.rs                # Tests for signature.rs
│   └── types.rs                    # Tests for types.rs
│
├── inference/                      # Directory module (has mod.rs)
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
│   └── tests/                      # ✅ inference's tests directory
│       ├── mod.rs
│       ├── expressions.rs          # Tests for expressions.rs
│       ├── statements.rs           # Tests for statements.rs
│       └── ...
│
└── traits/                         # Directory module (has mod.rs)
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
    └── tests/                      # ✅ traits's tests directory
        ├── mod.rs
        ├── solver.rs               # Tests for solver.rs
        ├── impl_check.rs           # Tests for impl_check.rs
        └── ...
```

**Why do single-file module tests go in the parent `tests/`?**

Because single-file modules (like `overload.rs`) don't have their own `mod.rs`, they cannot declare `#[cfg(test)] mod tests;`. According to Rust's module system, test files must be declared by a `mod.rs` to be compiled. Therefore, single-file module tests can only be declared by the parent module's `mod.rs` and placed in the parent's `tests/` directory.

**Decision flowchart**:

```
Encountering a module, where to put tests?
│
├── Is this module a directory (has mod.rs)?
│   └── Yes → Create tests/ in that directory, declared by that directory's mod.rs
│
├── Is this module a single file (only .rs)?
│   └── Yes → Tests go in parent's tests/, declared by parent's mod.rs
│
└── Not sure?
    └── Check if there's an independent directory and mod.rs
```

**Common mistakes**:

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

# ✅ Correct approach: Tests go in parent's tests/
src/frontend/core/typecheck/
├── overload.rs                     # Source file
└── tests/
    └── overload.rs                 # Test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern——do not do this:**

```
# ❌ Wrong: Submodule tests concentrated in parent
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
# ✅ Correct approach: Each module's tests are independent
src/frontend/core/types/
├── mod.rs              # Only declares pub mod base; pub mod computation;
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

**Why cannot aggregate upward?** Because Rust's module system requires `#[cfg(test)] mod tests;` to decide test file compilation at the declaration site. If `types/mod.rs` declares `mod tests;`, then the contents of `types/tests/` become private details of the `types` module——they should not cross into the territory of `base` or `computation`. Each module's tests should be that module's internal implementation details, not the parent module's. This rule also applies to module refactoring: when you split `types` into `base` and `computation`, tests should follow the split modules, not stay in place. **Test directories do not mirror source structure; they follow module boundaries.**

**Rule 1.2**: `tests/mod.rs` only handles module declaration and re-exports, no test functions go here.

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

**Rule 1.4**: `#[cfg(test)]` must only appear in two locations——declaring `mod tests` in `lib.rs`, or inline declaring `#[cfg(test)] mod tests;` in the source file under test. Must not be used elsewhere.

```rust
// src/frontend/core/parser/mod.rs or lib.rs
#[cfg(test)]
mod tests;
```

### Module Declaration Conventions

**Rule 2.1**: All test files must have module-level documentation comments `//!` at the top, explaining the specification source (language specification section number + RFC number). If a test doesn't reference any specification section, that code has no specification basis——it should not exist.

```rust
//! Literal Tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \nrt'"\, \x, \u{})
//! RFC-012: F-String Interpolation
```

**Why must specifications be referenced?** Because test expected values come from the specification, not from "the current code's output." If the code changes its output one day and tests are updated accordingly, the tests protect nothing. Only specification-anchored tests can distinguish "intentional breaking change" from "unintentional regression."

**Rule 2.2**: Test module `use` imports must be precise to specific types/functions, no glob imports `use super::*`.

```rust
// ✅ Good——precise imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// ❌ Trash——nobody knows what you're testing
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

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, you should know what's being tested and what's expected. Numeric sequential naming is prohibited.

```rust
// ✅ Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// ❌ Trash——completely unknown what's being tested
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**Rule 3.3**: Helper functions do not need the `test_` prefix; they should use verbs or nouns describing their purpose.

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### Test Structure Conventions (Arrange-Act-Assert)

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

**Rule 4.2**: Simple tests (single call + single assertion) may omit phase comments, but must not exceed 5 lines of logic code. Tests exceeding 5 lines must explicitly mark the three phases.

### Helper Function Conventions

**Rule 5.1**: Setup logic repeated 3 or more times must be extracted into helper functions.

```rust
// ✅ Good——extract common setup
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

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print sufficient context when panicking. Inside test functions (`#[test] fn ...`), direct `unwrap()` is fine——Rust automatically prints line numbers on failure; but when helper functions fail, the line number points to the helper function definition, hiding the call-site context.

```rust
// ✅ Good——helper function prints source content on failure
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// ❌ Trash——on failure you can't see which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, immediately after `use` imports. If shared across multiple test modules, place in `tests/mod.rs` and `pub(crate)` export.

### Assertion Style

**Rule 6.1**: Enum variant matching should prefer `assert!(matches!(...))`, do not use `if let` + `panic!`.

```rust
// ✅ Good
assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(42)));

// ❌ Trash
if let TokenKind::IntLiteral(v) = tokens[0].kind {
    assert_eq!(v, 42);
} else {
    panic!("Expected IntLiteral");
}
```

**Rule 6.2**: Use `assert_eq!` for precise value comparisons, `assert!` for boolean assertions. Do not use `assert!(a == b)` instead of `assert_eq!(a, b)`.

**Rule 6.3**: All assertions must include custom error messages unless the assertion itself fully describes the failure reason.

```rust
// ✅ Good——can quickly locate when assertion fails
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// ✅ Good——assert_eq! prints value diff on failure, no extra message needed
assert_eq!(error_count, 0);

// ❌ Trash——on failure you only know "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: Assertion order must be `assert_eq!(actual, expected)`, actual value first, expected value second.

### Anti-Pattern Checklist

The following are prohibited practices and their alternatives:

| Anti-pattern | Problem | Alternative |
|--------------|---------|-------------|
| Test accommodates code's wrong behavior | Obscures specification deviations, legalizes bugs | Fix code according to specification, keep tests unchanged |
| Back-calculate test expected values from code output | Test becomes "current implementation's tape recorder" | Derive expected values from specification |
| Permanent `#[ignore]` marking | Hides rotting tests | Fix or delete |
| `println!` debug output | Pollutes test output | Use `assert!` for explicit assertions |
| `thread::sleep` in tests | Random failures + slow | Use synchronization mechanisms or mocks |
| Operating real filesystem in tests | Slow and non-repeatable | Use `tempfile` |
| Depending on test execution order | Random failures | Each test has independent setup |
| Single test function exceeds 30 lines of logic | Unreadable | Split test or use helper functions |
| Helper function `unwrap()` without context | Hard to locate | Use `expect("why")` or custom panic (see Rule 5.2) |
| Copy-pasting same setup 3+ times | High modification cost | Extract helper functions |

---

## Integration Test Specification

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

**Rule 7.2**: Each `tests/integration/*.rs` file corresponds to one test theme (compiler backend, code generation, executor, etc.), no mixing.

**Rule 7.3**: Integration tests must test through the project's public API. Do not directly reference `crate::` internal modules in integration tests. Use the `yaoxiang::` public path.

```rust
// ✅ Good——through public API
use yaoxiang::run;

// ❌ Trash——bypasses public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests prefer inline source strings. Only use external fixture files when source exceeds 30 lines (placed in `tests/fixtures/`).

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

**Rule 8.2**: Fixture files must end with `.yx` extension, and filenames describe the test intent.

### E2E Coverage Principles

**Rule 9.1**: Integration tests for each language feature must cover three paths:

| Path | Description |
|------|-------------|
| Happy path | Legal input produces expected output |
| Error path | Illegal input produces clear error messages (not panic) |
| Boundary | Boundary values (empty input, maximum value, nested depth limit) |

**Rule 9.2**: Integration tests must not depend on network, system environment variables, or external services.

---

## Benchmark Test Specification

### Criterion.rs Usage Conventions

**Rule 10.1**: Benchmark tests are unified in the `benches/` directory, with entry file `benches/lib.rs`. Split by test theme.

```
benches/
├── lib.rs              # Entry, defines criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # Cross-language comparison benchmarks
├── parser.rs           # Parser benchmarks
└── codegen.rs          # Code generation benchmarks
```

**Rule 10.2**: Each benchmark function must include module documentation `//!` describing test purpose and measurement metrics.

```rust
//! YaoXiang Interpreter Performance Benchmarks
//!
//! Measurement metrics: Single iteration time (wall time)
//! Baseline: Rust native implementation
```

### Preventing Compiler Optimization

**Rule 11.1**: All benchmark test outputs under measurement must be passed through `criterion::black_box` to prevent compiler optimization from eliminating them.

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

**Rule 11.2**: Benchmark test input data must be `const` or `lazy_static`, not dynamically generated inside the `iter` closure——otherwise you're measuring data generation + code under test combined time.

### Benchmark Grouping and Naming

**Rule 12.1**: Benchmark test naming format is `<module_under_test>_<scenario>`, all lowercase with underscores. Consistent with unit test naming conventions.

**Rule 12.2**: Must use `criterion_group!` to logically group related benchmarks. All benchmarks must not be crammed into one group.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Doc Test Specification

### Use Cases

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code example in their documentation comments. These examples are executed via `cargo test --doc`.

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

**Rule 13.2**: Doc test code examples must compile successfully and assertions must pass. Do not include `ignore` markers unless the example demonstrates a compile-time error.

```rust
/// ```ignore
/// // Demonstrating compile-time error——can ignore
/// let x: int = "string";
/// ```
```

### Coverage Requirements

**Rule 14.1**: Doc tests cover the API happy path only. Boundary cases and error paths are covered by unit tests.

**Rule 14.2**: Code examples in doc tests must be concise——no more than 10 lines. If an example needs longer context, the API design has problems.

---

## Property-Based Test Specification

### Use Cases

**Rule 15.1**: The following scenarios must use property-based tests (proptest or quickcheck) instead of manually writing multiple boundary value cases:

| Scenario | Example |
|----------|---------|
| Parser round-trip | `parse(pretty_print(ast)) == ast` |
| Serialization/deserialization | `deserialize(serialize(data)) == data` |
| Mathematical operation identities | `a + b == b + a` |
| Compiler optimization preserves semantics | `eval(code) == eval(optimize(code))` |

**Rule 15.2**: Property-based tests use `proptest` as the primary framework (already declared in `Cargo.toml`'s `dev-dependencies`).

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

**Rule 16.1**: Each property-based test must have a clear property declaration——write the invariant being verified in comments.

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

**Rule 16.2**: If a property-based test discovers a failure, use proptest's regression mechanism——add the failing input to the `proptest-regressions/` directory, do not manually write a regular test instead.

---

## Coverage Requirements

### New Code Coverage Targets

**Rule 17.1**: Coverage requirements for new code:

| Code Type | Line Coverage | Branch Coverage |
|-----------|---------------|------------------|
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
- [ ] Test naming follows naming conventions
- [ ] Each test file declares the corresponding specification section (Rule 2.1)
- [ ] Test expected values come from specification definitions, not "current code output"
- [ ] No `#[ignore]` marked tests (unless accompanied by explicit issue number comment)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic messages)
- [ ] Commit message uses `:white_check_mark: test:` type
- [ ] **No modification of test expected values because "code behavior doesn't match specification"——fix the code, not the tests**

**Rule 18.2**: Reviewers must reject PRs containing the following issues:

- Only happy path tests, missing error paths
- Tests contain `thread::sleep` or depend on execution order
- Copy-pasted test code exceeds 3 times without extracting helper functions
- Test names don't follow naming conventions
- Permanent `#[ignore]` tests exist
- **Tests accommodate code's wrong behavior** (when code doesn't match specification, modify tests instead of code)
- **Tests don't declare corresponding specification sections** (see Rule 2.1)
- **Test expected values come from code output rather than specification definition** (back-calculated tests are equivalent to no tests)
- Tests only verify "no panic" without asserting specific behavior
- Deleted failing tests that exposed code bugs (instead of fixing code and seeing them turn green)

---

## Appendices

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

# Run single-threaded (investigate concurrency issues)
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
- Assignment operator right associativity
```

### C. New Test File Checklist

When creating new test modules, ensure the following files are included:

```
# Adding tests under src/<module>/ directory
src/<module>/tests/
├── mod.rs          # Module declaration + public helper functions
└── <subject>.rs    # Test file, named after the source file under test

# Adding integration tests under tests/ directory
tests/
├── integration.rs   # Update: add #[path] declaration
└── integration/
    └── <topic>.rs   # New test file
```

### D. References

- [YaoXiang Language Specification](../../design/language-spec.md) —— **Authoritative source for tests**
- [Accepted RFCs](../../design/rfc/accepted/) —— **Authoritative source for design decisions**
- [Rust Testing Book](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Conventions](./commit-convention.md)
- [Project Contributing Guide](./contributing.md)

---

> 💡 **Remember**: Tests don't verify that your code "works"——they verify that your code matches the specification. When the specification changes, tests follow the specification. When code is wrong, fix the code, don't fix the tests. **Code serves the specification; tests guard the specification. The moment tests accommodate code, you lose all protection.**