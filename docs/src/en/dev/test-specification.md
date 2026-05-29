```markdown
---
title: Testing Writing Standards
description: YaoXiang project testing hard rules, defining standards for unit tests, integration tests, benchmarks, doc tests, and property tests
---

# Testing Writing Standards

This document defines the hard rules for writing tests in the YaoXiang project. All contributors must follow these rules; violations will require changes during Code Review.

---

## Table of Contents

- [General Principles](#general-principles)
- [Unit Test Standards](#unit-test-standards)
- [Integration Test Standards](#integration-test-standards)
- [Benchmark Standards](#benchmark-standards)
- [Documentation Test Standards](#documentation-test-standards)
- [Property Test Standards](#property-test-standards)
- [Coverage Requirements](#coverage-requirements)
- [Appendix](#appendix)

---

## General Principles

### Scope of Application

This standard applies to all Rust test code in the YaoXiang project, including:

| Test Type | Location | Framework |
|-----------|----------|------------|
| Unit Tests | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| Integration Tests | `tests/` | `#[test]` |
| Benchmarks | `benches/` | Criterion.rs |
| Doc Tests | API doc comments | `cargo test --doc` |
| Property Tests | Any test location | proptest / quickcheck |

### Core Principles

**Principle 0: The specification is the authoritative source for tests, not the code.** This is the most important principle in this document. Tests verify that code conforms to the specification, not that code "works with the current implementation." When a test finds that code behavior differs from the specification, **fix the code, never fix the test**.

Specification files are located at:
- `docs/src/design/language-spec.md` —— Core language specification
- `docs/src/design/rfc/accepted/` —— Accepted RFC design documents

Each test file must declare the corresponding specification section at the top (see Rule 2.1). Any developer should be able to take the specification document and compare it against tests to verify implementation correctness. Conversely—if a piece of code has no corresponding specification description, it should not exist, let alone be tested.

```rust
// 🟢 Good——Test directly references the specification, verifies code follows the spec
//! Literal tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String Interpolation

#[test]
fn test_decimal_literal_parsing() {
    // Specification §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 Trash——Test accommodates current code's implementation behavior, not verifies spec
#[test]
fn test_literal_1() {
    // Don't know which section of the spec this corresponds to
    // If parse_literal returns a wrong value, this test will "pass"
    // because it only verifies the function doesn't panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and find that code behavior differs from the spec. You have two choices:
| Wrong Approach | Correct Approach |
|----------------|-------------------|
| Modify the test to "pass" | Modify the code to match the spec |
| Add `#[ignore]` to the test | Fix the code implementation immediately |
| Add special conditional branches in the test to accommodate code | Remove branches, let the test expose the problem directly |

Remember: **Red light = code is wrong, not the test.** (Unless your test itself has a bug, which is a different matter.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of the code under test by reading the tests, without needing additional comments or external documentation.

```rust
// 🟢 Good——Test name says what is being tested and what is expected
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 Trash——No one knows what this is testing
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for random failures.** Tests must be repeatable in any environment. Tests that depend on random numbers, system time, or thread scheduling order must use fixed seeds or mocks instead.

**Principle 3: One test, one thing.** If a test name needs "and" to connect multiple behaviors, split into multiple tests.

```rust
// 🟢 Good——Each test verifies only one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Trash——One test stuffed with too much unrelated content
#[test]
fn test_parser() {
    // Test tokenize, test parse, test typecheck, test codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring internal implementation should not cause tests to fail. If changing one line of implementation code causes 10 tests to fail, your tests are written incorrectly.

But there is a key distinction here: **the definition of "behavior" comes from the specification, not from the current code's behavior.** If the code changes behavior (i.e., new behavior that doesn't match the spec), tests must fail. If you can't do this, your tests are "tests that accommodate the code"—they let bugs pass through.

```
Specification (language-spec.md / RFC)  ──defines──►  Expected Behavior  ──drives──►  Tests
                                               │
Current Code  ──implements──►  Actual Behavior  ──compared──►  Test Results

If actual behavior ≠ expected behavior:
  Test must fail (red light)  ──►  Fix code  ──►  Test passes (green light)
  
If actual behavior = expected behavior (but implementation is poor):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is the meaning of Principle 4
```

**Principle 5: No fallback/compatibility/situational test code.** The test environment is one you have full control over. If you need `#[cfg(not(ci))]` to skip a test, that test's design has a fundamental problem.

### Terminology Definitions

| Term | Definition |
|------|------------|
| Unit Test | Tests the behavior of a single function or module, without external system dependencies |
| Integration Test | Tests multiple modules working together, through public API or CLI entry point |
| Benchmark | Measures code performance, detects performance regressions |
| Doc Test | Executable code examples embedded in documentation comments |
| Property Test | Tests that verify invariants using randomly generated inputs |

### Relation to Commit Standards

All test-related commits must use the `:white_check_mark: test:` type, as per the [commit standards](./commit-convention.md).

```
:white_check_mark: test(parser): Add Pratt parser infix expression tests
:white_check_mark: test(codegen): Complete switch statement IR generation tests
```

---

## Unit Test Standards

### File Organization

**Rule 1.1**: The `tests/` directory for unit tests must be at the **same level** as the `mod.rs` of the module being tested. `tests/` does not aggregate upward or cross-level aggregate.

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
└── tests/              # parser module-level tests (does not include pratt submodule content)
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

Key judgment criterion: **Where you put `tests/`, the `mod.rs` at that level must declare `#[cfg(test)] mod tests;`.**

#### Single-File Module vs Directory Module Test Placement Rules

**Core difference**: The module's organization form determines test placement.

| Module Type | Judgment Basis | Test Location | Example |
|-------------|----------------|----------------|---------|
| **Directory Module** | Has independent directory and `mod.rs` | `tests/` under that directory | `inference/tests/` |
| **Single-File Module** | Only has `.rs` file, no independent directory | Parent module's `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

**Detailed Explanation**:

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
│   ├── overload.rs                 # overload.rs tests (single-file module tests go here)
│   ├── type_eval.rs                # type_eval.rs tests
│   ├── dead_code.rs                # dead_code.rs tests
│   ├── spawn_placement.rs          # spawn_placement.rs tests
│   ├── signature.rs                # signature.rs tests
│   └── types.rs                    # types.rs tests
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
│       ├── expressions.rs          # expressions.rs tests
│       ├── statements.rs           # statements.rs tests
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
        ├── solver.rs               # solver.rs tests
        ├── impl_check.rs           # impl_check.rs tests
        └── ...
```

**Why do single-file module tests go in the parent `tests/`?**

Because single-file modules (like `overload.rs`) don't have their own `mod.rs`, they cannot declare `#[cfg(test)] mod tests;`. According to Rust's module system, test files must be declared by some `mod.rs` to compile. Therefore, single-file module tests can only be declared by the parent module's `mod.rs`, placed in the parent's `tests/` directory.

**Decision Flow**:

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
    └── Check if there's an independent directory and mod.rs
```

**Common Mistakes**:

```
# ❌ Wrong 1: Creating an independent tests/ directory for a single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ Should not create a directory for a single-file module
    └── tests/
        └── overload.rs

# ❌ Wrong 2: Declaring #[cfg(test)] mod tests; inside a single-file module
# overload.rs
#[cfg(test)]                        # ❌ Single-file module cannot declare like this
mod tests;                          # because there's no overload/tests/ directory

# ✅ Correct approach: Tests go in parent tests/
src/frontend/core/typecheck/
├── overload.rs                     # source file
└── tests/
    └── overload.rs                 # test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern——Don't do this:**

```
# ❌ Wrong: Submodule tests centralized to parent
src/frontend/core/types/
├── mod.rs              # Should only declare base and computation
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ Parent tests/ contains submodule tests
    ├── mod.rs          # ❌ Forced to declare mod base; mod computation;
    ├── base/           # ❌ This should go in base/tests/
    │   └── var.rs
    └── computation/    # ❌ This should go in computation/tests/
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

**Why can't tests aggregate upward?** Because Rust's module system requires `#[cfg(test)] mod tests;` to determine test file compilation at the declaration site. If `types/mod.rs` declares `mod tests;`, then the contents of `types/tests/` are private to the `types` module—it should not cross into `base` or `computation`. Each module's tests should be internal implementation details of that module, not the parent module. This rule also applies to module refactoring: when you split `types` into `base` and `computation`, tests should follow the split modules, not stay in place. **Test directories don't mirror source structure; they follow module boundaries.**

**Rule 1.2**: `tests/mod.rs` is only responsible for module declaration and re-export, no test functions go here.

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

**Rule 1.3**: Each test file corresponds to only one source file. Multiple source module tests are not allowed in the same file.

**Rule 1.4**: `#[cfg(test)]` must only appear in two places—declaring `mod tests` in `lib.rs`, or inline declaring `#[cfg(test)] mod tests;` in the source file under test. Must not be used elsewhere.

```rust
// src/frontend/core/parser/mod.rs or lib.rs
#[cfg(test)]
mod tests;
```

### Module Declaration Standards

**Rule 2.1**: All test files must have module-level doc comments `//!` at the top, explaining the specification source coverage (language specification section number + RFC number). If a test doesn't reference any specification section, that code has no specification basis—it shouldn't exist.

```rust
//! Literal tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String Interpolation
```

**Why must specifications be referenced?** Because test expectations come from specifications, not from "current code output." If code changes output and tests are updated accordingly, the tests protect nothing. Only specification-anchored tests can distinguish "intentional breaking changes" from "unintentional regressions."

**Rule 2.2**: Test module `use` imports must be precise to specific types/functions; glob imports `use super::*` are prohibited.

```rust
// 🟢 Good——Precise imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Trash——Others don't know what you're testing
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

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, you should know what is being tested and what is expected. Numeric sequence naming is prohibited.

```rust
// 🟢 Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Trash——Completely unknown what is being tested
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

**Rule 4.2**: Simple tests (single call + single assertion) may omit phase comments but must not exceed 5 lines of logic code. Tests exceeding 5 lines must explicitly mark three phases.

### Helper Function Standards

**Rule 5.1**: Setup logic repeated 3 or more times must be extracted into helper functions.

```rust
// 🟢 Good——Extract common setup
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

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print sufficient context on panic. In test function bodies (`#[test] fn ...`), direct `unwrap()` is fine—Rust automatically prints line number on failure; but in helper functions, line number points to the helper function definition, not the call site.

```rust
// 🟢 Good——Print source content when helper fails
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Trash——Can't see which source file caused the problem on failure
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of test files, right after `use` imports. If shared across multiple test modules, place in `tests/mod.rs` and `pub(crate)` export.

### Assertion Style

**Rule 6.1**: Enum variant matching should use `assert!(matches!(...))`, not `if let` + `panic!`.

```rust
// 🟢 Good
assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(42)));

// 🔴 Trash
if let TokenKind::IntLiteral(v) = tokens[0].kind {
    assert_eq!(v, 42);
} else {
    panic!("Expected IntLiteral");
}
```

**Rule 6.2**: Precise value comparison uses `assert_eq!`, boolean assertions use `assert!`. Using `assert!(a == b)` to replace `assert_eq!(a, b)` is prohibited.

**Rule 6.3**: All assertions must include custom error messages unless the assertion itself fully describes the failure reason.

```rust
// 🟢 Good——Can quickly locate on failure
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 Good——assert_eq! auto-prints value diff on failure, no extra message needed
assert_eq!(error_count, 0);

// 🔴 Trash——Only knows "assertion failed" on failure
assert!(state.infix_info().is_some());
```

**Rule 6.4**: Assertion order must be `assert_eq!(actual, expected)`, actual value first, expected value second.

### Anti-pattern Checklist

The following practices are prohibited with their alternatives:

| Anti-pattern | Problem | Alternative |
|--------------|---------|-------------|
| Tests accommodate code's wrong behavior | Hides spec deviation, legitimizes bugs | Fix code according to spec, keep tests unchanged |
| Reverse-engineering test expectations from code output | Tests become "recorders of current implementation" | Derive expectations from specification |
| Permanent `#[ignore]` markers | Hides rotting tests | Fix or delete |
| `println!` debug output | Pollutes test output | Use `assert!` for explicit assertions |
| `thread::sleep` | Random failures + slow | Use synchronization mechanisms or mocks |
| Operating real filesystem in tests | Slow and non-repeatable | Use `tempfile` |
| Depending on test execution order | Random failures | Each test has independent setup |
| Single test function exceeding 30 lines of logic | Unreadable | Split tests or use helper functions |
| `unwrap()` in helper functions without context | Hard to locate | Use `expect("why")` or custom panic (see Rule 5.2) |
| copy-paste same setup 3+ times | High modification cost | Extract helper functions |

---

## Integration Test Standards

### Test Organization

**Rule 7.1**: Integration tests go in the project root `tests/` directory. Entry file `tests/integration.rs` uses `#[path]` attribute to include submodules.

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

**Rule 7.3**: Integration tests must test through the project's public API. Directly referencing `crate::` internal modules in integration tests is prohibited. Use `yaoxiang::` public path.

```rust
// 🟢 Good——Through public API
use yaoxiang::run;

// 🔴 Trash——Bypasses public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests prefer inline source strings. External fixture files are only used when source exceeds 30 lines (placed in `tests/fixtures/`).

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

**Rule 8.2**: Fixture files must end with `.yx` extension, and file names describe the test intent.

### E2E Coverage Principle

**Rule 9.1**: Integration tests for each language feature must cover three paths:

| Path | Description |
|------|-------------|
| Happy path | Valid input produces expected output |
| Error path | Invalid input produces clear error messages (not panic) |
| Boundary | Boundary values (empty input, max value, nesting depth limit) |

**Rule 9.2**: Integration tests must not depend on network, system environment variables, or external services.

---

## Benchmark Standards

### Criterion.rs Usage Standards

**Rule 10.1**: Benchmarks are uniformly placed in `benches/` directory, with entry file `benches/lib.rs`. Divided into files by test topic.

```
benches/
├── lib.rs              # Entry, defines criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # Cross-language comparison benchmarks
├── parser.rs           # Parser benchmarks
└── codegen.rs          # Code generation benchmarks
```

**Rule 10.2**: Each benchmark function must include module doc comments `//!` describing test purpose and measurement metrics.

```rust
//! YaoXiang interpreter performance benchmarks
//!
//! Measurement metric: Single iteration elapsed time (wall time)
//! Baseline: Rust native implementation
```

### Preventing Compiler Optimization

**Rule 11.1**: All benchmark outputs under test must be passed through `criterion::black_box` to prevent compiler optimization elimination.

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

**Rule 11.2**: Benchmark input data must be `const` or `lazy_static`, not dynamically generated inside `iter` closures—otherwise you're measuring data generation + code under test combined time.

### Benchmark Grouping and Naming

**Rule 12.1**: Benchmark naming format is `<module_under_test>_<scenario>`, all lowercase with underscores. Consistent with unit test naming rules.

**Rule 12.2**: Related benchmarks must use `criterion_group!` for logical grouping. All benchmarks in one group is prohibited.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Documentation Test Standards

### Use Cases

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code example in doc comments. These examples are executed by `cargo test --doc`.

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

**Rule 13.2**: Doc test code examples must compile successfully and assertions must pass. Examples with `ignore` markers are not allowed unless the example demonstrates a compile-time error.

```rust
/// ```ignore
/// // Demonstrating compile-time error——can ignore
/// let x: int = "string";
/// ```
```

### Coverage Requirements

**Rule 14.1**: Doc tests cover API happy path only. Boundary cases and error paths are covered by unit tests.

**Rule 14.2**: Example code in doc tests must be concise—no more than 10 lines. If an example needs longer context, the API design has problems.

---

## Property Test Standards

### Use Cases

**Rule 15.1**: The following scenarios must use property tests (proptest or quickcheck) instead of hand-written multiple boundary value cases:

| Scenario | Example |
|----------|---------|
| Parser round-trip | `parse(pretty_print(ast)) == ast` |
| Serialization/deserialization | `deserialize(serialize(data)) == data` |
| Mathematical identity | `a + b == b + a` |
| Compiler optimization doesn't change semantics | `eval(code) == eval(optimize(code))` |

**Rule 15.2**: Property tests use `proptest` as the primary property testing framework (already declared in `Cargo.toml` `dev-dependencies`).

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

**Rule 16.1**: Each property test must have a clear property declaration—the invariant being verified written in comments.

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

**Rule 16.2**: If a property test fails, use proptest's regression mechanism—add the failing input to `proptest-regressions/` directory, don't manually write a regular test instead.

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

**Rule 17.2**: All error handling paths (all `Err` branches) must be 100% covered. Error messages visible to users must be verified by tests.

### PR Review Checklist

**Rule 18.1**: Before submitting PR, author must self-check:

- [ ] `cargo test` all pass
- [ ] `cargo test --doc` all pass
- [ ] `cargo bench` no performance regression (if hot path changes)
- [ ] New code meets coverage targets
- [ ] Test naming follows naming conventions
- [ ] Each test file declares corresponding specification section (Rule 2.1)
- [ ] Test expectations come from specification definitions, not "current code output"
- [ ] No `#[ignore]` markers (unless with explicit issue number comment)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic messages)
- [ ] Commit message uses `:white_check_mark: test:` type
- [ ] **No modification of test expectations due to "code behavior differs from specification"—modify code, not tests**

**Rule 18.2**: Reviewer must reject PRs containing:

- Only happy path tests, missing error paths
- `thread::sleep` or dependent execution order in tests
- Copy-pasted test code exceeding 3 times without extracting helper functions
- Test names not following naming conventions
- Permanent `#[ignore]` tests
- **Tests accommodate code's wrong behavior** (when code differs from spec, modifying tests instead of code)
- **Tests don't declare corresponding specification section** (see Rule 2.1)
- **Test expectations from code output rather than specification** (reverse-engineered tests equal no tests)
- Tests only verify "doesn't panic" without asserting specific behavior
- Deleted failing tests that exposed code bugs (should fix code first, then see it turn green)

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

Scenarios covered:
- Arithmetic operator precedence (+, -, *, /, %)
- Comparison operator chaining (1 < x < 10)
- Logical operator short-circuit
- Assignment operator right associativity
```

### C. New Test File Checklist

When creating new test modules, ensure these files are included:

```
# Adding tests under src/<module>/
src/<module>/tests/
├── mod.rs          # Module declaration + public helper functions
└── <subject>.rs    # Test file, named corresponding to source file under test

# Adding integration tests under tests/
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

> 💡 **Remember**: Tests don't verify your code "works"—they verify your code conforms to the specification. When the specification changes, tests follow. When code is wrong, fix the code, don't fix the tests. **Code serves the specification, tests guard the specification. The moment tests accommodate the code, you lose all protection.**
```