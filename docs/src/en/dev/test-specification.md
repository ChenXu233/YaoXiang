---
title: 'Test Writing Standards'
description:
  Hard standards for test writing in the YaoXiang project, defining the writing criteria for unit
  tests, integration tests, benchmarks, documentation tests, and property tests
---

# Test Writing Standards

This document defines the hard standards for test writing in the YaoXiang project. All contributors
must follow the rules below; violations will be required to be modified during Code Review.

---

## Table of Contents

- [General Principles](#general-principles)
- [yx Corpus and Library Test Tiers](#yx-corpus-and-library-test-tiers)
- [Unit Test Standards](#unit-test-standards)
- [Integration Test Standards](#integration-test-standards)
- [Benchmark Standards](#benchmark-standards)
- [Documentation Test Standards](#documentation-test-standards)
- [Property Test Standards](#property-test-standards)
- [Coverage Requirements](#coverage-requirements)
- [Appendix](#appendix)

---

## General Principles

### Scope of Application

This standard applies to all Rust test code in the YaoXiang project, including:

| Test Type          | Location                   | Framework                  |
| ------------------ | -------------------------- | -------------------------- |
| Unit Test          | `src/<module>/tests/`      | `#[test]` + `#[cfg(test)]` |
| Integration Test   | `tests/`                   | `#[test]`                  |
| Benchmark          | `benches/`                 | Criterion.rs               |
| Documentation Test | API documentation comments | `cargo test --doc`         |
| Property Test      | Anywhere in tests          | proptest / quickcheck      |

### Core Principles

**Principle 0: The authoritative source for tests is the specification, not the code.** This is the
most important principle in this document. Tests verify whether the code conforms to the
specification, not whether the code "runs correctly under the current implementation". When a test
finds that the code behavior is inconsistent with the specification, **fix the code, never fix the
test**.

The specification files are located at:

- `docs/src/design/language-spec.md` — Language core specification
- `docs/src/design/rfc/accepted/` — Accepted RFC design documents

The top of every test file must declare the corresponding specification section (see Rule 2.1). Any
developer should be able to take the specification document and compare it with the test to verify
the correctness of the implementation. Conversely — if a piece of code has no corresponding
specification description, it should not exist, let alone be tested.

```rust
// 🟢 好——测试直接引用规范，验证代码是否遵循规范
//! 字面量测试 — 基于语言规范 §2.6
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮点数（带小数点和指数）
//! §2.6.3: 字符串（转义序列 \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 插值

#[test]
fn test_decimal_literal_parsing() {
    // 规范 §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 垃圾——测试迁就了当前代码的实现行为，而非验证规范
#[test]
fn test_literal_1() {
    // 不知道这段代码对应规范的哪一节
    // 如果 parse_literal 返回了错误的值，这个测试会"绿灯通过"
    // 因为它只验证了函数不 panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**Scenario**: You wrote a test and found that the code behavior does not match the specification.
You have two choices:

| Wrong Approach                                                       | Correct Approach                                                   |
| -------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Modify the test to make it "pass"                                    | Modify the code so that the behavior conforms to the specification |
| Add `#[ignore]` to the test                                          | Fix the code implementation immediately                            |
| Add special conditional branches in the test to accommodate the code | Remove the branches and let the test expose the problem directly   |

Remember: **Red light = the code is wrong, not the test.** (Unless your test itself has a bug, which
is another matter.)

**Principle 1: Tests are documentation.** Any developer should be able to understand the behavior of
the code under test by reading the test, without needing additional comments or external
documentation.

```rust
// 🟢 好——测试名说了被测什么、期望什么
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 垃圾——谁也不知道这测的什么
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**Principle 2: Zero tolerance for flaky tests.** Tests must be reproducible in any environment.
Tests that depend on random numbers, system time, or thread scheduling order must use fixed seeds or
replace them with mocks.

**Principle 3: One test tests one thing.** If a test name needs to use "and" to connect multiple
behaviors, split it into multiple tests.

```rust
// 🟢 好——每个测试只验证一个场景
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 垃圾——一个测试塞了太多无关内容
#[test]
fn test_parser() {
    // 测 tokenize，测 parse，测 typecheck，测 codegen...
}
```

**Principle 4: Test behavior, not implementation.** Refactoring the internal implementation should
not cause tests to fail. If changing one line of implementation code causes 10 tests to fail, your
tests are written incorrectly.

But there is a key distinction here: **The definition of "behavior" comes from the specification,
not from the current code's performance.** If the code changes behavior (i.e., a new behavior that
does not conform to the specification), the test must fail. If you cannot do this, your test is a
"test that accommodates the code" — it lets bugs drive right through.

```
Specification (language-spec.md / RFC)  ──Defines──►  Expected Behavior  ──Drives──►  Test
                                           │
Current Code  ──Implements──►  Actual Behavior  ──Compared──►  Test Result

If Actual Behavior ≠ Expected Behavior:
  Test must fail (red light)  ──►  Fix code  ──►  Test passes (green light)

If Actual Behavior = Expected Behavior (but implementation is bad):
  Test passes  ──►  Refactor implementation  ──►  Test still passes  ← This is the meaning of Principle 4
```

**Principle 5: Do not write fallback/compatibility/mode-conditional test code.** The test
environment is one you can fully control. If you need `#[cfg(not(ci))]` to skip a test, that test
has a fundamental design problem.

### Terminology Definitions

| Term               | Definition                                                                                    |
| ------------------ | --------------------------------------------------------------------------------------------- |
| Unit Test          | Tests the behavior of a single function or module, does not depend on external systems        |
| Integration Test   | Tests the collaboration of multiple modules, through public APIs or command-line entry points |
| Benchmark          | Measures code performance, detects performance regressions                                    |
| Documentation Test | Executable code examples embedded in documentation comments                                   |
| Property Test      | Tests that verify invariants (properties) based on random input                               |

### Association with Commit Standards

All test-related commits must use the `:white_check_mark: test:` type, refer to
[Commit Standards](./commit-convention.md).

```
:white_check_mark: test(parser): Add tests for Pratt parser infix expressions
:white_check_mark: test(codegen): Complete tests for switch statement IR generation
```

---

## yx Corpus and Library Test Tiers

This standard constrains **Rust-side test code**. The tests of the YaoXiang language itself (`.yx`
corpus and library tests) are divided into two layers according to the object under test. The system
design and judgment contract belong to RFC-036 (§7 Suite Collection / §8 Negative Three Layers / §9
Test System Layering), and the corpus writing details belong to `tests/yaoxiang/TEST_STANDARDS.md`:

- **Language Usability Corpus** (`tests/yaoxiang/`) — The object under test is the language itself;
  std is only used as an assertion tool. The corpus is divided into three categories of judgment
  according to the failure layer: behavior tests / compile-time rejection tests / runtime failure
  tests
- **Library Tests** (with the library) — The object under test is the library's public API contract;
  the std yx-level tests are located in `src/std/tests/`, and the future user package tests are
  within the package and discovered via `[tool.test]`

The `.yx` test file header format, markers (`[test:error]` / `[test:ignore]` / `[test:runtime]` /
`预期: EXXXX`) and assertion conventions are subject to TEST_STANDARDS.md; the judgment parsing is
implemented by `src/util/test_markers.rs` shared by dual runners (Rust side, constrained by this
standard).

---

## Unit Test Standards

### File Organization

**Rule 1.1**: The `tests/` directory for unit tests must be **at the same level** as the `mod.rs` of
the module under test. The `tests/` directory does not aggregate upward, nor does it cross levels.

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——声明同级 tests/
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——pratt 自己的测试
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # parser 模块级别的测试（不包含 pratt 子模块的内容）
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

Key criterion: **Wherever the `tests/` directory is placed, the `mod.rs` in that directory must
declare it with `#[cfg(test)] mod tests;`.**

**Rule 1.1 Supplement: Upward aggregation is prohibited.** The tests of a submodule must be placed
in the submodule's own `tests/`, and must not be aggregated into the parent-level `tests/`.

| Module Type                      | Test Location                     | Example                                      |
| -------------------------------- | --------------------------------- | -------------------------------------------- |
| Directory module (with `mod.rs`) | The `tests/` under that directory | `emitter/tests/`, `codes/tests/`             |
| Single-file module (only `.rs`)  | Parent's `tests/`                 | `session.rs` → `diagnostic/tests/session.rs` |

```text
# ✅ 正确：每个目录模块的测试各自独立
src/util/diagnostic/
├── codes/
│   ├── mod.rs              # #[cfg(test)] mod tests;
│   └── tests/              # ✅ codes 自己的测试
│       ├── mod.rs
│       └── codes.rs
├── emitter/
│   ├── mod.rs              # #[cfg(test)] mod tests;
│   └── tests/              # ✅ emitter 自己的测试
│       ├── mod.rs
│       ├── text.rs
│       └── ansi.rs
└── tests/                  # ✅ diagnostic 级别（单文件模块）
    ├── mod.rs
    ├── session.rs
    ├── suggest.rs
    └── collect.rs

# ❌ 错误：将 emitter 和 codes 的测试聚合到 diagnostic/tests/
src/util/diagnostic/
└── tests/
    ├── mod.rs              # ❌ 被迫声明 mod emitter; mod codes;
    ├── emitter/            # ❌ 应该在 emitter/tests/
    └── codes/              # ❌ 应该在 codes/tests/
```

#### Test Placement Rules for Single-File Modules vs. Directory Modules

**Key difference**: The form of the module's organization determines where tests are placed.

| Module Type            | Criterion                                 | Test Location                     | Example                                       |
| ---------------------- | ----------------------------------------- | --------------------------------- | --------------------------------------------- |
| **Directory module**   | Has an independent directory and `mod.rs` | The `tests/` under that directory | `inference/tests/`                            |
| **Single-file module** | Only `.rs` file, no independent directory | Parent module's `tests/`          | `overload.rs` → `typecheck/tests/overload.rs` |

**Detailed Explanation**:

```
src/frontend/core/typecheck/
├── mod.rs                          # typecheck 模块的 mod.rs
├── checker.rs                      # 单文件模块
├── environment.rs                  # 单文件模块
├── overload.rs                     # 单文件模块
├── type_eval.rs                    # 单文件模块
├── dead_code.rs                    # 单文件模块
├── spawn_placement.rs              # 单文件模块
├── signature.rs                    # 单文件模块
├── types.rs                        # 单文件模块
│
├── tests/                          # ✅ typecheck 的测试目录
│   ├── mod.rs                      # 声明单文件模块的测试
│   ├── checker.rs                  # checker.rs 的测试
│   ├── environment.rs              # environment.rs 的测试
│   ├── overload.rs                 # overload.rs 的测试（单文件模块测试放这里）
│   ├── type_eval.rs                # type_eval.rs 的测试
│   ├── dead_code.rs                # dead_code.rs 的测试
│   ├── spawn_placement.rs          # spawn_placement.rs 的测试
│   ├── signature.rs                # signature.rs 的测试
│   └── types.rs                    # types.rs 的测试
│
├── inference/                      # 目录模块（有 mod.rs）
│   ├── mod.rs                      # #[cfg(test)] mod tests; ——声明同级 tests/
│   ├── expressions.rs
│   ├── statements.rs
│   ├── patterns.rs
│   ├── bounds.rs
│   ├── subtyping.rs
│   ├── generics.rs
│   ├── compatibility.rs
│   ├── scope.rs
│   ├── assignment.rs
│   └── tests/                      # ✅ inference 的测试目录
│       ├── mod.rs
│       ├── expressions.rs          # expressions.rs 的测试
│       ├── statements.rs           # statements.rs 的测试
│       └── ...
│
└── traits/                         # 已删除（逻辑合并进 types/trait_data.rs）
```

**Why are tests of single-file modules placed in the parent `tests/`?**

Because single-file modules (such as `overload.rs`) do not have their own `mod.rs`, they cannot
declare `#[cfg(test)] mod tests;`. According to the Rust module system, test files must be declared
by some `mod.rs` to compile. Therefore, the tests of a single-file module can only be declared by
the parent module's `mod.rs` and placed in the parent's `tests/` directory.

**Judgment Process**:

```
Encounter a module, decide where to place the test?
│
├── Is the module a directory (with mod.rs)?
│   └── Yes → Create tests/ in that directory, declared by that directory's mod.rs
│
├── Is the module a single file (only .rs)?
│   └── Yes → Tests placed in the parent's tests/ directory, declared by the parent's mod.rs
│
└── Not sure?
    └── Check whether there is an independent directory and mod.rs
```

**Common Errors**:

```
# ❌ 错误 1：为单文件模块创建独立的 tests/ 目录
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ 不应该为单文件模块创建目录
    └── tests/
        └── overload.rs

# ❌ 错误 2：在单文件模块内声明 #[cfg(test)] mod tests;
# overload.rs
#[cfg(test)]                        # ❌ 单文件模块不能这样声明
mod tests;                          # 因为没有 overload/tests/ 目录

# ✅ 正确做法：测试放在父级 tests/
src/frontend/core/typecheck/
├── overload.rs                     # 源文件
└── tests/
    └── overload.rs                 # 测试文件，由 typecheck/mod.rs 声明
```

⚠️ **Anti-pattern — Do not write this way:**

```
# ❌ 错误：子模块的测试集中到父级
src/frontend/core/types/
├── mod.rs              # 本应只声明 base 和 computation
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ 父级 tests/ 包含子模块的测试
    ├── mod.rs          # ❌ 被迫声明 mod base; mod computation;
    ├── base/           # ❌ 这部分应放在 base/tests/
    │   └── var.rs
    └── computation/    # ❌ 这部分应放在 computation/tests/
        └── ...
```

```
# ✅ 正确的做法：每个模块的测试各自独立
src/frontend/core/types/
├── mod.rs              # 只声明 pub mod base; pub mod computation;
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——声明同级的 tests/
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; ——声明同级的 tests/
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**Why can't it aggregate upward?** Because the Rust module system requires `#[cfg(test)] mod tests;`
to determine the compilation of test files at the declaration site. If `types/mod.rs` declares
`mod tests;`, then the content of `types/tests/` is private to the `types` module — it should not
cross into the territory of `base` or `computation`. Each module's tests should be internal
implementation details of that module, not the parent module. This rule also applies to module
refactoring: when you split `types` into `base` and `computation`, the tests should also follow the
split modules, not stay in place. **The test directory does not mirror the source code structure,
but follows module boundaries.**

**Rule 1.2**: `tests/mod.rs` is only responsible for module declarations and re-exports, not for
test functions.

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

**Rule 1.3**: Each test file corresponds to only one source file. It is not allowed to mix the tests
of multiple source modules in one file.

**Rule 1.4**: Test declarations must use the file form `mod tests;` (with a semicolon), pointing to
the `tests/` directory at the same level. **It is forbidden to use the inline form
`mod tests { ... }` to write test code directly in the source file.**

```rust
// ✅ 正确——文件形式声明，测试代码在独立文件中
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests;

// 🔴 禁止——inline 形式，测试代码寄生在源文件内
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // 测试代码不应该出现在源文件中
    }
}
```

**Why is inline forbidden?**

1. Single responsibility for source files: Source files only contain implementation, test files only
   contain tests. Mixing them together means modifying tests requires scrolling to the bottom of the
   file, and modifying implementation requires skipping the tests.
2. Clear module boundaries: The `tests/` directory is a physical boundary, making it clear at a
   glance which modules have tests and which do not.
3. Refactoring safety: When modules are split, the `tests/` directory follows along; inline tests
   need to be manually stripped from the source file.
4. Code review: In PR diffs, source code changes and test changes are separate files, not mixed
   together.

### Module Declaration Standards

**Rule 2.1**: All test files must have a module-level documentation comment `//!` at the top,
explaining the source of the specification covered by the test (language specification section
number + RFC number). If a test does not reference any specification section, it means this code has
no specification basis — it should not exist.

```rust
//! 字面量测试 — 基于语言规范 §2.6
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮点数（带小数点和指数）
//! §2.6.3: 字符串（转义序列 \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 插值
```

**Why must the specification be referenced?** Because the expected value of a test comes from the
specification, not from "the output of the current code". If one day the code changes its output and
the test is updated accordingly, the test has protected nothing. Only specification-anchored tests
can distinguish between "intentional breaking change" and "unintentional regression".

**Rule 2.2**: The `use` imports in the test module must be precise to specific types/functions, and
glob imports `use super::*` are forbidden.

```rust
// 🟢 好——精确导入
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 垃圾——别人不知道你在测什么
use super::*;
```

### Naming Standards

**Rule 3.1**: The test function naming format is `test_<what>_<scenario>`, all lowercase separated
by underscores.

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**Rule 3.2**: The test function name must be self-explanatory. After reading the function name, you
should know what is being tested and what is expected. Numeric serial naming is forbidden.

```rust
// 🟢 好
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 垃圾——完全不知道测什么
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

**Rule 4.1**: Each test function must follow the three-part structure: Arrange → Act → Assert,
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

**Rule 4.2**: Simple tests (single call + single assertion) may omit the section comments, but must
not exceed 5 lines of logical code. Tests with more than 5 lines must explicitly mark the three
sections.

### Helper Function Standards

**Rule 5.1**: Setup logic that repeats 3 times or more must be extracted into a helper function.

```rust
// 🟢 好——提取公共 setup
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

**Rule 5.2**: The `unwrap()` / `expect()` in helper functions must print enough context when
panicking. Inside a test function body (`#[test] fn ...`), you can directly use `unwrap()` — Rust
automatically prints the line number on failure; but when a helper function fails, the line number
points to the helper function's definition, and you cannot see the context of the call site.

```rust
// 🟢 好——辅助函数失败时打印源码内容
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 垃圾——失败时你看不到是哪个源文件导致的问题
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**Rule 5.3**: Helper functions should be placed at the top of the test file, immediately after the
`use` imports. If shared by multiple test modules, place them in `tests/mod.rs` and export with
`pub(crate)`.

### Assertion Style

**Rule 6.1**: For matching enum variants, prefer `assert!(matches!(...))`, and do not use `if let` +
`panic!`.

```rust
// 🟢 好
assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(42)));

// 🔴 垃圾
if let TokenKind::IntLiteral(v) = tokens[0].kind {
    assert_eq!(v, 42);
} else {
    panic!("Expected IntLiteral");
}
```

**Rule 6.2**: For precise value comparisons, use `assert_eq!`; for boolean assertions, use
`assert!`. It is forbidden to use `assert!(a == b)` instead of `assert_eq!(a, b)`.

**Rule 6.3**: All assertions must come with custom error messages, unless the assertion itself fully
describes the reason for failure.

```rust
// 🟢 好——断言失败时能快速定位
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 好——assert_eq! 失败时自动打印值差异，不需要额外消息
assert_eq!(error_count, 0);

// 🔴 垃圾——失败了只知道 "assertion failed"
assert!(state.infix_info().is_some());
```

**Rule 6.4**: The assertion order must be `assert_eq!(actual, expected)`, with the actual value
first and the expected value second.

### Anti-Pattern List

The following prohibited writing styles and their alternatives:

| Anti-pattern                                              | Problem                                                                | Alternative                                                                                |
| --------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `#[cfg(test)] mod tests { ... }` inline tests             | Source file bloat, blurred module boundaries, refactoring difficulties | Test code placed in separate `tests/` directory, declared with `mod tests;` (see Rule 1.4) |
| Tests accommodating incorrect code behavior               | Masking specification deviations, legitimizing bugs                    | Modify code against the specification, keep tests unchanged                                |
| Reverse-engineering test expected values from code output | Tests become "recorders of the current implementation"                 | Derive expected values from the specification                                              |
| Permanent `#[ignore]` markers                             | Hiding rotting tests                                                   | Fix or delete                                                                              |
| `println!` debug output                                   | Polluting test output                                                  | Use `assert!` to make explicit assertions                                                  |
| `thread::sleep`                                           | Flaky failures + slow                                                  | Use synchronization mechanisms or mocks                                                    |
| Manipulating the real file system in tests                | Slow and not repeatable                                                | Use `tempfile`                                                                             |
| Depending on test execution order                         | Flaky failures                                                         | Independent setup for each test                                                            |
| A single test function exceeds 30 lines of logic          | Nobody can understand it                                               | Split tests or use helper functions                                                        |
| `unwrap()` in helper functions without reporting context  | Hard to locate                                                         | Use `expect("why")` or custom panic (see Rule 5.2)                                         |
| Copy-paste the same setup more than 3 times               | High modification cost                                                 | Extract helper functions                                                                   |

---

## Integration Test Standards

### Test Organization

**Rule 7.1**: Integration tests are placed in the `tests/` directory at the project root. The entry
file `tests/integration.rs` uses the `#[path]` attribute to import submodules.

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
code generation, executors, etc.), and must not be mixed.

**Rule 7.3**: Integration tests must be conducted through the project's public API. It is forbidden
to directly reference `crate::` internal modules in integration tests. Use the `yaoxiang::` public
path.

```rust
// 🟢 好——通过公共 API
use yaoxiang::run;

// 🔴 垃圾——绕过了公共 API 边界
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### Test Data Management

**Rule 8.1**: Integration tests prefer inline source code strings. Only when the source code exceeds
30 lines, use external fixture files (placed in `tests/fixtures/`).

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

**Rule 8.2**: Fixture files must end with the `.yx` extension, and the file name describes the test
intent.

### E2E Coverage Principles

**Rule 9.1**: The integration test for each language feature must cover three paths:

| Path       | Description                                                      |
| ---------- | ---------------------------------------------------------------- |
| Happy path | Legal input produces expected output                             |
| Error path | Illegal input produces clear error message (not panic)           |
| Boundary   | Boundary values (empty input, maximum value, nested depth limit) |

**Rule 9.2**: Integration tests must not depend on the network, system environment variables, or
external services.

---

## Benchmark Standards

### Criterion.rs Usage Standards

**Rule 10.1**: Benchmarks are uniformly placed in the `benches/` directory, with the entry file
being `benches/lib.rs`. Divide files by test topic.

```
benches/
├── lib.rs              # 入口，定义 criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # 跨语言对比基准
├── parser.rs           # 解析器基准
└── codegen.rs          # 代码生成基准
```

**Rule 10.2**: Each benchmark function must include a module documentation comment `//!` explaining
the test purpose and measurement metrics.

```rust
//! YaoXiang 解释器性能基准测试
//!
//! 测量指标：单次迭代耗时（wall time）
//! 基准线：Rust 原生实现
```

### Preventing Compiler Optimizations

**Rule 11.1**: The output of all benchmarks must be prevented from being optimized away by the
compiler through `criterion::black_box`.

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
dynamically generated inside the `iter` closure — otherwise, what is measured is the total time of
data generation + the logic under test.

### Benchmark Grouping and Naming

**Rule 12.1**: The benchmark naming format is `<module_under_test>_<scenario>`, all lowercase
separated by underscores. Consistent with the unit test naming rules.

**Rule 12.2**: Must use `criterion_group!` to logically group related benchmarks. It is forbidden to
crowd all benchmarks into one group.

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
/// 将源码字符串分词为 Token 序列。
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

**Rule 13.2**: The code example in the documentation test must compile and its assertion must
succeed. It must not contain examples marked `ignore`, unless the example demonstrates a
compile-time error.

````rust
/// ```ignore
/// // 展示编译期错误——可以 ignore
/// let x: int = "string";
/// ```
````

### Coverage Requirements

**Rule 14.1**: Documentation tests need only cover the happy path of the API. Boundary cases and
error paths are covered by unit tests.

**Rule 14.2**: The example code in documentation tests must be concise — no more than 10 lines. If
the example needs a longer context, it indicates a problem with the API design.

---

## Property Test Standards

### Usage Scenarios

**Rule 15.1**: The following scenarios must use property tests (proptest or quickcheck) rather than
manually writing multiple boundary value cases:

| Scenario                                       | Example                                |
| ---------------------------------------------- | -------------------------------------- |
| Parser round-trip                              | `parse(pretty_print(ast)) == ast`      |
| Serialization/Deserialization                  | `deserialize(serialize(data)) == data` |
| Mathematical operation identity                | `a + b == b + a`                       |
| Compiler optimizations do not change semantics | `eval(code) == eval(optimize(code))`   |

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

**Rule 16.1**: Each property test must have a clear property declaration — the invariant being
verified must be written in a comment.

```rust
// 属性：任意整数字面量在 tokenize → tokens_to_string 后产生相同值
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**Rule 16.2**: If a property test finds a failure, you must use `proptest`'s regression mechanism —
add the failed input to the `proptest-regressions/` directory, and do not manually write a normal
test to replace it.

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

**Rule 17.2**: Error handling paths (all `Err` branches) must be 100% covered. The error messages
visible to users must be verified by tests.

### PR Review Checklist

**Rule 18.1**: Before submitting a PR, the author must self-check the following items:

- [ ] `cargo test` all pass
- [ ] `cargo test --doc` all pass
- [ ] `cargo bench` has no performance regression (if hot path changes are involved)
- [ ] New code meets coverage targets
- [ ] Test names follow naming standards
- [ ] Each test file declares the corresponding specification section (Rule 2.1)
- [ ] Test expected values come from specification definitions, not "output of the current code"
- [ ] No `#[ignore]`-marked tests (unless there is a clear issue number comment)
- [ ] No unnecessary `unwrap()` (should use `expect` or custom panic message)
- [ ] Commit message uses the `:white_check_mark: test:` type
- [ ] **No test expected values have been modified because "code behavior does not match the
      specification" — the code is changed, not the test**
- [ ] **No inline tests** (`#[cfg(test)] mod tests { ... }` must be changed to `mod tests;` +
      separate file, see Rule 1.4)

**Rule 18.2**: Reviewers must reject PRs containing the following issues:

- Only happy path tests, missing error paths
- `thread::sleep` in tests or dependence on execution order
- Copy-pasted test code more than 3 times without extracting helper functions
- Test names do not follow naming standards
- Existence of permanently `#[ignore]`d tests
- **Tests accommodating incorrect code behavior** (modifying tests instead of code when code does
  not match the specification)
- **Tests do not declare the corresponding specification section** (see Rule 2.1)
- **Test expected values come from code output rather than specification definition** (tests
  reverse-engineered are equivalent to no test)
- **Existence of inline tests** (`#[cfg(test)] mod tests { ... }` instead of `mod tests;` + separate
  file, see Rule 1.4)
- Tests only verify "does not panic" without asserting specific behavior
- Deleted failing tests that exposed code bugs (instead of fixing the code and then seeing it turn
  green)

---

## Appendix

### A. Test Command Quick Reference

```bash
# Run all tests
cargo test

# Only run unit tests
cargo test --lib

# Only run integration tests
cargo test --test integration

# Only run documentation tests
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
:white_check_mark: test(<scope>): <brief description>

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

When creating a new test module, make sure to include the following files:

```
# Add tests under src/<module>/
src/<module>/tests/
├── mod.rs          # 模块声明 + 公共辅助函数
└── <subject>.rs    # 测试文件，对应被测源文件命名

# Add integration tests under tests/
tests/
├── integration.rs   # 更新：添加 #[path] 声明
└── integration/
    └── <topic>.rs   # 新测试文件
```

### D. References

- [YaoXiang Language Specification](../../design/language-spec.md) — **The authoritative source for
  tests**
- [Accepted RFCs](../../design/rfc/accepted/) — **The authoritative source for design decisions**
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [Project Commit Standards](./commit-convention.md)
- [Project Contribution Guide](./contributing.md)

---

> 💡 **Remember**: Tests do not verify whether your code "runs" — they verify whether your code
> conforms to the specification. The specification changes, and tests follow the specification. When
> the code is written incorrectly, fix the code, not the test. **Code serves the specification,
> tests guard the specification. The moment tests accommodate the code, you have lost all
> protection.**
