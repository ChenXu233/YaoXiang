---
title: "语言服务器状态"
---

# 语言服务器（LSP）

> **模块状态**：稳定（5 项待改进）
> **位置**：`src/lsp/`
> **最后更新**：2026-06-01

---

## 模块概述

LSP 模块实现了 Language Server Protocol，为编辑器/IDE 提供代码智能功能。实现范围已覆盖 RFC-017 的阶段 0-5，并提前实现了语义 tokens、重命名、代码操作等高级功能。

**代码量**：7,542 行（21 个 Rust 文件）

---

## 功能清单

### 已实现的 LSP 功能（13 项）

| 功能 | 文件 | 状态 | 说明 |
|------|------|------|------|
| **生命周期管理** | `handlers/initialize.rs` | ✅ | initialize/shutdown/exit/initialized，含会话状态机 |
| **文档同步** | `handlers/text_document.rs` | ✅ | didOpen/didChange/didClose，全量同步模式 |
| **诊断发布** | `handlers/diagnostics.rs` | ✅ | tokenize + parse_with_recovery + check_module_collect_all 管线 |
| **代码补全** | `handlers/completion.rs` | ✅ | 17 个关键字 + 7 个保留字 + 2 个注解 + 标识符补全 |
| **跳转定义** | `handlers/definition.rs` | ✅ | SemanticDB 精确匹配 + 全局符号索引回退，支持跨文件 |
| **查找引用** | `handlers/references.rs` | ✅ | 变量/函数引用查找，支持跨文件 |
| **悬停提示** | `handlers/hover.rs` | ✅ | 变量类型、函数签名（参数数量/泛型）、类型定义信息 |
| **重命名** | `handlers/rename.rs` | ✅ | 符号重命名，生成 WorkspaceEdit |
| **代码操作** | `handlers/code_action.rs` | ✅ | 基于诊断的快速修复 + 重构建议 |
| **语义 Tokens** | `handlers/semantic_tokens.rs` | ✅ | full + full/delta 两种模式，含版本缓存 |
| **文档格式化** | `handlers/formatting.rs` | ✅ | 全文格式化 + 范围格式化 |
| **幽灵提示** | `handlers/inlay_hint.rs` | ✅ | 类型推断提示、常量值提示、可变性提示 |
| **工作区符号搜索** | `handlers/workspace_symbol.rs` | ✅ | 模糊匹配，按符号类型过滤 |

### 核心支撑模块

| 模块 | 文件 | 功能 |
|------|------|------|
| **编译世界** | `world.rs` (1,019 行) | 符号索引、语义数据库、标准库符号加载、AST 符号提取 |
| **光标定位** | `locate.rs` | 标识符查找、Span↔Range 转换、所有出现位置查找 |
| **会话管理** | `session.rs` | 生命周期状态机（Uninitialized→Initializing→Running→ShuttingDown） |
| **协议工具** | `protocol.rs` | JSON-RPC 消息构建、错误码定义 |
| **能力声明** | `capabilities.rs` | 服务端能力声明，含语义 tokens legend |

---

## 测试覆盖

**145 个单元测试**，分布如下：

| 文件 | 测试数 |
|------|--------|
| workspace_symbol.rs | 18 |
| completion.rs | 16 |
| server.rs | 15 |
| semantic_tokens.rs | 15 |
| world.rs | 10 |
| locate.rs | 10 |
| diagnostics.rs | 10 |
| hover.rs | 8 |
| definition.rs | 8 |
| capabilities.rs | 6 |
| protocol.rs | 5 |
| references.rs | 5 |
| 其他 | 19 |

---

## RFC 对比（RFC-017）

| 阶段 | RFC 设计内容 | 实现状态 | 差异说明 |
|------|-------------|----------|----------|
| **阶段 0（前置）** | 错误收集模式、Parser 错误恢复、DocumentCache、扩展符号表 | ✅ 已完成 | 使用 `check_module_collect_all` 实现收集模式 |
| **阶段 1 (v0.7)** | LSP 服务器骨架、生命周期方法 | ✅ 已完成 | 完整实现 |
| **阶段 2 (v0.7)** | 文本文档同步、诊断支持 | ✅ 已完成 | 完整实现 |
| **阶段 3 (v0.8)** | 符号索引构建、代码补全 | ✅ 已完成 | 完整实现，支持关键字/保留字/注解/标识符补全 |
| **阶段 4 (v0.8)** | 跳转定义、查找引用、悬停提示 | ✅ 已完成 | 完整实现，SemanticDB 精确匹配 + 全局索引回退 |
| **阶段 5 (v0.9)** | 工作区符号搜索、代码格式化 | ✅ 已完成 | 完整实现，含模糊匹配 |

### 超出 RFC 设计的额外实现

| 功能 | RFC 计划版本 | 说明 |
|------|-------------|------|
| **语义 Tokens** | v0.10 | 已提前实现，支持 full + delta 模式 |
| **重命名** | v0.9（RFC 提及） | 已实现 |
| **代码操作** | v0.9（RFC 提及） | 已实现快速修复 |
| **幽灵提示** | RFC 特有高级特性 | 已实现类型推断/常量值/可变性提示 |

### 未实现的 RFC 设计

| 功能 | RFC 状态 | 说明 |
|------|----------|------|
| **增量同步** | RFC 设计 | 当前使用全量同步（TextDocumentSyncKind::FULL） |
| **TCP/Unix Socket 通信** | RFC 设计 | 当前仅支持 stdio |
| **远程调试（DAP）** | RFC 设计 | 未实现 |
| **所有权可视化** | RFC 高级特性 | 未实现 |
| **编译期求值预览** | RFC 高级特性 | 未实现 |

---

## 代码质量评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 未完成事项 | 5 | 增量同步、TCP/Unix Socket、DAP、可视化、编译期求值预览 |
| 测试覆盖 | 优秀 | 145 个单元测试 |
| 文档质量 | 优秀 | 模块/函数级文档完整，包含 ASCII 架构图 |
| 代码架构 | 优秀 | 分层清晰：handlers/world/locate/session/protocol |
| RFC 合规 | 超出预期 | 实现范围超出 RFC 设计 |

---

## 待改进项

1. **实现增量同步**（TextDocumentSyncKind::INCREMENTAL）
2. **实现 TCP/Unix Socket 远程通信**
3. **实现 DAP 调试适配器**
4. **实现所有权语义可视化**
5. **实现编译期求值预览**
