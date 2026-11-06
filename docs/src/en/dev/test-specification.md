---
title: 'Test Authoring Standards'
description:
  Strict test-authoring standards for the YaoXiang project, defining the standards for unit tests,
  integration tests, benchmarks, documentation tests, and property tests
---

# Test Authoring Standards

This document defines strict test-authoring standards for the YaoXiang project. All contributors
must follow the rules below; violators will be required to make changes during Code Review.

---

## Table of Contents

- [General Rules](#总则)
- [yx Corpus and Library Test Hierarchy](#yx-语料与库测试层级)
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

This specification applies to all Rust test code in the YaoXiang project, including:

| Test Type          | Location                   | Framework                  |
| ------------------ | -------------------------- | -------------------------- |
| Unit test          | `src/<module>/tests/`      | `#[test]` + `#[cfg(test)]` |
| Integration test   | `tests/`                   | `#[test]`                  |
| Benchmark          | `benches/`                 | Criterion.rs               |
| Documentation test | API documentation comments | `cargo test --doc`         |
| Property test      | Any test location          | proptest / quickcheck      |

### Core Principles

**Principle 0: The authoritative source of a test is the specification, not the code.** This is the
most important principle in this document. Tests verify whether the code conforms to the
specification, not whether the code "works with the current implementation." When a test finds that
code behavior is inconsistent with the specification, **fix the code; never fix the test**.

The specification files are located at:

- `docs/src/reference/language-spec/` —— the core language specification
- `docs/rfc/accepted/` —— accepted RFC design documents

Every test file must declare its corresponding specification section at the top (see Rule 2.1). Any
developer should be able to compare the test against the specification document and verify the
correctness of the implementation. Conversely—if a piece of code has no corresponding specification,
it should not exist, much less be tested.

```rust
// 🟢 Good—the test directly cites the specification and verifies that the code follows it
//! Literal tests — based on the Language Specification §2.6
//!
//! §2.6.1: Integers: Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with a decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation

#[test]
fn test_decimal_literal_parsing() {
    // Specification §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 Garbage—the test accommodates the current implementation's behavior instead of verifying the specification
#[test]
fn test_literal_1() {
    // I do not know which section of the specification this code corresponds to
    // If parse_literal returns an incorrect value, this test will "pass with a green light"
    // Because it only verifies that the function does not panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You write a test and find that the code behavior does not conform to the
specification. You have two options:

| Wrong Approach                                           | Correct Approach                                                   |
| -------------------------------------------------------- | ------------------------------------------------------------------ |
| Modify the test to make it "pass"                        | Modify the code so that its behavior conforms to the specification |
| Add `#[ignore]` to the test                              | Fix the implementation immediately                                 |
| Add special conditional branches to accommodate the code | Remove the branches so that the test directly exposes the problem  |

Remember: **A red light means the code is wrong, not the test.** (Unless the test itself has a bug,
which is a separate issue.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of
the code under test by reading the tests, without needing additional comments or external
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

**Principle 2: Zero tolerance for flaky tests.** Tests must be repeatable in any environment. Tests
that depend on random numbers, system time, or thread scheduling order must use fixed seeds or mocks
instead.

**Principle 3: Each test verifies exactly one thing.** If a test name needs to connect multiple
behaviors with "and," split it into multiple tests.

```rust
// 🟢 Good—each test verifies only one scenario
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 Garbage—one test contains too many unrelated concerns
#[test]
fn test_parser() {
    // Test tokenize, parse, typecheck, codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring the internal implementation should
not cause tests to fail. If changing one line of implementation code causes 10 tests to fail, your
tests are wrong.

However, there is a critical distinction here: **The definition of "behavior" comes from the
specification, not from the current behavior of the code.** If the code changes its behavior—that
is, adopts new behavior that does not conform to the specification—the test must fail. If you cannot
achieve this, your test is a "test that accommodates the code"—it lets bugs run rampant.

```
规范（language-spec.md / RFC）  ──定义──►  期望行为  ──驱动──►  测试
                                           │
当前代码  ──实现──►  实际行为  ──对比──►  测试结果

如果实际行为 ≠ 期望行为：
  测试必须失败（红灯）  ──►  修正代码  ──►  测试通过（绿灯）

如果实际行为 = 期望行为（但实现很烂）：
  测试通过  ──►  重构实现  ──►  测试仍然通过  ← 这才是原则 4 的含义
```

**Principle 5: Do not write test code for fallbacks, compatibility, or mode-specific behavior.** The
test environment is one you can fully control. If you need `#[cfg(not(ci))]` to skip a test, the
test has a fundamental design problem.

### Terminology

| Term               | Definition                                                                                   |
| ------------------ | -------------------------------------------------------------------------------------------- |
| Unit test          | Tests the behavior of an individual function or module without depending on external systems |
| Integration test   | Tests the collaboration of multiple modules through public APIs or command-line entry points |
| Benchmark          | Measures code performance and detects performance regressions                                |
| Documentation test | An executable code example embedded in a documentation comment                               |
| Property test      | A test that verifies an invariant using random inputs                                        |

### Relationship to the Commit Convention

All test-related commits must use the `:white_check_mark: test:` type. See the
[Commit Convention](commit-convention.md).

```
:white_check_mark: test(parser): 添加 Pratt 解析器中缀表达式测试
:white_check_mark: test(codegen): 补全 switch 语句 IR 生成测试
```

---

## yx Corpus and Library Test Hierarchy

This specification governs **Rust-side test code**. Tests of the YaoXiang language itself (`.yx`
corpus and library tests) are divided into two layers according to the object under test. The system
design and adjudication contract are governed by RFC-036 (§7 Suite Collection / §8 Three-Layer
Negative Testing / §9 Test System Hierarchy), while corpus-authoring details are governed by
`tests/yaoxiang/TEST_STANDARDS.md`:

- **Language usability corpus** (`tests/yaoxiang/`)——the object under test is the language itself;
  std is used only as an assertion tool. Within the corpus, results are adjudicated in three
  categories according to the layer at which the failure occurs: behavior test / compile-time
  rejection test / runtime failure test
- **Library tests** (ship with the library)——the object under test is the library's public API
  contract; std's yx-level tests are located in `src/std/tests/`, and tests for future user packages
  are discovered within those packages through `[tool.test]`

The file header format for `.yx` tests, header directives (`// expect:` / `// skip:` / `// mode:`,
RFC-036 §8.2), and assertion conventions are governed by TEST_STANDARDS.md; adjudication parsing is
implemented by `src/util/test_markers.rs`, which is shared by both runners (Rust side; governed by
this specification).

---

## Unit Test Standards

### File Organization

**Rule 1.1**: A unit test's `tests/` directory must be at the same level as the `mod.rs` of the
module under test. `tests/` must neither aggregate upward nor summarize across levels.

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——declares the sibling tests/
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——pratt's own tests
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # parser module-level tests (do not include the contents of the pratt submodule)
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

The key criterion is: **Wherever the `tests/` directory is placed, that directory's `mod.rs` must
declare it using `#[cfg(test)] mod tests;`.**

**Rule 1.1 Addendum: Upward aggregation is prohibited.** A submodule's tests must be placed in that
submodule's own `tests/` directory and must not be aggregated into the parent `tests/` directory.

| Module Type                     | Test Location                 | Example                                      |
| ------------------------------- | ----------------------------- | -------------------------------------------- |
| Directory module (has `mod.rs`) | `tests/` under that directory | `emitter/tests/`, `codes/tests/`             |
| Single-file module (only `.rs`) | Parent `tests/`               | `session.rs` → `diagnostic/tests/session.rs` |

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
└── tests/                  # ✅ diagnostic-level (single-file modules)
    ├── mod.rs
    ├── session.rs
    ├── suggest.rs
    └── collect.rs

# ❌ Incorrect: aggregating emitter and codes tests into diagnostic/tests/
src/util/diagnostic/
└── tests/
    ├── mod.rs              # ❌ Forced to declare mod emitter; mod codes;
    ├── emitter/            # ❌ Should be in emitter/tests/
    └── codes/              # ❌ Should be in codes/tests/
```

#### Test Placement Rules for Single-File and Directory Modules

**Core distinction**: The module's organization determines where its tests are placed.

| Module Type            | Criterion                                         | Test Location                 | Example                                       |
| ---------------------- | ------------------------------------------------- | ----------------------------- | --------------------------------------------- |
| **Directory module**   | Has its own directory and `mod.rs`                | `tests/` under that directory | `inference/tests/`                            |
| **Single-file module** | Has only an `.rs` file and no dedicated directory | Parent module's `tests/`      | `overload.rs` → `typecheck/tests/overload.rs` |

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
│   ├── mod.rs                      # Declares the single-file module tests
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
│   ├── mod.rs                      # #[cfg(test)] mod tests; ——declares the sibling tests/
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
└── traits/                         # Deleted (logic merged into types/trait_data.rs)
```

**Why are single-file module tests placed in the parent `tests/`?**

Because a single-file module such as `overload.rs` has no `mod.rs` of its own and therefore cannot
declare `#[cfg(test)] mod tests;`. Under Rust's module system, a test file must be declared by some
`mod.rs` to compile. Consequently, single-file module tests can only be declared by the parent
module's `mod.rs` and placed in the parent `tests/` directory.

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

**Common mistakes**:

```
# ❌ Mistake 1: creating a separate tests/ directory for a single-file module
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ Should not create a directory for a single-file module
    └── tests/
        └── overload.rs

# ❌ Mistake 2: declaring #[cfg(test)] mod tests; inside a single-file module
# overload.rs
#[cfg(test)]                        # ❌ A single-file module cannot declare this
mod tests;                          # Because there is no overload/tests/ directory

# ✅ Correct approach: place the tests in the parent tests/
src/frontend/core/typecheck/
├── overload.rs                     # Source file
└── tests/
    └── overload.rs                 # Test file, declared by typecheck/mod.rs
```

⚠️ **Anti-pattern—do not do this:**

```
# ❌ Incorrect: concentrating submodule tests in the parent
src/frontend/core/types/
├── mod.rs              # Should declare only base and computation
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ Parent tests/ contains submodule tests
    ├── mod.rs          # ❌ Forced to declare mod base; mod computation;
    ├── base/           # ❌ This should be in base/tests/
    │   └── var.rs
    └── computation/    # ❌ This should be in computation/tests/
        └── ...
```

```
# ✅ Correct approach: each module's tests remain independent
src/frontend/core/types/
├── mod.rs              # Declare only pub mod base; pub mod computation;
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——declares the sibling tests/
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; ——declares the sibling tests/
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**Why is upward aggregation prohibited?** Rust's module system requires `#[cfg(test)] mod tests;` to
determine which test files are compiled at the declaration site. If `types/mod.rs` declares
`mod tests;`, then the contents of `types/tests/` are private to the `types` module—they should not
reach into the territory of `base` or `computation`. Each module's tests should be an internal
implementation detail of that module, not of its parent. This rule also applies to module
refactoring: when you split `types` into `base` and `computation`, the tests should follow the
refactored module boundaries rather than remain in place. **The test directory does not mirror the
source structure; it follows module boundaries.**

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

**Rule 1.4**: Test declarations must use the file form `mod tests;` (with a semicolon), pointing to
the sibling `tests/` directory. **Inline declarations such as `mod tests { ... }`, which place test
code directly in the source file, are prohibited.**

```rust
// ✅ Correct—a file-form declaration, with test code in separate files
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests;

// 🔴 Prohibited—an inline declaration, with test code embedded in the source file
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

**Why are inline declarations prohibited?**

1. Single responsibility for source files: source files contain only implementation, while test
   files contain only tests. Mixing them means scrolling to the bottom to modify tests and skipping
   over tests when modifying implementation.
2. Clear module boundaries: the `tests/` directory is a physical boundary that makes it immediately
   clear which modules have tests and which do not.
3. Refactoring safety: when a module is split, the `tests/` directory follows it; inline tests must
   be manually extracted from the source files.
4. Code review: source changes and test changes appear in separate files in the PR diff and are not
   mixed together.

### Module Declaration Standards

**Rule 2.1**: Every test file must have a module-level documentation comment `//!` at the top,
stating the specification source covered by the test (language specification section number + RFC
number). If a test does not cite any specification section, the code has no specification basis—it
should not exist.

```rust
//! Literal tests — based on the Language Specification §2.6
//!
//! §2.6.1: Integers: Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: Floating-point numbers (with a decimal point and exponent)
//! §2.6.3: Strings (escape sequences \\nrt'"\\, \\x, \\u{})
//! RFC-012: F-String interpolation
```

**Why must the specification be cited?** Because a test's expected values come from the
specification, not from the "current output of the code." If the code's output changes one day and
the test is updated accordingly, the test protects nothing. Only tests anchored to a specification
can distinguish an intentional breaking change from an unintentional regression.

**Rule 2.2**: A test module's `use` imports must specify the exact types and functions. Glob imports
such as `use super::*` are prohibited.

```rust
// 🟢 Good—exact imports
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

**Rule 3.2**: Test function names must be self-explanatory. After reading the function name, a
developer should know what is tested and what is expected. Numeric ordinal names are prohibited.

```rust
// 🟢 Good
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 Garbage—completely unclear what is tested
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**Rule 3.3**: Helper functions do not need the `test_` prefix; use a verb or noun that describes
their purpose.

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### Test Structure Standards (Arrange-Act-Assert)

**Rule 4.1**: Every test function must follow a three-part structure: Arrange → Act → Assert, with
the parts separated by blank lines.

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

**Rule 4.2**: Simple tests (a single call + a single assertion) may omit the section comments, but
must not exceed 5 lines of logical code. Tests longer than 5 lines must explicitly mark all three
sections.

### Helper Function Standards

**Rule 5.1**: Setup logic that appears three or more times must be extracted into a helper function.

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

**Rule 5.2**: `unwrap()` / `expect()` in a helper function must print sufficient context when it
panics. You may call `unwrap()` directly in a test function body (`#[test] fn ...`)—on failure, Rust
automatically prints the line number; however, when a helper function fails, the line number points
to the helper's definition, and the calling context is not visible.

```rust
// 🟢 Good—print the source content when the helper fails
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 Garbage—you cannot see which source file caused the problem
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, immediately after the
`use` imports. If they are shared by multiple test modules, place them in `tests/mod.rs` and export
them with `pub(crate)`.

### Assertion Style

**Rule 6.1**: Prefer `assert!(matches!(...))` when matching enum variants; `if let` + `panic!` is
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

**Rule 6.2**: Use `assert_eq!` for exact-value comparisons and `assert!` for boolean assertions.
Using `assert!(a == b)` in place of `assert_eq!(a, b)` is prohibited.

**Rule 6.3**: Every assertion must include a custom error message unless the assertion itself fully
describes the reason for failure.

```rust
// 🟢 Good—quickly locate the failure
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 Good—assert_eq! automatically prints the value difference; no extra message is needed
assert_eq!(error_count, 0);

// 🔴 Garbage—all you know is "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: Assertions must use the order `assert_eq!(actual, expected)`, with the actual value
first and the expected value second.

### Anti-Pattern Checklist

The following patterns are prohibited, along with their alternatives:

| Anti-Pattern                                           | Problem                                                                          | Alternative                                                                                      |
| ------------------------------------------------------ | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Inline tests using `#[cfg(test)] mod tests { ... }`    | Bloats source files, obscures module boundaries, and makes refactoring difficult | Place test code in a separate `tests/` directory and declare it with `mod tests;` (see Rule 1.4) |
| Tests accommodate incorrect code behavior              | Masks specification deviations and legitimizes bugs                              | Correct the code against the specification and keep the tests unchanged                          |
| Deriving test expectations from code output            | Turns tests into a "recording of the current implementation"                     | Derive expected values from the specification                                                    |
| Permanently marking tests with `#[ignore]`             | Hides rotting tests                                                              | Fix or delete them                                                                               |
| Debug output with `println!`                           | Pollutes test output                                                             | Use explicit `assert!` assertions                                                                |
| `thread::sleep`                                        | Causes flaky tests and slows execution                                           | Use synchronization mechanisms or mocks                                                          |
| Manipulating the real file system in tests             | Is slow and not repeatable                                                       | Use `tempfile`                                                                                   |
| Depending on test execution order                      | Causes flaky tests                                                               | Set up each test independently                                                                   |
| A test function exceeds 30 lines of logic              | No one can understand it                                                         | Split the test or use helper functions                                                           |
| `unwrap()` in a helper function without context        | Makes failures difficult to locate                                               | Use `expect("why")` or a custom panic message (see Rule 5.2)                                     |
| Copying and pasting the same setup three or more times | Makes changes costly                                                             | Extract a helper function                                                                        |

---

## Integration Test Standards

### Test Organization

**Rule 7.1**: Integration tests must be placed in the project's root `tests/` directory. The entry
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

**Rule 7.3**: Integration tests must be performed through the project's public APIs. Directly
referencing internal `crate::` modules in integration tests is prohibited. Use the public
`yaoxiang::` path instead.

```rust
// 🟢 Good—through the public API
use yaoxiang::run;

// 🔴 Garbage—bypasses the public API boundary
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests should prefer inline source strings. Use an external fixture file
only when the source exceeds 30 lines; place fixture files in `tests/fixtures/`.

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

---

## Benchmark Standards

### Criterion.rs Usage Standards

**Rule 10.1**: Benchmarks must be placed in the `benches/` directory, with `benches/lib.rs` as the
entry file. Organize them into files by test topic.

```
benches/
├── lib.rs              # Entry point; defines criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # Cross-language comparison benchmark
├── parser.rs           # Parser benchmark
└── codegen.rs          # Code generation benchmark
```

**Rule 10.2**: Every benchmark function must include a module documentation comment `//!` describing
the test's purpose and the metric being measured.

```rust
//! YaoXiang interpreter performance benchmark
//!
//! Measurement metric: elapsed time per iteration (wall time)
//! Baseline: native Rust implementation
```

### Preventing Compiler Optimizations

**Rule 11.1**: The output of every benchmark must be passed through `criterion::black_box` to
prevent the compiler from eliminating it as an optimization.

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

**Rule 12.1**: Benchmark names must follow the format `<module-under-test>_<scenario>`, using
lowercase words separated by underscores. This is consistent with the unit test naming rules.

**Rule 12.2**: You must use `criterion_group!` to logically group related benchmarks. Placing all
benchmarks in a single group is prohibited.

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## Documentation Test Standards

### Use Cases

**Rule 13.1**: Every `pub` function, type, and method must include at least one runnable code
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

**Rule 13.2**: Documentation test examples must compile and their assertions must succeed. Examples
marked with `ignore` are prohibited unless the example demonstrates a compile-time error.

````rust
/// ```ignore
/// // Demonstrates a compile-time error—may be ignored
/// let x: int = "string";
/// ```
````

### Coverage Requirements

**Rule 14.1**: Documentation tests only need to cover an API's happy path. Boundary cases and error
paths are covered by unit tests.

**Rule 14.2**: Documentation test examples must be concise—no more than 10 lines. If an example
requires more context, the API design is at fault.

---

## Property Test Standards

### Use Cases

**Rule 15.1**: The following scenarios must use property tests (proptest or quickcheck) instead of
hand-written cases for multiple boundary values:

| Scenario                                        | Example                                |
| ----------------------------------------------- | -------------------------------------- |
| Parser round-trip                               | `parse(pretty_print(ast)) == ast`      |
| Serialization/deserialization                   | `deserialize(serialize(data)) == data` |
| Mathematical-operation identities               | `a + b == b + a`                       |
| Compiler optimization does not change semantics | `eval(code) == eval(optimize(code))`   |

**Rule 15.2**: Property tests must use `proptest` as the primary property-testing framework (it is
already declared in `Cargo.toml` under `dev-dependencies`).

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

**Rule 16.1**: Every property test must have an explicit property declaration—the comment must state
the invariant being verified.

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

**Rule 16.2**: If a property test fails, you must use `proptest`'s regression mechanism—add the
failing input to the `proptest-regressions/` directory. Do not replace it with a hand-written
ordinary test.

---

## Coverage Requirements

### Coverage Targets for New Code

**Rule 17.1**: New code must meet the following test coverage requirements:

| Code Type                                        | Line Coverage | Branch Coverage |
| ------------------------------------------------ | ------------- | --------------- |
| Core compiler modules (frontend/middle/backends) | ≥ 85%         | ≥ 80%           |
| Utility/helper modules (util)                    | ≥ 75%         | ≥ 70%           |
| Runtime modules (vm/runtime)                     | ≥ 80%         | ≥ 75%           |
| Standard library (std)                           | ≥ 75%         | ≥ 70%           |
| Error handling and diagnostics                   | ≥ 90%         | ≥ 85%           |

**Rule 17.2**: Error-handling paths (all `Err` branches) must have 100% coverage. User-visible error
messages must be verified by tests.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, the author must check the following items:

- [ ] `python scripts/dev/testsuite.py run fast` passes—before submitting, this self-check runs
      `--lib` (src/ unit tests) and four integration-side targets (`integration` /
      `formatter_integration` / `yx_runner` / `yx_multifile_runner`); the first stage of
      `pre-commit` is this command, and the second runs `cargo test --all --quiet` to block the
      commit
- [ ] `cargo test --doc` passes—doctests are not covered by `run fast` and are handled by the CI
      test job
- [ ] `cargo bench` shows no performance regressions (if hot-path code has changed)
- [ ] New code meets the coverage targets
- [ ] Test names comply with the naming standards
- [ ] Every test file declares its corresponding specification section (Rule 2.1)
- [ ] Test expectations come from specification definitions, not from the "current output of the
      code"
- [ ] There are no tests marked with `#[ignore]` (unless accompanied by a comment containing an
      explicit issue number)
- [ ] There are no unnecessary `unwrap()` calls (use `expect` or a custom panic message instead)
- [ ] The commit message uses the `:white_check_mark: test:` type
- [ ] **No test expectations were changed because "the code behavior does not conform to the
      specification"—change the code, not the tests**
- [ ] **There are no inline tests** (`#[cfg(test)] mod tests { ... }` must be changed to
      `mod tests;` + separate files; see Rule 1.4)

**Rule 18.2**: Reviewers must reject PRs that contain any of the following problems:

- Only happy-path tests, with no error-path coverage
- `thread::sleep` in tests or a dependency on execution order
- Copy-pasted test code appearing more than three times without extracting a helper function
- Test names that do not comply with the naming standards
- Permanently ignored tests using `#[ignore]`
- **Tests that accommodate incorrect code behavior** (when the code does not conform to the
  specification, the test is modified rather than the code)
- **Tests that do not declare their corresponding specification section** (see Rule 2.1)
- **Test expectations derived from code output rather than specification definitions** (a test
  derived in reverse is effectively no test at all)
- **Inline tests** (`#[cfg(test)] mod tests { ... }` rather than `mod tests;` + separate files; see
  Rule 1.4)
- Tests that only verify "does not panic" without asserting specific behavior
- Failing tests that exposed code bugs were deleted instead of fixing the code and then observing
  the tests turn green

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

# Display test output (stdout is hidden by default)
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

### C. New Test File Checklist

When creating a new test module, ensure that it includes the following files:

```
# Add tests under src/<module>/
src/<module>/tests/
├── mod.rs          # Module declarations + shared helpers
└── <subject>.rs    # Test file, named after the source file under test

# Add integration tests under tests/
tests/
├── integration.rs   # Update: add #[path] declarations
└── integration/
    └── <topic>.rs   # New test file
```

### D. References

- [YaoXiang Language Specification](../reference/language-spec/index.md) —— **The authoritative
  source for tests**
- [Accepted RFCs](../rfc/index.md) —— **The authoritative source for design decisions**
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Convention](commit-convention.md)
- [Project Contribution Guide](contributing.md)

---

> 💡 **Remember**: Tests do not verify whether your code "runs"—they verify whether your code
> conforms to the specification. When the specification changes, the tests change with it. If the
> code is wrong, fix the code; do not fix the tests. **The code serves the specification, and tests
> safeguard the specification. The moment tests accommodate the code, you have lost all
> protection.**
