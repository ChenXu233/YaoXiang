---
title: 'RFC-017: 语言服务器协议（LSP）支持设计'
---

# RFC-017: 语言服务器协议（LSP）支持设计

> **状态**: 审核中
>
> **作者**: 晨煦
>
> **创建日期**: 2026-02-15
>
> **最后更新**: 2026-02-22

> **参考**: 查看 [完整示例](EXAMPLE_full_feature_proposal.md) 了解如何编写 RFC。

## ⚠️ 实现前置条件（重要）

在实现 LSP 之前，需要先解决以下两个核心问题：

### 问题 1：诊断错误收集

**现状**：当前类型检查器在遇到第一个错误时就直接返回（使用 `?` 操作符），无法收集所有错误。

**LSP 需求**：IDE 需要显示**所有**错误，而不是只显示第一个。

**解决方案**：
- 修改 `src/frontend/typecheck/inference/` 模块，使用 `ErrorCollector` 累积所有错误
- 不在遇到错误时立即返回，而是继续检查
- 检查完成后统一返回 `Vec<Diagnostic>`

### 问题 2：增量解析与缓存

**现状**：每次 LSP 请求都重新解析整个文件，没有缓存机制。

**LSP 需求**：像 `cargo check` 一样增量更新，但更轻量（每次编辑都应快速响应）。

**解决方案**：
- 文档版本管理 + 内容哈希
- 增量更新：只重新解析受影响的函数/模块
- 大面积修改时自动降级为全量解析

### 与现有模块的集成

| 现有模块 | LSP 集成方式 |
|----------|-------------|
| `util/span.rs` | ✅ 已有 `Position`/`Span`，直接映射到 LSP `Position` |
| `util/diagnostic/collect.rs` | ⚠️ 需修改为「收集模式」，持续累积错误 |
| `frontend/core/lexer/symbols.rs` | ⚠️ 需扩展，添加 `uri` + `span` 位置信息 |
| `frontend/typecheck/mod.rs` | ⚠️ 需修改 `TypeResult`，返回所有错误 |
| `frontend/core/parser/ast.rs` | ✅ 每个节点已有 `Span`，无需改动 |

---

## 摘要

为 YaoXiang 添加 Language Server Protocol（LSP）支持，实现完整的语言服务器，使主流 IDE（VS Code、Neovim、Emacs 等）能够提供代码补全、跳转定义、诊断、引用搜索等开发工具功能。

## 动机

### 为什么需要这个特性？

当前 YaoXiang 语言缺少官方的 IDE 集成支持，开发者只能使用基础的文本编辑器编写代码，缺乏：

1. **代码补全** - 无法根据上下文智能补全标识符、关键字、类型
2. **跳转到定义** - 无法快速跳转到函数、类型、变量的定义位置
3. **实时诊断** - 无法在编辑时即时显示语法错误、类型错误
4. **引用搜索** - 无法查找符号的所有引用位置
5. **悬停提示** - 无法在鼠标悬停时显示类型信息、文档注释

LSP 是现代编程语言的标配，主流语言（Rust、Python、TypeScript、Go 等）都提供了成熟的 LSP 实现。实现 LSP 支持将显著提升 YaoXiang 的开发体验。

### 当前的问题

1. **开发效率低** - 缺少代码补全和智能提示
2. **调试困难** - 无法快速定位符号定义
3. **学习曲线陡峭** - 缺少 IDE 的辅助功能
4. **生态不完善** - 无法吸引习惯现代 IDE 的开发者

## 提案

### 核心设计

实现一个独立的 LSP 服务器进程，通过 JSON-RPC 与 IDE 通信：

```
┌─────────────┐     JSON-RPC      ┌─────────────┐
│   IDE       │◄────────────────► │  YaoXiang   │
│ (VS Code)   │                   │   LSP       │
└─────────────┘                   │   Server    │
                                  └──────┬──────┘
                                         │
          ┌──────────────────────────────┴──────────────────────┐
          │                                                        │
          ▼                                                        ▼
┌─────────────────────┐                              ┌─────────────────────┐
│   World (编译世界)   │                              │  Document Cache    │
│  ├─ Symbol Index    │◄─── 增量更新 ───┐             │  ├─ 版本管理         │
│  ├─ Type Env       │                  │             │  ├─ 内容缓存         │
│  └─ Diagnostics    │                  │             │  ├─ AST 缓存         │
└─────────┬───────────┘                  │             │  └─ 增量变更区域     │
          │                              │             └─────────────────────┘
          │         ┌───────────────────┘
          │         │
          ▼         ▼
┌─────────────────────────────────────────────────┐
│           Compiler Frontend                     │
│  ├─ Lexer (util/span.rs Position)             │
│  ├─ Parser (ast.rs 已有 Span)                  │
│  ├─ Type Check (改为收集模式)                   │
│  └─ ErrorCollector (util/diagnostic/)          │
└─────────────────────────────────────────────────┘
```

### LSP 服务器架构

```
src/lsp/
├── main.rs              # LSP 服务器入口
├── server.rs           # 服务器核心逻辑
├── session.rs          # 会话管理
├── capabilities.rs     # 服务端能力声明
├── handlers/
│   ├── mod.rs
│   ├── initialize.rs   # 初始化处理
│   ├── text_document.rs # 文档操作处理
│   ├── completion.rs   # 补全处理
│   ├── definition.rs   # 跳转定义处理
│   ├── references.rs   # 引用搜索处理
│   ├── hover.rs        # 悬停提示处理
│   └── diagnostics.rs  # 诊断处理
├── world.rs            # 编译世界（符号表、AST 缓存）
├── scroller.rs         # 符号索引构建
├── protocol.rs         # LSP 协议类型定义
└── cache/              # 增量缓存模块（新增）
    ├── mod.rs
    ├── document.rs     # 文档缓存（版本、AST、符号表）
    └── incremental.rs  # 增量解析策略
```

### 编译世界（World）设计

管理全局编译状态：
- 文档缓存（版本、AST、符号表）
- 全局符号索引
- 错误收集器
- 类型环境缓存

核心方法：
- `on_document_change`：处理增量变更
- `incremental_reparse`：增量重解析
- `collect_diagnostics`：收集所有错误（不阻断）

### 核心 LSP 方法支持

| 类别 | 方法 | 说明 |
|------|------|------|
| **生命周期** | `initialize` / `initialized` / `shutdown` / `exit` | 服务端生命周期 |
| **文档同步** | `didOpen` / `didChange` / `didClose` | 文档管理 |
| **诊断** | `publishDiagnostics` | 发布诊断 |
| **补全** | `completion` | 代码补全 |
| **跳转** | `definition` | 跳转到定义 |
| **引用** | `references` | 查找引用 |
| **悬停** | `hover` | 悬停提示 |
| **符号** | `workspace/symbol` | 工作区符号搜索 |

### 文本文档同步机制

使用增量同步策略：
- 保留文档版本号
- 应用增量变更（range + text）
- 大变更时降级为全量替换

### 符号索引构建

利用现有的符号表系统，构建反向索引：
- 需要扩展 `SymbolEntry`，添加 `location` 字段
- 索引：名称 → 位置列表、文件 → 符号列表

### 代码补全实现

补全来源：关键字、变量、函数、类型、结构体字段、模块

### 跳转定义实现

基于 AST 的符号解析：查找标识符/函数调用对应的定义位置

## 详细设计

### 类型系统影响

1. **符号信息扩展** - 在符号表中添加位置信息（文件、行号、列号）
2. **类型信息暴露** - 为 LSP 提供类型查询接口
3. **文档注释集成** - 支持从注释生成文档字符串

### 运行时行为

- LSP 服务器作为独立进程运行
- 使用 stdin/stdout 进行 JSON-RPC 通信
- 支持多会话并发处理

### 编译器改动

| 组件 | 改动 |
|------|------|
| `frontend/events` | 扩展事件系统，支持 LSP 通知 |
| `frontend/core/lexer/symbols` | 增强符号表，添加位置信息 |
| 新增 `src/lsp/` | LSP 服务器实现 |

### 向后兼容性

- ✅ 完全向后兼容
- LSP 服务器是独立组件，不影响现有编译流程
- 现有 CLI 工具不受影响

### 与现有系统集成

1. **事件系统** - 利用 `frontend/events/` 的事件订阅机制
2. **诊断系统** - 复用 `util/diagnostic/` 的诊断输出
   - 复用 `ErrorCollector<E>` 收集所有错误
   - 将 `Diagnostic` 转换为 LSP 的 `Diagnostic` 格式
3. **符号表** - 扩展 `symbols.rs` 的符号定位能力
   - 扩展 `SymbolEntry`，添加 `location: Location` 字段
   - 构建 `SymbolIndex` 反向索引（名称 -> 位置列表）
4. **编译器前端** - 直接调用 Lexer、Parser、类型检查
   - **关键改动**：类型检查器需改为「收集模式」，不阻断执行

#### 诊断格式转换

```rust
/// 将 YaoXiang Diagnostic 转换为 LSP Diagnostic
fn to_lsp_diagnostic(diag: &Diagnostic) -> lsp_types::Diagnostic {
    let severity = match diag.severity() {
        Severity::Error => lsp_types::DiagnosticSeverity::ERROR,
        Severity::Warning => lsp_types::DiagnosticSeverity::WARNING,
        Severity::Info => lsp_types::DiagnosticSeverity::INFORMATION,
    };

    lsp_types::Diagnostic {
        range: to_lsp_range(diag.span()),
        severity: Some(severity),
        message: diag.message().to_string(),
        code: diag.code().map(|c| lsp_types::NumberOrString::String(c.as_string())),
        ..Default::default()
    }
}

/// 将 YaoXiang Span 转换为 LSP Range
fn to_lsp_range(span: &Span) -> lsp_types::Range {
    lsp_types::Range {
        start: lsp_types::Position {
            line: span.start.line.saturating_sub(1), // LSP 使用 0-indexed
            character: span.start.column.saturating_sub(1),
        },
        end: lsp_types::Position {
            line: span.end.line.saturating_sub(1),
            character: span.end.column.saturating_sub(1),
        },
    }
}
```

## YaoXiang 特有高级特性

利用 YaoXiang 强大的编译期求值和所有权系统，提供其他语言无法实现的独特开发体验：

### 1. 幽灵提示（Inlay Hints）

- **常量值提示**：显示编译期已计算好的常量（如 `const MAX = 100 + 200` 旁显示 `300`）
- **可变性提示**：显示变量是否可变（如 `let mut x` 旁显示 `mut`）
- **所有权消费提示**：显示函数参数是否被消费（如 `consumed` / `borrowed`）
- **类型推断提示**：显示推断出的具体类型（如 `let x = vec![]` 旁显示 `Vec<i32>`）

### 2. 所有权语义可视化

- 显示变量的 move 路径（从定义位置到所有使用位置）
- 借用生命周期可视化

### 3. 编译期求值预览

- 悬停显示常量表达式的编译期计算结果

### 实现优先级

| 特性 | 优先级 |
|------|--------|
| 常量值幽灵提示 | P0 |
| 可变性提示 | P0 |
| 所有权消费提示 | P1 |
| 所有权可视化 | P2 |

---

## 通信与远程支持

### 通信模式

支持三种模式：

| 模式 | 用途 |
|------|------|
| stdio | 本地开发（默认）|
| TCP Socket | 远程开发/调试 |
| Unix Domain Socket | 高性能本地通信 |

### 远程调试

基于 DAP（Debug Adapter Protocol）实现：
- 支持行断点、函数断点、条件断点
- YaoXiang 特有断点：变量被 move 时触发

### 启动参数

```bash
# 本地模式
yaoxiang-lsp

# TCP 服务器
yaoxiang-lsp --tcp --port 8765

# 同时启用调试
yaoxiang-lsp --tcp --port 8765 --enable-debug
```

---

## 并发模型

**设计决策：单线程 + 异步事件循环**

理由：
- 编译器非线程安全，改造成本高
- LSP 请求天然串行，无需并发
- 单线程更简单、易调试
- async I/O 单线程性能足够

后台任务使用 `spawn_blocking` 利用多核。

---

## LSP 内置测试工具

提供 JSON 测试用例格式：

```bash
# 运行测试
yaoxiang-lsp --test
```

---

## 权衡

### 优点

1. **开发体验提升** - 接近主流语言的 IDE 支持
2. **生态系统完善** - 吸引更多开发者使用 YaoXiang
3. **代码质量提升** - 实时诊断减少运行时错误
4. **社区贡献** - 开发者可参与 LSP 工具链开发

### 缺点

1. **实现复杂度高** - 需要处理大量 LSP 边缘情况
2. **维护成本** - 需要跟随 LSP 协议版本更新
3. **性能考虑** - 大型项目的索引和查询性能
4. **测试难度** - 需要模拟 IDE 行为进行测试

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| 仅提供语法高亮 | 无法满足现代开发需求 |
| 使用 Tree-sitter | 需要额外学习成本，且功能有限 |

## 实现策略

### 阶段划分

1. **阶段 0 (前置)**: 编译器适配 ⚠️ **关键**
   - 修改类型检查器为「收集模式」，使用 ErrorCollector 累积所有错误
   - 扩展符号表 `SymbolEntry`，添加 `location` 字段
   - 实现 DocumentCache 缓存系统
   - **此阶段是 LSP 实现的前提，必须先完成**

2. **阶段 1 (v0.7)**: 基础框架
   - LSP 服务器骨架
   - 生命周期方法（initialize/shutdown/exit）
   - 基础日志和错误处理

2. **阶段 2 (v0.7)**: 诊断支持
   - 文本文档同步
   - 编译诊断集成
   - `textDocument/publishDiagnostics`

3. **阶段 3 (v0.8)**: 补全支持
   - 符号索引构建
   - 关键字补全
   - 标识符补全

4. **阶段 4 (v0.8)**: 跳转支持
   - 跳转到定义
   - 查找引用
   - 悬停提示

5. **阶段 5 (v0.9)**: 高级功能
   - 工作区符号搜索
   - 代码格式化
   - 重构支持（可选）

### 依赖关系

- 无外部 LSP 库依赖（使用 `lsp-types` crate）
- 依赖现有编译器前端模块
- 依赖 `serde_json` 进行 JSON-RPC 序列化

### 风险

1. **性能问题** - 大文件解析可能导致卡顿
   - 解决：增量解析、后台线程处理
2. **内存占用** - 符号索引占用内存
   - 解决：延迟加载、LRU 缓存
3. **协议兼容性** - LSP 版本差异
   - 解决：声明支持的协议版本

## 开放问题

- [x] 错误收集机制（见「实现前置条件」章节）
- [x] 增量缓存系统（见「实现前置条件」章节）
- [x] LSP 协议版本：使用 3.18（支持 Inlay Hints、Inline Values 等新特性）
- [x] 远程通信支持（通过 TCP，兼顾 LSP + 调试）
- [x] 远程调试支持（基于 DAP 协议）
- [x] 并发模型：单线程 + async 事件循环
- [x] LSP 内置测试工具：使用 JSON 测试用例

---

## 附录（可选）

### 附录A：设计讨论记录

> 用于记录设计决策过程中的详细讨论。

### 附录B：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| LSP 服务器架构 | 独立进程，通过 stdio 通信 | 2026-02-15 | 晨煦 |
| 协议版本 | 支持 LSP 3.18（需要 Inlay Hints 等新特性） | 2026-02-22 | 晨煦 |
| 错误收集模式 | 使用 ErrorCollector 收集所有错误，不阻断 | 2026-02-22 | 晨煦 |
| 缓存策略 | Document 版本 + 内容哈希 + 增量重解析 | 2026-02-22 | 晨煦 |
| 通信模式 | 支持 stdio + TCP + UnixSocket | 2026-02-22 | 晨煦 |
| 远程调试 | 基于 DAP 协议，与 LSP 共享传输层 | 2026-02-22 | 晨煦 |
| 并发模型 | 单线程 + async 事件循环 | 2026-02-22 | 晨煦 |
| 测试工具 | JSON 测试用例 + 内置测试运行器 | 2026-02-22 | 晨煦 |

### 附录C：术语表

| 术语 | 定义 |
|------|------|
| LSP | Language Server Protocol，语言服务器协议 |
| JSON-RCP | JSON-Remote Procedure Call，JSON 远程过程调用 |
| DAP | Debug Adapter Protocol，调试适配协议 |
| 符号索引 | 编译时构建的符号位置映射表 |
| 编译世界 | 包含所有编译信息的上下文 |
| 幽灵提示 | Inlay Hints，行内显示的提示信息 |
| 所有权追踪 | Ownership Trace，变量所有权流动的可视化 |

---

## 参考文献

- [Language Server Protocol 规范](https://microsoft.github.io/language-server-protocol/)
- [LSP 规范 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Debug Adapter Protocol 规范](https://microsoft.github.io/debug-adapter-protocol/)
- [Rust Analyzer](https://rust-analyzer.github.io/) - 参考实现
- [lsp-types crate](https://crates.io/crates/lsp-types) - LSP 类型定义
- [JSON-RPC 2.0 规范](https://www.jsonrpc.org/specification)

---

## 生命周期与归宿

RFC 有以下状态流转：

```
┌─────────────┐
│   草案      │  ← 作者创建
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 社区讨论
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已接受     │    │   已拒绝     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │  rejected/  │
│ (正式设计)  │     │ (拒绝)     │
└─────────────┘    └─────────────┘
```

### 状态说明

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/draft/` | 作者草稿，等待提交审核 |
| **审核中** | `docs/design/rfc/review/` | 开放社区讨论和反馈 |
| **已接受** | `docs/design/accepted/` | 成为正式设计文档，进入实现阶段 |
| **已拒绝** | `docs/design/rfc/` | 保留在 RFC 目录，更新状态 |

### 接受后的操作

1. 将 RFC 移至 `docs/design/accepted/` 目录
2. 更新文件名为描述性名称（如 `lsp-support.md`）
3. 更新状态为 "正式"
4. 更新状态为 "已接受"，添加接受日期

### 拒绝后的操作

1. 保留在 `docs/design/rfc/draft/` 目录
2. 在文件顶部添加拒绝原因和日期
3. 更新状态为 "已拒绝"

### 讨论确定后的操作

当某个开放问题达成共识后：

1. **更新附录A**: 在讨论主题下填写「决议」
2. **更新正文**: 将决定同步到文档正文
3. **记录决策**: 添加到「附录B：设计决策记录」
4. **标记问题**: 在「开放问题」列表中勾选 `[x]`

---

> **注**: RFC 编号仅在讨论阶段使用。接受后移除编号，使用描述性文件名。
