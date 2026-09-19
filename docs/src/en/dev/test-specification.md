---
title: 'Test Writing Standards'
description:
  Hard standards for test writing in the YaoXiang project, defining writing criteria for unit tests,
  integration tests, benchmarks, doc tests, and property tests
---

# Test Writing Standards

This document defines the hard standards for test writing in the YaoXiang project. All contributors
must comply with the following rules; violations will be required to be modified during Code Review.

---

## Table of Contents

- [General Principles](#general-principles)
- [yx Corpus and Library Test Layers](#yx-corpus-and-library-test-layers)
- [Unit Test Standards](#unit-test-standards)
- [Integration Test Standards](#integration-test-standards)
- [Benchmark Standards](#benchmark-standards)
- [Doc Test Standards](#doc-test-standards)
- [Property Test Standards](#property-test-standards)
- [Coverage Requirements](#coverage-requirements)
- [Appendix](#appendix)

---

## General Principles

### Scope

This standard applies to all Rust test code in the YaoXiang project, including:

| Test Type        | Location              | Framework                  |
| ---------------- | --------------------- | -------------------------- |
| Unit Test        | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| Integration Test | `tests/`              | `#[test]`                  |
| Benchmark        | `benches/`            | Criterion.rs               |
| Doc Test         | API doc comments      | `cargo test --doc`         |
| Property Test    | Any test location     | proptest / quickcheck      |

### Core Principles

**Principle 0: The authoritative source of tests is the specification, not the code.** This is the
most important principle in this document. Tests verify whether the code conforms to the
specification, not whether the code "works according to the current implementation." When a test
discovers that the code's behavior is inconsistent with the specification, **fix the code, never fix
the test**.

Specification files are located at:

- `docs/src/design/language-spec.md` —— Core language specification
- `docs/src/design/rfc/accepted/` —— Accepted RFC design documents

Each test file must declare the corresponding specification section at the top (see Rule 2.1). Any
developer should be able to hold the specification document against the test and verify the
correctness of the implementation. Conversely —— if a piece of code has no corresponding
specification description, it should not exist, let alone be tested.

```rust
// 🟢 Good —— Tests reference the spec directly, verifying whether the code follows the spec
//! Literal tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floats (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation

#[test]
fn test_decimal_literal_parsing() {
    // Spec §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 Garbage —— Test accommodates the current code's implementation behavior, not the spec
#[test]
fn test_literal_1() {
    // Don't know which section of the spec this code corresponds to
    // If parse_literal returns a wrong value, this test will "pass green"
    // because it only verifies that the function doesn't panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and find that the code's behavior doesn't match the spec. You have
two choices:

| Wrong Approach                                         | Correct Approach                                          |
| ------------------------------------------------------ | --------------------------------------------------------- |
| Modify the test to "pass"                              | Modify the code to make behavior conform to the spec      |
| Add `#[ignore]` to the test                            | Fix the code implementation immediately                   |
| Add special condition branches to accommodate the code | Remove branches, let the test expose the problem directly |

Remember: **Red light = code is wrong, not the test is wrong.** (Unless your test itself has a bug,
that's another story.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of
the code under test by reading the tests, without needing additional comments or external
documentation.

```rust
// 🟢 Good —— The test name states what is tested and what is expected
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 Garbage —— Nobody knows what this is testing
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for random failures.** Tests must be repeatable in any environment.
Tests that depend on random numbers, system time, or thread scheduling order must use fixed seeds or
be replaced with mocks.

**Principle 3: One test, one thing.** If the test name needs to connect multiple behaviors with
"and", split into multiple tests.

```rust
// 🟢 Good —— Each test verifies only one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Garbage —— One test stuffed with too many unrelated things
#[test]
fn test_parser() {
    // Tests tokenize, parse, typecheck, codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring internal implementations should not
cause test failures. If changing one line of implementation code causes 10 tests to fail, your tests
are written wrong.

But here is a key distinction: **The definition of "behavior" comes from the specification, not from
the current code's performance.** If the code changes behavior (i.e., new behavior that doesn't
conform to the spec), the test must fail. If you can't do this, your tests are "tests that
accommodate the code" —— they let bugs drive straight in.

```
Specification (language-spec.md / RFC)  ──define──►  Expected Behavior  ──drive──►  Tests
                                              │
Current Code  ──implement──►  Actual Behavior  ──compare──►  Test Result

If actual behavior ≠ expected behavior:
  Test must fail (red light)  ──►  Fix code  ──►  Test passes (green light)

If actual behavior = expected behavior (but implementation is bad):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is what Principle 4 means
```

**Principle 5: Do not write fallback/compatibility/specific-mode-effective test code.** The test
environment is one you have full control over. If you need `#[cfg(not(ci))]` to skip a test, it
means the test design has a fundamental problem.

### Terminology

| Term             | Definition                                                                  |
| ---------------- | --------------------------------------------------------------------------- |
| Unit Test        | Tests a single function or module behavior, no external system dependencies |
| Integration Test | Tests multiple modules collaborating, through public API or CLI entry point |
| Benchmark        | Measures code performance, detects performance regressions                  |
| Doc Test         | Executable code examples embedded in doc comments                           |
| Property Test    | Tests that verify invariants (properties) based on random input             |

### Relation to Commit Convention

All test-related commits must use the `:white_check_mark: test:` type, refer to
[Commit Convention](./commit-convention.md).

```
:white_check_mark: test(parser): Add Pratt parser infix expression tests
:white_check_mark: test(codegen): Complete switch statement IR generation tests
```

---

## yx Corpus and Library Test Layers

This standard constrains **Rust-side test code**. The tests for the YaoXiang language itself (`.yx`
corpus and library tests) are divided into two layers based on the object under test. The system
design and judgment contract are governed by RFC-036 (§7 Suite Collection / §8 Negative Three Layers
/ §9 Test System Layering), and the corpus writing details are governed by
`tests/yaoxiang/TEST_STANDARDS.md`:

- **Language Usability Corpus** (`tests/yaoxiang/`) —— The object under test is the language itself;
  std is only used as an assertion tool. Within the corpus, judgments are divided into three
  categories by the layer where failure occurs: behavior tests / compile-time rejection tests /
  runtime failure tests
- **Library Tests** (along with the library) —— The object under test is the library's public API
  contract; the yx-level tests of std are located at `src/std/tests/`, and tests for future user
  packages are discovered within the package via `[tool.test]`

The file header format of `.yx` tests, the header directives (`// expect:` / `// skip:` /
`// mode:`, RFC-036 §8.2), and assertion conventions follow TEST_STANDARDS.md; judgment parsing is
implemented by the dual-runner shared `src/util/test_markers.rs` (Rust-side, constrained by this
standard).

---

## Unit Test Standards

### File Organization

**Rule 1.1**: The `tests/` directory of a unit test must be at the **same level** as the `mod.rs` of
the module under test. The `tests/` directory does not aggregate upward or cross-level.

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; —— declares same-level tests/
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; —— pratt's own tests
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # Tests at the parser module level (does not include pratt submodule content)
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

Key criterion: **Whichever directory the `tests/` is placed in, that directory's `mod.rs` must
declare it using `#[cfg(test)] mod tests;`.**

**Rule 1.1 Supplement: Upward aggregation is prohibited.** The tests of a submodule must be placed
in that submodule's own `tests/`, and must not be aggregated into the parent-level `tests/`.

| Module Type                     | Test Location                 | Example                                      |
| ------------------------------- | ----------------------------- | -------------------------------------------- |
| Directory module (has `mod.rs`) | `tests/` under that directory | `emitter/tests/`, `codes/tests/`             |
| Single-file module (only `.rs`) | Parent's `tests/`             | `session.rs` → `diagnostic/tests/session.rs` |

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

# ❌ Wrong: Aggregate emitter and codes' tests into diagnostic/tests/
src/util/diagnostic/
└── tests/
    ├── mod.rs              # ❌ Forced to declare mod emitter; mod codes;
    ├── emitter/            # ❌ Should be in emitter/tests/
    └── codes/              # ❌ Should be in codes/tests/
```

#### Test Placement Rules for Single-File Modules vs Directory Modules

**Core difference**: The organization of a module determines the test placement location.

| Module Type            | Criterion                                  | Test Location                 | Example                                       |
| ---------------------- | ------------------------------------------ | ----------------------------- | --------------------------------------------- |
| **Directory Module**   | Has independent directory and `mod.rs`     | `tests/` under that directory | `inference/tests/`                            |
| **Single-File Module** | Only `.rs` files, no independent directory | Parent module's `tests/`      | `overload.rs` → `typecheck/tests/overload.rs` |

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
│   ├── mod.rs                      # #[cfg(test)] mod tests; —— declares same-level tests/
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

Because single-file modules (such as `overload.rs`) don't have their own `mod.rs`, they cannot
declare `#[cfg(test)] mod tests;`. According to the Rust module system, test files must be declared
by some `mod.rs` to compile. Therefore, tests for single-file modules can only be declared by the
parent module's `mod.rs`, placed in the parent's `tests/` directory.

**Decision Flow**:

```
Encounter a module, decide where tests go?
│
├── Is this module a directory (has mod.rs)?
│   └── Yes → Create tests/ under that directory, declared by that directory's mod.rs
│
├── Is this module a single file (only .rs)?
│   └── Yes → Tests go in parent's tests/ directory, declared by parent's mod.rs
│
└── Not sure?
    └── Check if there's an independent directory and mod.rs
```

**Common Errors**:

```
# ❌ Error 1: Create independent tests/ directory for a single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ Should not create a directory for a single-file module
    └── tests/
        └── overload.rs

# ❌ Error 2: Declare #[cfg(test)] mod tests; within a single-file module
# overload.rs
#[cfg(test)]                        # ❌ Single-file module cannot declare this way
mod tests;                          # Because there's no overload/tests/ directory

# ✅ Correct: Tests placed in parent's tests/
src/frontend/core/typecheck/
├── overload.rs                     # Source file
└── tests/
    └── overload.rs                 # Test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern —— Do not write like this:**

```
# ❌ Wrong: Concentrate submodule tests into the parent
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
│   ├── mod.rs          # #[cfg(test)] mod tests; —— declares same-level tests/
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; —— declares same-level tests/
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**Why can't we aggregate upward?** Because the Rust module system requires `#[cfg(test)] mod tests;`
to decide the compilation of test files at the declaration point. If `types/mod.rs` declares
`mod tests;`, then the content of `types/tests/` is the private content of the `types` module —— it
should not cross into the territory of `base` or `computation`. Each module's tests should be the
internal implementation details of that module, not the parent module's. This rule also applies to
module refactoring: when you split `types` into `base` and `computation`, the tests should also
follow the split modules, not stay in place. **The test directory does not mirror the source code
structure, but follows the module boundaries.**

**Rule 1.2**: `tests/mod.rs` is only responsible for module declaration and re-export, no test
functions.

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

**Rule 1.3**: Each test file corresponds to only one source file. Tests for multiple source modules
mixed in one file are not allowed.

**Rule 1.4**: Test declarations must use the file form `mod tests;` (with semicolon), pointing to
the same-level `tests/` directory. **The inline form `mod tests { ... }` is prohibited, which puts
test code directly inside the source file.**

```rust
// ✅ Correct —— File form declaration, test code in independent files
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests;

// 🔴 Forbidden —— Inline form, test code parasitic in the source file
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

**Why is inline prohibited?**

1. Single responsibility for source files: Source files only contain implementation, test files only
   contain tests. Mixed together, you have to scroll to the bottom of the file to modify tests, and
   skip tests to modify implementation.
2. Clear module boundaries: The `tests/` directory is a physical boundary, immediately visible which
   modules have tests and which don't.
3. Refactoring safety: When modules are split, the `tests/` directory follows along; inline tests
   need to be manually extracted from the source file.
4. Code review: In the PR diff, source code changes and test changes are separate files, not mixed
   together.

### Module Declaration Standards

**Rule 2.1**: All test files must have a module-level doc comment `//!` at the top, describing the
specification source covered by the test (language specification section number + RFC number). If a
test does not reference any specification section, it means this code has no specification basis ——
it should not exist.

```rust
//! Literal tests — Based on Language Specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floats (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation
```

**Why must the spec be referenced?** Because the expected values of tests come from the spec, not
from "the current code's output." If one day the code changes its output and the test updates
accordingly, then the test protects nothing. Only spec-anchored tests can distinguish between
"intentional breaking change" and "unintentional regression."

**Rule 2.2**: The `use` imports in the test module must be precise to specific types/functions; glob
import `use super::*` is prohibited.

```rust
// 🟢 Good —— Precise imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Garbage —— Others don't know what you're testing
use super::*;
```

### Naming Standards

**Rule 3.1**: Test function naming format is `test_<what>_<scenario>`, all lowercase with underscore
separation.

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, you
should know what is tested and what is expected. Numeric sequence naming is prohibited.

```rust
// 🟢 Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Garbage —— Completely don't know what's being tested
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**Rule 3.3**: Helper functions don't need the `test_` prefix; they should use verbs or nouns to
describe their purpose.

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### Test Structure Standards (Arrange-Act-Assert)

**Rule 4.1**: Each test function must follow the three-segment structure: Arrange → Act → Assert,
with blank lines separating the three segments.

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

**Rule 4.2**: Simple tests (single call + single assertion) may omit the segment comments, but
cannot exceed 5 lines of logic code. Tests exceeding 5 lines must explicitly mark the three
segments.

### Helper Function Standards

**Rule 5.1**: Setup logic that appears 3 or more times must be extracted as a helper function.

```rust
// 🟢 Good —— Extract common setup
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

**Rule 5.2**: The `unwrap()` / `expect()` in helper functions must print enough context on panic.
The `unwrap()` can be used directly in the test function body (`#[test] fn ...`) —— Rust
automatically prints the line number on failure; but when a helper function fails, the line number
points to the helper function definition, and the context of the call is not visible.

```rust
// 🟢 Good —— Helper function prints source content on failure
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Garbage —— On failure, you can't see which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, immediately after `use`
imports. If shared by multiple test modules, place in `tests/mod.rs` and export as `pub(crate)`.

### Assertion Style

**Rule 6.1**: Enum variant matching should prefer `assert!(matches!(...))`, not `if let` + `panic!`.

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

**Rule 6.2**: Use `assert_eq!` for exact value comparison, `assert!` for boolean assertions. Using
`assert!(a == b)` instead of `assert_eq!(a, b)` is prohibited.

**Rule 6.3**: All assertions must have custom error messages, unless the assertion itself fully
describes the reason for failure.

```rust
// 🟢 Good —— Can quickly locate when assertion fails
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 Good —— assert_eq! automatically prints value differences on failure, no extra message needed
assert_eq!(error_count, 0);

// 🔴 Garbage —— On failure, only knows "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: The assertion order must be `assert_eq!(actual, expected)`, with the actual value
first and the expected value second.

### Anti-Pattern List

The following prohibited practices and their alternatives:

| Anti-Pattern                                           | Problem                                                              | Alternative                                                                            |
| ------------------------------------------------------ | -------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `#[cfg(test)] mod tests { ... }` inline tests          | Source file bloat, blurred module boundaries, refactoring difficulty | Test code in independent `tests/` directory, declared with `mod tests;` (see Rule 1.4) |
| Tests accommodating wrong code behavior                | Hides spec deviations, legitimizes bugs                              | Fix code against the spec, keep tests unchanged                                        |
| Reverse-engineering test expectations from code output | Test becomes "a recorder of the current implementation"              | Derive expected values from the spec                                                   |
| Permanent `#[ignore]` markers                          | Hides rotting tests                                                  | Fix or delete                                                                          |
| `println!` debug output                                | Pollutes test output                                                 | Use `assert!` for clear assertions                                                     |
| `thread::sleep`                                        | Random failures + slow                                               | Use synchronization mechanisms or mocks                                                |
| Manipulating real file system in tests                 | Slow and unrepeatable                                                | Use `tempfile`                                                                         |
| Depending on test execution order                      | Random failures                                                      | Each test has independent setup                                                        |
| One test function exceeds 30 lines of logic            | Nobody can understand it                                             | Split tests or use helper functions                                                    |
| `unwrap()` in helper functions without context         | Hard to locate                                                       | Use `expect("why")` or custom panic (see Rule 5.2)                                     |
| Copy-paste same setup more than 3 times                | High modification cost                                               | Extract helper function                                                                |

---

## Integration Test Standards

### Test Organization

**Rule 7.1**: Integration tests are placed in the `tests/` directory at the project root. The entry
file `tests/integration.rs` uses the `#[path]` attribute to include submodules.

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**Rule 7.2**: Each `tests/integration/*.rs` file corresponds to one test topic (compiler backend,
code generation, executor, etc.), and must not be mixed.

**Rule 7.3**: Integration tests must be performed through the project's public API. Directly
referencing `crate::` internal modules in integration tests is prohibited. Use the `yaoxiang::`
public path.

```rust
// 🟢 Good —— Through public API
use yaoxiang::run;

// 🔴 Garbage —— Bypasses the public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests should prefer inline source strings. Only when the source exceeds 30
lines, external fixture files should be used (placed in `tests/fixtures/`).

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

**Rule 8.2**: Fixture files must end with the `.yx` extension, and the filename should describe the
test intent.

### E2E Coverage Principles

**Rule 9.1**: The integration test for each language feature must cover three paths:

| Path       | Description                                                 |
| ---------- | ----------------------------------------------------------- |
| Happy path | Legal input produces expected output                        |
| Error path | Illegal input produces clear error information (not panic)  |
| Boundary   | Boundary values (empty input, max value, max nesting depth) |

**Rule 9.2**: Integration tests must not depend on the network, system environment variables, or
external services.

---

## Benchmark Standards

### Criterion.rs Usage Standards

**Rule 10.1**: Benchmarks are uniformly placed in the `benches/` directory, with the entry file
being `benches/lib.rs`. Files are organized by test topic.

```
benches/
├── lib.rs              # Entry, defines criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # Cross-language comparison benchmark
├── parser.rs           # Parser benchmark
└── codegen.rs          # Code generation benchmark
```

**Rule 10.2**: Each benchmark function must include a module doc comment `//!` describing the test
purpose and measurement metrics.

```rust
//! YaoXiang interpreter performance benchmark
//!
//! Measurement metric: Single iteration wall time
//! Baseline: Native Rust implementation
```

### Preventing Compiler Optimization

**Rule 11.1**: The output of all benchmark tests must be blocked from compiler optimization
elimination using `criterion::black_box`.

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

**Rule 11.2**: The input data for benchmarks must be `const` or `lazy_static`; dynamic generation
within the `iter` closure is not allowed —— otherwise what's measured is the total time of data
generation + the tested logic.

### Benchmark Grouping and Naming

**Rule 12.1**: The benchmark naming format is `<module under test>_<scenario>`, all lowercase with
underscore separation. Consistent with unit test naming rules.

**Rule 12.2**: Must use `criterion_group!` to logically group related benchmarks. Prohibiting all
benchmarks from being crammed into one group.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Doc Test Standards

### Use Cases

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code
example in the doc comment. This example is executed via `cargo test --doc`.

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

**Rule 13.2**: The code examples in doc tests must compile and pass assertions. Examples containing
`ignore` markers are not allowed, unless the example demonstrates a compile-time error.

````rust
/// ```ignore
/// // Demonstrating compile-time error —— can be ignored
/// let x: int = "string";
/// ```
````

### Coverage Requirements

**Rule 14.1**: Doc tests only need to cover the happy path of the API. Boundary cases and error
paths are covered by unit tests.

**Rule 14.2**: The example code in doc tests must be concise —— no more than 10 lines. If the
example needs longer context, it indicates a problem with the API design.

---

## Property Test Standards

### Use Cases

**Rule 15.1**: The following scenarios must use property tests (proptest or quickcheck) instead of
manually writing multiple boundary value cases:

| Scenario                                       | Example                                |
| ---------------------------------------------- | -------------------------------------- |
| Parser round-trip                              | `parse(pretty_print(ast)) == ast`      |
| Serialization/deserialization                  | `deserialize(serialize(data)) == data` |
| Mathematical operation identities              | `a + b == b + a`                       |
| Compiler optimization doesn't change semantics | `eval(code) == eval(optimize(code))`   |

**Rule 15.2**: Property tests use `proptest` as the primary property testing framework (already
declared in `Cargo.toml`'s `dev-dependencies`).

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

**Rule 16.1**: Each property test must have a clear property declaration —— the verified invariant
should be stated in a comment.

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

**Rule 16.2**: If a property test discovers a failure, the regression mechanism of `proptest` must
be used —— add the failing input to the `proptest-regressions/` directory; do not manually write a
regular test to replace it.

---

## Coverage Requirements

### New Code Coverage Targets

**Rule 17.1**: Test coverage requirements for new code:

| Code Type                                        | Line Coverage | Branch Coverage |
| ------------------------------------------------ | ------------- | --------------- |
| Core compiler modules (frontend/middle/backends) | ≥ 85%         | ≥ 80%           |
| Utility/helper modules (util)                    | ≥ 75%         | ≥ 70%           |
| Runtime modules (vm/runtime)                     | ≥ 80%         | ≥ 75%           |
| Standard library (std)                           | ≥ 75%         | ≥ 70%           |
| Error handling and diagnostics                   | ≥ 90%         | ≥ 85%           |

**Rule 17.2**: Error handling paths (all `Err` branches) must have 100% coverage. Error messages
visible to users must be tested and verified.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, the author must self-check the following items:

- [ ] `cargo test` all pass
- [ ] `cargo test --doc` all pass
- [ ] `cargo bench` has no performance regressions (if hot path changes are involved)
- [ ] New code meets coverage targets
- [ ] Test names follow naming standards
- [ ] Each test file declares the corresponding spec section (Rule 2.1)
- [ ] Test expected values come from spec definitions, not "current code output"
- [ ] No `#[ignore]` marked tests (unless with explicit issue number comments)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic messages)
- [ ] Commit messages use the `:white_check_mark: test:` type
- [ ] **No modification of test expected values because "code behavior doesn't match the spec" ——
      the code is changed, not the test**
- [ ] **No inline tests** (`#[cfg(test)] mod tests { ... }` must be changed to `mod tests;` +
      independent file, see Rule 1.4)

**Rule 18.2**: Reviewers must reject PRs containing the following issues:

- Only happy path tests, missing error paths
- Tests use `thread::sleep` or depend on execution order
- Copy-pasted test code more than 3 times without extracting helper functions
- Test names don't follow naming standards
- Permanent `#[ignore]` tests exist
- **Tests accommodating wrong code behavior** (modifying tests instead of code when code doesn't
  match the spec)
- **Tests don't declare the corresponding spec section** (see Rule 2.1)
- **Test expected values come from code output rather than spec definition** (tests
  reverse-engineered equal no tests)
- **Inline tests exist** (`#[cfg(test)] mod tests { ... }` instead of `mod tests;` + independent
  file, see Rule 1.4)
- Tests only verify "doesn't panic" without asserting specific behavior
- Deleted failing tests that exposed code bugs (instead of fixing the code and then seeing it turn
  green)

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

# Run specific test (filter by name)
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
:white_check_mark: test(<scope>): <Short description>

<Optional: List of covered scenarios>
```

Example:

```
:white_check_mark: test(parser): Add Pratt parser infix operator tests

Scenarios covered:
- Arithmetic operator precedence (+, -, *, /, %)
- Comparison operator chaining (1 < x < 10)
- Logical operator short-circuit
- Assignment operator right-associativity
```

### C. New Test File Checklist

When creating a new test module, ensure the following files are included:

```
# Add tests under src/<module>/
src/<module>/tests/
├── mod.rs          # Module declaration + public helper functions
└── <subject>.rs    # Test file, named corresponding to the source file under test

# Add integration tests under tests/
tests/
├── integration.rs   # Update: add #[path] declaration
└── integration/
    └── <topic>.rs   # New test file
```

### D. References

- [YaoXiang Language Specification](../../design/language-spec.md) —— **The authoritative source for
  tests**
- [Accepted RFCs](../../design/rfc/accepted/) —— **The authoritative source for design decisions**
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Convention](./commit-convention.md)
- [Project Contributing Guide](./contributing.md)

---

> 💡 **Remember**: Tests don't verify that your code "can run" —— they verify whether your code
> conforms to the spec. The spec changes, tests follow the spec. When the code is wrong, fix the
> code, don't fix the test. **The code serves the spec, the tests guard the spec. The moment tests
> accommodate the code, you lose all protection.**
