---
title: 'Test Writing Specification'
description:
  Hard specification for test writing in the YaoXiang project, defining the writing standards for
  unit tests, integration tests, benchmarks, doc tests, and property tests
---

# Test Writing Specification

This document defines the hard specification for test writing in the YaoXiang project. All
contributors must follow the rules below; violations will be required to be modified during Code
Review.

---

## Table of Contents

- [General Principles](#general-principles)
- [yx Corpus and Library Test Layers](#yx-corpus-and-library-test-layers)
- [Unit Test Specification](#unit-test-specification)
- [Integration Test Specification](#integration-test-specification)
- [Benchmark Specification](#benchmark-specification)
- [Doc Test Specification](#doc-test-specification)
- [Property Test Specification](#property-test-specification)
- [Coverage Requirements](#coverage-requirements)
- [Appendix](#appendix)

---

## General Principles

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

**Principle 0: The authoritative source of tests is the specification, not the code.** This is the
most important principle in this document. Tests verify whether the code conforms to the
specification, not whether the code "runs with the current implementation." When a test discovers
that code behavior is inconsistent with the specification, **fix the code, never fix the test**.

Specification files are located at:

- `docs/src/reference/language-spec/` — Language core specification
- `docs/src/rfc/accepted/` — Accepted RFC design documents

Every test file must declare the corresponding specification section at the top (see Rule 2.1). Any
developer should be able to take the specification document and cross-check it against the tests to
verify correctness of the implementation. Conversely — if a piece of code has no corresponding
specification description, it should not exist, much less be tested.

```rust
// 🟢 Good — the test directly references the specification and verifies whether the code follows it
//! Literal tests — based on language specification §2.6
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

// 🔴 Garbage — the test accommodates the implementation behavior of the current code instead of verifying the specification
#[test]
fn test_literal_1() {
    // Don't know which section of the specification this code corresponds to
    // If parse_literal returns the wrong value, this test would "pass green"
    // because it only verifies that the function does not panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You wrote a test and found the code behavior doesn't match the specification. You have
two choices:

| Wrong approach                                                       | Correct approach                                               |
| -------------------------------------------------------------------- | -------------------------------------------------------------- |
| Modify the test to make it "pass"                                    | Modify the code so the behavior conforms to the specification  |
| Add `#[ignore]` to the test                                          | Fix the code implementation immediately                        |
| Add special conditional branches to the test to accommodate the code | Remove the branch and let the test expose the problem directly |

Remember: **Red light = the code is wrong, not the test.** (Unless the test itself has a bug, but
that's a different story.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of
the code under test by reading the tests, without needing extra comments or external documentation.

```rust
// 🟢 Good — the test name states what is being tested and what is expected
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

**Principle 3: One test tests one thing.** If a test name needs to connect multiple behaviors with
"and", split it into multiple tests.

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
cause tests to fail. If changing one line of implementation code causes 10 tests to break, your
tests are written wrong.

But there's a key distinction here: **the definition of "behavior" comes from the specification, not
from the current code's performance.** If the code changes behavior (i.e., new behavior inconsistent
with the specification), the test must fail. If you can't do this, your test is a "test that
accommodates the code" — it lets bugs drive right in.

```
Specification (language-spec.md / RFC)  ──defines──►  Expected behavior  ──drives──►  Test
                                           │
Current code  ──implements──►  Actual behavior  ──compared to──►  Test result

If actual behavior ≠ expected behavior:
  Test must fail (red)  ──►  Fix code  ──►  Test passes (green)

If actual behavior = expected behavior (but implementation is bad):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is the meaning of Principle 4
```

**Principle 5: Do not write fallback/compatibility/specific-mode-active test code.** The test
environment is one you can fully control. If you need `#[cfg(not(ci))]` to skip a test, it indicates
a fundamental problem with the test design.

### Terminology

| Term             | Definition                                                                        |
| ---------------- | --------------------------------------------------------------------------------- |
| Unit test        | Tests a single function or module's behavior, no external system dependency       |
| Integration test | Tests collaboration of multiple modules, through public API or command-line entry |
| Benchmark        | Measures code performance, detects performance regressions                        |
| Doc test         | Executable code examples embedded in doc comments                                 |
| Property test    | Tests that verify invariants (properties) based on random inputs                  |

### Relation to Commit Specification

All test-related commits must use the `:white_check_mark: test:` type, refer to
[Commit Specification](commit-convention.md).

```
:white_check_mark: test(parser): add Pratt parser infix expression tests
:white_check_mark: test(codegen): complete switch statement IR generation tests
```

---

## yx Corpus and Library Test Layers

This specification constrains **Rust-side test code**. Tests for the YaoXiang language itself (`.yx`
corpus and library tests) are divided into two layers by the object under test. System design and
judgment contracts belong to RFC-036 (§7 suite collection / §8 three negative layers / §9 test
system layering), and corpus writing details belong to `tests/yaoxiang/TEST_STANDARDS.md`:

- **Language usability corpus** (`tests/yaoxiang/`) — The object under test is the language itself;
  std is only used as an assertion tool. Within the corpus, judgments are divided into three
  categories by where failures occur: behavior tests / compile-time rejection tests / runtime
  failure tests
- **Library tests** (travel with the library) — The object under test is the library's public API
  contract; std's yx-level tests are located at `src/std/tests/`, and future user packages' tests
  will be discovered via `[tool.test]` within the package

The file header format of `.yx` tests, header directives (`// expect:` / `// skip:` / `// mode:`,
RFC-036 §8.2) and assertion conventions follow TEST_STANDARDS.md; judgment parsing is implemented by
the dual-runner shared `src/util/test_markers.rs` (Rust-side, constrained by this specification).

---

## Unit Test Specification

### File Organization

**Rule 1.1**: The `tests/` directory of a unit test must be at the same level as the `mod.rs` of the
module under test. `tests/` does not aggregate upward, nor does it aggregate across levels.

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——declare sibling tests/
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

Key judgment criterion: **wherever `tests/` is placed, that directory's `mod.rs` must declare it
with `#[cfg(test)] mod tests;`.**

**Rule 1.1 Supplement: Aggregation upward is prohibited.** Tests for submodule must be placed in
that submodule's own `tests/`, and must not be aggregated into the parent level's `tests/`.

| Module type                     | Test location                 | Example                                      |
| ------------------------------- | ----------------------------- | -------------------------------------------- |
| Directory module (has `mod.rs`) | `tests/` under that directory | `emitter/tests/`, `codes/tests/`             |
| Single-file module (only `.rs`) | Parent level's `tests/`       | `session.rs` → `diagnostic/tests/session.rs` |

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

#### Single-file Module vs Directory Module Test Placement Rules

**Core difference**: The module's organization form determines where tests are placed.

| Module type            | Basis for judgment                        | Test location                 | Example                                       |
| ---------------------- | ----------------------------------------- | ----------------------------- | --------------------------------------------- |
| **Directory module**   | Has independent directory and `mod.rs`    | `tests/` under that directory | `inference/tests/`                            |
| **Single-file module** | Only `.rs` file, no independent directory | Parent module's `tests/`      | `overload.rs` → `typecheck/tests/overload.rs` |

**Detailed description**:

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
│   ├── mod.rs                      # #[cfg(test)] mod tests; ——declare sibling tests/
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
└── traits/                         # Deleted (logic merged into types/trait_data.rs)
```

**Why are single-file module tests placed in the parent's `tests/`?**

Because a single-file module (like `overload.rs`) does not have its own `mod.rs`, and cannot declare
`#[cfg(test)] mod tests;`. According to Rust's module system, test files must be declared by some
`mod.rs` to be compiled. Therefore, single-file module tests can only be declared by the parent
module's `mod.rs` and placed in the parent's `tests/` directory.

**Judgment flow**:

```
Encounter a module, decide where tests go?
│
├── Is this module a directory (has mod.rs)?
│   └── Yes → Create tests/ under that directory, declared by that directory's mod.rs
│
├── Is this module a single file (only .rs)?
│   └── Yes → Tests go in the parent's tests/ directory, declared by the parent's mod.rs
│
└── Not sure?
    └── Check if there's an independent directory and mod.rs
```

**Common mistakes**:

```
# ❌ Mistake 1: Create independent tests/ directory for a single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ Should not create directory for single-file module
    └── tests/
        └── overload.rs

# ❌ Mistake 2: Declare #[cfg(test)] mod tests; inside a single-file module
# overload.rs
#[cfg(test)]                        # ❌ Single-file module cannot declare this way
mod tests;                          # Because there's no overload/tests/ directory

# ✅ Correct approach: tests go in the parent's tests/
src/frontend/core/typecheck/
├── overload.rs                     # Source file
└── tests/
    └── overload.rs                 # Test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern — do not write like this:**

```
# ❌ Wrong: concentrate submodule tests into the parent level
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
# ✅ Correct approach: each module's tests are independent
src/frontend/core/types/
├── mod.rs              # Only declare pub mod base; pub mod computation;
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——declare sibling tests/
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; ——declare sibling tests/
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**Why can tests not aggregate upward?** Because Rust's module system requires
`#[cfg(test)] mod tests;` to determine the compilation of test files at the declaration point. If
`types/mod.rs` declares `mod tests;`, then the content of `types/tests/` is the private content of
the `types` module — it should not encroach into `base` or `computation`'s territory. Each module's
tests should be the internal implementation details of that module, not the parent module's. This
rule also applies to module refactoring: when you split `types` into `base` and `computation`, the
tests should also follow the split modules, not stay in place. **The test directory does not mirror
the source code structure, but follows module boundaries.**

**Rule 1.2**: `tests/mod.rs` is only responsible for module declaration and re-export, and does not
contain test functions.

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
are not allowed to be mixed in a single file.

**Rule 1.4**: Test declarations must use the file form `mod tests;` (with semicolon), pointing to
the sibling `tests/` directory. **The inline form `mod tests { ... }` is prohibited from placing
test code directly inside the source file.**

```rust
// ✅ Correct — file form declaration, test code in independent files
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests;

// 🔴 Forbidden — inline form, test code parasitizes inside the source file
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

1. Single responsibility for source files: source files only contain implementation, test files only
   contain tests. Mixed together, modifying tests requires scrolling to the bottom of the file,
   modifying implementation requires skipping past tests.
2. Clear module boundaries: the `tests/` directory is a physical boundary, making it clear at a
   glance which modules have tests and which don't.
3. Refactoring safety: when splitting modules, the `tests/` directory follows along; inline tests
   need to be manually peeled from the source file.
4. Code review: in PR diffs, source code changes and test changes are separate files, not mixed
   together.

### Module Declaration Specification

**Rule 2.1**: All test files must have a module-level doc comment `//!` at the top, indicating the
source of the specification (language specification section number + RFC number) covered by the
test. If a test does not reference any specification section, it indicates this code has no
specification basis — it should not exist.

```rust
//! Literal tests — based on language specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floats (with decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation
```

**Why must the specification be referenced?** Because test expectations come from the specification,
not from "the output of the current code." If one day the code changes its output and the test is
updated accordingly, the test protects nothing. Only specification-anchored tests can distinguish
between "intentional breaking change" and "unintentional regression."

**Rule 2.2**: The `use` imports in test modules must be specific to concrete types/functions; glob
imports `use super::*` are prohibited.

```rust
// 🟢 Good — precise import
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Garbage — others don't know what you're testing
use super::*;
```

### Naming Specification

**Rule 3.1**: Test function naming follows the format `test_<what>_<scenario>`, all lowercase with
underscore separation.

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, one
should know what is being tested and what is expected. Numeric ordering naming is prohibited.

```rust
// 🟢 Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Garbage — completely unknown what's being tested
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**Rule 3.3**: Helper functions do not need the `test_` prefix; they should use verbs or nouns to
describe their purpose.

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### Test Structure Specification (Arrange-Act-Assert)

**Rule 4.1**: Every test function must follow the three-part structure: Arrange → Act → Assert, with
blank lines separating the three parts.

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

**Rule 4.2**: Simple tests (single call + single assertion) may omit the section comments, but
cannot exceed 5 lines of logic code. Tests exceeding 5 lines must explicitly mark the three parts.

### Helper Function Specification

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

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print sufficient context when
panicking. Inside test function bodies (`#[test] fn ...`) direct `unwrap()` is allowed — Rust
automatically prints the line number on failure; but when a helper function fails, the line number
points to the helper function's definition, making it impossible to see the call context.

```rust
// 🟢 Good — helper function prints source content on failure
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Garbage — on failure, you can't see which source file caused the issue
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, immediately following
the `use` imports. If shared by multiple test modules, place them in `tests/mod.rs` and export as
`pub(crate)`.

### Assertion Style

**Rule 6.1**: Enum variant matching should prefer `assert!(matches!(...))`; `if let` + `panic!` is
not allowed.

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

**Rule 6.2**: For exact value comparison use `assert_eq!`, for boolean assertions use `assert!`.
Using `assert!(a == b)` in place of `assert_eq!(a, b)` is prohibited.

**Rule 6.3**: All assertions must have custom error messages, unless the assertion itself fully
describes the failure reason.

```rust
// 🟢 Good — quickly locate on failure
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 Good — assert_eq! automatically prints value difference on failure, no extra message needed
assert_eq!(error_count, 0);

// 🔴 Garbage — on failure, all you know is "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: The assertion order must be `assert_eq!(actual, expected)`, with the actual value
first and the expected value second.

### Anti-pattern List

The following are prohibited patterns and their replacements:

| Anti-pattern                                        | Problem                                                              | Replacement                                                                                    |
| --------------------------------------------------- | -------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `#[cfg(test)] mod tests { ... }` inline tests       | Source file bloat, blurred module boundaries, refactoring difficulty | Place test code in an independent `tests/` directory, declare with `mod tests;` (see Rule 1.4) |
| Tests accommodating code's wrong behavior           | Masking specification deviations, legitimizing bugs                  | Fix the code against the specification, keep tests unchanged                                   |
| Reverse-deriving test expectations from code output | Tests become "a recorder of the current implementation"              | Derive expectations from the specification                                                     |
| Permanent `#[ignore]` markers                       | Hiding rotten tests                                                  | Fix or delete                                                                                  |
| `println!` debug output                             | Pollutes test output                                                 | Use `assert!` for explicit assertions                                                          |
| `thread::sleep`                                     | Flaky failures + slow                                                | Use synchronization mechanisms or mocks                                                        |
| Operating real file system in tests                 | Slow and non-repeatable                                              | Use `tempfile`                                                                                 |
| Depending on test execution order                   | Flaky failures                                                       | Independent setup for each test                                                                |
| Test function exceeds 30 lines of logic             | No one can understand it                                             | Split tests or use helper functions                                                            |
| `unwrap()` in helper function reports no context    | Hard to locate                                                       | Use `expect("why")` or custom panic (see Rule 5.2)                                             |
| Copy-pasted setup 3 or more times                   | High modification cost                                               | Extract helper functions                                                                       |

---

## Integration Test Specification

### Test Organization

**Rule 7.1**: Integration tests go in the `tests/` directory at the project root. The entry file
`tests/integration.rs` uses the `#[path]` attribute to import submodules.

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
code generation, executor, etc.) and must not be mixed.

**Rule 7.3**: Integration tests must be performed through the project's public API. Directly
referencing `crate::` internal modules in integration tests is prohibited. Use the `yaoxiang::`
public path.

```rust
// 🟢 Good — through public API
use yaoxiang::run;

// 🔴 Garbage — bypasses the public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests prefer using inline source strings. Only when the source exceeds 30
lines should external fixture files be used (placed in `tests/fixtures/`).

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

**Rule 9.1**: Integration tests for each language feature must cover three paths:

| Path       | Description                                                 |
| ---------- | ----------------------------------------------------------- |
| Happy path | Legal input produces expected output                        |
| Error path | Illegal input produces clear error messages (not panic)     |
| Boundary   | Boundary values (empty input, max value, max nesting depth) |

**Rule 9.2**: Integration tests must not depend on the network, system environment variables, or
external services.

---

## Benchmark Specification

### Criterion.rs Usage Specification

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

**Rule 10.2**: Each benchmark function must contain a module-level doc comment `//!` explaining the
test purpose and measurement metrics.

```rust
//! YaoXiang interpreter performance benchmarks
//!
//! Measurement metric: single iteration time (wall time)
//! Baseline: native Rust implementation
```

### Preventing Compiler Optimization

**Rule 11.1**: The output under test in all benchmarks must be blocked from being optimized away by
the compiler through `criterion::black_box`.

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

**Rule 11.2**: Benchmark input data must be `const` or `lazy_static`, and must not be dynamically
generated within the `iter` closure — otherwise what's being measured is the total time of data
generation + the logic under test.

### Benchmark Grouping and Naming

**Rule 12.1**: Benchmark naming follows the format `<module under test>_<scenario>`, all lowercase
with underscore separation. Consistent with unit test naming rules.

**Rule 12.2**: Must use `criterion_group!` to logically group related benchmarks. Cramming all
benchmarks into one group is prohibited.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Doc Test Specification

### Usage Scenarios

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code
example in their doc comments. This example is executed via `cargo test --doc`.

````rust
/// Tokenize the source string into a sequence of Tokens.
///
/// ```
/// use yaoxiang::frontend::core::lexer::tokenize;
///
/// let tokens = tokenize("42").unwrap();
/// assert_eq!(tokens.len(), 2); // IntLiteral + Eof
/// ```
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    // ...
````

**Rule 13.2**: Doc test code examples must compile and have successful assertions. Examples with the
`ignore` marker are not allowed, unless the example demonstrates a compile-time error.

````rust
/// ```ignore
/// // Demonstrating a compile-time error — ignore is allowed
/// let x: int = "string";
/// ```
````

### Coverage Requirements

**Rule 14.1**: Doc tests only need to cover the happy path of the API. Boundary conditions and error
paths are covered by unit tests.

**Rule 14.2**: Example code in doc tests must be concise — no more than 10 lines. If an example
requires longer context, it indicates a problem with the API design.

---

## Property Test Specification

### Usage Scenarios

**Rule 15.1**: The following scenarios must use property tests (proptest or quickcheck) instead of
hand-writing multiple boundary value cases:

| Scenario                                      | Example                                |
| --------------------------------------------- | -------------------------------------- |
| Parser round-trip                             | `parse(pretty_print(ast)) == ast`      |
| Serialization/deserialization                 | `deserialize(serialize(data)) == data` |
| Mathematical operation identity               | `a + b == b + a`                       |
| Compiler optimizations don't change semantics | `eval(code) == eval(optimize(code))`   |

**Rule 15.2**: Property tests use `proptest` as the main property testing framework (already
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

**Rule 16.1**: Each property test must have a clear property declaration — the verified invariant
must be stated in a comment.

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

**Rule 16.2**: If a property test discovers a failure, must use `proptest`'s regression mechanism —
add the failing input to the `proptest-regressions/` directory, do not manually write a normal test
as a substitute.

---

## Coverage Requirements

### New Code Coverage Targets

**Rule 17.1**: Coverage requirements for new code:

| Code type                                        | Line coverage | Branch coverage |
| ------------------------------------------------ | ------------- | --------------- |
| Core compiler modules (frontend/middle/backends) | ≥ 85%         | ≥ 80%           |
| Utility/helper modules (util)                    | ≥ 75%         | ≥ 70%           |
| Runtime modules (vm/runtime)                     | ≥ 80%         | ≥ 75%           |
| Standard library (std)                           | ≥ 75%         | ≥ 70%           |
| Error handling and diagnostics                   | ≥ 90%         | ≥ 85%           |

**Rule 17.2**: Error handling paths (all `Err` branches) must be 100% covered. Error messages that
users can see must be verified by tests.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, the author must self-check the following items:

- [ ] `cargo test` all pass
- [ ] `cargo test --doc` all pass
- [ ] `cargo bench` has no performance regression (if hot path changes are involved)
- [ ] New code meets coverage targets
- [ ] Test names follow naming conventions
- [ ] Each test file declares the corresponding specification section (Rule 2.1)
- [ ] Test expectations come from specification definitions, not "output of the current code"
- [ ] No tests with `#[ignore]` markers (unless there's a clear issue number comment)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic messages)
- [ ] Commit message uses the `:white_check_mark: test:` type
- [ ] **No modification of test expectations due to "code behavior inconsistent with specification"
      — what gets changed is the code, not the test**
- [ ] **No inline tests** (`#[cfg(test)] mod tests { ... }` must be changed to `mod tests;` +
      independent file, see Rule 1.4)

**Rule 18.2**: Reviewers must reject PRs containing the following issues:

- Only happy path tests, missing error paths
- `thread::sleep` in tests or dependency on execution order
- Copy-pasted test code more than 3 times without extracting helper functions
- Test names do not follow naming conventions
- Existence of permanently `#[ignore]` tests
- **Tests accommodating code's wrong behavior** (modifying the test instead of the code when the
  code is inconsistent with the specification)
- **Tests do not declare the corresponding specification section** (see Rule 2.1)
- **Test expectations come from code output rather than specification definition** (reverse-derived
  tests equal no tests)
- **Existence of inline tests** (`#[cfg(test)] mod tests { ... }` instead of `mod tests;` +
  independent file, see Rule 1.4)
- Tests only verify "does not panic" without asserting specific behavior
- Deleting failing tests that expose code bugs (instead of fixing the code and then seeing it turn
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

# Run a specific test (filter by name)
cargo test test_parse_expr

# Run benchmarks
cargo bench

# Show test output (hidden by default)
cargo test -- --nocapture

# Run single-threaded (for debugging concurrency issues)
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
:white_check_mark: test(parser): add Pratt parser infix operator tests

Covered scenarios:
- Arithmetic operator precedence (+, -, *, /, %)
- Chained comparison operators (1 < x < 10)
- Logical operator short-circuit
- Right-associative assignment operators
```

### C. New Test File Checklist

When creating a new test module, ensure the following files are included:

```
# Add tests under src/<module>/
src/<module>/tests/
├── mod.rs          # Module declaration + common helper functions
└── <subject>.rs    # Test file, named after the corresponding source file

# Add integration tests under tests/
tests/
├── integration.rs   # Update: add #[path] declarations
└── integration/
    └── <topic>.rs   # New test file
```

### D. References

- [YaoXiang Language Specification](../reference/language-spec/index.md) — **The authoritative
  source of tests**
- [Accepted RFCs](../rfc/index.md) — **The authoritative source of design decisions**
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Specification](commit-convention.md)
- [Project Contribution Guide](contributing.md)

---

> 💡 **Remember**: Tests do not verify whether your code "can run" — they verify whether your code
> conforms to the specification. The specification changes, and tests change with the specification.
> When the code is wrong, fix the code, don't fix the test. **Code serves the specification, tests
> guard the specification. The moment tests accommodate the code, you've lost all protection.**
