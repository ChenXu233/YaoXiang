---
title: 'Test Writing Standards'
description:
  Hard standards for test writing in the YaoXiang project, defining the writing standards for unit
  tests, integration tests, benchmark tests, documentation tests, and property tests
---

# Test Writing Standards

This document defines the hard standards for test writing in the YaoXiang project. All contributors
must comply with the following rules, and violations will be required to be fixed in Code Review.

---

## Table of Contents

- [General Rules](#general-rules)
- [yx Corpus and Library Test Levels](#yx-corpus-and-library-test-levels)
- [Unit Test Standards](#unit-test-standards)
- [Integration Test Standards](#integration-test-standards)
- [Benchmark Test Standards](#benchmark-test-standards)
- [Documentation Test Standards](#documentation-test-standards)
- [Property Test Standards](#property-test-standards)
- [Coverage Requirements](#coverage-requirements)
- [Appendix](#appendix)

---

## General Rules

### Scope

This standard applies to all Rust test code in the YaoXiang project, including:

| Test Type           | Location              | Framework                  |
| ------------------- | --------------------- | -------------------------- |
| Unit Tests          | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| Integration Tests   | `tests/`              | `#[test]`                  |
| Benchmark Tests     | `benches/`            | Criterion.rs               |
| Documentation Tests | API doc comments      | `cargo test --doc`         |
| Property Tests      | Any test location     | proptest / quickcheck      |

### Core Principles

**Principle 0: The authoritative source for tests is the specification, not the code.** This is the
most important principle of this document. Tests verify whether the code conforms to the
specification, not whether the code "runs with the current implementation". When a test finds that
the code behavior does not match the specification, **fix the code, never fix the test**.

The specification files are located at:

- `docs/src/design/language-spec.md` —— Language core specification
- `docs/src/design/rfc/accepted/` —— Accepted RFC design documents

The top of every test file must declare the corresponding specification sections (see Rule 2.1). Any
developer should be able to take the specification document and compare it against the tests to
verify the correctness of the implementation. Conversely—if a piece of code has no corresponding
specification description, it should not exist, let alone be tested.

```rust
// 🟢 Good — The test directly references the specification and verifies whether the code follows it
//! Literal tests — based on language specification §2.6
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

// 🔴 Garbage — The test accommodates the implementation behavior of the current code, instead of verifying the specification
#[test]
fn test_literal_1() {
    // Don't know which section of the specification this test corresponds to
    // If parse_literal returns the wrong value, this test will "pass with green"
    // because it only verifies that the function does not panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You wrote a test and found that the code behavior does not match the specification.
You have two choices:

| Wrong Approach                                           | Correct Approach                                                 |
| -------------------------------------------------------- | ---------------------------------------------------------------- |
| Modify the test to make it "pass"                        | Modify the code so the behavior matches the spec                 |
| Add `#[ignore]` in the test                              | Immediately fix the code implementation                          |
| Add special conditional branches to accommodate the code | Remove the branches and let the test expose the problem directly |

Remember: **Red light = code is wrong, not the test.** (Unless your test itself has a bug, that's
another matter.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of
the code being tested by reading the tests, without additional comments or external documentation.

```rust
// 🟢 Good — The test name says what is being tested and what is expected
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 Garbage — Nobody knows what this is testing
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for flaky tests.** Tests must be reproducible in any environment.
Tests that depend on random numbers, system time, or thread scheduling order must use fixed seeds or
mocks instead.

**Principle 3: One test tests only one thing.** If the test name needs to connect multiple behaviors
with "and", split it into multiple tests.

```rust
// 🟢 Good — Each test verifies only one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Garbage — One test stuffed with too many unrelated things
#[test]
fn test_parser() {
    // Tests tokenize, tests parse, tests typecheck, tests codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring the internal implementation should
not cause test failures. If changing one line of implementation code causes 10 tests to fail, your
tests are written wrong.

But there is a key distinction here: **the definition of "behavior" comes from the specification,
not from the current code's performance.** If the code changes behavior (i.e., new behavior that
does not match the specification), the test must fail. If you can't do this, your test is "a test
that accommodates the code"—it lets bugs drive right in.

```
Specification (language-spec.md / RFC)  ──defines──►  Expected behavior  ──drives──►  Tests
                                                  │
Current code  ──implements──►  Actual behavior  ──compared with──►  Test result

If actual behavior ≠ expected behavior:
  Test must fail (red light)  ──►  Fix code  ──►  Test passes (green light)

If actual behavior = expected behavior (but the implementation is terrible):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is the meaning of Principle 4
```

**Principle 5: Don't write test code for fallback/compatibility/specific mode activation.** The test
environment is one you can completely control. If you need `#[cfg(not(ci))]` to skip a test, it
indicates a fundamental problem with the test's design.

### Term Definitions

| Term               | Definition                                                                  |
| ------------------ | --------------------------------------------------------------------------- |
| Unit Test          | Tests a single function or module behavior, without external dependencies   |
| Integration Test   | Tests multiple modules collaborating, through public API or CLI entry point |
| Benchmark Test     | Measures code performance, detects performance regressions                  |
| Documentation Test | Executable code examples embedded in documentation comments                 |
| Property Test      | Tests that verify invariants (properties) based on random inputs            |

### Relation to Commit Standards

All test-related commits must use the `:white_check_mark: test:` type, following the
[Commit Standards](./commit-convention.md).

```
:white_check_mark: test(parser): Add Pratt parser infix expression tests
:white_check_mark: test(codegen): Complete switch statement IR generation tests
```

---

## yx Corpus and Library Test Levels

This standard constrains **Rust-side test code**. Tests for the YaoXiang language itself (`.yx`
corpus and library tests) are divided into two layers by the object under test; the system design
and judgment contract are governed by RFC-036 (§7 Suite Collection / §8 Negative Three Layers / §9
Test System Layering), and the corpus writing details are governed by
`tests/yaoxiang/TEST_STANDARDS.md`:

- **Language Usability Corpus** (`tests/yaoxiang/`) —— The object under test is the language itself;
  std is only used as an assertion tool. Corpus content is divided into three judgment types based
  on the layer where the failure occurs: behavior tests / compile-time rejection tests / runtime
  failure tests
- **Library Tests** (ship with the library) —— The object under test is the public API contract of
  the library; yx-level tests for std are located in `src/std/tests/`, and tests for future user
  packages will be discovered within the package via `[tool.test]`

The file header format, header directives (`// expect:` / `// skip:` / `// mode:`, RFC-036 §8.2) and
assertion conventions for `.yx` tests follow TEST_STANDARDS.md; judgment parsing is implemented by
the dual-runner shared `src/util/test_markers.rs` (Rust-side, governed by this standard).

---

## Unit Test Standards

### File Organization

**Rule 1.1**: The `tests/` directory for unit tests must be at the **same level** as the `mod.rs` of
the module being tested. `tests/` does not aggregate upward and does not cross-level summarize.

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——declare same-level tests/
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——pratt's own tests
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # parser module-level tests (not including pratt submodule content)
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

Key judgment criterion: **Whichever directory the `tests/` is placed in, that directory's `mod.rs`
must declare it with `#[cfg(test)] mod tests;`.**

**Rule 1.1 Supplement: No upward aggregation.** Subdirectory module tests must be placed in that
subdirectory's own `tests/`, and must not be aggregated into the parent `tests/`.

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

# ❌ Wrong: Aggregating emitter and codes tests into diagnostic/tests/
src/util/diagnostic/
└── tests/
    ├── mod.rs              # ❌ Forced to declare mod emitter; mod codes;
    ├── emitter/            # ❌ Should be in emitter/tests/
    └── codes/              # ❌ Should be in codes/tests/
```

#### Placement Rules for Single-File Module vs Directory Module Tests

**Core Difference**: The module's organization form determines the test placement location.

| Module Type            | Judgment Basis                                | Test Location                 | Example                                       |
| ---------------------- | --------------------------------------------- | ----------------------------- | --------------------------------------------- |
| **Directory Module**   | Has independent directory and `mod.rs`        | `tests/` under that directory | `inference/tests/`                            |
| **Single-File Module** | Only has `.rs` file, no independent directory | Parent module's `tests/`      | `overload.rs` → `typecheck/tests/overload.rs` |

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
│   ├── mod.rs                      # Declare single-file module tests
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
│   ├── mod.rs                      # #[cfg(test)] mod tests; ——declare same-level tests/
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
└── traits/                         # Removed (logic merged into types/trait_data.rs)
```

**Why are single-file module tests placed in the parent's `tests/`?**

Because single-file modules (like `overload.rs`) do not have their own `mod.rs`, so they cannot
declare `#[cfg(test)] mod tests;`. According to Rust's module system, test files must be declared by
some `mod.rs` to be compiled. Therefore, single-file module tests can only be declared by the parent
module's `mod.rs`, placed in the parent's `tests/` directory.

**Decision Flow**:

```
Encounter a module, determine where to place tests?
│
├── Is this module a directory (has mod.rs)?
│   └── Yes → Create tests/ under that directory, declared by that directory's mod.rs
│
├── Is this module a single file (only .rs)?
│   └── Yes → Tests placed in the parent's tests/ directory, declared by the parent's mod.rs
│
└── Not sure?
    └── Check whether there is an independent directory and mod.rs
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
#[cfg(test)]                        # ❌ Single-file module cannot declare this way
mod tests;                          # Because there is no overload/tests/ directory

# ✅ Correct approach: Tests placed in the parent's tests/
src/frontend/core/typecheck/
├── overload.rs                     # Source file
└── tests/
    └── overload.rs                 # Test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern—Do not write this way:**

```
# ❌ Wrong: Submodule tests collected at the parent level
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
├── mod.rs              # Only declare pub mod base; pub mod computation;
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——declare same-level tests/
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; ——declare same-level tests/
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**Why can't we aggregate upward?** Because Rust's module system requires `#[cfg(test)] mod tests;`
to determine the compilation of test files at the declaration point. If `types/mod.rs` declares
`mod tests;`, then the content of `types/tests/` is the private content of the `types` module—it
should not cross into the territory of `base` or `computation`. Each module's tests should be the
internal implementation details of that module, not the parent module's. This rule also applies to
module refactoring: when you split `types` into `base` and `computation`, the tests should also
follow the split modules, not stay in place. **The test directory does not mirror the source code
structure, but follows module boundaries.**

**Rule 1.2**: `tests/mod.rs` is only responsible for module declarations and re-exports, and does
not contain test functions.

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
the same-level `tests/` directory. **The inline form `mod tests { ... }` is forbidden, placing test
code directly inside the source file.**

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
        // Test code should not appear in source files
    }
}
```

**Why is inline forbidden?**

1. Single responsibility of source files: source files only contain implementation, test files only
   contain tests. Mixed together, modifying tests requires scrolling to the bottom of the file, and
   modifying implementation requires skipping over tests.
2. Clear module boundaries: the `tests/` directory is a physical boundary, making it immediately
   clear which modules have tests and which don't.
3. Refactoring safety: when modules are split, the `tests/` directory follows; inline tests need to
   be manually extracted from the source file.
4. Code review: source code changes and test changes are in separate files in the PR diff, not mixed
   together.

### Module Declaration Standards

**Rule 2.1**: The top of all test files must have a module-level doc comment `//!` explaining the
specification source covered by the tests (language specification section number + RFC number). If a
test does not reference any specification section, it means this code has no specification basis—it
should not exist.

```rust
//! Literal tests — based on language specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation
```

**Why must the specification be referenced?** Because the expected values for tests come from the
specification, not from "the output of the current code". If one day the code changes its output and
the test is updated accordingly, the test protects nothing. Only specification-anchored tests can
distinguish between "intentional breaking change" and "unintentional regression".

**Rule 2.2**: The `use` imports in the test module must be precise to specific types/functions, and
glob imports `use super::*` are forbidden.

```rust
// 🟢 Good — precise imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Garbage — others don't know what you're testing
use super::*;
```

### Naming Standards

**Rule 3.1**: Test function naming format is `test_<what>_<scenario>`, all lowercase with
underscores.

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, one
should know what is tested and what is expected. Numerical serial naming is forbidden.

```rust
// 🟢 Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Garbage — completely unknown what is tested
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**Rule 3.3**: Helper functions do not need the `test_` prefix and should use verbs or nouns to
describe their purpose.

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### Test Structure Standards (Arrange-Act-Assert)

**Rule 4.1**: Every test function must follow the three-segment structure: Arrange → Act → Assert,
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

**Rule 4.2**: Simple tests (a single call + a single assertion) may omit segment comments, but may
not exceed 5 lines of logic code. Tests exceeding 5 lines must explicitly mark the three segments.

### Helper Function Standards

**Rule 5.1**: Setup logic that appears 3 or more times must be extracted into a helper function.

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

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print sufficient context when
panicking. Inside test function bodies (`#[test] fn ...`), `unwrap()` can be used directly—on
failure, Rust automatically prints the line number; but when a helper function fails, the line
number points to the helper function definition, and the context at the call site is not visible.

```rust
// 🟢 Good — helper function prints source content on failure
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Garbage — on failure, you can't see which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, immediately after `use`
imports. If shared by multiple test modules, place them in `tests/mod.rs` and export with
`pub(crate)`.

### Assertion Style

**Rule 6.1**: For enum variant matching, prefer `assert!(matches!(...))`. `if let` + `panic!` is not
allowed.

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

**Rule 6.2**: Use `assert_eq!` for exact value comparison, and `assert!` for boolean assertions.
Using `assert!(a == b)` to replace `assert_eq!(a, b)` is forbidden.

**Rule 6.3**: All assertions must include custom error messages, unless the assertion itself fully
describes the reason for failure.

```rust
// 🟢 Good — can quickly locate the issue when assertion fails
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 Good — assert_eq! automatically prints value differences on failure, no extra message needed
assert_eq!(error_count, 0);

// 🔴 Garbage — on failure, you only know "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: The assertion order must be `assert_eq!(actual, expected)`, with the actual value
first and the expected value second.

### Anti-Pattern List

The following are prohibited writing patterns and their replacements:

| Anti-Pattern                                   | Issue                                                         | Replacement                                                                                   |
| ---------------------------------------------- | ------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `#[cfg(test)] mod tests { ... }` inline tests  | Source file bloat, blurry module boundaries, hard to refactor | Test code placed in independent `tests/` directory, declared with `mod tests;` (see Rule 1.4) |
| Tests accommodating wrong code behavior        | Conceal specification deviations, legitimize bugs             | Fix the code against the specification, keep the tests unchanged                              |
| Deriving test expected values from code output | Tests become "recorders of the current implementation"        | Derive expected values from the specification                                                 |
| Permanent `#[ignore]` markers                  | Hide rotting tests                                            | Fix or delete                                                                                 |
| `println!` debug output                        | Pollute test output                                           | Use `assert!` for explicit assertions                                                         |
| `thread::sleep`                                | Flaky + slow                                                  | Use synchronization mechanisms or mocks                                                       |
| Manipulating real filesystem in tests          | Slow and non-reproducible                                     | Use `tempfile`                                                                                |
| Relying on test execution order                | Flaky                                                         | Each test has independent setup                                                               |
| One test function exceeds 30 lines of logic    | Unreadable                                                    | Split the test or use helper functions                                                        |
| `unwrap()` in helper functions without context | Hard to locate                                                | Use `expect("why")` or custom panic (see Rule 5.2)                                            |
| Copy-pasted identical setup more than 3 times  | High modification cost                                        | Extract helper functions                                                                      |

---

## Integration Test Standards

### Test Organization

**Rule 7.1**: Integration tests are placed in the project root's `tests/` directory. The entry file
`tests/integration.rs` uses the `#[path]` attribute to include submodules.

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**Rule 7.2**: Each `tests/integration/*.rs` file corresponds to one test topic (compiler backends,
code generation, executor, etc.), and must not be mixed.

**Rule 7.3**: Integration tests must be performed through the project's public API. Directly
referencing `crate::` internal modules in integration tests is forbidden. Use the `yaoxiang::`
public path.

```rust
// 🟢 Good — through the public API
use yaoxiang::run;

// 🔴 Garbage — bypassing the public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests prefer inline source code strings. Only when the source code exceeds
30 lines should external fixture files be used (placed in `tests/fixtures/`).

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

**Rule 8.2**: Fixture files must end with the `.yx` extension, and the file name describes the test
intent.

### E2E Coverage Principles

**Rule 9.1**: The integration test for each language feature must cover three paths:

| Path       | Description                                                      |
| ---------- | ---------------------------------------------------------------- |
| Happy path | Legal input produces expected output                             |
| Error path | Illegal input produces clear error message (not a panic)         |
| Boundary   | Boundary values (empty input, maximum value, nested depth limit) |

**Rule 9.2**: Integration tests must not depend on the network, system environment variables, or
external services.

---

## Benchmark Test Standards

### Criterion.rs Usage Standards

**Rule 10.1**: Benchmark tests are uniformly placed in the `benches/` directory, with the entry file
`benches/lib.rs`. They are organized by test topic.

```
benches/
├── lib.rs              # Entry, defining criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # Cross-language comparison benchmark
├── parser.rs           # Parser benchmark
└── codegen.rs          # Code generation benchmark
```

**Rule 10.2**: Every benchmark function must include a module doc comment `//!` explaining the test
purpose and measurement metrics.

```rust
//! YaoXiang interpreter performance benchmark
//!
//! Measurement metrics: single iteration time (wall time)
//! Baseline: Rust native implementation
```

### Preventing Compiler Optimization

**Rule 11.1**: The tested output of all benchmark tests must be passed through
`criterion::black_box` to prevent compiler optimization elimination.

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

**Rule 11.2**: The input data for benchmark tests must be `const` or `lazy_static`, and must not be
dynamically generated inside the `iter` closure—otherwise, the total time of data generation + the
logic being tested is measured.

### Benchmark Grouping and Naming

**Rule 12.1**: Benchmark test naming format is `<tested module>_<scenario>`, all lowercase with
underscores. Consistent with the unit test naming rules.

**Rule 12.2**: `criterion_group!` must be used to logically group related benchmarks. All benchmarks
crammed into one group are forbidden.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Documentation Test Standards

### Usage Scenarios

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code
example in their documentation comments. This example is executed via `cargo test --doc`.

````rust
/// Tokenize the source code string into a Token sequence.
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

**Rule 13.2**: Code examples in documentation tests must compile and assertions must succeed.
Examples marked with `ignore` are not allowed, unless the example demonstrates a compile-time error.

````rust
/// ```ignore
/// // Demonstrating a compile-time error — can ignore
/// let x: int = "string";
/// ```
````

### Coverage Requirements

**Rule 14.1**: Documentation tests only need to cover the API's happy path. Boundary cases and error
paths are covered by unit tests.

**Rule 14.2**: Example code in documentation tests must be concise—no more than 10 lines. If the
example requires longer context, it indicates a problem with the API design.

---

## Property Test Standards

### Usage Scenarios

**Rule 15.1**: The following scenarios must use property tests (proptest or quickcheck) rather than
manually writing multiple boundary value cases:

| Scenario                                       | Example                                |
| ---------------------------------------------- | -------------------------------------- |
| Parser round-trip                              | `parse(pretty_print(ast)) == ast`      |
| Serialization/Deserialization                  | `deserialize(serialize(data)) == data` |
| Mathematical identity                          | `a + b == b + a`                       |
| Compiler optimization doesn't change semantics | `eval(code) == eval(optimize(code))`   |

**Rule 15.2**: Property tests use `proptest` as the primary property test framework (already
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

**Rule 16.1**: Every property test must have a clear property declaration—the comment states the
invariant being verified.

```rust
// Property: Any integer literal, after tokenize → tokens_to_string, produces the same value
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**Rule 16.2**: If a property test finds a failure, the `proptest` regression mechanism must be
used—add the failing input to the `proptest-regressions/` directory, do not manually write an
ordinary test to replace it.

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

**Rule 17.2**: Error handling paths (all `Err` branches) must be 100% covered. Error messages
visible to users must be verified by tests.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, the author must self-check the following items:

- [ ] `cargo test` all pass
- [ ] `cargo test --doc` all pass
- [ ] `cargo bench` has no performance regressions (if hot path changes are involved)
- [ ] New code meets coverage targets
- [ ] Test naming complies with naming standards
- [ ] Each test file declares the corresponding specification sections (Rule 2.1)
- [ ] Test expected values come from the specification definition, not "the output of the current
      code"
- [ ] No `#[ignore]` marked tests (unless with explicit issue number comment)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic messages)
- [ ] Commit message uses the `:white_check_mark: test:` type
- [ ] **The test expected values have not been modified because "code behavior does not match the
      specification"—what is changed is the code, not the test**
- [ ] **No inline tests** (`#[cfg(test)] mod tests { ... }` must be changed to `mod tests;` +
      independent file, see Rule 1.4)

**Rule 18.2**: Reviewers must reject PRs that contain the following issues:

- Only happy path tests, missing error paths
- Tests contain `thread::sleep` or depend on execution order
- Copy-pasted test code more than 3 times without extracting helper functions
- Test names do not comply with naming standards
- Permanent `#[ignore]` tests exist
- **Tests accommodating wrong code behavior** (modifying tests instead of code when the code does
  not match the specification)
- **Tests do not declare the corresponding specification sections** (see Rule 2.1)
- **Test expected values come from code output rather than specification definition** (tests derived
  by reverse engineering equal no tests)
- **Inline tests exist** (`#[cfg(test)] mod tests { ... }` instead of `mod tests;` + independent
  file, see Rule 1.4)
- Tests only verify "does not panic" without asserting specific behavior
- Failed tests that expose code bugs are deleted (instead of fixing the code and then seeing them
  turn green)

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

# Run benchmark tests
cargo bench

# Show test output (hidden by default)
cargo test -- --nocapture

# Single-threaded run (debug concurrency issues)
cargo test -- --test-threads=1

# Generate coverage report (requires cargo-llvm-cov)
cargo llvm-cov --html
```

### B. Commit Message Template

Test-related commits must follow the following template:

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

When creating a new test module, ensure the following files are included:

```
# Add tests under the src/<module>/ directory
src/<module>/tests/
├── mod.rs          # Module declaration + common helper functions
└── <subject>.rs    # Test file, named after the source file being tested

# Add integration tests under the tests/ directory
tests/
├── integration.rs   # Update: add #[path] declaration
└── integration/
    └── <topic>.rs   # New test file
```

### D. References

- [YaoXiang Language Specification](../reference/language-spec/index.md) —— **The authoritative
  source for tests**
- [Accepted RFCs](../design/rfc/index.md) —— **The authoritative source for design decisions**
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Standards](./commit-convention.md)
- [Project Contributing Guide](./contributing.md)

---

> 💡 **Remember**: Tests do not verify whether your code "can run"—they verify whether your code
> conforms to the specification. As the specification changes, tests follow the specification. When
> the code is wrong, fix the code, not the test. **Code serves the specification, tests guard the
> specification. The moment tests accommodate the code, you have lost all protection.**
