---
title: 'Test Writing Standards'
description:
  Mandatory test-writing standards for the YaoXiang project, defining the standards for unit tests,
  integration tests, benchmarks, documentation tests, and property tests
---

# Test Writing Standards

This document defines the mandatory test-writing standards for the YaoXiang project. All
contributors must comply with the following rules; violations will be required to be corrected
during Code Review.

---

## Table of Contents

- [General Rules](#总则)
- [yx Corpus and Library Test Layers](#yx-语料与库测试层级)
- [Unit Test Standards](#单元测试规范)
- [Integration Test Standards](#集成测试规范)
- [Benchmark Standards](#基准测试规范)
- [Documentation Test Standards](#文档测试规范)
- [Property Test Standards](#属性测试规范)
- [Coverage Requirements](#覆盖率要求)
- [Appendix](#附录)

---

## General Rules

### Scope

This standard applies to all Rust test code in the YaoXiang project, including:

| Test Type          | Location                   | Framework                  |
| ------------------ | -------------------------- | -------------------------- |
| Unit test          | `src/<module>/tests/`      | `#[test]` + `#[cfg(test)]` |
| Integration test   | `tests/`                   | `#[test]`                  |
| Benchmark          | `benches/`                 | Criterion.rs               |
| Documentation test | API documentation comments | `cargo test --doc`         |
| Property test      | Any test location          | proptest / quickcheck      |

### Core Principles

**Principle 0: The authoritative source for tests is the specification, not the code.** This is the
most important principle in this document. Tests verify that the code conforms to the specification,
not that it "passes under the current implementation." When a test finds that the code's behavior
differs from the specification, **fix the code; never fix the test**.

The specification files are located at:

- `docs/src/reference/language-spec/` — core language specifications
- `docs/rfc/accepted/` — accepted RFC design documents

Every test file must declare the corresponding specification section at the top (see Rule 2.1). Any
developer should be able to consult the specification while reviewing a test and verify the
correctness of the implementation. Conversely, if a piece of code has no corresponding
specification, it should not exist and should certainly not be tested.

```rust
// 🟢 Good—the test directly references the specification and verifies whether the code follows it
//! Literal tests — based on the language specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with a decimal point and an exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation

#[test]
fn test_decimal_literal_parsing() {
    // Specification §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 Garbage—the test accommodates the current implementation instead of verifying the specification
#[test]
fn test_literal_1() {
    // It is unknown which specification section this code corresponds to
    // If parse_literal returns an incorrect value, this test will "pass green"
    // Because it only verifies that the function does not panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and discover that the code's behavior differs from the specification.
You have two choices:

| Incorrect Approach                                       | Correct Approach                                           |
| -------------------------------------------------------- | ---------------------------------------------------------- |
| Modify the test so it "passes"                           | Modify the code so its behavior conforms                   |
| Add `#[ignore]` to the test                              | Fix the code implementation immediately                    |
| Add a special conditional branch to accommodate the code | Remove the branch so the test directly exposes the problem |

Remember: **A red test means the code is wrong, not the test.** (Unless the test itself has a bug,
which is a separate issue.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of
the code under test by reading its tests, without needing additional comments or external
documentation.

```rust
// 🟢 Good—the test name states what is tested and what is expected
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 Garbage—nobody knows what this tests
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for flaky tests.** Tests must be repeatably executable in any
environment. Tests that depend on random numbers, system time, or thread scheduling order must use
fixed seeds or mocks instead.

**Principle 3: Each test tests exactly one thing.** If the test name needs to connect multiple
behaviors with "and," split it into multiple tests.

```rust
// 🟢 Good—each test verifies exactly one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Garbage—one test contains too many unrelated things
#[test]
fn test_parser() {
    // Test tokenize, test parse, test typecheck, test codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring the internal implementation should
not cause tests to fail. If changing one line of implementation code causes 10 tests to fail, your
tests are wrong.

However, there is one critical distinction: **the definition of "behavior" comes from the
specification, not from the current behavior of the code.** If the code's behavior changes—that is,
it adopts new behavior that does not conform to the specification—the test must fail. If you cannot
achieve this, your test is a "test that accommodates the code"—it allows bugs to run rampant.

```
规范（language-spec.md / RFC）  ──定义──►  期望行为  ──驱动──►  测试
                                           │
当前代码  ──实现──►  实际行为  ──对比──►  测试结果

如果实际行为 ≠ 期望行为：
  测试必须失败（红灯）  ──►  修正代码  ──►  测试通过（绿灯）

如果实际行为 = 期望行为（但实现很烂）：
  测试通过  ──►  重构实现  ──►  测试仍然通过  ← 这才是原则 4 的含义
```

**Principle 5: Do not write test code that relies on fallbacks, compatibility paths, or
mode-specific behavior.** The test environment is one you can fully control. If you need
`#[cfg(not(ci))]` to skip a test, that test has a fundamental design problem.

### Terminology

| Term               | Definition                                                                                   |
| ------------------ | -------------------------------------------------------------------------------------------- |
| Unit test          | Tests the behavior of an individual function or module without depending on external systems |
| Integration test   | Tests how multiple modules collaborate through the public API or command-line entry point    |
| Benchmark          | Measures code performance and detects performance regressions                                |
| Documentation test | Executable code examples embedded in documentation comments                                  |
| Property test      | Verifies invariants from random inputs                                                       |

### Relationship to Commit Conventions

All test-related commits must use the `:white_check_mark: test:` type. See the
[Commit Convention](commit-convention.md).

```
:white_check_mark: test(parser): 添加 Pratt 解析器中缀表达式测试
:white_check_mark: test(codegen): 补全 switch 语句 IR 生成测试
```

---

## yx Corpus and Library Test Layers

This standard governs **Rust-side test code**. The YaoXiang language's own tests (`.yx` corpora and
library tests) are divided into two layers according to the subject under test. The system design
and adjudication contract are defined by RFC-036 (§7 suite collection / §8 negative three-layer
testing / §9 test system hierarchy), while corpus-authoring details are defined in
`tests/yaoxiang/TEST_STANDARDS.md`:

- **Language usability corpora** (`tests/yaoxiang/`)——the subject under test is the language itself;
  std is used only as an assertion tool. Within each corpus, outcomes are classified into three
  categories according to the layer at which the failure occurs: behavior tests / compile-time
  rejection tests / runtime failure tests
- **Library tests** (shipped with each library)——the subject under test is the library's public API
  contract; std's yx-level tests are located in `src/std/tests/`, and tests for future user packages
  are discovered within their packages through `[tool.test]`

The file-header format, header directives (`// expect:` / `// skip:` / `// mode:`, RFC-036 §8.2),
and assertion conventions for `.yx` tests are defined by TEST_STANDARDS.md. Adjudication parsing is
implemented by the two runners in the shared `src/util/test_markers.rs` (on the Rust side, and
therefore governed by this standard).

---

## Unit Test Standards

### File Organization

**Rule 1.1**: A unit test's `tests/` directory must be at the same level as the `mod.rs` of the
module under test. `tests/` must not aggregate upward or collect tests across module levels.

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; — declares the sibling tests/
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; — pratt's own tests
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # parser module-level tests (excluding the contents of the pratt submodule)
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

The key criterion is: **wherever `tests/` is placed, that directory's `mod.rs` must declare it with
`#[cfg(test)] mod tests;`.**

**Rule 1.1 Supplement: Upward aggregation is prohibited.** A submodule's tests must be placed in
that submodule's own `tests/` directory and must not be aggregated into a parent `tests/` directory.

| Module Type                     | Test Location                   | Example                                      |
| ------------------------------- | ------------------------------- | -------------------------------------------- |
| Directory module (has `mod.rs`) | `tests/` under that directory   | `emitter/tests/`, `codes/tests/`             |
| Single-file module (only `.rs`) | The parent directory's `tests/` | `session.rs` → `diagnostic/tests/session.rs` |

```text
# ✅ Correct: each directory module has independent tests
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
    ├── mod.rs              # ❌ Forced to declare mod emitter; mod codes;
    ├── emitter/            # ❌ Should be placed in emitter/tests/
    └── codes/              # ❌ Should be placed in codes/tests/
```

#### Test Placement Rules for Single-File and Directory Modules

**Core distinction**: The module's organizational form determines where its tests are placed.

| Module Type            | Identification Criteria                            | Test Location                 | Example                                       |
| ---------------------- | -------------------------------------------------- | ----------------------------- | --------------------------------------------- |
| **Directory module**   | Has an independent directory and `mod.rs`          | `tests/` under that directory | `inference/tests/`                            |
| **Single-file module** | Has only a `.rs` file and no independent directory | The parent module's `tests/`  | `overload.rs` → `typecheck/tests/overload.rs` |

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
├── tests/                          # ✅ typecheck's test directory
│   ├── mod.rs                      # Declares tests for single-file modules
│   ├── checker.rs                  # checker.rs tests
│   ├── environment.rs              # environment.rs tests
│   ├── overload.rs                 # overload.rs tests (single-file module tests go here)
│   ├── type_eval.rs                # type_eval.rs tests
│   ├── dead_code.rs                # dead_code.rs tests
│   ├── spawn_placement.rs          # spawn_placement.rs tests
│   ├── signature.rs                # signature.rs tests
│   └── types.rs                    # types.rs tests
│
├── inference/                      # Directory module (has mod.rs)
│   ├── mod.rs                      # #[cfg(test)] mod tests; — declares the sibling tests/
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

**Why are single-file module tests placed in the parent `tests/` directory?**

Because a single-file module such as `overload.rs` has no `mod.rs` of its own and therefore cannot
declare `#[cfg(test)] mod tests;`. According to Rust's module system, a test file must be declared
by some `mod.rs` before it can be compiled. Consequently, tests for a single-file module can only be
declared by the parent module's `mod.rs` and must be placed in the parent's `tests/` directory.

**Decision process**:

```
遇到一个模块，判断测试放哪里？
│
├── 该模块是目录（有 mod.rs）？
│   └── 是 → 在该目录下创建 tests/，由该目录的 mod.rs 声明
│
├── 该模块是单文件（只有 .rs）？
│   └── 是 → 测试放在父级的 tests/ 目录，由父级的 mod.rs 声明
│
└── 不确定？
    └── 检查是否有独立目录和 mod.rs
```

**Common errors**:

```
# ❌ Error 1: Do not create a separate tests/ directory for a single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ Do not create a directory for a single-file module
    └── tests/
        └── overload.rs

# ❌ Error 2: Do not declare #[cfg(test)] mod tests; inside a single-file module
# overload.rs
#[cfg(test)]                        # ❌ A single-file module cannot declare it this way
mod tests;                          # Because there is no overload/tests/ directory

# ✅ Correct approach: place tests in the parent tests/
src/frontend/core/typecheck/
├── overload.rs                     # Source file
└── tests/
    └── overload.rs                 # Test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern—do not do this:**

```
# ❌ Wrong: concentrate submodule tests in the parent
src/frontend/core/types/
├── mod.rs              # Should only declare base and computation
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ Parent tests/ contains submodule tests
    ├── mod.rs          # ❌ Forced to declare mod base; mod computation;
    ├── base/           # ❌ This part should be placed in base/tests/
    │   └── var.rs
    └── computation/    # ❌ This part should be placed in computation/tests/
        └── ...
```

```
# ✅ Correct approach: each module has independent tests
src/frontend/core/types/
├── mod.rs              # Only declares pub mod base; pub mod computation;
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; — declares the sibling tests/
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; — declares the sibling tests/
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**Why is upward aggregation prohibited?** Rust's module system requires `#[cfg(test)] mod tests;` to
determine where test files are compiled at the declaration site. If `types/mod.rs` declares
`mod tests;`, then the contents of `types/tests/` are private contents of the `types` module—they
should not cross into the territory of `base` or `computation`. Each module's tests should be an
internal implementation detail of that module, rather than of the parent module. This rule also
applies during module refactoring: when you split `types` into `base` and `computation`, the tests
should follow the split modules instead of remaining in place. **Test directories do not mirror the
source structure; they follow module boundaries.**

**Rule 1.2**: `tests/mod.rs` is responsible only for module declarations and re-exports; it must not
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

**Rule 1.3**: Each test file must correspond to exactly one source file. Tests for multiple source
modules must not be mixed into one file.

**Rule 1.4**: Test declarations must use the file form `mod tests;` (with a semicolon) to point to
the sibling `tests/` directory. **Using the inline form `mod tests { ... }` to place test code
directly in a source file is prohibited.**

```rust
// ✅ Correct—use a file-form declaration; test code lives in separate files
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests;

// 🔴 Forbidden—inline form; test code is parasitic on the source file
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

**Why is the inline form prohibited?**

1. A source file has a single responsibility: implementation belongs only in source files, and tests
   belong only in test files. When mixed together, changing a test requires scrolling to the bottom
   of the file, while changing the implementation requires skipping over the tests.
2. Clear module boundaries: the `tests/` directory provides a physical boundary that makes it
   immediately obvious which modules have tests and which do not.
3. Refactoring safety: when a module is split, the `tests/` directory moves with it; inline tests
   must be manually extracted from source files.
4. Code review: source changes and test changes appear in separate files in the PR diff and are not
   mixed together.

### Module Declaration Standards

**Rule 2.1**: Every test file must have a module-level documentation comment `//!` at the top,
explaining the source of the specifications covered by the test (language specification section
number + RFC number). If a test does not reference any specification section, that code has no
specification basis—it should not exist.

```rust
//! Literal tests — based on the language specification §2.6
//!
//! §2.6.1: Integers Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with a decimal point and an exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation
```

**Why must the specification be referenced?** Because a test's expected values come from the
specification, not from the "current output of the code." If one day the code's output changes and
the test is updated accordingly, the test protects nothing. Only specification-anchored tests can
distinguish an "intentional breaking change" from an "unintentional regression."

**Rule 2.2**: A test module's `use` imports must be specific to individual types/functions; glob
imports such as `use super::*` are prohibited.

```rust
// 🟢 Good—specific imports
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 Garbage—others cannot tell what you are testing
use super::*;
```

### Naming Standards

**Rule 3.1**: Test function names must follow the format `test_<what>_<scenario>`, using lowercase
words separated by underscores.

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, one
should know what is tested and what is expected. Numeric sequence names are prohibited.

```rust
// 🟢 Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Garbage—completely unclear what is being tested
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**Rule 3.3**: Helper functions do not need a `test_` prefix; use a verb or noun that describes their
purpose.

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### Test Structure Standards (Arrange-Act-Assert)

**Rule 4.1**: Every test function must follow a three-stage structure: Arrange → Act → Assert, with
the stages separated by blank lines.

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

**Rule 4.2**: Simple tests (a single call + a single assertion) may omit the stage comments, but
must not exceed five lines of logical code. Tests exceeding five lines must explicitly mark all
three stages.

### Helper Function Standards

**Rule 5.1**: Setup logic that occurs three or more times must be extracted into a helper function.

```rust
// 🟢 Good—extract common setup
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

**Rule 5.2**: `unwrap()` / `expect()` in helper functions must print sufficient context when they
panic. Calls inside a test function body (`#[test] fn ...`) may use `unwrap()` directly—Rust
automatically prints the line number on failure; however, when a helper function fails, the line
number points to the helper's definition and does not show the caller's context.

```rust
// 🟢 Good—print the source contents when the helper fails
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Garbage—you cannot tell which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, immediately after the
`use` imports. If shared by multiple test modules, place them in `tests/mod.rs` and export them with
`pub(crate)`.

### Assertion Style

**Rule 6.1**: Enum variant matching should preferentially use `assert!(matches!(...))`; using
`if let` + `panic!` is prohibited.

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

**Rule 6.2**: Use `assert_eq!` for exact-value comparisons and `assert!` for Boolean assertions.
Using `assert!(a == b)` in place of `assert_eq!(a, b)` is prohibited.

**Rule 6.3**: Every assertion must include a custom error message unless the assertion itself fully
describes the reason for failure.

```rust
// 🟢 Good—quickly locate the failure
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 Good—assert_eq! automatically prints the value difference; no additional message is needed
assert_eq!(error_count, 0);

// 🔴 Garbage—you only know "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: Assertions must use the order `assert_eq!(actual, expected)`, with the actual value
first and the expected value second.

### Anti-Pattern Checklist

The following prohibited patterns and their alternatives are listed below:

| Anti-Pattern                                    | Problem                                                                       | Alternative                                                                                      |
| ----------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `#[cfg(test)] mod tests { ... }` inline tests   | Bloats source files, blurs module boundaries, and makes refactoring difficult | Place test code in a separate `tests/` directory and declare it with `mod tests;` (see Rule 1.4) |
| Tests accommodate incorrect code behavior       | Hides specification deviations and legitimizes bugs                           | Correct the code against the specification and keep the tests unchanged                          |
| Derive expected values from code output         | Turns tests into a "recording of the current implementation"                  | Derive expected values from the specification                                                    |
| Permanently mark with `#[ignore]`               | Hides rotting tests                                                           | Fix or delete                                                                                    |
| `println!` debug output                         | Pollutes test output                                                          | Use explicit `assert!` assertions                                                                |
| `thread::sleep`                                 | Causes flaky tests and slows the suite                                        | Use synchronization mechanisms or mocks                                                          |
| Operate on the real file system in tests        | Slow and non-reproducible                                                     | Use `tempfile`                                                                                   |
| Depend on test execution order                  | Causes flaky tests                                                            | Set up each test independently                                                                   |
| A test function exceeds 30 lines of logic       | No one can understand it                                                      | Split the test or use helper functions                                                           |
| `unwrap()` in a helper function without context | Makes failures difficult to locate                                            | Use `expect("why")` or a custom panic message (see Rule 5.2)                                     |
| Copy-paste the same setup more than three times | Expensive to maintain                                                         | Extract a helper function                                                                        |

---

## Integration Test Standards

### Test Organization

**Rule 7.1**: Integration tests must be placed in the project root's `tests/` directory. The entry
file `tests/integration.rs` must use the `#[path]` attribute to include submodules.

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**Rule 7.2**: Each `tests/integration/*.rs` file must correspond to one test topic (compiler
backends, code generation, executors, etc.); different topics must not be mixed together.

**Rule 7.3**: Integration tests must exercise the project through its public API. Directly
referencing internal modules through `crate::` in integration tests is prohibited. Use the public
`yaoxiang::` path.

```rust
// 🟢 Good—use the public API
use yaoxiang::run;

// 🔴 Garbage—bypasses the public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests should preferentially use inline source strings. Use an external
fixture file only when the source exceeds 30 lines; place fixture files in `tests/fixtures/`.

```rust
#[test]
fn test_fibonacci() {
    run_ok(
        r#"
        // 内联源码以 Script 模式运行：顶层语句即程序主体（SPEC syntax §3.11）。
        // 带型 main 若不显式调用只是惰性 lambda——循环体永远不会执行。
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

**Rule 8.2**: Fixture files must end with the `.yx` extension, and their filenames must describe the
test's intent.

### E2E Coverage Principles

**Rule 9.1**: The integration test for every language feature must cover three paths:

| Path       | Description                                                         |
| ---------- | ------------------------------------------------------------------- |
| Happy path | Valid input produces the expected output                            |
| Error path | Invalid input produces a clear error message (not a panic)          |
| Boundary   | Boundary values (empty input, maximum value, maximum nesting depth) |

**Rule 9.2**: Integration tests must not depend on the network, system environment variables, or
external services.

**Rule 9.3**: After the corpora (`tests/yaoxiang/`, `src/std/tests/`, `tests/yaoxiang-multifile/`)
exit with 0 for `check`, `run` must not report errors from the same family as compile-time errors:
E0 lexical/syntax, E1 type, E2 semantic, E3 code generation, E4 generics, or E5 modules. The
authoritative source for family-to-category mappings is `ErrorCategory` in
`src/util/diagnostic/codes/mod.rs`; `run` reporting only E6–E8 (runtime / I/O / internal) does not
constitute a violation.

The criteria are enforced by `python scripts/ci/check-soundness.py`. It is currently **report-only**
(forks only print and do not block; `--strict` is the gate-enable mode), and `pre-commit` runs it
with `--report-only` when the staging area contains corpus `.yx` files. The gate-enable conditions
(the three known forks reaching zero + mutation self-certification) and the currently measured
status are documented in the script's header comment.

The criteria surface has two layers: the **project form** in `tests/project-corpus/` (project
fixtures containing `yaoxiang.toml`, with one set each for E3022 / E3020 / E3005), and the
**single-file corpora** at the three locations listed above. Only this probe consumes the
project-form layer; it does not enter the corpus differential baseline and is not collected by
`yx_multifile_runner`—writing behavior corpora would amount to recording a violation as expected
behavior. The three corresponding implementation defects were fixed on 2026-11-06 (E3005: `[mono]`
was connected through the check chain; E3020 / E3022: entry-contract adjudication was added to
check). They changed from "forks" to regression guards where check and run use the same
interpretation—if any criterion regresses, check will return 0 while run still reports the same
code, and the probe will report the fork again.

---

## Benchmark Standards

### Criterion.rs Usage Standards

**Rule 10.1**: Benchmarks must all be placed in the `benches/` directory, with `benches/lib.rs` as
the entry file. Organize files by test topic.

```
benches/
├── lib.rs              # Entry point; defines criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # Cross-language comparison benchmark
├── parser.rs           # Parser benchmark
└── codegen.rs          # Code-generation benchmark
```

**Rule 10.2**: Every benchmark function must include a module documentation comment `//!` describing
the test's purpose and measurement metric.

```rust
//! YaoXiang interpreter performance benchmark
//!
//! Measurement metric: elapsed time per iteration (wall time)
//! Baseline: native Rust implementation
```

### Preventing Compiler Optimizations

**Rule 11.1**: The output under test in every benchmark must be passed through
`criterion::black_box` to prevent compiler optimizations from eliminating it.

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

**Rule 11.2**: Benchmark input data must be `const` or `lazy_static` and must not be generated
dynamically inside the `iter` closure—otherwise, the measurement includes both data generation and
the logic under test.

### Benchmark Grouping and Naming

**Rule 12.1**: Benchmark names must follow the format `<module under test>_<scenario>`, using
lowercase words separated by underscores. This is consistent with the unit-test naming rules.

**Rule 12.2**: Related benchmarks must be logically grouped with `criterion_group!`. Placing all
benchmarks in a single group is prohibited.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Documentation Test Standards

### Use Cases

**Rule 13.1**: Every `pub` function, type, and method must contain at least one runnable code
example in its documentation comments. The example is executed through `cargo test --doc`.

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

**Rule 13.2**: Code examples in documentation tests must compile and their assertions must succeed.
Examples with an `ignore` marker are prohibited unless the example demonstrates a compile-time
error.

````rust
/// ```ignore
/// // Demonstrates a compile-time error—may be ignored
/// let x: int = "string";
/// ```
````

### Coverage Requirements

**Rule 14.1**: Documentation tests only need to cover an API's happy path. Boundary cases and error
paths are covered by unit tests.

**Rule 14.2**: Example code in documentation tests must be concise—no more than 10 lines. If an
example requires more context, the API design is at fault.

---

## Property Test Standards

### Use Cases

**Rule 15.1**: The following scenarios must use property testing (proptest or quickcheck) instead of
hand-writing multiple boundary-value cases:

| Scenario                                  | Example                                |
| ----------------------------------------- | -------------------------------------- |
| Parser round-trip                         | `parse(pretty_print(ast)) == ast`      |
| Serialization/deserialization             | `deserialize(serialize(data)) == data` |
| Mathematical operation identity           | `a + b == b + a`                       |
| Compiler optimization preserves semantics | `eval(code) == eval(optimize(code))`   |

**Rule 15.2**: Property tests use `proptest` as the primary property-testing framework (already
declared under `dev-dependencies` in `Cargo.toml`).

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

**Rule 16.1**: Every property test must have an explicit property statement—a comment describing the
invariant being verified.

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

**Rule 16.2**: If a property test fails, use `proptest`'s regression mechanism—add the failing input
to the `proptest-regressions/` directory. Do not replace it with a hand-written ordinary test.

---

## Coverage Requirements

### Coverage Targets for New Code

**Rule 17.1**: New code must meet the following test coverage requirements:

| Code Type                                        | Line Coverage | Branch Coverage |
| ------------------------------------------------ | ------------- | --------------- |
| Core compiler modules (frontend/middle/backends) | ≥ 85%         | ≥ 80%           |
| Tooling/helper modules (util)                    | ≥ 75%         | ≥ 70%           |
| Runtime modules (vm/runtime)                     | ≥ 80%         | ≥ 75%           |
| Standard library (std)                           | ≥ 75%         | ≥ 70%           |
| Error handling and diagnostics                   | ≥ 90%         | ≥ 85%           |

**Rule 17.2**: Error-handling paths (all `Err` branches) must have 100% coverage. User-visible error
messages must be verified by tests.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, the author must verify the following items:

- [ ] `python scripts/dev/testsuite.py run fast` passes—the pre-submission self-check runs `--lib`
      (src/ unit tests) and four integration-side targets (`integration` / `formatter_integration` /
      `yx_runner` / `yx_multifile_runner`); the first stage of `pre-commit` is this command, and the
      second runs `cargo test --all --quiet` to block the commit
- [ ] `cargo test --doc` passes—doctests are not covered by `run fast` and are handled by the CI
      test job
- [ ] `cargo bench` shows no performance regressions (if hot-path changes are involved)
- [ ] New code meets the coverage targets
- [ ] Test names comply with the naming standards
- [ ] Every test file declares the corresponding specification section (Rule 2.1)
- [ ] Test expected values come from the specification, not from the "current output of the code"
- [ ] There are no `#[ignore]` markers (unless there is an explicit issue-number comment)
- [ ] There are no unnecessary `unwrap()` calls (use `expect` or a custom panic message)
- [ ] The commit message uses the `:white_check_mark: test:` type
- [ ] **Test expected values were not changed because "the code's behavior differs from the
      specification"—change the code, not the tests**
- [ ] **There are no inline tests** (`#[cfg(test)] mod tests { ... }` must be replaced with
      `mod tests;` + separate files; see Rule 1.4)

**Rule 18.2**: Reviewers must reject PRs containing any of the following problems:

- Only a happy-path test, with no error path
- `thread::sleep` or dependence on execution order in tests
- Copy-pasted test code repeated more than three times without extracting a helper function
- Test names that do not comply with the naming standards
- A permanent `#[ignore]` marker
- **Tests that accommodate incorrect code behavior** (when the code differs from the specification,
  the test is modified instead of the code)
- **Tests that do not declare the corresponding specification section** (see Rule 2.1)
- **Test expected values derived from code output rather than the specification** (a
  reverse-engineered test is equivalent to no test)
- **Inline tests** (`#[cfg(test)] mod tests { ... }` rather than `mod tests;` + separate files; see
  Rule 1.4)
- Tests that only verify that something "does not panic" without asserting specific behavior
- A failing test that exposed a code bug was deleted instead of fixing the code and seeing it turn
  green

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

# Run a specific test (filter by name)
cargo test test_parse_expr

# Run benchmarks
cargo bench

# Show test output (stdout is hidden by default)
cargo test -- --nocapture

# Run single-threaded (troubleshoot concurrency issues)
cargo test -- --test-threads=1

# Generate a coverage report (requires cargo-llvm-cov)
cargo llvm-cov --html
```

### B. Commit Message Template

Test-related commits must follow this template:

```
:white_check_mark: test(<scope>): <简短描述>

<可选：覆盖的场景列表>
```

Example:

```
:white_check_mark: test(parser): 添加 Pratt 解析器中缀运算符测试

覆盖场景：
- 算术运算符优先级（+, -, *, /, %）
- 比较运算符链接（1 < x < 10）
- 逻辑运算符短路
- 赋值运算符右结合
```

### C. Checklist for New Test Files

When creating a new test module, ensure that it contains the following files:

```
# Add tests under src/<module>/
src/<module>/tests/
├── mod.rs          # Module declarations + shared helper functions
└── <subject>.rs    # Test file, named after the source file under test

# Add integration tests under tests/
tests/
├── integration.rs   # Update: add a #[path] declaration
└── integration/
    └── <topic>.rs   # New test file
```

### D. References

- [YaoXiang Language Specification](../reference/language-spec/index.md) — **the authoritative
  source for tests**
- [Accepted RFCs](../rfc/index.md) — **the authoritative source for design decisions**
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Convention](commit-convention.md)
- [Project Contribution Guide](contributing.md)

---

> 💡 **Remember**: Tests do not verify whether your code "works"; they verify whether your code
> conforms to the specification. When the specification changes, the tests change with it. If the
> code is wrong, fix the code, not the tests. **Code serves the specification, and tests safeguard
> it. The moment tests accommodate the code, you lose all protection.**
