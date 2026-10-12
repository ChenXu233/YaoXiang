---
title: 'Test Specification'
description:
  Hard specification for writing tests in the YaoXiang project, defining the standards for unit
  tests, integration tests, benchmarks, doc tests, and property tests
---

# Test Specification

This document defines the hard specification for writing tests in the YaoXiang project. All
contributors must follow the rules below; violations will be required to be modified in Code Review.

---

## Table of Contents

- [General Rules](#general-rules)
- [yx Corpus and Library Test Layers](#yx-corpus-and-library-test-layers)
- [Unit Test Specification](#unit-test-specification)
- [Integration Test Specification](#integration-test-specification)
- [Benchmark Specification](#benchmark-specification)
- [Doc Test Specification](#doc-test-specification)
- [Property Test Specification](#property-test-specification)
- [Coverage Requirements](#coverage-requirements)
- [Appendix](#appendix)

---

## General Rules

### Scope

This specification applies to all Rust test code in the YaoXiang project, including:

| Test Type         | Location              | Framework                  |
| ----------------- | --------------------- | -------------------------- |
| Unit tests        | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| Integration tests | `tests/`              | `#[test]`                  |
| Benchmarks        | `benches/`            | Criterion.rs               |
| Doc tests         | API doc comments      | `cargo test --doc`         |
| Property tests    | Any test location     | proptest / quickcheck      |

### Core Principles

**Principle 0: The authoritative source for tests is the specification, not the code.** This is the
most important principle in this document. Tests verify whether the code complies with the
specification, not whether the code "runs with the current implementation". When a test discovers
that the code behavior does not match the specification, **fix the code, never fix the test**.

The specification documents are located at:

- `docs/src/reference/language-spec/` —— language core specification
- `docs/src/rfc/accepted/` —— accepted RFC design documents

The top of every test file must declare the corresponding specification section (see Rule 2.1). Any
developer should be able to take the specification document and cross-check it with the tests to
verify the correctness of the implementation. Conversely—if a piece of code has no corresponding
specification description, it should not exist, and certainly should not be tested.

```rust
// 🟢 Good — the test directly references the specification and verifies whether the code follows it
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

// 🔴 Garbage — the test accommodates the current implementation's behavior, instead of verifying the specification
#[test]
fn test_literal_1() {
    // Don't know which section of the specification this code corresponds to
    // If parse_literal returns a wrong value, this test will "pass green"
    // because it only verifies that the function does not panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and find that the code behavior does not match the specification. You
have two options:

| Wrong Approach                                                       | Correct Approach                                               |
| -------------------------------------------------------------------- | -------------------------------------------------------------- |
| Modify the test to make it "pass"                                    | Modify the code to make behavior conform to the specification  |
| Add `#[ignore]` to the test                                          | Fix the code implementation immediately                        |
| Add special conditional branches in the test to accommodate the code | Remove the branch and let the test expose the problem directly |

Remember: **Red light = code is wrong, not the test.** (Unless your test itself has a bug, that's a
different story.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of
the code under test by reading the tests, without needing extra comments or external documentation.

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

**Principle 2: Zero tolerance for flaky tests.** Tests must be repeatable in any environment. Tests
that depend on random numbers, system time, or thread scheduling order must use fixed seeds or be
replaced with mocks.

**Principle 3: One test, one thing.** If a test name needs to connect multiple behaviors with "and",
split it into multiple tests.

```rust
// 🟢 Good — each test verifies only one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Garbage — one test stuffed with too many unrelated things
#[test]
fn test_parser() {
    // Test tokenize, test parse, test typecheck, test codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring internal implementation should not
cause test failures. If changing one line of implementation code causes 10 tests to fail, your tests
are written wrong.

But there is a key distinction here: **the definition of "behavior" comes from the specification,
not from the current code's performance.** If the code changes behavior (i.e., new behavior that
does not match the specification), the test must fail. If you cannot do this, your test is an
"accommodating test"—it lets bugs run rampant.

```
Specification (language-spec.md / RFC)  ──defines──►  Expected behavior  ──drives──►  Tests
                                                  │
Current code  ──implements──►  Actual behavior  ──compares──►  Test result

If actual behavior ≠ expected behavior:
  Test must fail (red)  ──►  Fix code  ──►  Test passes (green)

If actual behavior = expected behavior (but implementation is ugly):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is what Principle 4 means
```

**Principle 5: Do not write fallback/compatibility/condition-activated test code.** The test
environment is one you can fully control. If you need `#[cfg(not(ci))]` to skip a certain test, it
means there is a fundamental problem with that test's design.

### Terminology

| Term             | Definition                                                                       |
| ---------------- | -------------------------------------------------------------------------------- |
| Unit test        | Tests a single function or module behavior, no external system dependency        |
| Integration test | Tests collaboration of multiple modules through public API or command-line entry |
| Benchmark        | Measures code performance, detects performance regressions                       |
| Doc test         | Executable code examples embedded in doc comments                                |
| Property test    | Tests that verify invariants (properties) based on random inputs                 |

### Relation to Commit Convention

All test-related commits must use the `:white_check_mark: test:` type, refer to
[Commit Convention](commit-convention.md).

```
:white_check_mark: test(parser): Add tests for infix expressions in Pratt parser
:white_check_mark: test(codegen): Complete IR generation tests for switch statements
```

---

## yx Corpus and Library Test Layers

This specification constrains **Rust-side test code**. Tests for the YaoXiang language itself (`.yx`
corpus and library tests) are divided into two layers based on the object under test. The system
design and judgment contract belong to RFC-036 (§7 Suite Collection / §8 Negative Three Layers / §9
Test System Layering), and the corpus writing details belong to `tests/yaoxiang/TEST_STANDARDS.md`:

- **Language usability corpus** (`tests/yaoxiang/`)——the object under test is the language itself;
  std is only used as an assertion tool. Within the corpus, judgments are divided into three
  categories based on the layer where failures occur: behavior tests / compile-time rejection tests
  / runtime failure tests
- **Library tests** (ship with the library)——the object under test is the library's public API
  contract; std's yx-level tests are located at `src/std/tests/`, and future user package tests are
  discovered within the package via `[tool.test]`

The file header format of `.yx` tests, the header directives (`// expect:` / `// skip:` /
`// mode:`, RFC-036 §8.2), and the assertion conventions follow TEST_STANDARDS.md; judgment parsing
is implemented by `src/util/test_markers.rs` shared by both runners (Rust side, constrained by this
specification).

---

## Unit Test Specification

### File Organization

**Rule 1.1**: The unit test `tests/` directory must be **at the same level** as the `mod.rs` of the
module under test. `tests/` does not aggregate upward or summarize across levels.

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
└── tests/              # parser module-level tests (does not include pratt submodule content)
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

Key judgment criterion: **wherever the `tests/` directory is placed, that directory's `mod.rs` must
use `#[cfg(test)] mod tests;` to declare it.**

**Rule 1.1 Supplement: Aggregation upward is forbidden.** Tests for submodule directories must be
placed in that submodule's own `tests/`, and must not be aggregated to the parent-level `tests/`.

| Module Type                      | Test Location                 | Example                                      |
| -------------------------------- | ----------------------------- | -------------------------------------------- |
| Directory module (with `mod.rs`) | `tests/` under that directory | `emitter/tests/`, `codes/tests/`             |
| Single-file module (only `.rs`)  | Parent's `tests/`             | `session.rs` → `diagnostic/tests/session.rs` |

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

# ❌ Wrong: aggregate emitter and codes' tests into diagnostic/tests/
src/util/diagnostic/
└── tests/
    ├── mod.rs              # ❌ Forced to declare mod emitter; mod codes;
    ├── emitter/            # ❌ Should be in emitter/tests/
    └── codes/              # ❌ Should be in codes/tests/
```

#### Test Placement Rules for Single-File Modules vs Directory Modules

**Core Difference**: The organization of a module determines the placement of its tests.

| Module Type            | Judgment Basis                                | Test Location                 | Example                                       |
| ---------------------- | --------------------------------------------- | ----------------------------- | --------------------------------------------- |
| **Directory module**   | Has an independent directory and `mod.rs`     | `tests/` under that directory | `inference/tests/`                            |
| **Single-file module** | Has only `.rs` file, no independent directory | Parent module's `tests/`      | `overload.rs` → `typecheck/tests/overload.rs` |

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
│       ├── expressions.rs          # tests for expressions.rs
│       ├── statements.rs           # tests for statements.rs
│       └── ...
│
└── traits/                         # removed (logic merged into types/trait_data.rs)
```

**Why are single-file module tests placed in the parent's `tests/`?**

Because single-file modules (e.g., `overload.rs`) do not have their own `mod.rs`, they cannot
declare `#[cfg(test)] mod tests;`. According to Rust's module system, test files must be declared by
some `mod.rs` to be compiled. Therefore, tests for single-file modules can only be declared by the
parent module's `mod.rs` and placed in the parent's `tests/` directory.

**Judgment Flow**:

```
Encounter a module, determine where to put the tests?
│
├── Is this module a directory (with mod.rs)?
│   └── Yes → Create tests/ under that directory, declared by that directory's mod.rs
│
├── Is this module a single file (only .rs)?
│   └── Yes → Place tests in parent's tests/ directory, declared by parent's mod.rs
│
└── Not sure?
    └── Check if there is an independent directory and mod.rs
```

**Common Mistakes**:

```
# ❌ Mistake 1: Create an independent tests/ directory for a single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ Should not create a directory for a single-file module
    └── tests/
        └── overload.rs

# ❌ Mistake 2: Declare #[cfg(test)] mod tests; inside a single-file module
# overload.rs
#[cfg(test)]                        # ❌ A single-file module cannot declare this way
mod tests;                          # because there is no overload/tests/ directory

# ✅ Correct approach: place tests in the parent's tests/
src/frontend/core/typecheck/
├── overload.rs                     # source file
└── tests/
    └── overload.rs                 # test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern——do not write it this way:**

```
# ❌ Wrong: Submodule tests consolidated at the parent level
src/frontend/core/types/
├── mod.rs              # should only declare base and computation
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
# ✅ Correct approach: each module's tests are independent
src/frontend/core/types/
├── mod.rs              # only declare pub mod base; pub mod computation;
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

**Why is upward aggregation not allowed?** Because Rust's module system requires
`#[cfg(test)] mod tests;` to determine the compilation of test files at the declaration site. If
`types/mod.rs` declares `mod tests;`, then the content of `types/tests/` is the private content of
the `types` module—it should not reach into the territory of `base` or `computation`. Each module's
tests should be internal implementation details of that module, not the parent module's. This rule
also applies to module refactoring: when you split `types` into `base` and `computation`, the tests
should follow the split modules, not stay where they are. **The test directory does not mirror the
source structure, but follows the module boundaries.**

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

**Rule 1.3**: Each test file corresponds to only one source file. Mixing tests for multiple source
modules in one file is not allowed.

**Rule 1.4**: Test declarations must use the file form `mod tests;` (with a semicolon), pointing to
the same-level `tests/` directory. **The inline form `mod tests { ... }` that embeds test code
directly in the source file is forbidden.**

```rust
// ✅ Correct — file form declaration, test code is in independent files
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

1. Single source file responsibility: source files only contain implementation, test files only
   contain tests. When mixed, modifying tests requires scrolling to the bottom of the file,
   modifying implementation requires skipping past tests.
2. Clear module boundaries: the `tests/` directory is a physical boundary, making it immediately
   clear which modules have tests and which don't.
3. Refactoring safety: when modules are split, the `tests/` directory follows along; inline tests
   need to be manually peeled out from the source file.
4. Code review: in PR diffs, source code changes and test changes are separate files, not mixed
   together.

### Module Declaration Specification

**Rule 2.1**: All test files must have a module-level doc comment `//!` at the top, indicating the
specification source (language specification section number + RFC number) covered by the tests. If a
test does not reference any specification section, it means this code has no specification basis—it
should not exist.

```rust
//! Literal tests — based on language specification §2.6
//!
//! §2.6.1: Integer Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation
```

**Why must the specification be referenced?** Because the expected values of tests come from the
specification, not from "the current code's output". If one day the code's output changes and the
test is updated accordingly, then the test protected nothing. Only specification-anchored tests can
distinguish between "intentional breaking change" and "unintentional regression".

**Rule 2.2**: Test module `use` imports must be specific to concrete types/functions, glob imports
`use super::*` are forbidden.

```rust
// 🟢 Good — specific imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Garbage — others don't know what you're testing
use super::*;
```

### Naming Specification

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

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, you
should know what is being tested and what is expected. Numeric serial naming is forbidden.

```rust
// 🟢 Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Garbage — completely unknown what's being tested
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**Rule 3.3**: Helper functions do not need the `test_` prefix, they should describe their purpose
with verbs or nouns.

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### Test Structure Specification (Arrange-Act-Assert)

**Rule 4.1**: Each test function must follow the three-section structure: Arrange → Act → Assert,
separated by blank lines.

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

**Rule 4.2**: Simple tests (single call + single assertion) may omit section comments, but cannot
exceed 5 lines of logical code. Tests exceeding 5 lines must explicitly mark the three sections.

### Helper Function Specification

**Rule 5.1**: Setup logic that repeats 3 times or more must be extracted into a helper function.

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

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print enough context on panic. Inside
the test function body (`#[test] fn ...`), `unwrap()` is allowed directly—on failure Rust
automatically prints the line number; but when failing in a helper function, the line number points
to the helper function definition, and the calling context is not visible.

```rust
// 🟢 Good — print source content when helper function fails
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Garbage — on failure you can't see which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, right after `use`
imports. If shared by multiple test modules, place in `tests/mod.rs` and export as `pub(crate)`.

### Assertion Style

**Rule 6.1**: Enum variant matching should use `assert!(matches!(...))` preferentially, `if let` +
`panic!` is not allowed.

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

**Rule 6.2**: Use `assert_eq!` for exact value comparisons, use `assert!` for boolean assertions.
`assert!(a == b)` as a replacement for `assert_eq!(a, b)` is forbidden.

**Rule 6.3**: All assertions must have a custom error message, unless the assertion itself fully
describes the failure reason.

```rust
// 🟢 Good — quickly locate on assertion failure
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 Good — assert_eq! automatically prints value differences on failure, no extra message needed
assert_eq!(error_count, 0);

// 🔴 Garbage — on failure only "assertion failed" is known
assert!(state.infix_info().is_some());
```

**Rule 6.4**: Assertion order must be `assert_eq!(actual, expected)`, actual value first, expected
value second.

### Anti-pattern List

The following are forbidden patterns and their replacements:

| Anti-pattern                                   | Problem                                                              | Replacement                                                                                 |
| ---------------------------------------------- | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `#[cfg(test)] mod tests { ... }` inline tests  | Source file bloat, blurred module boundaries, refactoring difficulty | Place test code in independent `tests/` directory, declare with `mod tests;` (see Rule 1.4) |
| Test accommodates code's wrong behavior        | Conceals specification deviations, legalizes bugs                    | Fix code against the specification, keep tests unchanged                                    |
| Derive test expectations from code output      | Test becomes "tape recorder of current implementation"               | Derive expected values from the specification                                               |
| Permanent `#[ignore]` marking                  | Hides rotting tests                                                  | Fix or delete                                                                               |
| `println!` debug output                        | Pollutes test output                                                 | Use `assert!` for clear assertions                                                          |
| `thread::sleep`                                | Flaky + slow                                                         | Use synchronization mechanisms or mocks                                                     |
| Manipulating real file system in tests         | Slow and unrepeatable                                                | Use `tempfile`                                                                              |
| Depending on test execution order              | Flaky                                                                | Each test has independent setup                                                             |
| A test function exceeding 30 lines of logic    | No one can understand it                                             | Split tests or use helper functions                                                         |
| `unwrap()` in helper functions without context | Hard to locate                                                       | Use `expect("why")` or custom panic (see Rule 5.2)                                          |
| Copy-paste the same setup 3+ times             | High modification cost                                               | Extract helper functions                                                                    |

---

## Integration Test Specification

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

**Rule 7.2**: Each `tests/integration/*.rs` file corresponds to one test topic (compiler backends,
code generation, executor, etc.), and must not be mixed.

**Rule 7.3**: Integration tests must be conducted through the project's public API. Direct reference
to `crate::` internal modules in integration tests is forbidden. Use the `yaoxiang::` public path.

```rust
// 🟢 Good — through the public API
use yaoxiang::run;

// 🔴 Garbage — bypasses the public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests prefer inline source strings. Only when the source exceeds 30 lines
should external fixture files (placed in `tests/fixtures/`) be used.

```rust
#[test]
fn test_fibonacci() {
    run_ok(
        r#"
        // Inline source runs in Script mode: top-level statements are the program body (SPEC syntax §3.11).
        // A typed main without explicit call is just a lazy lambda——the loop body never executes.
        main: () -> Void = {
            mut a = 0
            mut b = 1
            while a < 100 {
                mut next = a + b
                a = b
                b = next
            }
        }
        main()
        "#,
    );
}
```

**Rule 8.2**: Fixture files must end with the `.yx` extension, and the file name describes the test
intent.

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

## Benchmark Specification

### Criterion.rs Usage Specification

**Rule 10.1**: Benchmarks are uniformly placed in the `benches/` directory, with the entry file as
`benches/lib.rs`. Files are separated by test topic.

```
benches/
├── lib.rs              # entry, define criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # cross-language comparison benchmark
├── parser.rs           # parser benchmark
└── codegen.rs          # code generation benchmark
```

**Rule 10.2**: Each benchmark function must include a module-level doc comment `//!` explaining the
test purpose and measurement metrics.

```rust
//! YaoXiang interpreter performance benchmarks
//!
//! Measurement metric: single iteration time (wall time)
//! Baseline: Rust native implementation
```

### Preventing Compiler Optimizations

**Rule 11.1**: All benchmark outputs under test must use `criterion::black_box` to prevent the
compiler from optimizing them away.

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

**Rule 11.2**: The input data for the benchmark must be `const` or `lazy_static`, and must not be
dynamically generated inside the `iter` closure—otherwise what is measured is the total time of data
generation + the logic under test.

### Benchmark Grouping and Naming

**Rule 12.1**: The benchmark naming format is `<module under test>_<scenario>`, all lowercase with
underscores. Consistent with unit test naming rules.

**Rule 12.2**: You must use `criterion_group!` to logically group related benchmarks. Squeezing all
benchmarks into one group is forbidden.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Doc Test Specification

### Use Cases

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code
example in their doc comments. The example is executed via `cargo test --doc`.

````rust
/// Tokenize a source string into a Token sequence.
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

**Rule 13.2**: The code example in the doc test must compile and pass assertions. Examples marked
with `ignore` are not allowed, unless the example demonstrates a compile-time error.

````rust
/// ```ignore
/// // Demonstrate compile-time error——can be ignored
/// let x: int = "string";
/// ```
````

### Coverage Requirements

**Rule 14.1**: Doc tests only need to cover the happy path of the API. Boundary cases and error
paths are covered by unit tests.

**Rule 14.2**: The example code in doc tests must be concise—no more than 10 lines. If the example
requires longer context, the API design has a problem.

---

## Property Test Specification

### Use Cases

**Rule 15.1**: The following scenarios must use property tests (proptest or quickcheck) instead of
hand-written multiple boundary value cases:

| Scenario                                  | Example                                |
| ----------------------------------------- | -------------------------------------- |
| Parser round-trip                         | `parse(pretty_print(ast)) == ast`      |
| Serialization/deserialization             | `deserialize(serialize(data)) == data` |
| Mathematical identity                     | `a + b == b + a`                       |
| Compiler optimization preserves semantics | `eval(code) == eval(optimize(code))`   |

**Rule 15.2**: Property tests use `proptest` as the primary property testing framework (already
declared in `Cargo.toml` under `dev-dependencies`).

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

**Rule 16.1**: Each property test must have a clear property declaration—the invariant being
verified must be stated in a comment.

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

**Rule 16.2**: If a property test discovers a failure, the regression mechanism of `proptest` must
be used—add the failing input to the `proptest-regressions/` directory, and do not manually write an
ordinary test as a replacement.

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
visible to users must be verified by tests.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, the author must self-check the following items:

- [ ] `cargo test` all pass
- [ ] `cargo test --doc` all pass
- [ ] `cargo bench` has no performance regression (if hot path changes are involved)
- [ ] New code meets coverage targets
- [ ] Test naming follows the naming specification
- [ ] Each test file declares the corresponding specification section (Rule 2.1)
- [ ] Test expected values come from the specification definition, not "current code's output"
- [ ] No tests marked with `#[ignore]` (unless there is a clear issue number comment)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic message)
- [ ] Commit message uses the `:white_check_mark: test:` type
- [ ] **No test expected values have been modified because "code behavior does not match the
      specification"—what is changed is the code, not the test**
- [ ] **No inline tests** (`#[cfg(test)] mod tests { ... }` must be changed to `mod tests;` +
      independent file, see Rule 1.4)

**Rule 18.2**: Reviewers must reject PRs that contain the following issues:

- Only happy path tests, missing error paths
- Tests contain `thread::sleep` or depend on execution order
- Copy-pasted test code more than 3 times without extracting helper functions
- Test names do not follow the naming specification
- Existence of permanently `#[ignore]` tests
- **Test accommodates code's wrong behavior** (when code does not match the specification, modifying
  the test rather than the code)
- **Test does not declare the corresponding specification section** (see Rule 2.1)
- **Test expected values come from code output rather than specification definition**
  (reverse-engineered tests are equivalent to no test)
- **Existence of inline tests** (`#[cfg(test)] mod tests { ... }` rather than `mod tests;` +
  independent file, see Rule 1.4)
- Tests only verify "does not panic" without asserting specific behavior
- Deleted failing tests that exposed code bugs (instead of fixing the code and then seeing them turn
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

# Run specific tests (filter by name)
cargo test test_parse_expr

# Run benchmarks
cargo bench

# Show test output (stdout hidden by default)
cargo test -- --nocapture

# Run single-threaded (for troubleshooting concurrency issues)
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
:white_check_mark: test(parser): Add infix operator tests for Pratt parser

Covered scenarios:
- Arithmetic operator precedence (+, -, *, /, %)
- Comparison operator chaining (1 < x < 10)
- Logical operator short-circuit
- Assignment operator right-associativity
```

### C. New Test File Checklist

When creating a new test module, make sure to include the following files:

```
# Add tests under src/<module>/
src/<module>/tests/
├── mod.rs          # module declaration + public helper functions
└── <subject>.rs    # test file, named after the source file under test

# Add integration tests under tests/
tests/
├── integration.rs   # Update: add #[path] declaration
└── integration/
    └── <topic>.rs   # new test file
```

### D. References

- [YaoXiang Language Specification](../reference/language-spec/index.md) —— **Authoritative source
  for tests**
- [Accepted RFCs](../rfc/index.md) —— **Authoritative source for design decisions**
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Convention](commit-convention.md)
- [Project Contributing Guide](contributing.md)

---

> 💡 **Remember**: Tests do not verify whether your code "runs"—they verify whether your code
> complies with the specification. The specification changes, and tests change with it. When the
> code is wrong, fix the code, not the test. **The code serves the specification, and the tests
> guard the specification. The moment a test accommodates the code, you lose all protection.**
