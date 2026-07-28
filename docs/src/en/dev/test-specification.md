---
title: 'Test Writing Standards'
description:
  YaoXiang project test writing hard rules, defining standards for unit tests, integration tests, benchmark tests, documentation tests, and property-based tests
---

# Test Writing Standards

This document defines the hard rules for test writing in the YaoXiang project. All contributors must adhere to the following rules; violators will be requested to make changes during Code Review.

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

These standards apply to all Rust test code in the YaoXiang project:

| Test Type      | Location               | Framework                        |
| -------------- | ---------------------- | -------------------------------- |
| Unit tests     | `src/<module>/tests/`  | `#[test]` + `#[cfg(test)]`      |
| Integration    | `tests/`               | `#[test]`                        |
| Benchmark      | `benches/`             | Criterion.rs                     |
| Documentation  | API doc comments       | `cargo test --doc`               |
| Property-based | Any test location      | proptest / quickcheck            |

### Core Principles

**Principle 0: The spec is the authoritative source of truth for tests, not the code.**
This is the most important principle in this document. Tests verify that code conforms to the specification, not that code "works with the current implementation." When a test reveals that code behavior differs from the specification, **fix the code, never fix the test**.

Specification documents are located at:

- `docs/src/design/language-spec.md` —— Core language specification
- `docs/src/design/rfc/accepted/` —— Accepted RFC design documents

Each test file must declare the corresponding specification section at the top (see Rule 2.1). Any developer should be able to take the specification document and compare it against tests to verify implementation correctness. Conversely—if a piece of code has no corresponding specification description, it shouldn't exist, let alone be tested.

```rust
// ✅ Good — test directly references the spec, verifying code follows the spec
//! Literal tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation

#[test]
fn test_decimal_literal_parsing() {
    // Spec §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 Trash — test accommodates current code behavior instead of verifying spec
#[test]
fn test_literal_1() {
    // Don't know which spec section this corresponds to
    // If parse_literal returns the wrong value, this test will "pass"
    // because it only verifies the function doesn't panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and discover code behavior differs from the spec. You have two choices:

| Wrong Approach                    | Correct Approach                  |
| --------------------------------- | --------------------------------- |
| Modify the test to make it "pass" | Modify the code to match the spec |
| Add `#[ignore]` to the test       | Fix the code implementation now   |
| Add special condition branches in test to accommodate code | Delete branches, let the test expose the problem directly |

Remember: **Red light = code is wrong, not the test.** (Unless your test itself has a bug, which is a different matter.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of the code under test by reading the tests, without additional comments or external documentation.

```rust
// ✅ Good — test name says what is being tested and what is expected
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 Trash — no one knows what this tests
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for flaky tests.**
Tests must be repeatable in any environment. Tests relying on random numbers, system time, or thread scheduling order must use fixed seeds or mock alternatives.

**Principle 3: One test, one thing.** If a test name needs "and" to connect multiple behaviors, split into multiple tests.

```rust
// ✅ Good — each test verifies one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Trash — one test stuffed with too much unrelated content
#[test]
fn test_parser() {
    // Tests tokenize, tests parse, tests typecheck, tests codegen...
}
```

**Principle 4: Test behavior, not implementation.**
Refactoring internal implementation should not cause test failures. If you changed one line of implementation code and 10 tests broke, your tests are wrong.

But there's a critical distinction here: **"Behavior" is defined by the spec, not by the current code's behavior.**
If the code changed behavior (a new behavior that doesn't match the spec), tests must fail. If you can't achieve this, your tests are "tests that accommodate the code"—they let bugs pass right through.

```
Specification (language-spec.md / RFC)  ──defines──►  Expected behavior  ──drives──►  Tests
                                           │
Current code  ──implements──►  Actual behavior  ──compared──►  Test results

If actual behavior ≠ expected behavior:
  Test must fail (red light)  ──►  Fix code  ──►  Test passes (green light)

If actual behavior = expected behavior (but implementation is poor):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is the meaning of Principle 4
```

**Principle 5: Don't write fallback/compatibility/situational test code.** The test environment is one you can fully control. If you need
`#[cfg(not(ci))]` to skip a test, that test has a fundamental design problem.

### Terminology Definitions

| Term            | Definition                                                            |
| --------------- | --------------------------------------------------------------------- |
| Unit test       | Tests behavior of a single function or module, without external dependencies |
| Integration test | Tests multiple modules collaborating through public APIs or CLI entry points |
| Benchmark test  | Measures code performance, detects performance regressions            |
| Documentation test | Executable code examples embedded in documentation comments        |
| Property test   | Tests that verify invariants based on random inputs                  |

### Relation to Commit Conventions

All test-related commits must use the `:white_check_mark: test:` type, as defined in the [commit conventions](./commit-convention.md).

```
:white_check_mark: test(parser): add infix expression tests for Pratt parser
:white_check_mark: test(codegen): complete switch statement IR generation tests
```

---

## Unit Test Standards

### File Organization

**Rule 1.1**: Unit test `tests/` directories must be at the **same level** as the `mod.rs` of the module under test. `tests/`
does not aggregate upward or skip levels.

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; —— declares tests/ at same level
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; —— pratt's own tests
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

Key judgment criterion: **When `tests/` is placed in which directory, that directory's `mod.rs` must declare it with `#[cfg(test)] mod tests;`.**

**Rule 1.1 Supplement: Aggregating upward is prohibited.** Tests for subdirectory modules must be placed in that subdirectory's own `tests/`, not aggregated to a parent `tests/`.

| Module Type               | Test Location             | Example                                         |
| ------------------------- | -------------------------- | ----------------------------------------------- |
| Directory module (has `mod.rs`) | That directory's `tests/` | `emitter/tests/`, `codes/tests/`                |
| Single-file module (only `.rs`) | Parent's `tests/`     | `session.rs` → `diagnostic/tests/session.rs`    |

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

# ❌ Wrong: aggregate emitter and codes tests into diagnostic/tests/
src/util/diagnostic/
└── tests/
    ├── mod.rs              # ❌ forced to declare mod emitter; mod codes;
    ├── emitter/            # ❌ should be in emitter/tests/
    └── codes/              # ❌ should be in codes/tests/
```

#### Test Placement Rules for Single-File vs Directory Modules

**Core distinction**: The module's organization form determines test placement.

| Module Type       | Determination                      | Test Location            | Example                                           |
| ----------------- | ---------------------------------- | ------------------------ | ------------------------------------------------- |
| **Directory module** | Has own directory and `mod.rs` | That directory's `tests/` | `inference/tests/`                                |
| **Single-file module** | Only `.rs` files, no own directory | Parent's `tests/`      | `overload.rs` → `typecheck/tests/overload.rs`     |

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
├── tests/                          # ✅ typecheck's tests directory
│   ├── mod.rs                      # declares single-file module tests
│   ├── checker.rs                  # tests for checker.rs
│   ├── environment.rs              # tests for environment.rs
│   ├── overload.rs                 # tests for overload.rs (single-file module test here)
│   ├── type_eval.rs                # tests for type_eval.rs
│   ├── dead_code.rs                # tests for dead_code.rs
│   ├── spawn_placement.rs          # tests for spawn_placement.rs
│   ├── signature.rs                # tests for signature.rs
│   └── types.rs                    # tests for types.rs
│
├── inference/                      # directory module (has mod.rs)
│   ├── mod.rs                      # #[cfg(test)] mod tests; —— declares tests/ at same level
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
│       ├── expressions.rs          # tests for expressions.rs
│       ├── statements.rs           # tests for statements.rs
│       └── ...
│
└── traits/                         # deleted (logic merged into types/trait_data.rs)
```

**Why are single-file module tests placed in the parent `tests/`?**

Because single-file modules (like `overload.rs`) don't have their own `mod.rs`, they cannot declare
`#[cfg(test)] mod tests;`. According to Rust's module system, test files must be declared by some `mod.rs`
to be compiled. Therefore, single-file module tests can only be declared by the parent module's `mod.rs`, and placed in the parent's `tests/` directory.

**Decision flow**:

```
Encounter a module, where to place tests?
│
├── Is this module a directory (has mod.rs)?
│   └── Yes → Create tests/ in that directory, declared by that directory's mod.rs
│
├── Is this module a single file (only .rs)?
│   └── Yes → Place tests in parent's tests/ directory, declared by parent's mod.rs
│
└── Not sure?
    └── Check if there's an independent directory and mod.rs
```

**Common mistakes**:

```
# ❌ Mistake 1: Creating an independent tests/ directory for a single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ shouldn't create a directory for single-file module
    └── tests/
        └── overload.rs

# ❌ Mistake 2: Declaring #[cfg(test)] mod tests; inside a single-file module
# overload.rs
#[cfg(test)]                        # ❌ single-file module can't declare like this
mod tests;                          # because there's no overload/tests/ directory

# ✅ Correct approach: place tests in parent's tests/
src/frontend/core/typecheck/
├── overload.rs                     # source file
└── tests/
    └── overload.rs                 # test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern — don't do this:**

```
# ❌ Wrong: submodule tests centralized to parent
src/frontend/core/types/
├── mod.rs              # should only declare base and computation
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ parent tests/ contains submodule tests
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
│   ├── mod.rs          # #[cfg(test)] mod tests; —— declares tests/ at same level
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; —— declares tests/ at same level
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**Why can't we aggregate upward?** Because Rust's module system requires `#[cfg(test)] mod tests;`
to decide test file compilation at the declaration site. If `types/mod.rs` declares `mod tests;`, then the contents of `types/tests/` are private to the
`types` module—they should not cross into `base` or `computation`'s territory. Each module's tests should be internal implementation details of that module, not the parent module's. This rule also applies during module refactoring: when you split
`types` into `base` and `computation`,
tests should follow the split modules, not stay in place. **Test directories don't mirror source structure; they follow module boundaries.**

**Rule 1.2**: `tests/mod.rs` is only responsible for module declarations and re-exports; test functions don't go here.

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

**Rule 1.3**: Each test file corresponds to only one source file. Tests from multiple source modules are not allowed in a single file.

**Rule 1.4**: Test declarations must use file form `mod tests;` (with semicolon), pointing to the same-level `tests/`
directory. **Inline form `mod tests { ... }` that puts test code directly in the source file is prohibited.**

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
        // test code shouldn't appear in source files
    }
}
```

**Why is inline prohibited?**

1. Single responsibility for source files: source files only contain implementations, test files only contain tests. Mixing them means scrolling to the bottom to change tests, skipping tests to change implementations.
2. Clear module boundaries: `tests/` directory is a physical boundary—一目了然 which modules have tests and which don't.
3. Safe refactoring: when splitting modules, `tests/` directories follow; inline tests need manual extraction from source files.
4. Code review: in PR diffs, source changes and test changes are separate files, not mixed together.

### Module Declaration Standards

**Rule 2.1**: All test files must have module-level documentation comments `//!` at the top, explaining the specification sources covered (language specification section numbers +
RFC numbers). If a test doesn't reference any specification section, it means this code has no specification basis—it shouldn't exist.

```rust
//! Literal tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation
```

**Why must specifications be referenced?**
Because test expected values come from specifications, not from "current code output." If someday the code changes output and tests are updated accordingly, the tests protect nothing. Only spec-anchored tests can distinguish "intentional breaking changes" from "unintentional regressions."

**Rule 2.2**: Test module `use` imports must be precise to specific types/functions; glob imports `use super::*` are prohibited.

```rust
// ✅ Good — precise imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Trash — others don't know what you're testing
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

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, you should know what's being tested and what's expected. Numbered naming is prohibited.

```rust
// ✅ Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Trash — completely unclear what's being tested
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**Rule 3.3**: Helper functions don't need `test_` prefix; use verbs or nouns describing their purpose.

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

**Rule 5.1**: Setup logic that appears 3 or more times must be extracted to a helper function.

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

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print sufficient context on panic. In test function bodies (`#[test] fn ...`), direct
`unwrap()` is acceptable—on failure, Rust automatically prints the line number; but when helper functions fail, the line number points to the helper function definition, not showing call-site context.

```rust
// ✅ Good — helper function failure prints source content
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Trash — on failure you can't see which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of test files, immediately after `use` imports. If shared across multiple test modules, place in
`tests/mod.rs` and export with `pub(crate)`.

### Assertion Style

**Rule 6.1**: Enum variant matching should prefer `assert!(matches!(...))`, not `if let` + `panic!`.

```rust
// ✅ Good
assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(42)));

// 🔴 Trash
if let TokenKind::IntLiteral(v) = tokens[0].kind {
    assert_eq!(v, 42);
} else {
    panic!("Expected IntLiteral");
}
```

**Rule 6.2**: Use `assert_eq!` for precise value comparison, `assert!` for boolean assertions. Using `assert!(a == b)` instead of
`assert_eq!(a, b)` is prohibited.

**Rule 6.3**: All assertions must include custom error messages unless the assertion itself already fully describes the failure.

```rust
// ✅ Good — can quickly locate on assertion failure
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// ✅ Good — assert_eq! automatically prints value diff on failure, no extra message needed
assert_eq!(error_count, 0);

// 🔴 Trash — on failure you only know "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: Assertion order must be `assert_eq!(actual, expected)`, actual value first, expected value second.

### Anti-pattern Checklist

The following practices are prohibited, with alternatives:

| Anti-pattern                                | Problem                               | Alternative                                                            |
| -------------------------------------------- | ------------------------------------- | ---------------------------------------------------------------------- |
| `#[cfg(test)] mod tests { ... }` inline tests | Source file bloat, blurred boundaries, difficult refactoring | Place test code in separate `tests/` directory, declare with `mod tests;` (see Rule 1.4) |
| Tests accommodating code's incorrect behavior | Hides spec deviations, legitimizes bugs | Fix code according to spec, keep tests unchanged                        |
| Reverse-engineering test expectations from code output | Tests become "recorders of current implementation" | Derive expected values from spec                                        |
| Permanent `#[ignore]` markers               | Hides rotting tests                   | Fix or delete                                                           |
| `println!` debug output                     | Pollutes test output                  | Use `assert!` for clear assertions                                      |
| `thread::sleep`                             | Flaky + slow                          | Use synchronization or mock                                            |
| Operating real filesystem in tests           | Slow and non-repeatable               | Use `tempfile`                                                          |
| Depending on test execution order           | Flaky tests                           | Each test has independent setup                                         |
| Single test function exceeding 30 lines      | Unreadable                            | Split tests or use helper functions                                     |
| `unwrap()` in helper functions without context | Hard to locate failures             | Use `expect("why")` or custom panic (see Rule 5.2)                      |
| Copy-pasting same setup 3+ times            | High modification cost                | Extract helper functions                                                |

---

## Integration Test Standards

### Test Organization

**Rule 7.1**: Integration tests go in the project root `tests/` directory. Entry file `tests/integration.rs` uses `#[path]`
attributes to include submodules.

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**Rule 7.2**: Each `tests/integration/*.rs`
file corresponds to one test topic (compiler backend, code generation, executor, etc.), no mixing.

**Rule 7.3**: Integration tests must test through the project's public API. Direct references to `crate::`
internal modules in integration tests are prohibited. Use `yaoxiang::` public path.

```rust
// ✅ Good — through public API
use yaoxiang::run;

// 🔴 Trash — bypasses public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests prefer inline source strings. Only use external fixture files (placed in
`tests/fixtures/`) when source exceeds 30 lines.

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

**Rule 8.2**: Fixture files must have `.yx` extension, filename describes test intent.

### E2E Coverage Principles

**Rule 9.1**: Integration tests for each language feature must cover three paths:

| Path         | Description                                      |
| ------------ | ------------------------------------------------ |
| Happy path   | Valid input produces expected output             |
| Error path   | Invalid input produces clear error message (not panic) |
| Boundary     | Boundary values (empty input, max value, max nesting depth) |

**Rule 9.2**: Integration tests must not depend on network, system environment variables, or external services.

---

## Benchmark Test Standards

### Criterion.rs Usage Standards

**Rule 10.1**: Benchmark tests are uniformly placed in `benches/` directory, entry file is `benches/lib.rs`. Organized by test topic.

```
benches/
├── lib.rs              # entry, defines criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # cross-language comparison benchmark
├── parser.rs           # parser benchmarks
└── codegen.rs          # code generation benchmarks
```

**Rule 10.2**: Each benchmark function must include module documentation `//!` explaining test purpose and measurement metrics.

```rust
//! YaoXiang interpreter performance benchmarks
//!
//! Measurement metric: single iteration duration (wall time)
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

**Rule 11.2**: Benchmark test input data must be `const` or `lazy_static`, not dynamically generated inside the `iter`
closure—otherwise you're measuring data generation + code under test time combined.

### Benchmark Grouping and Naming

**Rule 12.1**: Benchmark naming format is `<module_under_test>_<scenario>`, all lowercase with underscore separation. Consistent with unit test naming rules.

**Rule 12.2**: `criterion_group!` must be used to logically group related benchmarks. All benchmarks in one group is prohibited.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Documentation Test Standards

### Use Cases

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code example in documentation comments. These examples are executed via
`cargo test --doc`.

````rust
/// Tokenize a source string into a sequence of Tokens.
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

**Rule 13.2**: Documentation test code examples must compile and assertions must pass. Examples with `ignore`
markers are not allowed, unless the example demonstrates a compile-time error.

````rust
/// ```ignore
/// // Demonstrating compile-time error — may use ignore
/// let x: int = "string";
/// ```
````

### Coverage Requirements

**Rule 14.1**: Documentation tests cover API happy path only. Boundary cases and error paths are covered by unit tests.

**Rule 14.2**: Code examples in documentation tests must be concise—no more than 10 lines. If an example needs longer context, the API design has issues.

---

## Property Test Standards

### Use Cases

**Rule 15.1**: The following scenarios must use property tests (proptest or quickcheck) instead of hand-writing multiple boundary value cases:

| Scenario                   | Example                                       |
| -------------------------- | --------------------------------------------- |
| Parser round-trip          | `parse(pretty_print(ast)) == ast`             |
| Serialization/deserialization | `deserialize(serialize(data)) == data`    |
| Mathematical identities    | `a + b == b + a`                              |
| Compiler optimization preserves semantics | `eval(code) == eval(optimize(code))` |

**Rule 15.2**: Property tests use `proptest` as the primary property testing framework (already declared in `Cargo.toml`'s
`dev-dependencies`).

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

**Rule 16.1**: Each property test must have a clear property declaration—comment explaining the invariant being verified.

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

**Rule 16.2**: If a property test discovers a failure, the regression mechanism from `proptest` must be used—add the failing input to the
`proptest-regressions/` directory. Don't replace it with a hand-written regular test.

---

## Coverage Requirements

### New Code Coverage Targets

**Rule 17.1**: Coverage requirements for new code:

| Code Type                              | Line Coverage | Branch Coverage |
| -------------------------------------- | ------------- | --------------- |
| Core compiler modules (frontend/middle/backends) | ≥ 85%    | ≥ 80%           |
| Utility/auxiliary modules (util)       | ≥ 75%         | ≥ 70%           |
| Runtime modules (vm/runtime)           | ≥ 80%         | ≥ 75%           |
| Standard library (std)                 | ≥ 75%         | ≥ 70%           |
| Error handling and diagnostics         | ≥ 90%         | ≥ 85%           |

**Rule 17.2**: Error handling paths (all `Err` branches) must be 100% covered. Error messages visible to users must be verified by tests.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, the author must self-check the following:

- [ ] `cargo test` all pass
- [ ] `cargo test --doc` all pass
- [ ] `cargo bench` no performance regression (if hot path changes involved)
- [ ] New code meets coverage targets
- [ ] Test naming follows naming conventions
- [ ] Each test file declares corresponding spec section (Rule 2.1)
- [ ] Test expected values come from spec definitions, not "current code output"
- [ ] No `#[ignore]` marked tests (unless with explicit issue number comment)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic message)
- [ ] Commit message uses `:white_check_mark: test:` type
- [ ] **Test expected values were NOT modified because "code behavior differs from spec"—the code was changed, not the test**
- [ ] **No inline tests** (`#[cfg(test)] mod tests { ... }` must become
      `mod tests;` + separate files, see Rule 1.4)

**Rule 18.2**: Reviewers must reject PRs containing the following issues:

- Only happy path tests, missing error paths
- `thread::sleep` or dependency on execution order in tests
- Copy-pasted test code more than 3 times without extracting helper functions
- Test names not following naming conventions
- Permanent `#[ignore]` tests
- **Tests accommodating code's incorrect behavior** (when code differs from spec, modifying tests instead of code)
- **Tests not declaring corresponding spec section** (see Rule 2.1)
- **Test expected values from code output instead of spec** (reverse-engineered tests equal no tests)
- **Inline tests present** (`#[cfg(test)] mod tests { ... }` instead of
  `mod tests;` + separate files, see Rule 1.4)
- Tests only verify "no panic" without asserting specific behavior
- Deleted failing tests that exposed code bugs (instead of fixing code and seeing it turn green)

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
:white_check_mark: test(parser): add infix operator tests for Pratt parser

Scenarios covered:
- Arithmetic operator precedence (+, -, *, /, %)
- Comparison operator chaining (1 < x < 10)
- Logical operator short-circuit
- Assignment operator right-associativity
```

### C. New Test File Checklist

When creating new test modules, ensure the following files are included:

```
# Add tests under src/<module>/ directory
src/<module>/tests/
├── mod.rs          # module declarations + public helper functions
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
- [Project Commit Conventions](./commit-convention.md)
- [Project Contributing Guide](./contributing.md)

---

> 💡
> **Remember**: Tests don't verify that your code "works"—they verify that your code conforms to the specification. When the spec changes, tests follow the spec. When code is wrong, fix the code, not the tests. **Code serves the specification, tests guard the specification. The moment tests accommodate code, you've lost all protection.**
