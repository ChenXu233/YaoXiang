---
title: 测试编写规范
description: YaoXiang 项目测试编写硬规范，定义单元测试、集成测试、基准测试、文档测试与属性测试的编写标准
---

# 测试编写规范

本文档定义了 YaoXiang 项目的测试编写硬规范。所有贡献者必须遵守以下规则，违反者将在 Code Review 中被要求修改。

---

## 目录

- [总则](#总则)
- [单元测试规范](#单元测试规范)
- [集成测试规范](#集成测试规范)
- [基准测试规范](#基准测试规范)
- [文档测试规范](#文档测试规范)
- [属性测试规范](#属性测试规范)
- [覆盖率要求](#覆盖率要求)
- [附录](#附录)

---

## 总则

### 适用范围

本规范适用于 YaoXiang 项目中所有 Rust 测试代码，包括：

| 测试类型 | 位置 | 框架 |
|----------|------|------|
| 单元测试 | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| 集成测试 | `tests/` | `#[test]` |
| 基准测试 | `benches/` | Criterion.rs |
| 文档测试 | API 文档注释 | `cargo test --doc` |
| 属性测试 | 任意测试位置 | proptest / quickcheck |

### 核心原则

**原则 1：测试即文档。** 任何开发者应该能通过阅读测试理解被测代码的行为，无需额外注释或外部文档。

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

**原则 2：零容忍随机失败。** 测试必须在任何环境下可重复执行。依赖随机数、系统时间、线程调度顺序的测试必须使用种子固定或用 mock 替代。

**原则 3：一个测试只测一件事。** 如果测试名需要用"和"连接多个行为，拆成多个测试。

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

**原则 4：测试行为，不测试实现。** 重构内部实现不应该导致测试失败。如果改了一行实现代码导致 10 个测试挂了，你的测试写错了。

**原则 5：不写回退/兼容/特定模式生效的测试代码。** 测试环境是你可以完全控制的环境。如果你需要 `#[cfg(not(ci))]` 来跳过某个测试，说明这个测试设计有根本问题。

### 术语定义

| 术语 | 定义 |
|------|------|
| 单元测试 | 测试单个函数或模块行为，不依赖外部系统 |
| 集成测试 | 测试多个模块协作，通过公共 API 或命令行入口 |
| 基准测试 | 测量代码性能，检测性能回归 |
| 文档测试 | 嵌入在文档注释中的可执行代码示例 |
| 属性测试 | 基于随机输入验证不变量（property）的测试 |

### 与提交规范的关联

所有测试相关提交必须使用 `:white_check_mark: test:` 类型，参照[提交规范](./commit-convention.md)。

```
:white_check_mark: test(parser): 添加 Pratt 解析器中缀表达式测试
:white_check_mark: test(codegen): 补全 switch 语句 IR 生成测试
```

---

## 单元测试规范

### 文件组织

**规则 1.1**：单元测试必须放在被测模块同级的 `tests/` 子目录中，与源码结构镜像。

```
src/frontend/core/parser/
├── mod.rs              # mod tests;  (或 #[cfg(test)] mod tests; 在里面声明)
├── ast.rs
├── pratt/
│   ├── mod.rs
│   └── tests/          # 镜像源码目录结构
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/
    ├── mod.rs           # 声明所有测试子模块
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

**规则 1.2**：`tests/mod.rs` 只负责模块声明和 re-export，不放测试函数。

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

**规则 1.3**：每个测试文件只对应一个源码文件。不允许多个源码模块的测试混在一个文件里。

**规则 1.4**：`#[cfg(test)]` 必须只在两种位置出现——`lib.rs` 中声明 `mod tests`，或者被测源文件内 inline 声明 `#[cfg(test)] mod tests;`。不得在其他地方使用。

```rust
// src/frontend/core/parser/mod.rs 或 lib.rs
#[cfg(test)]
mod tests;
```

### 模块声明规范

**规则 2.1**：所有测试文件顶部必须有模块级文档注释 `//!`，说明测试覆盖的源码模块和对应的规范章节。

```rust
//! 字面量测试 — 基于语言规范 §2.6
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮点数（带小数点和指数）
//! §2.6.3: 字符串（转义序列 \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 插值
```

**规则 2.2**：测试模块的 `use` 导入必须精确到具体类型/函数，禁止 glob 导入 `use super::*`。

```rust
// 🟢 好——精确导入
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 垃圾——别人不知道你在测什么
use super::*;
```

### 命名规范

**规则 3.1**：测试函数命名格式为 `test_<what>_<scenario>`，全小写下划线分隔。

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**规则 3.2**：测试函数名必须自解释。读完函数名就应知道测了什么、期望什么。禁止数字序号命名。

```rust
// 🟢 好
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 垃圾——完全不知道测什么
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**规则 3.3**：辅助函数不需要 `test_` 前缀，应该用动词或名词描述其用途。

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### 测试结构规范 (Arrange-Act-Assert)

**规则 4.1**：每个测试函数必须遵循三段式结构：准备（Arrange）→ 执行（Act）→ 断言（Assert），三段之间用空行分隔。

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

**规则 4.2**：简单测试（单一调用 + 单一断言）可以不写分段注释，但不能超过 5 行逻辑代码。超过 5 行的测试必须显式标出三段。

### 辅助函数规范

**规则 5.1**：重复出现 3 次及以上的 setup 逻辑必须提取为辅助函数。

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

**规则 5.2**：辅助函数中的 `unwrap()` / `expect()` 必须在 panic 时打印足够的上下文。测试函数体内 (`#[test] fn ...`) 可以直接 `unwrap()`——失败时 Rust 自动打印行号；但辅助函数内失败时，行号指向的是辅助函数定义处，看不到调用时的上下文。

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

**规则 5.3**：辅助函数应放在测试文件顶部，紧接 `use` 导入之后。如果被多个测试模块共享，放在 `tests/mod.rs` 中并 `pub(crate)` 导出。

### 断言风格

**规则 6.1**：枚举变体匹配优先使用 `assert!(matches!(...))`，不得使用 `if let` + `panic!`。

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

**规则 6.2**：精确值比较使用 `assert_eq!`，布尔断言使用 `assert!`。禁止使用 `assert!(a == b)` 替代 `assert_eq!(a, b)`。

**规则 6.3**：所有断言必须带自定义错误消息，除非断言本身已完整描述失败原因。

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

**规则 6.4**：断言顺序必须是 `assert_eq!(actual, expected)`，实际值在前，期望值在后。

### 反模式清单

以下是禁止的写法及其替代方案：

| 反模式 | 问题 | 替代方案 |
|--------|------|----------|
| `#[ignore]` 永久标记 | 隐藏腐烂的测试 | 修复或删除 |
| `println!` 调试输出 | 污染测试输出 | 使用 `assert!` 明确断言 |
| `thread::sleep` | 随机失败 + 慢 | 使用同步机制或 mock |
| 在测试中操作真实文件系统 | 慢且不可重复 | 使用 `tempfile` |
| 依赖测试执行顺序 | 随机失败 | 每个测试独立 setup |
| 一个测试函数超过 30 行逻辑 | 没人看得懂 | 拆分测试或用辅助函数 |
| 辅助函数中的 `unwrap()` 不报上下文 | 定位困难 | 使用 `expect("why")` 或自定义 panic（参见规则 5.2） |
| copy-paste 3 次以上相同 setup | 修改成本高 | 提取辅助函数 |

---

## 集成测试规范

### 测试组织

**规则 7.1**：集成测试放在项目根目录的 `tests/` 目录。入口文件 `tests/integration.rs` 使用 `#[path]` 属性引入子模块。

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**规则 7.2**：每个 `tests/integration/*.rs` 文件对应一个测试主题（编译器后端、代码生成、执行器等），不得混放。

**规则 7.3**：集成测试必须通过项目的公共 API 进行测试。禁止在集成测试中直接引用 `crate::` 内部模块。使用 `yaoxiang::` 公共路径。

```rust
// 🟢 好——通过公共 API
use yaoxiang::run;

// 🔴 垃圾——绕过了公共 API 边界
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### 测试数据管理

**规则 8.1**：集成测试优先使用内联源码字符串。只有当源码超过 30 行时，才使用外部 fixture 文件（放在 `tests/fixtures/`）。

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

**规则 8.2**：fixture 文件必须以 `.yx` 扩展名结尾，文件名描述测试意图。

### E2E 覆盖原则

**规则 9.1**：每个语言特性的集成测试必须覆盖三条路径：

| 路径 | 说明 |
|------|------|
| Happy path | 合法输入产生预期输出 |
| Error path | 非法输入产生清晰的错误信息（非 panic） |
| Boundary | 边界值（空输入、最大值、嵌套深度上限） |

**规则 9.2**：集成测试不得依赖网络、系统环境变量或外部服务。

---

## 基准测试规范

### Criterion.rs 使用规范

**规则 10.1**：基准测试统一放在 `benches/` 目录，入口文件为 `benches/lib.rs`。按测试主题分文件。

```
benches/
├── lib.rs              # 入口，定义 criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # 跨语言对比基准
├── parser.rs           # 解析器基准
└── codegen.rs          # 代码生成基准
```

**规则 10.2**：每个基准函数必须包含模块文档注释 `//!` 说明测试目的和测量指标。

```rust
//! YaoXiang 解释器性能基准测试
//!
//! 测量指标：单次迭代耗时（wall time）
//! 基准线：Rust 原生实现
```

### 防止编译器优化

**规则 11.1**：所有基准测试的被测输出必须通过 `criterion::black_box` 阻止编译器优化消除。

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

**规则 11.2**：基准测试的输入数据必须是 `const` 或 `lazy_static`，不得在 `iter` 闭包内动态生成——否则测量的是数据生成 + 被测逻辑的总时间。

### 基准分组与命名

**规则 12.1**：基准测试命名格式为 `<被测模块>_<场景>`，全小写下划线分隔。与单元测试命名规则一致。

**规则 12.2**：必须使用 `criterion_group!` 对相关基准进行逻辑分组。禁止所有基准挤在一个分组中。

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## 文档测试规范

### 使用场景

**规则 13.1**：所有 `pub` 函数、类型、方法必须在文档注释中包含至少一个可运行的代码示例。该示例通过 `cargo test --doc` 执行。

```rust
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
```

**规则 13.2**：文档测试的代码示例必须编译通过且断言成功。不得包含 `ignore` 标记的示例，除非该示例展示的是编译期错误。

```rust
/// ```ignore
/// // 展示编译期错误——可以 ignore
/// let x: int = "string";
/// ```
```

### 覆盖要求

**规则 14.1**：文档测试覆盖 API 的 happy path 即可。边界情况和错误路径由单元测试覆盖。

**规则 14.2**：文档测试中的示例代码必须简洁——不超过 10 行。如果示例需要更长的上下文，说明 API 设计有问题。

---

## 属性测试规范

### 使用场景

**规则 15.1**：以下场景必须使用属性测试（proptest 或 quickcheck）而不是手写多个边界值用例：

| 场景 | 示例 |
|------|------|
| 解析器 round-trip | `parse(pretty_print(ast)) == ast` |
| 序列化/反序列化 | `deserialize(serialize(data)) == data` |
| 数学运算恒等式 | `a + b == b + a` |
| 编译器优化不改变语义 | `eval(code) == eval(optimize(code))` |

**规则 15.2**：属性测试使用 `proptest` 作为主要的属性测试框架（已在 `Cargo.toml` 的 `dev-dependencies` 中声明）。

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

### 属性定义原则

**规则 16.1**：每个属性测试必须有明确的属性声明——注释中写明验证的不变量。

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

**规则 16.2**：如果属性测试发现失败，必须使用 `proptest` 的回归机制——将失败的输入添加到 `proptest-regressions/` 目录，不要手写一个普通测试代替。

---

## 覆盖率要求

### 新增代码覆盖率目标

**规则 17.1**：新增代码的测试覆盖率要求：

| 代码类型 | 行覆盖率 | 分支覆盖率 |
|----------|----------|------------|
| 核心编译器模块（frontend/middle/backends） | ≥ 85% | ≥ 80% |
| 工具/辅助模块（util） | ≥ 75% | ≥ 70% |
| 运行时模块（vm/runtime） | ≥ 80% | ≥ 75% |
| 标准库（std） | ≥ 75% | ≥ 70% |
| 错误处理和诊断 | ≥ 90% | ≥ 85% |

**规则 17.2**：错误处理路径（所有 `Err` 分支）必须 100% 覆盖。用户能看到的错误信息必须被测试验证过。

### PR 审查检查清单

**规则 18.1**：提交 PR 前，作者必须自查以下项目：

- [ ] `cargo test` 全部通过
- [ ] `cargo test --doc` 全部通过
- [ ] `cargo bench` 无性能回归（如涉及热路径变更）
- [ ] 新增代码符合覆盖率目标
- [ ] 测试命名符合命名规范
- [ ] 无 `#[ignore]` 标记的测试（除非有明确 issue 号注释）
- [ ] 无不必要的 `unwrap()` （应使用 `expect` 或自定义 panic 消息）
- [ ] 提交信息使用 `:white_check_mark: test:` 类型

**规则 18.2**：Reviewer 必须拒绝包含以下问题的 PR：

- 只有 happy path 测试，缺少错误路径
- 测试中有 `thread::sleep` 或依赖执行顺序
- 复制粘贴的测试代码超过 3 次而未提取辅助函数
- 测试名不符合命名规范
- 存在永久 `#[ignore]` 的测试

---

## 附录

### A. 测试命令速查

```bash
# 运行所有测试
cargo test

# 只运行单元测试
cargo test --lib

# 只运行集成测试
cargo test --test integration

# 只运行文档测试
cargo test --doc

# 运行特定测试（按名称过滤）
cargo test test_parse_expr

# 运行基准测试
cargo bench

# 显示测试输出（默认隐藏 stdout）
cargo test -- --nocapture

# 单线程运行（排查并发问题）
cargo test -- --test-threads=1

# 生成覆盖率报告（需要 cargo-tarpaulin）
cargo tarpaulin --out Html
```

### B. 提交信息模板

测试相关提交必须遵循以下模板：

```
:white_check_mark: test(<scope>): <简短描述>

<可选：覆盖的场景列表>
```

示例：

```
:white_check_mark: test(parser): 添加 Pratt 解析器中缀运算符测试

覆盖场景：
- 算术运算符优先级（+, -, *, /, %）
- 比较运算符链接（1 < x < 10）
- 逻辑运算符短路
- 赋值运算符右结合
```

### C. 新增测试文件清单

创建新的测试模块时，确保包含以下文件：

```
# 在 src/<module>/ 目录下新增测试
src/<module>/tests/
├── mod.rs          # 模块声明 + 公共辅助函数
└── <subject>.rs    # 测试文件，对应被测源文件命名

# 在 tests/ 目录下新增集成测试
tests/
├── integration.rs   # 更新：添加 #[path] 声明
└── integration/
    └── <topic>.rs   # 新测试文件
```

### D. 参考资料

- [Rust 测试文档](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs 用户指南](https://bheisler.github.io/criterion.rs/book/)
- [proptest 文档](https://docs.rs/proptest/latest/proptest/)
- [项目提交规范](./commit-convention.md)
- [项目贡献指南](./contributing.md)

---

> 💡 **记住**：测试是你在未来对自己代码的理解。写测试时的每一点投入，都在为未来省下 10 倍的调试时间。
