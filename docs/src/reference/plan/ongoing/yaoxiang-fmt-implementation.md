# YaoXiang 代码格式化工具实现计划

> **任务**：实现 `yaoxiang fmt` 代码格式化工具
> **基于 RFC**：RFC-010 统一语法、RFC-017 LSP 支持设计
> **日期**：2026-02-28
> **状态**：阶段 0-2 已完成 ✅
> **目标版本**：v0.6.7
> **实现日期**：2026-02-28

---

## 概述

本计划基于 issue_13 需求文档，将 `yaoxiang fmt` 格式化工具分解为多个实现阶段。每个阶段包含详细的目标、验收标准和测试项目。

### 架构选择

采用 **AST + 源映射** 方案：
1. **Lexer** 记录注释/空白位置 → 生成 Token 流 + 源映射
2. **Parser** 生成 AST（包含 Span 信息）
3. **Formatter** 根据配置遍历 AST + 源映射 → 输出格式化代码

**关键优势**：
- 保留完整注释信息
- 支持复杂的代码结构重构
- 与 LSP 深度集成

---

## 阶段 0：前置准备（源映射系统） ✅ 已完成

> **重要性**：此阶段是格式化工具的前提，必须先完成
> **目标版本**：v0.8
> **实现文件**：`src/formatter/source_map.rs`

### 0.1 源映射（SourceMap）数据结构

**实现目标**：
- 设计并实现 `SourceMap` 结构，记录源代码位置映射
- 记录内容：注释位置、空白区域、原始 Token 位置
- 支持从字符偏移到行/列的转换

**数据结构设计**：
```
SourceMap {
    source: String,                    // 原始源代码
    comments: Vec<Comment>,            // 注释列表
    tokens: Vec<TokenWithSpan>,        // Token + 位置
    line_offsets: Vec<usize>,          // 每行起始偏移
    blank_lines: Vec<usize>,           // 空白行号列表
}

Comment {
    content: String,
    span: Span,
    style: CommentStyle,  // SingleLine, MultiLine, Doc
}
```

**验收标准**：
- [x] SourceMap 正确解析源代码
- [x] 所有注释（单行、多行、文档注释）位置被记录
- [x] 空白行位置被正确记录
- [x] 支持字节偏移到行列的转换

**测试项目**：
- [x] 单行注释位置记录测试
- [x] 多行注释位置记录测试
- [x] 嵌套注释位置记录测试
- [x] 空白行检测测试
- [x] 源映射偏移转换测试

---

### 0.2 Lexer 增强 ✅ 已完成

**实现目标**：
- 修改 Lexer，使其在解析时记录注释而非跳过
- 在 `tokenize()` 返回时同时返回 `SourceMap`
- 保持向后兼容，不影响现有 Token 流

**实现说明**：采用独立 SourceMap 扫描方案，不修改 Lexer 本身。
`SourceMap::build()` 独立扫描源代码提取注释信息，完全不影响现有 Lexer/Parser 流程。

**验收标准**：
- [x] 现有 Token 流输出不变
- [x] 注释作为独立 Token 或附加信息返回
- [x] 不影响解析性能

**测试项目**：
- [x] 回归测试：现有代码解析结果不变
- [x] 注释收集测试

---

## 阶段 1：Formatter 核心（CLI 命令） ✅ 已完成

> **目标版本**：v0.9
> **实现文件**：`src/formatter/` 目录

### 1.1 Formatter 基础结构 ✅ 已完成

**实现目标**：
- 创建 `src/formatter/` 目录结构
- 实现 `Formatter` 核心结构
- 实现 `FormatOptions` 配置

**目录结构**：
```
src/formatter/
├── mod.rs                 # 模块导出
├── formatter.rs           # Formatter 主结构
├── options.rs             # 格式化选项
├── context.rs             # 格式化上下文
├── writers/
│   ├── mod.rs
│   └── buffer.rs          # 格式化输出缓冲
├── handlers/
│   ├── mod.rs
│   ├── expr.rs            # 表达式格式化
│   ├── stmt.rs            # 语句格式化
│   ├── module.rs          # 模块格式化
│   └── comment.rs         # 注释格式化
├── rules/
│   ├── mod.rs
│   ├── indent.rs          # 缩进规则
│   ├── spacing.rs         # 空格规则
│   ├── line_break.rs      # 换行规则
│   └── alignment.rs       # 对齐规则
└── tests/
    └── mod.rs
```

**验收标准**：
- [x] 目录结构创建完成
- [x] Formatter 核心结构编译通过
- [x] 基础模块可以调用

**测试项目**：
- [x] 模块编译测试

---

### 1.2 表达式格式化 ✅ 已完成

**实现目标**：
- 实现基础表达式格式化：字面量、变量、二元运算、一元运算
- 实现函数调用格式化
- 实现括号保留策略

**实现文件**：`src/formatter/handlers/expr.rs`

**格式化规则**：
```
字面量:
  - 数字：保持原始格式
  - 字符串：根据 single_quote 配置
  - 布尔值：true/false

二元运算:
  - 运算符前后各一个空格
  - 保持运算符优先级换行

函数调用:
  - 参数换行时使用逗号对齐
  - 单行保持紧凑
```

**验收标准**：
- [x] 字面量格式化正确
- [x] 变量名格式化正确
- [x] 二元运算格式化正确
- [x] 函数调用格式化正确
- [x] 括号对正确保留

**测试项目**：
- [x] 字面量格式化测试（数字、字符串、布尔）
- [x] 二元运算格式化测试
- [x] 一元运算格式化测试
- [x] 函数调用格式化测试（单行、多行参数）
- [x] 括号保留测试

---

### 1.3 语句格式化 ✅ 已完成

**实现目标**：
- 实现变量声明格式化（let, mut）
- 实现函数定义格式化
- 实现类型定义格式化
- 实现控制流格式化（if, match, while, for）

**实现文件**：`src/formatter/handlers/stmt.rs`

**格式化规则**：
```
变量声明:
  x = 1
  mut y = 2

函数定义:
  add(a: i32, b: i32) -> i32 {
      a + b
  }

控制流:
  if condition {
      // ...
  } elif condition {
      // ...
  } else {
      // ...
  }

  match expr {
      pattern => value,
      _ => default,
  }
```

**验收标准**：
- [x] 变量声明格式化正确
- [x] 函数定义格式化正确
- [x] 类型定义格式化正确
- [x] if/elif/else 格式化正确
- [x] match 格式化正确
- [x] while/for 格式化正确

**测试项目**：
- [x] 变量声明格式化测试
- [x] 函数定义格式化测试（单行、多行参数）
- [x] 类型定义格式化测试
- [x] if/elif/else 格式化测试
- [x] match 格式化测试（多分支对齐）
- [x] while/for 格式化测试

---

### 1.4 模块格式化 ✅ 已完成

**实现目标**：
- 实现文件级格式化（Module）
- 实现导入语句格式化（use）
- 实现语句间空行处理
- 实现模块级注释保留

**实现文件**：`src/formatter/handlers/module.rs`

**格式化规则**：
```
导入语句:
  use foo
  use foo.bar
  use foo.{ a, b, c }

空行处理:
  - 最多保留一个空行
  - 保持注释周围的空行
  - 顶级语句间空行

模块注释:
  - 文件头注释保留
  - 文档注释保留
```

**验收标准**：
- [x] use 语句格式化正确
- [x] 空行处理正确
- [x] 模块级注释保留
- [x] 顶级语句顺序不变

**测试项目**：
- [x] use 语句格式化测试
- [x] 空行保留测试
- [x] 多语句格式化测试
- [x] 模块注释保留测试

---

### 1.5 注释处理 ✅ 已完成

**实现目标**：
- 实现单行注释格式化
- 实现多行注释格式化
- 实现文档注释保留
- 实现注释对齐

**实现文件**：`src/formatter/handlers/comment.rs`

**格式化规则**：
```
单行注释:
  // 保持原位置
  // 缩进时跟随代码

多行注释:
  /* 保持原始格式 */
  /* 多行
     注释 */

注释对齐:
  // Comment
  let x = 1;
```

**验收标准**：
- [x] 单行注释位置正确
- [x] 多行注释格式保留
- [x] 注释缩进正确
- [x] 注释与代码相对位置正确

**测试项目**：
- [x] 单行注释保留测试
- [x] 多行注释保留测试
- [x] 行末注释测试
- [x] 多行注释对齐测试
- [x] 注释与代码间距测试

---

### 1.6 配置集成 ✅ 已完成

**实现目标**：
- 集成现有 `FmtConfig` 配置
- 支持从 `yaoxiang.toml` 读取配置
- 支持 CLI 参数覆盖配置

**实现文件**：`src/formatter/options.rs`

**配置项**：
```
[fmt]
line_width = 120        # 最大行宽
indent_width = 4        # 缩进宽度
use_tabs = false       # 使用 tab
single_quote = false   # 字符串单引号
```

**验收标准**：
- [x] FmtConfig 正确加载
- [x] CLI 参数覆盖配置文件
- [x] 默认值正确

**测试项目**：
- [x] 配置文件读取测试
- [x] CLI 参数测试
- [x] 默认配置测试

---

### 1.7 CLI 命令集成 ✅ 已完成

**实现目标**：
- 实现 `yaoxiang fmt <file>` 命令
- 实现 `yaoxiang fmt <dir>` 命令
- 实现 `--check` 模式
- 实现 `--write` 原地写入模式
- 实现 `--stdout` 输出到标准输出

**实现文件**：`src/main.rs` (Commands::Format)

**命令设计**：
```
yaoxiang fmt [OPTIONS] <PATH>...

位置参数:
  PATH                  # 文件或目录路径

选项:
  --check               # 检查是否需要格式化，不修改文件
  --write, -w           # 原地写入（默认输出到 stdout）
  --stdout              # 输出到标准输出（默认）
  --indent <SIZE>       # 覆盖缩进宽度
  --line-width <WIDTH>  # 覆盖最大行宽
  --use-tabs            # 使用 tab 缩进
  --single-quote        # 使用单引号

退出码:
  0                     # 文件已格式化或写入成功
  1                     # --check 模式下有文件需要格式化
  2                     # 错误
```

**验收标准**：
- [x] 格式化单个文件正确
- [x] 格式化目录正确（递归处理 .yx 文件）
- [x] --check 模式正确检测未格式化文件
- [x] --write 模式正确原地写入
- [x] 退出码正确

**测试项目**：
- [x] 单文件格式化测试
- [x] 目录格式化测试
- [x] --check 模式测试
- [x] --write 模式测试
- [x] 退出码测试

---

## 阶段 2：LSP 集成 ✅ 已完成

> **目标版本**：v0.9

### 2.1 LSP 格式化 Handler ✅ 已完成

**实现目标**：
- 实现 `textDocument/formatting` 处理
- 实现 `textDocument/rangeFormatting` 处理
- 在 `capabilities.rs` 中声明支持

**实现文件**：
- `src/lsp/handlers/formatting.rs` — 格式化请求处理
- `src/lsp/server.rs` — 请求分发
- `src/lsp/capabilities.rs` — 能力声明

**LSP 方法**：
```
textDocument/formatting
  - 输入: DocumentFormattingParams
  - 输出: Vec<TextEdit>

textDocument/rangeFormatting
  - 输入: DocumentRangeFormattingParams
  - 输出: Vec<TextEdit>
```

**验收标准**：
- [x] 整文件格式化响应正确
- [x] 范围格式化响应正确
- [x] capabilities 正确声明

**测试项目**：
- [x] LSP 整文件格式化测试
- [x] LSP 范围格式化测试
- [x] capabilities 声明测试

---

### 2.2 文档变更触发格式化（未实现，待后续版本）

**实现目标**：
- 在文档保存时自动格式化（可选配置）
- 实现 `DocumentFormattingEdit` 协调

> **备注**：保存触发格式化可通过编辑器侧配置（如 VS Code 的 `editor.formatOnSave`）实现，无需 LSP 服务端额外处理。

**验收标准**：
- [ ] 格式化编辑正确应用
- [ ] 版本号正确处理

**测试项目**：
- [ ] 格式化编辑应用测试

---

## 阶段 3：高级特性（后续版本）

### 3.1 智能换行

**实现目标**：
- 实现超过行宽时的智能换行
- 保持运算符对齐
- 保持函数参数对齐

**换行策略**：
```
函数调用换行:
  foo(
      arg1,
      arg2,
  )

表达式换行:
  result = some_function()
      .chain()
      .filter()

match 分支对齐:
  match expr {
      Pattern1 => value1,
      Pattern2 => value2,
  }
```

**验收标准**：
- [ ] 超过行宽自动换行
- [ ] 换行后缩进正确
- [ ] 保持对齐

**测试项目**：
- [ ] 长表达式换行测试
- [ ] 函数参数换行测试
- [ ] 链式调用换行测试

---

### 3.2 导入语句排序

**实现目标**：
- 实现 use 语句自动排序
- 支持分组排序（标准库、外部 crates、项目内部）

**排序规则**：
```
1. 标准库 (std, core, alloc)
2. 外部 crates (from Cargo.toml)
3. 项目内部 (相对路径)

use std::collections::HashMap;
use crate::module::foo;
use some_crate::Bar;
```

**验收标准**：
- [ ] 导入语句正确排序
- [ ] 分组正确

**测试项目**：
- [ ] 导入排序测试
- [ ] 分组测试

---

## 测试策略

### 单元测试

每个格式化模块独立的单元测试：
- 表达式格式化测试
- 语句格式化测试
- 配置选项测试

### 集成测试

- CLI 命令测试
- LSP 协议测试
- 与真实编辑器集成测试（VS Code）

### 快照测试（Snapshot Testing）

使用快照测试确保格式化输出稳定：
- 收集社区标准代码样式
- 每次修改后对比快照
- 自动更新快照脚本

### 性能测试

- 大文件格式化性能
- 批量文件格式化性能
- 内存占用测试

---

## 验收标准汇总

### 阶段 0 ✅

- [x] SourceMap 正确解析源代码
- [x] 注释位置被记录
- [x] 空白行被记录

### 阶段 1 ✅

- [x] 表达式格式化正确
- [x] 语句格式化正确
- [x] 注释保留正确
- [x] 配置正确加载
- [x] CLI 命令正确工作
- [x] --check 模式正确

### 阶段 2 ✅（部分）

- [x] LSP 整文件格式化
- [x] LSP 范围格式化
- [x] capabilities 声明
- [ ] 保存时自动格式化（待后续版本，可由编辑器配置实现）

### 阶段 3（后续）

- [ ] 智能换行
- [ ] 导入排序

---

## 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| 注释丢失 | 源映射系统详细记录注释位置 |
| 语义改变 | 格式化前后 AST 等价性测试 |
| 性能问题 | 增量格式化、缓存机制 |
| 配置冲突 | 明确的配置优先级 |

---

## 参考资料

- [Rustfmt 设计文档](https://rust-lang.github.io/rfcs/rfcs-2437-rustfmt.html)
- [Prettier 架构](https://prettier.io/docs/en/architecture)
- [Language Server Protocol 规范](https://microsoft.github.io/language-server-protocol/)
