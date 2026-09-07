---
title: 'Test Writing Standards'
description:
  YaoXiang project test writing hard standards, defining writing standards for unit tests,
  integration tests, benchmarks, documentation tests, and property tests
---

# Test Writing Standards

This document defines the hard standards for test writing in the YaoXiang project. All contributors
must follow the rules below; violations will be required to be modified during Code Review.

---

## Table of Contents

- [General Rules](#general-rules)
- [yx Corpus and Library Test Levels](#yx-corpus-and-library-test-levels)
- [Unit Test Standards](#unit-test-standards)
- [Integration Test Standards](#integration-test-standards)
- [Benchmark Standards](#benchmark-standards)
- [Documentation Test Standards](#documentation-test-standards)
- [Property Test Standards](#property-test-standards)
- [Coverage Requirements](#coverage-requirements)
- [Appendix](#appendix)

---

## General Rules

### Scope

These standards apply to all Rust test code in the YaoXiang project, including:

| Test Type           | Location              | Framework                  |
| ------------------- | --------------------- | -------------------------- |
| Unit tests          | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| Integration tests   | `tests/`              | `#[test]`                  |
| Benchmarks          | `benches/`            | Criterion.rs               |
| Documentation tests | API doc comments      | `cargo test --doc`         |
| Property tests      | Any test location     | proptest / quickcheck      |

### Core Principles

**Principle 0: The authoritative source for tests is the specification, not the code.** This is the
most important principle in this document. Tests verify whether the code conforms to the
specification, not whether the code "runs with the current implementation". When a test finds that
the code's behavior is inconsistent with the specification, **fix the code, never fix the test**.

The specification documents are located at:

- `docs/src/design/language-spec.md` —— Language core specification
- `docs/src/design/rfc/accepted/` —— Accepted RFC design documents

Each test file must declare the corresponding specification section at the top (see Rule 2.1). Any
developer should be able to take the specification document and compare it against the tests to
verify the correctness of the implementation. Conversely——if a piece of code has no corresponding
specification description, it should not exist, and certainly should not be tested.

```rust
// 🟢 Good——tests directly reference the specification, verifying whether the code follows the spec
//! Literal tests — based on language specification §2.6
//!
//! §2.6.1: Integer Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating point numbers (with decimal points and exponents)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation

#[test]
fn test_decimal_literal_parsing() {
    // Specification §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 Garbage——tests accommodate the current implementation behavior instead of verifying the spec
#[test]
fn test_literal_1() {
    // Don't know which section of the spec this code corresponds to
    // If parse_literal returns the wrong value, this test will "pass with a green light"
    // because it only verifies that the function doesn't panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and find that the code's behavior is inconsistent with the
specification. You have two options:

| Wrong approach                                           | Correct approach                                            |
| -------------------------------------------------------- | ----------------------------------------------------------- |
| Modify the test to make it "pass"                        | Modify the code so the behavior conforms to the spec        |
| Add `#[ignore]` to the test                              | Immediately fix the code implementation                     |
| Add special conditional branches to accommodate the code | Remove the branch, let the test expose the problem directly |

Remember: **Red light = code is wrong, not the test.** (Unless the test itself has a bug, which is
another matter.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of
the code under test by reading the tests, without needing additional comments or external
documentation.

```rust
// 🟢 Good——test name states what is tested and what is expected
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 Garbage——no one knows what this is testing
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for random failures.** Tests must be repeatable in any environment.
Tests that depend on random numbers, system time, or thread scheduling order must use fixed seeds or
mocks instead.

**Principle 3: One test tests one thing.** If a test name needs to connect multiple behaviors with
"and", split it into multiple tests.

```rust
// 🟢 Good——each test verifies only one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Garbage——one test stuffed with too many unrelated things
#[test]
fn test_parser() {
    // Tests tokenize, parse, typecheck, codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring internal implementation should not
cause tests to fail. If changing one line of implementation code causes 10 tests to fail, your tests
are written wrong.

But there is a key distinction here: **the definition of "behavior" comes from the specification,
not from the current code's performance.** If the code changes behavior (i.e., new behavior that
does not conform to the spec), the test must fail. If you cannot do this, your test is a "test that
accommodates the code"——it lets bugs drive straight in.

```
Specification (language-spec.md / RFC)  ──defines──►  Expected behavior  ──drives──►  Tests
                                                    │
Current code  ──implements──►  Actual behavior  ──compares──►  Test results

If actual behavior ≠ expected behavior:
  Test must fail (red light)  ──►  Fix code  ──►  Test passes (green light)

If actual behavior = expected behavior (but implementation is poor):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is what Principle 4 means
```

**Principle 5: Do not write fallback/compatibility/mode-specific test code.** The test environment
is one you have full control over. If you need `#[cfg(not(ci))]` to skip a test, it indicates a
fundamental problem with the test design.

### Terminology Definitions

| Term               | Definition                                                                      |
| ------------------ | ------------------------------------------------------------------------------- |
| Unit test          | Tests a single function or module behavior, does not depend on external systems |
| Integration test   | Tests collaboration of multiple modules through public API or CLI entry         |
| Benchmark          | Measures code performance, detects performance regressions                      |
| Documentation test | Executable code examples embedded in documentation comments                     |
| Property test      | Tests based on random inputs that verify invariants (properties)                |

### Relationship with Commit Standards

All test-related commits must use the `:white_check_mark: test:` type, refer to
[Commit Standards](./commit-convention.md).

```
:white_check_mark: test(parser): Add Pratt parser infix expression tests
:white_check_mark: test(codegen): Complete switch statement IR generation tests
```

---

## yx Corpus and Library Test Levels

These standards govern **Rust-side test code**. Tests for the YaoXiang language itself (`.yx` corpus
and library tests) are divided into two levels by the object under test; the system design and
judgment contract are governed by RFC-036 (§7 Suite Collection / §8 Negative Three Layers / §9 Test
System Layering), and corpus writing details are governed by `tests/yaoxiang/TEST_STANDARDS.md`:

- **Language usability corpus** (`tests/yaoxiang/`)——the object under test is the language itself;
  std is only used as an assertion tool. Within the corpus, judgments are divided into three
  categories by where failures occur: behavior tests / compile-time rejection tests / runtime
  failure tests
- **Library tests** (distributed with libraries)——the object under test is the public API contract
  of the library; std's yx-level tests are located at `src/std/tests/`, and future user package
  tests are discovered within packages via `[tool.test]`

The `.yx` test file header format, header directives (`// expect:` / `// skip:` / `// mode:`,
RFC-036 §8.2) and assertion conventions follow TEST_STANDARDS.md; judgment parsing is implemented by
the shared `src/util/test_markers.rs` (Rust-side, subject to these standards).

---

## Unit Test Standards

### File Organization

**Rule 1.1**: The `tests/` directory of unit tests must be **at the same level** as the `mod.rs` of
the module under test. `tests/` does not aggregate upward and does not cross-level summarize.

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
└── tests/              # parser module-level tests (does not include content of pratt submodule)
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

Key judgment criterion: **the `mod.rs` of the directory where `tests/` is placed must use
`#[cfg(test)] mod tests;` to declare it.**

**Rule 1.1 Supplement: Upward aggregation is prohibited.** Tests for submodule must be placed in
that submodule's own `tests/`, and may not be aggregated into the parent `tests/`.

| Module Type                     | Test Location           | Example                                      |
| ------------------------------- | ----------------------- | -------------------------------------------- |
| Directory module (has `mod.rs`) | `tests/` under that dir | `emitter/tests/`, `codes/tests/`             |
| Single-file module (only `.rs`) | Parent's `tests/`       | `session.rs` → `diagnostic/tests/session.rs` |

```text
# ✅ Correct: tests for each directory module are independent
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

# ❌ Wrong: aggregating tests of emitter and codes into diagnostic/tests/
src/util/diagnostic/
└── tests/
    ├── mod.rs              # ❌ Forced to declare mod emitter; mod codes;
    ├── emitter/            # ❌ Should be in emitter/tests/
    └── codes/              # ❌ Should be in codes/tests/
```

#### Test Placement Rules for Single-File Module vs Directory Module

**Core difference**: The organization of the module determines where tests are placed.

| Module Type            | Judgment Basis                                | Test Location            | Example                                       |
| ---------------------- | --------------------------------------------- | ------------------------ | --------------------------------------------- |
| **Directory module**   | Has independent directory and `mod.rs`        | `tests/` under that dir  | `inference/tests/`                            |
| **Single-file module** | Only has `.rs` file, no independent directory | Parent module's `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

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
│       ├── expressions.rs          # tests for expressions.rs
│       ├── statements.rs           # tests for statements.rs
│       └── ...
│
└── traits/                         # already deleted (logic merged into types/trait_data.rs)
```

**Why are tests for single-file module placed in the parent `tests/`?**

Because single-file module (like `overload.rs`) does not have its own `mod.rs`, it cannot declare
`#[cfg(test)] mod tests;`. According to the Rust module system, test files must be declared by some
`mod.rs` to be compiled. Therefore, tests for single-file module can only be declared by the parent
module's `mod.rs` and placed in the parent's `tests/` directory.

**Judgment flow**:

```
Encounter a module, decide where to place tests?
│
├── Is this module a directory (has mod.rs)?
│   └── Yes → Create tests/ under that directory, declared by that directory's mod.rs
│
├── Is this module a single file (only .rs)?
│   └── Yes → Tests are placed in the parent's tests/ directory, declared by the parent's mod.rs
│
└── Not sure?
    └── Check if it has an independent directory and mod.rs
```

**Common errors**:

```
# ❌ Error 1: Create independent tests/ directory for single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ Should not create directory for single-file module
    └── tests/
        └── overload.rs

# ❌ Error 2: Declare #[cfg(test)] mod tests; inside single-file module
# overload.rs
#[cfg(test)]                        # ❌ Single-file module cannot declare this way
mod tests;                          # because there is no overload/tests/ directory

# ✅ Correct approach: tests placed in the parent's tests/
src/frontend/core/typecheck/
├── overload.rs                     # source file
└── tests/
    └── overload.rs                 # test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern——do not write like this:**

```
# ❌ Wrong: submodule tests concentrated in the parent
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

**Why can't we aggregate upward?** Because Rust's module system requires `#[cfg(test)] mod tests;`
to decide the compilation of test files at the declaration point. If `types/mod.rs` declares
`mod tests;`, then the content of `types/tests/` is private content of the `types` module——it should
not reach into the territory of `base` or `computation`. Each module's tests should be internal
implementation details of that module, not of the parent module. This rule also applies to module
refactoring: when you split `types` into `base` and `computation`, tests should follow the split
modules, not stay in place. **Test directories do not mirror source structure; they follow module
boundaries.**

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
the same-level `tests/` directory. **The inline form `mod tests { ... }` is prohibited, which places
test code directly in the source file.**

```rust
// ✅ Correct——file-form declaration, test code in independent files
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests;

// 🔴 Prohibited——inline form, test code parasitically lives in the source file
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
   contain tests. Mixed together, you have to scroll to the bottom to modify tests, and skip tests
   to modify implementation.
2. Clear module boundaries: the `tests/` directory is a physical boundary that makes it immediately
   clear which modules have tests and which don't.
3. Refactoring safety: when modules are split, the `tests/` directory follows; inline tests need to
   be manually stripped from the source file.
4. Code review: in PR diff, source changes and test changes are in separate files and won't be mixed
   together.

### Module Declaration Standards

**Rule 2.1**: All test files must have a module-level documentation comment `//!` at the top,
describing the specification sources covered by the test (language specification section number +
RFC number). If a test does not reference any specification section, it means this code has no
specification basis——it should not exist.

```rust
//! Literal tests — based on language specification §2.6
//!
//! §2.6.1: Integer Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating point numbers (with decimal points and exponents)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation
```

**Why must we reference the specification?** Because test expectations come from the specification,
not from "the output of the current code". If one day the code changes output and the test is
updated accordingly, the test protects nothing. Only specification-anchored tests can distinguish
between "intentional breaking change" and "unintentional regression".

**Rule 2.2**: The `use` imports in test modules must be precise to specific types/functions; glob
imports like `use super::*` are prohibited.

```rust
// 🟢 Good——precise imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Garbage——no one knows what you're testing
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

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, you
should know what is tested and what is expected. Numeric sequence naming is prohibited.

```rust
// 🟢 Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Garbage——no idea what's being tested
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

### Test Structure Standards (Arrange-Act-Assert)

**Rule 4.1**: Each test function must follow a three-part structure: Arrange → Act → Assert,
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

**Rule 4.2**: Simple tests (single call + single assertion) may omit the partition comments, but
should not exceed 5 lines of logic code. Tests exceeding 5 lines must explicitly mark the three
parts.

### Helper Function Standards

**Rule 5.1**: Setup logic that repeats 3 or more times must be extracted as a helper function.

```rust
// 🟢 Good——extracting common setup
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

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print enough context on panic. Test
function bodies (`#[test] fn ...`) can use `unwrap()` directly——Rust automatically prints the line
number on failure; but when a helper function fails, the line number points to the helper function
definition, and you cannot see the context of the call.

```rust
// 🟢 Good——helper function prints source content on failure
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Garbage——on failure you can't see which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, right after the `use`
imports. If shared by multiple test modules, place them in `tests/mod.rs` and export them as
`pub(crate)`.

### Assertion Style

**Rule 6.1**: Prefer `assert!(matches!(...))` for enum variant matching; `if let` + `panic!` is
prohibited.

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

**Rule 6.2**: Use `assert_eq!` for exact value comparisons, and `assert!` for boolean assertions.
The use of `assert!(a == b)` instead of `assert_eq!(a, b)` is prohibited.

**Rule 6.3**: All assertions must have custom error messages, unless the assertion itself fully
describes the failure reason.

```rust
// 🟢 Good——can quickly locate on failure
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 Good——assert_eq! automatically prints value differences on failure, no extra message needed
assert_eq!(error_count, 0);

// 🔴 Garbage——on failure you only know "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: Assertion order must be `assert_eq!(actual, expected)`, with the actual value first
and the expected value second.

### Anti-pattern Checklist

The following are prohibited patterns and their alternatives:

| Anti-pattern                                   | Problem                                                              | Alternative                                                                                 |
| ---------------------------------------------- | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `#[cfg(test)] mod tests { ... }` inline tests  | Source file bloat, blurred module boundaries, refactoring difficulty | Place test code in independent `tests/` directory, declare with `mod tests;` (see Rule 1.4) |
| Tests accommodate wrong code behavior          | Conceals specification deviations, legalizes bugs                    | Fix code against the specification, keep tests unchanged                                    |
| Deriving test expectations from code output    | Tests become "recorders of current implementation"                   | Derive expectations from the specification                                                  |
| `#[ignore]` permanent markers                  | Hides rotting tests                                                  | Fix or delete                                                                               |
| `println!` debug output                        | Pollutes test output                                                 | Use `assert!` for explicit assertions                                                       |
| `thread::sleep`                                | Random failures + slow                                               | Use synchronization mechanisms or mocks                                                     |
| Manipulating real file system in tests         | Slow and unrepeatable                                                | Use `tempfile`                                                                              |
| Depending on test execution order              | Random failures                                                      | Each test sets up independently                                                             |
| One test function exceeds 30 lines of logic    | No one can understand it                                             | Split tests or use helper functions                                                         |
| `unwrap()` in helper functions without context | Hard to locate                                                       | Use `expect("why")` or custom panic (see Rule 5.2)                                          |
| Copy-paste identical setup 3+ times            | High modification cost                                               | Extract helper function                                                                     |

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
// 🟢 Good——through public API
use yaoxiang::run;

// 🔴 Garbage——bypasses the public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests should prefer inline source strings. Only when the source exceeds 30
lines should external fixture files be used (placed in `tests/fixtures/`).

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

**Rule 8.2**: Fixture files must end with the `.yx` extension, and the filename describes the test
intent.

### E2E Coverage Principles

**Rule 9.1**: Integration tests for each language feature must cover three paths:

| Path       | Description                                                     |
| ---------- | --------------------------------------------------------------- |
| Happy path | Legal input produces expected output                            |
| Error path | Illegal input produces clear error messages (not panic)         |
| Boundary   | Boundary values (empty input, maximum value, max nesting depth) |

**Rule 9.2**: Integration tests must not depend on the network, system environment variables, or
external services.

---

## Benchmark Standards

### Criterion.rs Usage Standards

**Rule 10.1**: Benchmarks are uniformly placed in the `benches/` directory, with the entry file
`benches/lib.rs`. Files are organized by test topic.

```
benches/
├── lib.rs              # entry point, defines criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # cross-language comparison benchmark
├── parser.rs           # parser benchmark
└── codegen.rs          # code generation benchmark
```

**Rule 10.2**: Each benchmark function must include a module-level documentation comment `//!`
describing the test purpose and measurement metrics.

```rust
//! YaoXiang interpreter performance benchmark
//!
//! Measurement metric: wall time per iteration
//! Baseline: native Rust implementation
```

### Preventing Compiler Optimizations

**Rule 11.1**: The output under test in all benchmarks must pass through `criterion::black_box` to
prevent compiler optimization elimination.

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

**Rule 11.2**: Input data for benchmarks must be `const` or `lazy_static`, and must not be
dynamically generated within the `iter` closure——otherwise it measures the total time of data
generation + logic under test.

### Benchmark Grouping and Naming

**Rule 12.1**: Benchmark naming format is `<module_under_test>_<scenario>`, all lowercase with
underscores. Consistent with unit test naming rules.

**Rule 12.2**: `criterion_group!` must be used to logically group related benchmarks. Squeezing all
benchmarks into one group is prohibited.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Documentation Test Standards

### Usage Scenarios

**Rule 13.1**: All `pub` functions, types, and methods must include at least one runnable code
example in their documentation comments. The example is executed via `cargo test --doc`.

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

**Rule 13.2**: Code examples in documentation tests must compile and pass assertions. Examples must
not include the `ignore` marker, unless the example demonstrates a compile-time error.

````rust
/// ```ignore
/// // Demonstrating compile-time error——can be ignored
/// let x: int = "string";
/// ```
````

### Coverage Requirements

**Rule 14.1**: Documentation tests cover the happy path of the API. Boundary cases and error paths
are covered by unit tests.

**Rule 14.2**: Example code in documentation tests must be concise——no more than 10 lines. If the
example needs longer context, it indicates an API design problem.

---

## Property Test Standards

### Usage Scenarios

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

**Rule 16.1**: Each property test must have a clear property declaration——the comment must state the
invariant being verified.

```rust
// Property: any integer literal after tokenize → tokens_to_string produces the same value
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**Rule 16.2**: If a property test discovers a failure, the `proptest` regression mechanism must be
used——add the failing input to the `proptest-regressions/` directory, and do not manually write a
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

**Rule 17.2**: Error handling paths (all `Err` branches) must be 100% covered. Error messages that
users can see must be verified by tests.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, the author must self-check the following items:

- [ ] All `cargo test` pass
- [ ] All `cargo test --doc` pass
- [ ] No performance regression in `cargo bench` (if hot path changes are involved)
- [ ] New code meets coverage targets
- [ ] Test naming conforms to naming standards
- [ ] Each test file declares the corresponding specification section (Rule 2.1)
- [ ] Test expectations come from the specification definition, not "the output of the current code"
- [ ] No `#[ignore]`-marked tests (unless there is a clear issue number comment)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic messages)
- [ ] Commit message uses the `:white_check_mark: test:` type
- [ ] **No modification of test expectations because "code behavior is inconsistent with the
      specification"——the code is changed, not the test**
- [ ] **No inline tests** (`#[cfg(test)] mod tests { ... }` must be changed to `mod tests;` +
      independent file, see Rule 1.4)

**Rule 18.2**: Reviewers must reject PRs containing the following issues:

- Only happy path tests, missing error paths
- `thread::sleep` or dependency on execution order in tests
- Copy-pasted test code exceeding 3 times without extracting helper functions
- Test names do not conform to naming standards
- Existence of permanently `#[ignore]`'d tests
- **Tests accommodating wrong code behavior** (modifying tests instead of code when the code does
  not conform to the specification)
- **Tests not declaring corresponding specification sections** (see Rule 2.1)
- **Test expectations come from code output rather than specification definition** (reverse-derived
  tests are equivalent to no test)
- **Existence of inline tests** (`#[cfg(test)] mod tests { ... }` instead of `mod tests;` +
  independent file, see Rule 1.4)
- Tests only verify "no panic" without asserting specific behavior
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

# Run only documentation tests
cargo test --doc

# Run specific tests (filter by name)
cargo test test_parse_expr

# Run benchmarks
cargo bench

# Show test output (hidden by default)
cargo test -- --nocapture

# Run single-threaded (troubleshoot concurrency issues)
cargo test -- --test-threads=1

# Generate coverage report (requires cargo-llvm-cov)
cargo llvm-cov --html
```

### B. Commit Message Template

Test-related commits must follow the template below:

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
- Logical operator short-circuiting
- Assignment operator right associativity
```

### C. New Test File Checklist

When creating a new test module, ensure the following files are included:

```
# Add tests under src/<module>/
src/<module>/tests/
├── mod.rs          # module declarations + public helper functions
└── <subject>.rs    # test file, named after the source file under test

# Add integration tests under tests/
tests/
├── integration.rs   # Update: add #[path] declarations
└── integration/
    └── <topic>.rs   # new test file
```

### D. References

- [YaoXiang Language Specification](../../design/language-spec.md) —— **The authoritative source for
  tests**
- [Accepted RFCs](../../design/rfc/accepted/) —— **The authoritative source for design decisions**
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Standards](./commit-convention.md)
- [Project Contribution Guide](./contributing.md)

---

> 💡 **Remember**: Tests don't verify whether your code "can run"——they verify whether your code
> conforms to the specification. The specification changes, and tests follow the specification. When
> the code is wrong, fix the code, not the test. **Code serves the specification, tests guard the
> specification. The moment tests accommodate the code, you lose all protection.**
