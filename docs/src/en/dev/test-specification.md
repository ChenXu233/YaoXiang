---
title: "Test Writing Standards"
description: YaoXiang project test writing hard rules, defining standards for unit tests, integration tests, benchmark tests, documentation tests, and property tests
---

# Test Writing Standards

This document defines the hard rules for test writing in the YaoXiang project. All contributors must follow these rules; violators will be required to make changes during Code Review.

---

## Table of Contents

- [General Principles](#general-principles)
- [Unit Test Standards](#unit-test-standards)
- [Integration Test Standards](#integration-test-standards)
- [Benchmark Test Standards](#benchmark-test-standards)
- [Documentation Test Standards](#documentation-test-standards)
- [Property Test Standards](#property-test-standards)
- [Coverage Requirements](#coverage-requirements)
- [Appendices](#appendices)

---

## General Principles

### Scope of Application

These standards apply to all Rust test code in the YaoXiang project, including:

| Test Type | Location | Framework |
|-----------|----------|-----------|
| Unit tests | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| Integration tests | `tests/` | `#[test]` |
| Benchmark tests | `benches/` | Criterion.rs |
| Documentation tests | API documentation comments | `cargo test --doc` |
| Property tests | Any test location | proptest / quickcheck |

### Core Principles

**Principle 0: The specification is the authoritative source for tests, not the code.** This is the most important principle in this document. Tests verify that code conforms to the specification, not that code "passes with the current implementation." When a test finds that code behavior differs from the specification, **fix the code, never fix the test**.

Specification documents are located at:
- `docs/src/design/language-spec.md` —— Language core specification
- `docs/src/design/rfc/accepted/` —— Accepted RFC design documents

Each test file must declare the corresponding specification section at the top (see Rule 2.1). Any developer should be able to take the specification document and cross-reference it with the tests to verify correctness of the implementation. Conversely—if a piece of code has no corresponding specification description, it should not exist, let alone be tested.

```rust
// 🟢 Good — test directly references the spec, verifying code follows the spec
//! Literal tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integer Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation

#[test]
fn test_decimal_literal_parsing() {
    // Specification §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 Trash — test accommodates the current code's implementation behavior, rather than verifying the spec
#[test]
fn test_literal_1() {
    // Can't tell which section of the spec this corresponds to
    // If parse_literal returns a wrong value, this test will "pass"
    // Because it only verifies the function doesn't panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and discover that the code behavior differs from the spec. You have two choices:

| Wrong Approach | Correct Approach |
|----------------|------------------|
| Modify the test to make it "pass" | Modify the code to match the spec |
| Add `#[ignore]` to the test | Fix the code implementation immediately |
| Add special conditional branches in the test to accommodate the code | Remove branches, let the test expose the problem directly |

Remember: **Red light = the code is wrong, not the test.** (Unless your test itself has a bug, which is a different matter.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of the code under test by reading the tests alone, without additional comments or external documentation.

```rust
// 🟢 Good — test name states what is being tested and what is expected
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 Trash — no one knows what this is testing
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for flaky tests.** Tests must be repeatable in any environment. Tests that depend on random numbers, system time, or thread scheduling order must use fixed seeds or mock implementations.

**Principle 3: One test, one thing.** If a test name needs to connect multiple behaviors with "and", split it into multiple tests.

```rust
// 🟢 Good — each test verifies one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Trash — one test crammed with too much unrelated content
#[test]
fn test_parser() {
    // Test tokenize, test parse, test typecheck, test codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring internal implementation should not cause tests to fail. If you changed one line of implementation and 10 tests broke, your tests are written incorrectly.

But there is a critical distinction here: **the definition of "behavior" comes from the specification, not from the current code's behavior.** If the code changed behavior (i.e., new behavior that doesn't match the spec), tests must fail. If you can't achieve this, your tests are "code-accommodating tests"—they let bugs march right in.

```
Specification (language-spec.md / RFC)  ──defines──►  Expected behavior  ──drives──►  Tests
                                           │
Current code  ──implements──►  Actual behavior  ──compare──►  Test results

If actual behavior ≠ expected behavior:
  Test must fail (red light)  ──►  Fix code  ──►  Test passes (green light)
  
If actual behavior = expected behavior (but implementation is poor):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is the meaning of Principle 4
```

**Principle 5: Do not write fallback/compatibility/feature-flagged test code.** The test environment is one you have full control over. If you need `#[cfg(not(ci))]` to skip a test, that indicates a fundamental problem with the test design.

### Terminology Definitions

| Term | Definition |
|------|------------|
| Unit test | Tests the behavior of a single function or module without depending on external systems |
| Integration test | Tests collaboration of multiple modules through public APIs or command-line entry points |
| Benchmark test | Measures code performance and detects performance regressions |
| Documentation test | Executable code examples embedded in documentation comments |
| Property test | Tests that verify invariants using random inputs |

### Relationship with Commit Standards

All test-related commits must use the `:white_check_mark: test:` type, referring to the [Commit Standards](./commit-convention.md).

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
└── tests/              # parser module-level tests (does not include pratt submodule contents)
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

Key judgment criterion: **When `tests/` is placed in which directory, that directory's `mod.rs` must declare it with `#[cfg(test)] mod tests;`.**

#### Single-File Module vs. Directory Module Test Placement Rules

**Core distinction**: The module's organization form determines where tests are placed.

| Module Type | Judgment Basis | Test Location | Example |
|-------------|----------------|---------------|---------|
| **Directory module** | Has its own directory and `mod.rs` | `tests/` under that directory | `inference/tests/` |
| **Single-file module** | Only has `.rs` file, no independent directory | `tests/` of parent module | `overload.rs` → `typecheck/tests/overload.rs` |

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

**Why do single-file module tests go into the parent's `tests/`?**

Because single-file modules (like `overload.rs`) don't have their own `mod.rs`, they cannot declare `#[cfg(test)] mod tests;`. According to Rust's module system, test files must be declared by a `mod.rs` to compile. Therefore, single-file module tests can only be declared by the parent module's `mod.rs` and placed in the parent's `tests/` directory.

**Decision flow**:

```
Encountering a module, where do the tests go?
│
├── Is this module a directory (has mod.rs)?
│   └── Yes → Create tests/ under that directory, declared by that directory's mod.rs
│
├── Is this module a single file (only .rs)?
│   └── Yes → Tests go into parent's tests/ directory, declared by parent's mod.rs
│
└── Not sure?
    └── Check if there's an independent directory and mod.rs
```

**Common mistakes**:

```
# ❌ Wrong 1: Creating an independent tests/ directory for a single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ Should not create a directory for a single-file module
    └── tests/
        └── overload.rs

# ❌ Wrong 2: Declaring #[cfg(test)] mod tests; inside a single-file module
# overload.rs
#[cfg(test)]                        # ❌ Single-file modules cannot declare this way
mod tests;                          # because there's no overload/tests/ directory

# ✅ Correct: Tests go into parent's tests/
src/frontend/core/typecheck/
├── overload.rs                     # source file
└── tests/
    └── overload.rs                 # test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern — Don't do this:**

```
# ❌ Wrong: Submodule tests centralized to parent
src/frontend/core/types/
├── mod.rs              # should only declare base and computation
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ Parent's tests/ contains submodule tests
    ├── mod.rs          # ❌ forced to declare mod base; mod computation;
    ├── base/           # ❌ this should go in base/tests/
    │   └── var.rs
    └── computation/    # ❌ this should go in computation/tests/
        └── ...
```

```
# ✅ Correct approach: Each module's tests are independent
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

**Why can't tests aggregate upward?** Because Rust's module system requires `#[cfg(test)] mod tests;` to decide test file compilation at the declaration site. If `types/mod.rs` declares `mod tests;`, then the contents of `types/tests/` become private implementation details of the `types` module—they should not cross into `base` or `computation`'s territory. Each module's tests should be that module's internal implementation details, not the parent module's. This rule also applies to module refactoring: when you split `types` into `base` and `computation`, the tests should also follow the split modules, not stay where they are. **Test directories don't mirror source structure; they follow module boundaries.**

**Rule 1.2**: `tests/mod.rs` is only responsible for module declaration and re-export; test functions do not go here.

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

**Rule 1.3**: Each test file corresponds to only one source file. Tests for multiple source modules are not allowed in the same file.

**Rule 1.4**: `#[cfg(test)]` may only appear in two places—in `lib.rs` declaring `mod tests`, or inline declaration `#[cfg(test)] mod tests;` inside the source file being tested. It must not be used elsewhere.

```rust
// src/frontend/core/parser/mod.rs or lib.rs
#[cfg(test)]
mod tests;
```

### Module Declaration Standards

**Rule 2.1**: All test files must have module-level documentation comments `//!` at the top, explaining the specification source coverage (language specification section numbers + RFC numbers). If a test doesn't reference any specification section, it means this code has no specification basis—it should not exist.

```rust
//! Literal tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integer Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation
```

**Why must we reference the specification?** Because test expected values come from the specification, not from "the current code's output." If the code changes output one day and the test is updated accordingly, the test protects nothing. Only specification-anchored tests can distinguish between "intentional breaking change" and "unintentional regression."

**Rule 2.2**: Test module `use` imports must be specific to the exact type/function; glob imports `use super::*` are prohibited.

```rust
// 🟢 Good — precise imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Trash — no one knows what you're testing
use super::*;
```

### Naming Standards

**Rule 3.1**: Test function naming format is `test_<what>_<scenario>`, all lowercase with underscore separation.

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, you should know what is being tested and what is expected. Numeric ordinal naming is prohibited.

```rust
// 🟢 Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Trash — completely unclear what is being tested
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**Rule 3.3**: Helper functions do not need the `test_` prefix; they should use verbs or nouns describing their purpose.

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### Test Structure Standards (Arrange-Act-Assert)

**Rule 4.1**: Each test function must follow a three-phase structure: Arrange → Act → Assert, with blank lines separating the three phases.

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
// 🟢 Good — extract common setup
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

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print sufficient context on panic. `unwrap()` can be used directly in test function bodies (`#[test] fn ...`)—on failure, Rust automatically prints the line number; but when failure occurs inside a helper function, the line number points to the helper function definition, not the call site context.

```rust
// 🟢 Good — helper function failure prints source content
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Trash — on failure you can't tell which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, right after `use` imports. If shared across multiple test modules, place them in `tests/mod.rs` and `pub(crate)` export them.

### Assertion Style

**Rule 6.1**: Enum variant matching should prioritize `assert!(matches!(...))`, not use `if let` + `panic!`.

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

**Rule 6.2**: Use `assert_eq!` for precise value comparison, `assert!` for boolean assertions. Do not use `assert!(a == b)` as a substitute for `assert_eq!(a, b)`.

**Rule 6.3**: All assertions must include custom error messages unless the assertion itself already fully describes the failure reason.

```rust
// 🟢 Good — when assertion fails, can quickly locate the issue
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 Good — assert_eq! failure automatically prints value differences, no extra message needed
assert_eq!(error_count, 0);

// 🔴 Trash — failure only shows "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: Assertion order must be `assert_eq!(actual, expected)`, actual value first, expected value second.

### Anti-Pattern Checklist

The following are prohibited practices and their alternatives:

| Anti-Pattern | Problem | Alternative |
|--------------|---------|-------------|
| Tests accommodate code's wrong behavior | Obscures spec deviations, legitimizes bugs | Fix code according to spec, keep tests unchanged |
| Back-calculate test expected values from code output | Tests become "recorders of current implementation" | Derive expected values from the spec |
| Permanent `#[ignore]` markers | Hides rotting tests | Fix or delete |
| `println!` debug output | pollutes test output | Use `assert!` for clear assertions |
| `thread::sleep` | Random failures + slow | Use synchronization mechanisms or mock |
| Operating real filesystem in tests | Slow and non-repeatable | Use `tempfile` |
| Depending on test execution order | Random failures | Each test has independent setup |
| Single test function exceeds 30 lines of logic | Unreadable | Split tests or use helper functions |
| `unwrap()` in helper functions without context | Hard to locate | Use `expect("why")` or custom panic (see Rule 5.2) |
| Copy-paste same setup 3+ times | High modification cost | Extract helper functions |

---

## Integration Test Standards

### Test Organization

**Rule 7.1**: Integration tests are placed in the `tests/` directory at the project root. The entry file `tests/integration.rs` uses the `#[path]` attribute to include submodules.

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**Rule 7.2**: Each `tests/integration/*.rs` file corresponds to one test topic (compiler backend, code generation, executor, etc.), and must not be mixed.

**Rule 7.3**: Integration tests must test through the project's public API. Direct references to `crate::` internal modules are prohibited in integration tests. Use the `yaoxiang::` public path.

```rust
// 🟢 Good — through public API
use yaoxiang::run;

// 🔴 Trash — bypassing public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests prefer inline source code strings. Only when source code exceeds 30 lines should external fixture files be used (placed in `tests/fixtures/`).

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

**Rule 8.2**: Fixture files must use the `.yx` extension, and filenames should describe the test intent.

### E2E Coverage Principle

**Rule 9.1**: Integration tests for each language feature must cover three paths:

| Path | Description |
|------|-------------|
| Happy path | Legal input produces expected output |
| Error path | Illegal input produces clear error messages (not panic) |
| Boundary | Boundary values (empty input, max value, nested depth limit) |

**Rule 9.2**: Integration tests must not depend on network, system environment variables, or external services.

---

## Benchmark Test Standards

### Criterion.rs Usage Standards

**Rule 10.1**: Benchmark tests are uniformly placed in the `benches/` directory, with the entry file being `benches/lib.rs`. Organized by test topic into files.

```
benches/
├── lib.rs              # entry, defines criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # cross-language comparison benchmarks
├── parser.rs           # parser benchmarks
└── codegen.rs          # code generation benchmarks
```

**Rule 10.2**: Each benchmark function must include module-level documentation comments `//!` explaining the test purpose and measurement metrics.

```rust
//! YaoXiang interpreter performance benchmarks
//!
//! Measurement metric: single iteration elapsed time (wall time)
//! Baseline: Rust native implementation
```

### Preventing Compiler Optimization

**Rule 11.1**: All benchmark test outputs under measurement must be passed through `criterion::black_box` to prevent compiler optimization elimination.

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

**Rule 11.2**: Benchmark test input data must be `const` or `lazy_static`, not dynamically generated inside the `iter` closure—otherwise you're measuring the total time of data generation + measured logic.

### Benchmark Grouping and Naming

**Rule 12.1**: Benchmark test naming format is `<tested_module>_<scenario>`, all lowercase with underscore separation. Consistent with unit test naming rules.

**Rule 12.2**: `criterion_group!` must be used to logically group related benchmarks. All benchmarks must not be crammed into one group.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Documentation Test Standards

### Use Cases

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code example in documentation comments. This example is executed via `cargo test --doc`.

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

**Rule 13.2**: Documentation test code examples must compile successfully and assertions must pass. Examples with `ignore` markers are not allowed unless the example demonstrates a compile-time error.

```rust
/// ```ignore
/// // Demonstrating compile-time error — ignore is allowed
/// let x: int = "string";
/// ```
```

### Coverage Requirements

**Rule 14.1**: Documentation tests cover the API happy path only. Boundary cases and error paths are covered by unit tests.

**Rule 14.2**: Example code in documentation tests must be concise—no more than 10 lines. If an example needs longer context, the API design has problems.

---

## Property Test Standards

### Use Cases

**Rule 15.1**: The following scenarios must use property tests (proptest or quickcheck) rather than manually writing multiple boundary value cases:

| Scenario | Example |
|----------|---------|
| Parser round-trip | `parse(pretty_print(ast)) == ast` |
| Serialization/deserialization | `deserialize(serialize(data)) == data` |
| Mathematical identity | `a + b == b + a` |
| Compiler optimization preserves semantics | `eval(code) == eval(optimize(code))` |

**Rule 15.2**: Property tests use `proptest` as the primary property testing framework (already declared in `Cargo.toml` under `dev-dependencies`).

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

**Rule 16.1**: Each property test must have a clear property declaration—a comment explaining the invariant being verified.

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

**Rule 16.2**: If a property test discovers a failure, the regression mechanism of `proptest` must be used—add the failing input to the `proptest-regressions/` directory, do not manually write a regular test instead.

---

## Coverage Requirements

### New Code Coverage Goals

**Rule 17.1**: Coverage requirements for new code:

| Code Type | Line Coverage | Branch Coverage |
|-----------|---------------|-----------------|
| Core compiler modules (frontend/middle/backends) | ≥ 85% | ≥ 80% |
| Utility/helper modules (util) | ≥ 75% | ≥ 70% |
| Runtime modules (vm/runtime) | ≥ 80% | ≥ 75% |
| Standard library (std) | ≥ 75% | ≥ 70% |
| Error handling and diagnostics | ≥ 90% | ≥ 85% |

**Rule 17.2**: Error handling paths (all `Err` branches) must be 100% covered. Error messages visible to users must be verified by tests.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, the author must self-check the following items:

- [ ] `cargo test` passes all tests
- [ ] `cargo test --doc` passes all tests
- [ ] `cargo bench` shows no performance regression (if hot path changes are involved)
- [ ] New code meets coverage goals
- [ ] Test naming follows naming standards
- [ ] Each test file declares the corresponding specification section (Rule 2.1)
- [ ] Test expected values come from specification definitions, not "current code output"
- [ ] No `#[ignore]` marked tests (unless with explicit issue number comment)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic messages)
- [ ] Commit message uses `:white_check_mark: test:` type
- [ ] **No modification of test expected values due to "code behavior differing from spec"—fix the code, not the test**

**Rule 18.2**: Reviewers must reject PRs containing the following issues:

- Only happy path tests, missing error paths
- Tests contain `thread::sleep` or depend on execution order
- Copied-pasted test code more than 3 times without extracting helper functions
- Test names don't follow naming standards
- Permanent `#[ignore]` tests exist
- **Tests accommodate code's wrong behavior** (when code doesn't match spec, modify tests instead of code)
- **Tests don't declare corresponding specification sections** (see Rule 2.1)
- **Test expected values come from code output instead of specification definitions** (back-calculated tests equal no testing)
- Tests only verify "doesn't panic" without asserting specific behavior
- Deleted failing tests that exposed code bugs (rather than fixing the code and then seeing them turn green)

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

# Run documentation tests only
cargo test --doc

# Run specific test (filter by name)
cargo test test_parse_expr

# Run benchmark tests
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
- Assignment operator right associativity
```

### C. New Test File Checklist

When creating a new test module, ensure the following files are included:

```
# Add tests under src/<module>/ directory
src/<module>/tests/
├── mod.rs          # module declaration + public helper functions
└── <subject>.rs    # test file, named corresponding to the source file under test

# Add integration tests under tests/ directory
tests/
├── integration.rs   # Update: add #[path] declaration
└── integration/
    └── <topic>.rs   # new test file
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