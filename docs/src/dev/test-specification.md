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

**原则 0：测试的权威来源是规范，不是代码。** 这是本文档最重要的原则。测试验证的是代码是否符合规范，而不是代码是否"按当前实现跑通了"。当测试发现代码行为与规范不一致时，**修正代码，绝不修正测试**。

规范文件位于：
- `docs/src/design/language-spec.md` —— 语言核心规范
- `docs/src/design/rfc/accepted/` —— 已接受的 RFC 设计文档

每个测试文件顶部必须声明对应的规范章节（见规则 2.1）。任何开发者应该能拿着规范文档对照测试，验证实现的正确性。反过来——如果一段代码没有对应的规范描述，它就不应该存在，更不应该被测试。

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

**场景**：你写了一个测试，发现代码行为与规范不符。你有两个选择：
| 错误做法 | 正确做法 |
|----------|----------|
| 修改测试让它"通过" | 修改代码，使行为符合规范 |
| 在测试里加 `#[ignore]` | 立即修复代码实现 |
| 给测试加特殊条件分支迁就代码 | 删除分支，让测试直接暴露问题 |

记住：**红灯 = 代码错了，不是测试错了。**（除非你的测试本身有 bug，那是另一回事。）

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

但这里有一个关键区分：**"行为"的定义来自规范，不是来自当前代码的表现。** 如果代码改了行为（即与规范不符的新行为），测试必须失败。做不到这一点，你的测试就是"迁就代码的测试"——它让 bug 长驱直入。

```
规范（language-spec.md / RFC）  ──定义──►  期望行为  ──驱动──►  测试
                                           │
当前代码  ──实现──►  实际行为  ──对比──►  测试结果

如果实际行为 ≠ 期望行为：
  测试必须失败（红灯）  ──►  修正代码  ──►  测试通过（绿灯）
  
如果实际行为 = 期望行为（但实现很烂）：
  测试通过  ──►  重构实现  ──►  测试仍然通过  ← 这才是原则 4 的含义
```

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

**规则 1.1**：单元测试的 `tests/` 目录必须与被测模块的 `mod.rs` **同级**。`tests/` 不向上聚合、不跨级汇总。

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

关键判断标准：**`tests/` 放在哪个目录，哪个目录的 `mod.rs` 就必须用 `#[cfg(test)] mod tests;` 声明它。**

#### 单文件模块 vs 目录模块的测试放置规则

**核心区别**：模块的组织形式决定了测试的放置位置。

| 模块类型 | 判断依据 | 测试位置 | 示例 |
|----------|----------|----------|------|
| **目录模块** | 有独立目录和 `mod.rs` | 该目录下的 `tests/` | `inference/tests/` |
| **单文件模块** | 只有 `.rs` 文件，无独立目录 | 父级模块的 `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

**详细说明**：

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
└── traits/                         # 目录模块（有 mod.rs）
    ├── mod.rs                      # #[cfg(test)] mod tests; ——声明同级 tests/
    ├── solver.rs
    ├── impl_check.rs
    ├── inheritance.rs
    ├── coherence.rs
    ├── auto_derive.rs
    ├── object_safety.rs
    ├── resolution.rs
    ├── std_traits.rs
    ├── gat/
    ├── specialization/
    └── tests/                      # ✅ traits 的测试目录
        ├── mod.rs
        ├── solver.rs               # solver.rs 的测试
        ├── impl_check.rs           # impl_check.rs 的测试
        └── ...
```

**为什么单文件模块的测试放在父级 `tests/`？**

因为单文件模块（如 `overload.rs`）没有自己的 `mod.rs`，它无法声明 `#[cfg(test)] mod tests;`。根据 Rust 模块系统，测试文件必须由某个 `mod.rs` 声明才能编译。因此，单文件模块的测试只能由父级模块的 `mod.rs` 声明，放在父级的 `tests/` 目录中。

**判断流程**：

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

**常见错误**：

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

⚠️ **反模式——不要这样写：**

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

**为什么不能向上聚合？** 因为 Rust 的模块系统要求 `#[cfg(test)] mod tests;` 在声明处决定测试文件的编译。如果 `types/mod.rs` 声明了 `mod tests;`，那么 `types/tests/` 的内容就是 `types` 模块的私有内容——它不应该跨进 `base` 或 `computation` 的领地。每个模块的测试应该是该模块的内部实现细节，而非父模块的。这条规则同样适用于模块重构：当你把 `types` 拆分为 `base` 和 `computation` 时，测试也应该跟着拆分后的模块走，而不是留在原地。**测试目录不镜像源码结构，而是跟随模块边界。**

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

**规则 2.1**：所有测试文件顶部必须有模块级文档注释 `//!`，说明测试覆盖的规范来源（语言规范章节号 + RFC 编号）。如果某个测试不引用任何规范章节，说明这段代码没有规范依据——它不应该存在。

```rust
//! 字面量测试 — 基于语言规范 §2.6
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮点数（带小数点和指数）
//! §2.6.3: 字符串（转义序列 \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 插值
```

**为什么必须引用规范？** 因为测试的期望值从规范来，不应当从"当前代码的输出"来。如果有一天代码改了输出而测试随之更新，那测试什么也没保护。只有规范锚定的测试才能区分"故意的 breaking change"和"无意的回归"。

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
| 测试迁就代码的错误行为 | 掩盖规范偏差，让 bug 合法化 | 对照规范修正代码，保持测试不变 |
| 根据代码输出反推测试期望值 | 测试变成"当前实现的录音机" | 从规范中推导期望值 |
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
- [ ] 每个测试文件声明了对应的规范章节（规则 2.1）
- [ ] 测试期望值来自规范定义，而非"当前代码的输出"
- [ ] 无 `#[ignore]` 标记的测试（除非有明确 issue 号注释）
- [ ] 无不必要的 `unwrap()` （应使用 `expect` 或自定义 panic 消息）
- [ ] 提交信息使用 `:white_check_mark: test:` 类型
- [ ] **没有因为"代码行为与规范不符"而修改测试期望值——改的是代码，不是测试**

**规则 18.2**：Reviewer 必须拒绝包含以下问题的 PR：

- 只有 happy path 测试，缺少错误路径
- 测试中有 `thread::sleep` 或依赖执行顺序
- 复制粘贴的测试代码超过 3 次而未提取辅助函数
- 测试名不符合命名规范
- 存在永久 `#[ignore]` 的测试
- **测试迁就代码的错误行为**（代码与规范不符时修改测试而非修改代码）
- **测试没有声明对应的规范章节**（参见规则 2.1）
- **测试期望值来自代码输出而非规范定义**（反推出来的测试等于没测）
- 测试只验证"不 panic"而不断言具体行为
- 删除了暴露代码 bug 的失败测试（而不是修复代码后再看到它变绿）

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

- [YaoXiang 语言规范](../../design/language-spec.md) —— **测试的权威来源**
- [已接受的 RFC](../../design/rfc/accepted/) —— **设计决策的权威来源**
- [Rust 测试文档](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs 用户指南](https://bheisler.github.io/criterion.rs/book/)
- [proptest 文档](https://docs.rs/proptest/latest/proptest/)
- [项目提交规范](./commit-convention.md)
- [项目贡献指南](./contributing.md)

---

> 💡 **记住**：测试不验证你的代码是否"能跑"——它验证你的代码是否符合规范。规范在变，测试跟着规范变。代码写错了，改代码，不要改测试。**代码服务于规范，测试守护规范。测试迁就代码的那一刻，你就失去了所有保护。**
