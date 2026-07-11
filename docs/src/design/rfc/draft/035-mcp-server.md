---
title: "RFC-035: MCP Server 支持（AI Agent 集成）"
status: "草案"
author: "晨煦"
created: "2026-07-11"
updated: "2026-07-11"
issue: "#154"
---

# RFC-035: MCP Server 支持（AI Agent 集成）

## 摘要

为 YaoXiang 添加一个 MCP（Model Context Protocol）服务器，让 AI agent（Claude Code、Continue、Cody、Zed 等）能直接查询 YaoXiang 源码的 **AST、解析错误、类型、符号、引用、格式化结果**。复用 RFC-017 已落地的 `World` 后端，新增 `yaoxiang mcp` 子命令，单二进制双模式，多进程独立 World。

## 动机

### 为什么需要这个特性？

RFC-017 让 YaoXiang **能**被编辑器理解（hover / goto-def / completion）。但 LSP 是**位置驱动**的协议：
- 每个请求都强依赖 `textDocument` URI + `Position`
- 编辑器必须先打开文件、保存、与 LSP server 维持长连接
- AI agent 的工作流是**代码片段**：在对话里"贴一段代码"问问题，**不**先存盘

AI agent 实际可用的 LSP 客户端（vscode-langservers-extracted、`mcp-lsp-bridge` 类项目）都**只翻译 L1**：goto-def、hover。AI 想做的：
- 「这段代码**解析得对不对**」——要 parse + 完整 diagnostic 流
- 「这个符号**在文件里怎么用**」——要 lookup_symbol 按名查
- 「这段代码**格式化后什么样**」——要 format_source
- 「**全部**类型错在哪儿」——要 typecheck 跑完整工作区

这些 L1 LSP 翻译能力**做不到**，因为 LSP 设计上就不支持。

### 当前的问题

1. AI agent 调 LSP 体验差：需要 mock 文档、JSON 巨大、强 URI 依赖
2. YaoXiang 项目缺少「AI-First」接口层：人类开 IDE 用 LSP，AI agent 用不了 LSP
3. Claude Code / Continue 等主流 AI agent 已默认支持 MCP，对 YaoXiang 是空白生态

### MCP 是什么？

MCP（Model Context Protocol）是 2024-2025 年由 Anthropic 主导发布并开源的 AI agent 工具调用协议，已成事实标准（OpenAI、Google、Microsoft、Zed、Continue、Cody 等接入）。特点：
- 基于 JSON-RPC 2.0（与 LSP 同源）
- 三大原语：**Tools**（动作）、Resources（数据）、Prompts（模板）
- 传输：`stdio`（子进程）/ streamable `HTTP` / SSE
- 工具输入输出有 **JSON Schema** 强类型（对 LLM 友好）
- 2025-06+ 已发布 streamable HTTP 规范，本 RFC 同时兼容旧 SSE

**本 RFC 只用 Tools 原语**——与 LSP 的"提供服务"对齐，不引入 Resources 的文件模型复杂度。

## 提案

### 核心设计

单二进制双模式：

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang（v0.7.7+）                  │
│  ┌─────────────────┐      ┌──────────────────────────┐  │
│  │ yaoxiang lsp    │      │   yaoxiang mcp           │  │
│  │ (stdio JSON-RPC)│      │   (stdio default         │  │
│  │ RFC-017 已实现  │      │    + HTTP 可选)          │  │
│  └────────┬────────┘      └──────────┬───────────────┘  │
│           │                         │                   │
│           ▼                         ▼                   │
│  ┌──────────────────────────────────────────────────┐  │
│  │  共享 lib crate（`yaoxiang`）                      │  │
│  │  src/lsp/{server,session,world}.rs                │  │
│  │  src/frontend/{lexer,parser,core}/...             │  │
│  │  src/middle/...                                   │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │            src/mcp/  ← 新增                       │  │
│  │  ├── mod.rs          （模块入口 + 启动函数）       │  │
│  │  ├── transport/      （stdio + HTTP/SSE）         │  │
│  │  ├── server.rs       （JSON-RPC 消息循环）         │  │
│  │  ├── tools/          （6 个 tool handler）        │  │
│  │  ├── schema.rs       （输入输出 JSON Schema）     │  │
│  │  └── project.rs      （项目根识别 + 路径解析）    │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**关键决策**：
- **同一二进制**：`yaoxiang` 通过子命令切换；LSP 进程和 MCP 进程**不共存**于同一运行时
- **多进程独立 World**：每个 `yaoxiang mcp` 进程持有一个 `World`；与 LSP 进程、其他 MCP 进程互不影响（无锁竞争、独立崩溃隔离）
- **stdio 默认**：避免端口冲突、零网络配置；HTTP 作为可选后路
- **复用而非重复**：直接调 `yaoxiang::frontend` / `yaoxiang::middle` / `yaoxiang::lsp::handlers` 的 lib API，**不**走 LSP-client 中转

### 工具集（8 个工具，分 3 阶段交付）

按"消除特殊情况 + 分阶段"原则设计：纯源工具 stateless 先行，工作区工具共享 LSP World，AST 改写工具独立新加。

| Tool 名称 | 输入 | 输出 | 复用 | 阶段 |
|---|---|---|---|---|
| `parse_source` | `source: String`, `tab_size?: u32` | `{ast: Node, diagnostics: Diagnostic[]}` | 直接调 `frontend::parse` | v0.8.x |
| `format_source` | `source: String`, `tab_size?: u32` | `{formatted: String, diff: Hunk[]}` | 直接调 `formatter::format` | v0.8.x |
| `lookup_symbol` | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]` | `{symbols: Symbol[]}` | 复用 `lsp::handlers::workspace_symbol`（按 `query` 模糊匹配） | v0.8.x |
| `find_references` | `query: String`, `workspace_root?: String` | `{locations: Location[]}` | 复用 `lsp::handlers::references`（按 `query` 而非位置） | v0.8.x |
| `typecheck` | `file_paths: String[]`, `project_root: String` | `{diagnostics: Diagnostic[], summary: Counts}` | 复用 `lsp::world::typecheck_full` | v0.8.x |
| `explain_diagnostic` | `code: String`（如 `E0001`），`lang?: String` | `{code, category, title, description, example, help}` | **直接调** `util::diagnostic::command::render_explain_output` | **v0.9.x** |
| `list_imports` | `file_path: String`, `project_root?: String` | `{imports: [{module, items, is_public}]}` | 复用 `middle::passes::module::ModuleGraph::validate_imports` | **v0.9.x** |
| `rename_symbol` | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | **新加** `src/middle/rename.rs`（AST 改写） | **v0.10.x** |

**8 个工具的边界**：
- `parse_source` / `format_source` —— **纯源 stateless**，不入 World
- `lookup_symbol` / `find_references` —— 接 `workspace_root`（不传则用启动时的 `--project-root`）
- `typecheck` —— **必填** `file_paths`，保证工作区完整
- `explain_diagnostic` —— **零文件依赖**，纯字符串查询错误码注册表
- `list_imports` —— `file_path` 物理文件，输出该文件的 import 解析结果
- `rename_symbol` —— **纯源 AST 改写**，不做 LSP-style 位置查询（与已有 `lsp::handlers::rename` 语义不同）
- ~~`hover` / `completion` / `signature_help`~~ —— **全部砍**：AI agent 不做"位置敏感"语义，靠 `lookup_symbol` 按名查代替

**World 加载时机**：server 启动时按 `--project-root` 扫 `yaoxiang.toml` 和 `src/**/*.yx`，复用 LSP-017 已落地的 `World::load_*` API 一次性灌入 `World.documents`。**不**新增任何 lib API。

### 工具契约

**输入**：用 JSON Schema 描述，每个字段有 `description` + `examples`（LLM 自动读懂）。

**输出**：结构化 JSON，统一带 `schemaVersion: "1.0"` 字段：

```jsonc
// 成功响应
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [
    { "type": "json", "json": { /* 工具特定数据 */ } }
  ]
}

// 诊断被结构化返回（不视作 tool 错误）
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [{ "type": "json", "json": {
    "ast": {...},
    "diagnostics": [
      { "code": "E0001", "severity": "error", "message": "...", "span": [12, 4, 12, 18] }
    ]
  }}]
}

// 工具级错误（如 parse_source 接收非法 UTF-8）
{
  "schemaVersion": "1.0",
  "isError": true,
  "content": [{ "type": "text", "text": "MCP-INVALID-INPUT: source 不是合法 UTF-8" }],
  "errorCode": "MCP-INVALID-INPUT"
}
```

**错误体系**：
- **诊断（diagnostic）**：解析/类型错误，沿用 RFC-013（`E0001` 等）—— **不算 tool 错误**
- **工具级错误**：用 `MCP-` 前缀（`MCP-INVALID-INPUT`、`MCP-PROJECT-NOT-FOUND`、`MCP-INTERNAL`）—— 视作 `isError: true`
- **panic/crash**：JSON-RPC `-32603 Internal error`，server 不退出

**路径解析规则**（适用于 `lookup_symbol` / `find_references` 的 `workspace_root`、`typecheck` 的 `file_paths`）：
1. 命令行 `--project-root <dir>` 最高优先级（覆盖默认）
2. 否则：cwd 向上找 `yaoxiang.toml` 直到文件系统根（沿用 RFC-015）
3. 否则：cwd 本身
4. `file_paths` 必须落在项目根内（防穿越）；越界 → `MCP-PATH-OUTSIDE-PROJECT`

### 传输层

**stdio（默认）**：

```bash
yaoxiang mcp
# 启动后从 stdin 读 JSON-RPC，写 stdout，stderr 用于日志
```

AI agent 配置（Claude Code `.mcp.json` / Continue `config.json`）：
```jsonc
{
  "mcpServers": {
    "yaoxiang": {
      "command": "yaoxiang",
      "args": ["mcp", "--project-root", "${workspaceFolder}"]
    }
  }
}
```

**streamable HTTP（可选）**：

```bash
yaoxiang mcp --http --addr 127.0.0.1:7325  # 单 HTTP 端口，新 MCP 规范
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # 兼容旧 SSE（v0.10）
```

**安全约束**：
- **仅监听 loopback**（127.0.0.1 / ::1）；公网绑定显式拒绝并报错退出
- HTTP **无鉴权**（loopback 默认信任）；未来加 `--require-token <hex>` 字段
- stdio 子进程模式天然隔离（parent 进程控制权限）

### 多进程与并发

每个 `yaoxiang mcp` 进程持有一个 `World`，互不共享：

```text
┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│ yaoxiang    │   │ yaoxiang    │   │ yaoxiang    │
│   lsp       │   │   mcp       │   │   mcp       │
│ (Editor 1)  │   │ (Claude 1)  │   │ (Claude 2)  │
└──────┬──────┘   └──────┬──────┘   └──────┬──────┘
       │ stdio/stdout    │ stdio          │ stdio
   ┌───┴────┐        ┌───┴────┐        ┌───┴────┐
   │ Editor │        │ Claude │        │ Claude │
   └────────┘        └────────┘        └────────┘
```

**端口冲突**：AI agent 配置"启动子进程"——天然零端口冲突。HTTP 模式需用户自管端口分配。
**World 隔离**：每个进程独立 LSP 同步状态——一个 MCP 进程崩溃**不影响**LSP/其他 MCP 进程。
**future Sessions**：v2 才考虑多工作区分发（同一进程内多个 `Session`），**本 RFC 不做**。

## 详细设计

### 数据结构

新增 `src/mcp/project.rs`：

```rust
pub struct ProjectRoot {
    /// 绝对路径
    pub root: PathBuf,
    /// 加载时识别项目根的策略来源
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // 向上找 yaoxiang.toml
    FallbackCwd,       // fallback 到 cwd
}

pub struct ResolvedPath {
    /// 相对项目根的相对路径（推荐给 AI 读）
    pub relative: String,
    /// 解析后的绝对路径（用于 World 操作）
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// 把"file_path"解析为安全路径——防穿越
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

`ProjectRoot` 单例 + `src/mcp/schema.rs` 工具 schema 自动生成：

```rust
pub struct ProjectRoot {
    /// 绝对路径（必含 `yaoxiang.toml` 或向下兼容回退）
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// CLI 启动时识别一次，结果缓存在 `McpServer` 上下文里——所有工具复用
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

工具 schema 用 `schemars` crate 从 input struct 自动生成，避免手写 JSON Schema 漂移：

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// 完整 YaoXiang 源码片段——**不**保存到磁盘，纯 transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**`parse_source` / `format_source` 工具 schema 中没有 `file_path` 字段**——这两个工具只接受字符串源，不参与项目语义。`lookup_symbol` / `find_references` / `typecheck` 接受 `workspace_root` 或 `file_paths`（必填与否见工具表）。


### 编译器改动

| 模块 | 改动 |
|---|---|
| `src/lsp/world.rs` | **零改动**——MCP 启动时调 LSP 已有的 `World::load_*` API 一次性加载工作区 |
| `src/lsp/handlers/workspace_symbol.rs` | **零改动**——`mcp/tools/lookup.rs` 包一层把 `query` 转 LSP 入参 |
| `src/lsp/handlers/references.rs` | **零改动**——同上 |
| `src/lsp/handlers/formatter.rs` | **零改动**——format_source 直接调 |
| `src/main.rs` | 加 `Mcp` 子命令分支 |
| `Cargo.toml` | 加 `mcp-server` feature（或主二进制始终带） |
| `src/util/diagnostic/` | **零改动**（RFC-017 已落地） |

**关键约束**：`src/mcp/` **不**允许反向依赖 `src/lsp/` 的私有符号——只能通过 `crate::lsp::` 的公开 API 调 handlers。

### 向后兼容性

- ✅ **完全向后兼容**：新子命令 `yaoxiang mcp`，不改变 `yaoxiang` / `yaoxiang lsp` 任何现有行为
- ✅ **LSP server 不动**：RFC-017 实现的所有能力、API、内部状态均不变
- ✅ **lib crate 公开 API 不动**：所有 `pub` 路径不变；MCP 仅消费现有 API——**零**新增 `pub` 方法

### 与现有系统集成

| 现有模块 | MCP 集成方式 |
|---|---|
| `src/frontend/lexer` | parse_source 直接调 lexer |
| `src/frontend/core/parser` | parse_source 直接调 parser；失败产出 `Missing*` 节点（RFC-017） |
| `src/frontend/core/typecheck/inference/*` | typecheck 复用 `collect_diagnostics` 模式（RFC-017 §问题1） |
| `src/middle/` | typecheck 跑全部 middle pass（依赖分析等） |
| `src/lsp/world.rs` | 启动时调 `World::load_*` API（已有）；World **不**接受任何"虚拟文档" |
| `src/lsp/handlers/workspace_symbol.rs` | `mcp/tools/lookup.rs` 包一层，把 `query: String` 转 LSP 入参（按名查） |
| `src/lsp/handlers/references.rs` | `mcp/tools/find_refs.rs` 包一层，把 `query: String` 转 LSP 入参 |
| `src/lsp/handlers/formatter.rs` | `mcp/tools/format.rs` 直接调（若未实现，新加 `formatter::format_with_diff`） |
| `src/util/i18n/` | 错误消息走多语种资源文件（zh-CN/en） |

### 错误处理

| 来源 | 处理 |
|---|---|
| 解析错 | `Diagnostic{code:"E0xxx", severity, message, span}`（**非 tool 错误**，在 content 里返回） |
| 类型错 | 同上 |
| `file_paths` 越界（`typecheck` 工具） | tool 级错误 `MCP-PATH-OUTSIDE-PROJECT` |
| `source` 非法 UTF-8 | tool 级错误 `MCP-INVALID-INPUT` |
| 工具 panic | JSON-RPC `-32603 Internal error`；server **不退出** |
| 客户端发非 JSON-RPC | 直接断流（stdio EOF），重启即新会话 |

诊断严重级别沿用 RFC-017（已落地的）`enum ErrorKind { Error, Warning, Note }`。

### 测试策略

| 层 | 测试 |
|---|---|
| **Unit** | `src/mcp/project.rs::resolve` 路径穿越、`src/mcp/schema.rs` schema 校验 |
| **Integration** | mock stdio：起一个 server，stdin 灌 JSON-RPC，stdout 读响应，比对 fixture |
| **E2E** | 跑 `yaoxiang mcp` 真进程，Claude Code 风格的工具调用链：parse → 修 → format → typecheck |
| **Fuzz** | MCP JSON-RPC 解析的 `cargo-fuzz`（libFuzzer harness） |

每个 tool 必须有至少 1 个 happy path + 1 个 diagnostic 场景 + 1 个 tool-error 场景的 integration 测试。

## 权衡

### 优点

- **复用成本极低**：`World` / `Session` / `handlers` / 诊断收集全已落地（RFC-017），本 RFC 是"加一层 MCP 壳"
- **AI-First 接口**：tool 契约比 LSP 直观 3-5 倍；LLM 直接读 schema
- **多进程隔离**：与 LSP 编辑器会话、与其他 MCP 进程解耦，**零锁竞争**
- **stdio 友好**：所有主流 AI agent 默认子进程模式，零配置接入
- **YAGNI 通过**：本 RFC 砍掉 Resources、Sessions、跨进程状态、远程 MCP——v2 再开

### 缺点

- **协议分裂**：未来 LSP / MCP / DAP 三套协议各自演进，一致性维护成本
- **HTTP 模式第二公民**：loopback 限制定位为本地工具，远程场景需 v2 重设计
- **重复 parse 开销**：AI 反复微调源码反复调 `parse_source` 会重新 lexer+parser。**缓解**：依赖 RFC-017 的 `DocumentCache` 仍可加速**磁盘**上同 source 的二次解析；纯 transient source 走一次解析不可避免
- **测试覆盖成本**：5 个 tool × 3 种场景 = 15 个 integration 测试起步

## 替代方案

| 方案 | 为什么不选 |
|---|---|
| **进程内嵌入双协议**（LSP+MCPlistener 共存） | stdin/stdout 只能一个消费者；HTTP 也得并存——复杂度 > 收益 |
| **MCP 作为 LSP-client 桥接** | 多一层 IPC；LSP 设计就不支持按名查符号——MCP 想要的能力 LSP 给不了 |
| **走 gRPC / 自定义协议** | 偏离事实标准；社区已有 MCP SDK（TypeScript、Python、Rust），自带生态 |
| **复用 LSP handler 全部能力**（L3 工具集） | 大量 position↔intent 适配工作；边际收益递减 |
| **首个版本只做 HTTP**（不 stdio） | Claude Code / Continue 等默认 stdio，门槛过高 |

## 实现策略

### 依赖关系

- **强依赖**：RFC-017 LSP 实现（已落地）
- **强依赖**：RFC-013 错误代码体系（已落地）
- **强依赖**：RFC-014 / RFC-015 项目根识别（部分已落地）
- **新增依赖**（Rust crate）：
  - `mcp-rust-sdk`（待评估，参考 [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk)）
  - `tokio`（**已有**，optional feature）
  - `axum`（HTTP 模式）或 `hyper` 直接——待评估
- **零语言规范变化**：纯工具链增量

### 阶段（与 #154 同步）

| 阶段 | 内容 | 时长估算 |
|---|---|---|
| **v0.8.x (MVP)** | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck`（**5 工具**）+ `yaoxiang mcp` 子命令 + 启动时 `World::load_*` | **3-4 周** |
| **v0.9.x (YaoXiang 智能)** | `+ explain_diagnostic`（**直接调** `render_explain_output`）+ `+ list_imports`（包 `ModuleGraph::validate_imports`） + 单元/集成测试 | **1-2 周** |
| **v0.10.x (AST + HTTP)** | `+ rename_symbol`（**新加** `src/middle/rename.rs`，AST 改写）+ streamable HTTP transport + 性能调优（parse_source P99 < 100ms） | **2-3 周** |


**为什么分 3 阶段**：MVP 先跑通 stdio + 5 工具验证接口设计合理；v0.9.x 加低风险零适配的"YaoXiang 特有"工具验证集成正确；v0.10.x 再开高风险的"AST 改写"新模块（独立 PR 评审更聚焦）。

### 风险

1. **`mcp-rust-sdk` 维护活跃度**：2025 年才发布，API 可能剧烈变化。**缓解**：评估若不稳，自己写轻量 JSON-RPC 2.0 + tool dispatcher（< 500 行）
2. **重复 parse 开销**：AI 反复微调源码反复调 `parse_source` 会重新 lexer+parser。**缓解**：依赖 RFC-017 的 `DocumentCache` 仍可加速**磁盘**上同 source 的二次解析；纯 transient source 走一次解析不可避免
3. **AI agent schema 兼容性**：不同 agent 的 MCP schema 严格度不同。**缓解**：用 `schemars` crate 从 Rust input 结构自动生成 schema，零手写漂移
4. **路径解析多平台**：Windows 路径大小写不敏感、UNC 路径、`\\` 边界。**缓解**：路径解析用 `camino::Utf8Path` 替代 `std::path`
5. **MCP 工具 schema 与 LSP 入参不 1:1**：LSP `workspace_symbol` 接 `(query)`；传 LSP 内部时需要包装成位置+URI 才能让现有 handler 复用。**缓解**：在 `mcp/tools/lookup.rs` 做适配层，封装细节在 MCP 侧
6. **`rename_symbol` AST 改写与 LSP `rename` 语义不同**：LSP `textDocument/rename` 是 URI + 位置 + new_name → WorkspaceEdit；MCP `rename_symbol` 是 source + old_name + new_name → 新 source。**不能直接复用**。**缓解**：单独实现 `src/middle/rename.rs`，scope-aware 改写引用，与 LSP handler 实现互不干扰

## 开放问题

- [ ] `mcp-rust-sdk` 选型 / 自实现？（@Chen Xu：先评 rust-sdk 6 月版本，再决定）
- [ ] HTTP 鉴权路径？（v0.10 RFC 再开）
- [ ] 是否需要 `MCP` 启动时输出 `tools/list` 给 AI 主动发现？（MCP 标准要求，**默认实现**）
- [ ] `typecheck` 是否支持 `mode: "fast|full"`（fast = 仅当前文件子集，full = 全工作区）？
- [ ] 性能预算 parse_source P99 < 100ms 是否现实？（需 benchmark RFC-017 已落地的 `DocumentCache` 在 source-string 模式下的实际开销）

## 参考文献

- [RFC-017: 语言服务器协议（LSP）支持设计](./accepted/017-lsp-support.md)
- [RFC-013: 错误代码规范设计](./accepted/013-error-code-specification.md)
- [RFC-014: 包管理系统设计](./accepted/014-package-manager.md)
- [RFC-015: YaoXiang 配置系统设计](./accepted/015-configuration-system.md)
- [MCP 规范](https://modelcontextprotocol.io/)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [LSP 规范 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Rust Analyzer](https://rust-analyzer.github.io/) —— M2 / MCP 集成参考
- [zed-industries/zed 的 MCP 实现](https://github.com/zed-industries/zed/tree/main/crates/mcp)
