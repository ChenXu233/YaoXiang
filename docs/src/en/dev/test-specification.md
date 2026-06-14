---
title: "Test Writing Specification"
description: YaoXiang project test writing hard specification, defining the standards for writing unit tests, integration tests, benchmark tests, documentation tests, and property tests
---

# Test Writing Specification

This document defines the hard specification for writing tests in the YaoXiang project. All contributors must comply with the following rules; violators will be required to modify their code in Code Review.

---

## Table of Contents

- [General Principles](#general-principles)
- [Unit Test Specification](#unit-test-specification)
- [Integration Test Specification](#integration-test-specification)
- [Benchmark Test Specification](#benchmark-test-specification)
- [Documentation Test Specification](#documentation-test-specification)
- [Property Test Specification](#property-test-specification)
- [Coverage Requirements](#coverage-requirements)
- [Appendix](#appendix)

---

## General Principles

### Scope

This specification applies to all Rust test code in the YaoXiang project, including:

| Test Type | Location | Framework |
|-----------|----------|-----------|
| Unit test | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| Integration test | `tests/` | `#[test]` |
| Benchmark test | `benches/` | Criterion.rs |
| Documentation test | API documentation comments | `cargo test --doc` |
| Property test | Anywhere tests are placed | proptest / quickcheck |

### Core Principles

**Principle 0: The authoritative source for tests is the specification, not the code.** This is the most important principle of this document. Tests verify whether the code conforms to the specification, not whether the code "runs correctly under the current implementation." When a test discovers a discrepancy between code behavior and the specification, **fix the code, never fix the test**.

The specification files are located at:
- `docs/src/design/language-spec.md` — Language core specification
- `docs/src/design/rfc/accepted/` — Accepted RFC design documents

Every test file must declare the corresponding specification chapter at the top (see Rule 2.1). Any developer should be able to take the specification document and compare it against the tests to verify the correctness of the implementation. Conversely—if a piece of code has no corresponding specification description, it should not exist, and certainly should not be tested.

```rust
// 🟢 Good — the test directly references the specification and verifies that the code follows it
//! Literal tests — based on language specification §2.6
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

// 🔴 Garbage — the test accommodates the current code's implementation behavior, rather than verifying the specification
#[test]
fn test_literal_1() {
    // Don't know which section of the specification this code corresponds to
    // If parse_literal returns the wrong value, this test will "pass with green light"
    // because it only verifies that the function does not panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and discover that the code's behavior does not match the specification. You have two choices:
| Wrong Approach | Correct Approach |
|----------------|------------------|
| Modify the test to make it "pass" | Modify the code so its behavior conforms to the specification |
| Add `#[ignore]` to the test | Fix the code implementation immediately |
| Add special conditional branches to the test to accommodate the code | Remove the branches and let the test expose the problem directly |

Remember: **Red light = the code is wrong, not the test.** (Unless your test itself has a bug—that's a different matter.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of the tested code by reading the tests, without needing additional comments or external documentation.

```rust
// 🟢 Good — the test name says what is being tested and what is expected
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 Garbage — no one knows what this is testing
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for random failures.** Tests must be executable in any environment. Tests that depend on random numbers, system time, or thread scheduling order must use fixed seeds or mocks as substitutes.

**Principle 3: One test, one thing.** If a test name needs "and" to connect multiple behaviors, split it into multiple tests.

```rust
// 🟢 Good — each test verifies only one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Garbage — one test stuffed with too many unrelated things
#[test]
fn test_parser() {
    // Tests tokenize, parse, typecheck, codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring internal implementation should not cause test failures. If changing one line of implementation code causes 10 tests to fail, your tests are written incorrectly.

But there is a key distinction here: **the definition of "behavior" comes from the specification, not from the current code's performance.** If the code changes its behavior (i.e., new behavior that doesn't match the specification), the test must fail. If you can't do this, your test is "a test that accommodates the code"—it lets bugs drive right in.

```
Specification (language-spec.md / RFC) ──defines──► Expected behavior ──drives──► Tests
                                                                         │
Current code ──implements──► Actual behavior ──compared against──► Test result

If actual behavior ≠ expected behavior:
  Test must fail (red light) ──► Fix code ──► Test passes (green light)

If actual behavior = expected behavior (but implementation is bad):
  Test passes ──► Refactor implementation ──► Test still passes  ← This is what Principle 4 means
```

**Principle 5: Do not write fallback/compatibility/specific-mode-active test code.** The test environment is one you can fully control. If you need `#[cfg(not(ci))]` to skip a test, it means there is a fundamental problem with that test's design.

### Terminology

| Term | Definition |
|------|------------|
| Unit test | Tests a single function or module's behavior, independent of external systems |
| Integration test | Tests the collaboration of multiple modules through public APIs or command-line entry points |
| Benchmark test | Measures code performance and detects performance regressions |
| Documentation test | Executable code samples embedded in documentation comments |
| Property test | Tests that verify invariants (properties) based on random input |

### Relation to Commit Convention

All test-related commits must use the `:white_check_mark: test:` type, referring to the [Commit Convention](./commit-convention.md).

```
:white_check_mark: test(parser): add Pratt parser infix expression tests
:white_check_mark: test(codegen): complete switch statement IR generation tests
```

---

## Unit Test Specification

### File Organization

**Rule 1.1**: The unit test's `tests/` directory must be **at the same level** as the tested module's `mod.rs`. `tests/` does not aggregate upward, nor does it summarize across levels.

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——declares the same-level tests/
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——pratt's own tests
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # Parser module-level tests (not including pratt submodule content)
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

Key judgment criterion: **Whichever directory the `tests/` is placed in, that directory's `mod.rs` must declare it with `#[cfg(test)] mod tests;`.**

**Rule 1.1 Supplement: Aggregation upward is forbidden.** A submodule's tests must be placed in that submodule's own `tests/`, and may not be aggregated into the parent-level `tests/`.

| Module Type | Test Location | Example |
|-------------|---------------|---------|
| Directory module (has `mod.rs`) | `tests/` under that directory | `emitter/tests/`, `codes/tests/` |
| Single-file module (only `.rs`) | Parent-level `tests/` | `session.rs` → `diagnostic/tests/session.rs` |

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
└── tests/                  # ✅ diagnostic level (single-file modules)
    ├── mod.rs
    ├── session.rs
    ├── suggest.rs
    └── collect.rs

# ❌ Wrong: aggregating emitter and codes' tests into diagnostic/tests/
src/util/diagnostic/
└── tests/
    ├── mod.rs              # ❌ Forced to declare mod emitter; mod codes;
    ├── emitter/            # ❌ Should be in emitter/tests/
    └── codes/              # ❌ Should be in codes/tests/
```

#### Test Placement Rules for Single-File Modules vs. Directory Modules

**Core difference**: The module's organization form determines the test placement location.

| Module Type | Judgment Basis | Test Location | Example |
|-------------|----------------|---------------|---------|
| **Directory module** | Has independent directory and `mod.rs` | `tests/` under that directory | `inference/tests/` |
| **Single-file module** | Only `.rs` file, no independent directory | Parent module's `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

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
│   ├── mod.rs                      # Declares single-file module tests
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
│   ├── mod.rs                      # #[cfg(test)] mod tests; ——declares same-level tests/
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
│       ├── expressions.rs          # Tests for expressions.rs
│       ├── statements.rs           # Tests for statements.rs
│       └── ...
│
└── traits/                         # Removed (logic merged into types/trait_data.rs)
```

**Why are single-file module tests placed in the parent `tests/`?**

Because single-file modules (such as `overload.rs`) don't have their own `mod.rs`, they cannot declare `#[cfg(test)] mod tests;`. According to Rust's module system, test files must be declared by some `mod.rs` to be compiled. Therefore, single-file module tests can only be declared by the parent module's `mod.rs` and placed in the parent-level `tests/` directory.

**Judgment Flow**:

```
Encounter a module, determine where to place tests?
│
├── Is the module a directory (has mod.rs)?
│   └── Yes → Create tests/ under that directory, declared by that directory's mod.rs
│
├── Is the module a single file (only .rs)?
│   └── Yes → Tests go in parent-level tests/, declared by parent's mod.rs
│
└── Not sure?
    └── Check whether it has an independent directory and mod.rs
```

**Common Errors**:

```
# ❌ Error 1: Creating an independent tests/ directory for a single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ Should not create a directory for a single-file module
    └── tests/
        └── overload.rs

# ❌ Error 2: Declaring #[cfg(test)] mod tests; inside a single-file module
# overload.rs
#[cfg(test)]                        # ❌ Single-file modules cannot declare this way
mod tests;                          # Because there is no overload/tests/ directory

# ✅ Correct approach: tests go in parent-level tests/
src/frontend/core/typecheck/
├── overload.rs                     # Source file
└── tests/
    └── overload.rs                 # Test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern—do not write it this way:**

```
# ❌ Wrong: submodules' tests consolidated into the parent level
src/frontend/core/types/
├── mod.rs              # Should only declare base and computation
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ Parent-level tests/ contains submodule tests
    ├── mod.rs          # ❌ Forced to declare mod base; mod computation;
    ├── base/           # ❌ This part should be in base/tests/
    │   └── var.rs
    └── computation/    # ❌ This part should be in computation/tests/
        └── ...
```

```
# ✅ Correct approach: each module's tests are independent
src/frontend/core/types/
├── mod.rs              # Only declares pub mod base; pub mod computation;
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——declares same-level tests/
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; ——declares same-level tests/
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**Why is aggregation upward not allowed?** Because Rust's module system requires `#[cfg(test)] mod tests;` to determine the compilation of test files at the declaration site. If `types/mod.rs` declares `mod tests;`, then the contents of `types/tests/` are the private contents of the `types` module—they should not reach into the territory of `base` or `computation`. Each module's tests should be the internal implementation details of that module, not the parent module's. This rule also applies to module refactoring: when you split `types` into `base` and `computation`, the tests should follow the split modules, not stay in place. **The test directory does not mirror the source structure; it follows the module boundary.**

**Rule 1.2**: `tests/mod.rs` is only responsible for module declarations and re-exports; it does not contain test functions.

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

**Rule 1.3**: Each test file corresponds to only one source file. It is not allowed to mix tests for multiple source modules in one file.

**Rule 1.4**: Test declarations must use the file form `mod tests;` (with semicolon), pointing to the same-level `tests/` directory. **It is forbidden to use the inline form `mod tests { ... }` to write test code directly inside the source file.**

```rust
// ✅ Correct — file form declaration, test code in independent files
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests;

// 🔴 Forbidden — inline form, test code parasitic in the source file
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

**Why is inline forbidden?**
1. Single responsibility for source files: source files contain only implementation, test files contain only tests. Mixed together, modifying tests requires scrolling to the bottom of the file, and modifying implementation requires skipping over tests.
2. Clear module boundaries: the `tests/` directory is a physical boundary that makes it immediately obvious which modules have tests and which don't.
3. Refactoring safety: when modules are split, the `tests/` directory follows along; inline tests need to be manually stripped from the source file.
4. Code review: in a PR diff, source code changes and test changes are in separate files, not mixed together.

### Module Declaration Specification

**Rule 2.1**: Every test file must have a module-level documentation comment `//!` at the top, explaining the specification source covered by the tests (language specification chapter number + RFC number). If a test does not reference any specification chapter, it means this code has no specification basis—it should not exist.

```rust
//! Literal tests — based on language specification §2.6
//!
//! §2.6.1: Integer Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation
```

**Why must the specification be referenced?** Because the expected values of tests come from the specification, not from "the current code's output." If one day the code changes its output and the test is updated accordingly, the test protected nothing. Only specification-anchored tests can distinguish between "intentional breaking change" and "unintentional regression."

**Rule 2.2**: The `use` imports in a test module must be precise to the specific type/function; glob imports `use super::*` are forbidden.

```rust
// 🟢 Good — precise imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Garbage — others don't know what you're testing
use super::*;
```

### Naming Convention

**Rule 3.1**: Test function naming format is `test_<what>_<scenario>`, all lowercase, separated by underscores.

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, you should know what is tested and what is expected. Numerical sequence naming is forbidden.

```rust
// 🟢 Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Garbage — no idea what is being tested
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**Rule 3.3**: Helper functions do not need the `test_` prefix; they should describe their purpose with a verb or noun.

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### Test Structure Specification (Arrange-Act-Assert)

**Rule 4.1**: Every test function must follow the three-part structure: Arrange → Act → Assert, with blank lines separating the three parts.

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

**Rule 4.2**: Simple tests (single call + single assertion) may omit the partition comments, but may not exceed 5 lines of logical code. Tests with more than 5 lines must explicitly mark the three parts.

### Helper Function Specification

**Rule 5.1**: Setup logic that repeats 3 or more times must be extracted into a helper function.

```rust
// 🟢 Good — extracting common setup
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

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print sufficient context when panicking. Inside test function bodies (`#[test] fn ...`), you can use `unwrap()` directly—when it fails, Rust automatically prints the line number; but when a helper function fails, the line number points to the helper function's definition, and the calling context cannot be seen.

```rust
// 🟢 Good — print source content when the helper function fails
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Garbage — when it fails, you can't see which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, right after the `use` imports. If shared by multiple test modules, place them in `tests/mod.rs` and export with `pub(crate)`.

### Assertion Style

**Rule 6.1**: Enum variant matching should preferentially use `assert!(matches!(...))`; `if let` + `panic!` is not allowed.

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

**Rule 6.2**: Use `assert_eq!` for precise value comparison, and `assert!` for boolean assertions. It is forbidden to use `assert!(a == b)` as a substitute for `assert_eq!(a, b)`.

**Rule 6.3**: All assertions must include a custom error message, unless the assertion itself already fully describes the reason for failure.

```rust
// 🟢 Good — quick location when assertion fails
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 Good — assert_eq! automatically prints the value difference on failure, no extra message needed
assert_eq!(error_count, 0);

// 🔴 Garbage — when it fails, you only know "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: The assertion order must be `assert_eq!(actual, expected)`, with the actual value first and the expected value second.

### Anti-pattern Checklist

The following are prohibited writing styles and their alternatives:

| Anti-pattern | Problem | Alternative |
|--------------|---------|-------------|
| `#[cfg(test)] mod tests { ... }` inline tests | Source file bloat, blurred module boundaries, refactoring difficulty | Place test code in independent `tests/` directory, declare with `mod tests;` (see Rule 1.4) |
| Tests accommodating the code's wrong behavior | Conceals specification deviations, legitimizes bugs | Fix the code against the specification, keep tests unchanged |
| Deducing test expected values from code output | Tests become "recorders of the current implementation" | Derive expected values from the specification |
| Permanent `#[ignore]` markers | Hide rotting tests | Fix or delete |
| `println!` debug output | Pollute test output | Use `assert!` to make explicit assertions |
| `thread::sleep` | Random failure + slow | Use synchronization mechanisms or mocks |
| Operating on real file system in tests | Slow and unrepeatable | Use `tempfile` |
| Depending on test execution order | Random failure | Independent setup for each test |
| A test function exceeding 30 lines of logic | No one can understand it | Split the test or use helper functions |
| `unwrap()` in helper functions without context | Hard to locate | Use `expect("why")` or custom panic (see Rule 5.2) |
| Same setup copy-pasted 3+ times | High modification cost | Extract helper function |

---

## Integration Test Specification

### Test Organization

**Rule 7.1**: Integration tests are placed in the project root's `tests/` directory. The entry file `tests/integration.rs` uses the `#[path]` attribute to include submodules.

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

**Rule 7.3**: Integration tests must be performed through the project's public API. Direct references to `crate::` internal modules are forbidden in integration tests. Use the `yaoxiang::` public path.

```rust
// 🟢 Good — through public API
use yaoxiang::run;

// 🔴 Garbage — bypasses the public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests should preferentially use inline source code strings. External fixture files (placed in `tests/fixtures/`) should only be used when the source code exceeds 30 lines.

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

**Rule 8.2**: Fixture files must end with the `.yx` extension, and the file name should describe the test's intent.

### E2E Coverage Principles

**Rule 9.1**: The integration tests for each language feature must cover three paths:

| Path | Description |
|------|-------------|
| Happy path | Legal input produces expected output |
| Error path | Illegal input produces clear error message (not panic) |
| Boundary | Boundary values (empty input, maximum value, maximum nesting depth) |

**Rule 9.2**: Integration tests must not depend on the network, system environment variables, or external services.

---

## Benchmark Test Specification

### Criterion.rs Usage Specification

**Rule 10.1**: Benchmark tests are uniformly placed in the `benches/` directory, with the entry file being `benches/lib.rs`. Files are divided by test topic.

```
benches/
├── lib.rs              # Entry point, defines criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # Cross-language comparison benchmarks
├── parser.rs           # Parser benchmarks
└── codegen.rs          # Code generation benchmarks
```

**Rule 10.2**: Each benchmark function must include a module documentation comment `//!` explaining the test's purpose and measurement metrics.

```rust
//! YaoXiang interpreter performance benchmarks
//!
//! Measurement metric: per-iteration wall time
//! Baseline: native Rust implementation
```

### Preventing Compiler Optimization

**Rule 11.1**: The tested output of all benchmark tests must be wrapped with `criterion::black_box` to prevent the compiler from optimizing it away.

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

**Rule 11.2**: The input data for benchmark tests must be `const` or `lazy_static`; dynamic generation within the `iter` closure is not allowed—otherwise what's measured is the total time of data generation + the tested logic.

### Benchmark Grouping and Naming

**Rule 12.1**: Benchmark test naming format is `<tested module>_<scenario>`, all lowercase, separated by underscores. Consistent with the unit test naming rules.

**Rule 12.2**: `criterion_group!` must be used to logically group related benchmarks. It is forbidden to cram all benchmarks into one group.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Documentation Test Specification

### Usage Scenarios

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code sample in their documentation comments. This sample is executed through `cargo test --doc`.

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

**Rule 13.2**: Documentation test code samples must compile successfully and pass assertions. Samples marked `ignore` are not allowed, unless the sample demonstrates a compile-time error.

```rust
/// ```ignore
/// // Demonstrates a compile-time error — ignore is allowed
/// let x: int = "string";
/// ```
```

### Coverage Requirements

**Rule 14.1**: Documentation tests only need to cover the API's happy path. Boundary and error paths are covered by unit tests.

**Rule 14.2**: The sample code in documentation tests must be concise—no more than 10 lines. If the sample requires longer context, it indicates a problem with the API design.

---

## Property Test Specification

### Usage Scenarios

**Rule 15.1**: The following scenarios must use property tests (proptest or quickcheck) rather than manually writing multiple boundary value cases:

| Scenario | Example |
|----------|---------|
| Parser round-trip | `parse(pretty_print(ast)) == ast` |
| Serialization/deserialization | `deserialize(serialize(data)) == data` |
| Mathematical operation identity | `a + b == b + a` |
| Compiler optimization does not change semantics | `eval(code) == eval(optimize(code))` |

**Rule 15.2**: Property tests use `proptest` as the primary property testing framework (already declared in `Cargo.toml`'s `dev-dependencies`).

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

**Rule 16.1**: Every property test must have a clear property declaration—the invariant being verified must be stated in comments.

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

**Rule 16.2**: If a property test discovers a failure, you must use proptest's regression mechanism—add the failing input to the `proptest-regressions/` directory, and do not manually write a regular test as a substitute.

---

## Coverage Requirements

### New Code Coverage Targets

**Rule 17.1**: Test coverage requirements for new code:

| Code Type | Line Coverage | Branch Coverage |
|-----------|---------------|-----------------|
| Core compiler modules (frontend/middle/backends) | ≥ 85% | ≥ 80% |
| Utility/helper modules (util) | ≥ 75% | ≥ 70% |
| Runtime modules (vm/runtime) | ≥ 80% | ≥ 75% |
| Standard library (std) | ≥ 75% | ≥ 70% |
| Error handling and diagnostics | ≥ 90% | ≥ 85% |

**Rule 17.2**: Error handling paths (all `Err` branches) must be 100% covered. Error messages visible to users must be tested and verified.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, the author must self-check the following items:

- [ ] `cargo test` all pass
- [ ] `cargo test --doc` all pass
- [ ] `cargo bench` has no performance regression (if hot path changes are involved)
- [ ] New code meets the coverage target
- [ ] Test naming conforms to the naming specification
- [ ] Every test file declares the corresponding specification chapter (Rule 2.1)
- [ ] Test expected values come from the specification definition, not "the current code's output"
- [ ] No tests marked with `#[ignore]` (unless there is a clear issue number comment)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic message)
- [ ] Commit message uses the `:white_check_mark: test:` type
- [ ] **No test expected values were modified because "code behavior does not match the specification"—the code was changed, not the test**
- [ ] **No inline tests** (`#[cfg(test)] mod tests { ... }` must be changed to `mod tests;` + independent file, see Rule 1.4)

**Rule 18.2**: Reviewers must reject PRs containing the following issues:

- Only happy path tests, missing error paths
- `thread::sleep` in tests or dependency on execution order
- Copy-pasted test code more than 3 times without extracting helper functions
- Test names that do not conform to the naming specification
- Existence of permanently `#[ignore]`d tests
- **Tests accommodating the code's wrong behavior** (modifying tests instead of code when code does not match the specification)
- **Tests do not declare the corresponding specification chapter** (see Rule 2.1)
- **Test expected values come from code output rather than specification definition** (tests deduced backward are equivalent to no tests)
- **Existence of inline tests** (`#[cfg(test)] mod tests { ... }` rather than `mod tests;` + independent file, see Rule 1.4)
- Tests only verify "does not panic" without asserting specific behavior
- Deleting failing tests that expose code bugs (rather than fixing the code and then seeing it turn green)

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

# Run specific tests (filter by name)
cargo test test_parse_expr

# Run benchmarks
cargo bench

# Show test output (stdout hidden by default)
cargo test -- --nocapture

# Single-threaded run (troubleshoot concurrency issues)
cargo test -- --test-threads=1

# Generate coverage report (requires cargo-llvm-cov)
cargo llvm-cov --html
```

### B. Commit Message Template

Test-related commits must follow the following template:

```
:white_check_mark: test(<scope>): <short description>

<optional: list of covered scenarios>
```

Example:

```
:white_check_mark: test(parser): add Pratt parser infix operator tests

Covered scenarios:
- Arithmetic operator precedence (+, -, *, /, %)
- Comparison operator chaining (1 < x < 10)
- Logical operator short-circuit
- Assignment operator right associativity
```

### C. New Test File Checklist

When creating a new test module, make sure it includes the following files:

```
# Add tests under src/<module>/
src/<module>/tests/
├── mod.rs          # Module declaration + public helper functions
└── <subject>.rs    # Test file, named after the tested source file

# Add integration tests under tests/
tests/
├── integration.rs   # Update: add #[path] declaration
└── integration/
    └── <topic>.rs   # New test file
```

### D. References

- [YaoXiang Language Specification](../../design/language-spec.md) — **The authoritative source for tests**
- [Accepted RFCs](../../design/rfc/accepted/) — **The authoritative source for design decisions**
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Convention](./commit-convention.md)
- [Project Contributing Guide](./contributing.md)

---

> 💡 **Remember**: Tests do not verify whether your code "can run"—they verify whether your code conforms to the specification. The specification changes, and tests change with the specification. When the code is wrong, fix the code, don't fix the test. **The code serves the specification, the tests guard the specification. The moment a test accommodates the code, you have lost all protection.**