---
title: 'RFC-017: 语言服务器协议（LSP）支持设计'
---

# RFC-017: 语言服务器协议（LSP）支持设计

> **状态**: 草案
>
> **作者**: 晨旭
>
> **创建日期**: 2026-02-15
>
> **最后更新**: 2026-02-15

> **参考**: 查看 [完整示例](EXAMPLE_full_feature_proposal.md) 了解如何编写 RFC。

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
                                         ▼
                                  ┌─────────────┐
                                  │ Compiler    │
                                  │ Frontend    │
                                  └─────────────┘
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
└── protocol.rs         # LSP 协议类型定义
```

### 核心 LSP 方法支持

| 类别 | 方法 | 说明 |
|------|------|------|
| **生命周期** | `initialize` | 客户端初始化 |
| | `initialized` | 服务端初始化完成通知 |
| | `shutdown` | 请求关闭 |
| | `exit` | 退出通知 |
| **文档同步** | `textDocument/didOpen` | 文档打开通知 |
| | `textDocument/didChange` | 文档变更通知 |
| | `textDocument/didClose` | 文档关闭通知 |
| **诊断** | `textDocument/publishDiagnostics` | 发布诊断信息 |
| **补全** | `textDocument/completion` | 代码补全 |
| **跳转** | `textDocument/definition` | 跳转到定义 |
| **引用** | `textDocument/references` | 查找引用 |
| **悬停** | `textDocument/hover` | 悬停提示 |
| **符号** | `workspace/symbol` | 工作区符号搜索 |

### 文本文档同步机制

使用增量同步策略：

```rust
/// 文本文档内容缓存
struct Document {
    uri: DocumentUri,
    version: i32,
    content: String,
    changes: Vec<TextDocumentContentChangeEvent>,
}

impl Document {
    /// 应用增量变更
    fn apply_changes(&mut self, changes: Vec<TextDocumentContentChangeEvent>) {
        for change in changes {
            if let Some(range) = change.range {
                // 替换指定范围
                self.content.replace_range(range, &change.text);
            } else {
                // 完全替换
                self.content = change.text;
            }
        }
        self.version += 1;
    }
}
```

### 符号索引构建

利用现有的符号表系统，构建反向索引：

```rust
/// 符号位置信息
struct SymbolLocation {
    uri: DocumentUri,
    span: Span,
    name: String,
    kind: SymbolKind,
}

/// 符号索引
struct SymbolIndex {
    /// 名称 -> 位置列表
    by_name: HashMap<String, Vec<SymbolLocation>>,
    /// 文件 -> 符号列表
    by_file: HashMap<DocumentUri, Vec<SymbolLocation>>,
}
```

### 代码补全实现

```rust
/// 补全项
struct CompletionItem {
    label: String,
    kind: CompletionItemKind,
    detail: Option<String>,
    documentation: Option<String>,
    insert_text: Option<String>,
}

/// 补全来源
enum CompletionSource {
    Keywords,      // 关键字
    Variables,     // 变量
    Functions,     // 函数
    Types,         // 类型
    Fields,        // 结构体字段
    Modules,       // 模块
}
```

### 跳转定义实现

基于 AST 的符号解析：

```rust
/// 查找符号定义位置
fn find_definition(ast: &Ast, position: Position) -> Option<Location> {
    let node = ast.find_node_at(position)?;
    match node.kind() {
        NodeKind::Identifier(name) => {
            // 查找符号表
            world.lookup_symbol(&name)
        }
        NodeKind::FunctionCall(name) => {
            world.lookup_symbol(&name)
        }
        _ => None
    }
}
```

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
3. **符号表** - 扩展 `symbols.rs` 的符号定位能力
4. **编译器前端** - 直接调用 Lexer、Parser、类型检查

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

1. **阶段 1 (v0.7)**: 基础框架
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

- [ ] LSP 协议版本选择（3.16 vs 3.18）
- [ ] 是否支持远程 LSP（通过 TCP）
- [ ] 并发模型设计（单线程 vs 多线程）
- [ ] 是否提供 LSP 内置测试工具

---

## 附录（可选）

### 附录A：设计讨论记录

> 用于记录设计决策过程中的详细讨论。

### 附录B：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| LSP 服务器架构 | 独立进程，通过 stdio 通信 | 2026-02-15 | 晨煦 |
| 协议版本 | 支持 LSP 3.16+ | 2026-02-15 | 晨煦 |

### 附录C：术语表

| 术语 | 定义 |
|------|------|
| LSP | Language Server Protocol，语言服务器协议 |
| JSON-RCP | JSON-Remote Procedure Call，JSON 远程过程调用 |
| 符号索引 | 编译时构建的符号位置映射表 |
| 编译世界 | 包含所有编译信息的上下文 |

---

## 参考文献

- [Language Server Protocol 规范](https://microsoft.github.io/language-server-protocol/)
- [LSP 规范 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
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
