# 语义信息中台与增量编译实现计划

> **任务**：实现语义信息中台，提供 LSP 语义高亮、增量编译、死代码警告能力
> **基于 RFC**：本计划为新增功能设计
> **关联 RFC**：[RFC-008: 运行时并发模型](../design/rfc/accepted/008-runtime-concurrency-model.md) - DAG 并发属于运行时，不是本计划范围
> **日期**：2026-02-23
> **状态**：阶段 1 + 阶段 2 已完成
> **目标版本**：v0.10 - v0.11

---

## 概述

本计划将语义信息中台实现分解为 3 个主要阶段。核心思路是**一次遍历，多处使用**：

1. **语义收集在 typecheck 阶段完成**（而非 LSP 层单独遍历 AST）
2. 收集的语义信息同时服务于 LSP 语义高亮、增量编译、死代码分析

> **重要澄清**：
> - **DAG 并发**属于运行时特性（RFC-008），不是本计划范围
> - **模块依赖图的并行编译**是构建系统特性，与运行时 DAG 是两个不同概念
> - 语义收集应该在 typecheck 阶段完成，LSP 直接复用，而不是再写一个独立的遍历器

---

## 阶段 1：SemanticDB 基础设施

> **重要性**：此阶段是所有后续功能的基础，必须先完成
> **目标版本**：v0.10
> **状态**：✅ 已完成


**实现目标**：
- 定义 `SemanticDB` 结构，统一管理语义信息
- 定义 `SemanticToken` 枚举，包含 LSP 标准 TokenType
- 定义 `SymbolReference` 结构，记录符号引用位置
- 定义 `ModuleSymbol` 结构，记录模块级符号定义

**数据结构设计**：

```rust
// 语义信息数据库（实现于 src/frontend/typecheck/semantic_db.rs）
pub struct SemanticDB {
    // 文件路径 -> 该文件中的语义信息
    by_file: HashMap<String, FileSemanticInfo>,
    // 符号名 -> 所有定义位置
    symbol_defs: HashMap<String, Vec<SymbolLocation>>,
    // 符号名 -> 所有引用位置
    symbol_refs: HashMap<String, Vec<SymbolLocation>>,
}

// 单文件的语义信息
pub struct FileSemanticInfo {
    pub file_path: String,
    pub tokens: Vec<SemanticToken>,
    pub scopes: Vec<ScopeInfo>,
}

// 语义 Token（使用结构体 + 类型枚举，而非计划中的枚举变体）
pub struct SemanticToken {
    pub name: String,
    pub token_type: SemanticTokenType,
    pub modifiers: Vec<SemanticTokenModifier>,
    pub span: Span,
}

pub enum SemanticTokenType {
    Function, Type, Variable, Property, Method,
    Namespace, Parameter, LocalVariable, TypeParameter,
    Keyword, String, Number,
}

pub enum SemanticTokenModifier {
    Declaration, Readonly, Mutable, Public, Generic,
}

// 作用域信息
pub struct ScopeInfo {
    pub span: Span,
    pub parent: Option<usize>,  // 父作用域索引
    pub symbols: Vec<String>,   // 作用域内的符号
    pub kind: ScopeKind,        // Global, Function, Block, Lambda
}
```

**验收标准**：
- [x] SemanticDB 结构定义完成
- [x] SemanticToken 覆盖 LSP 标准 token 类型（12 种类型 + 5 种修饰符）
- [x] 支持按文件查询语义信息
- [x] 支持按符号名查询定义和引用位置

**测试项目**：
- [x] SemanticDB 结构创建测试
- [x] 按文件查询测试
- [x] 按符号名查询测试
- [x] 空数据库边界测试
- [x] 多文件管理测试
- [x] 文件覆盖更新测试

---

### 1.2 TypeCheck 语义收集器集成

**设计决策**：语义收集**不应该**在 LSP 层单独实现，而应该在 typecheck 阶段完成。

**原因**：
- typecheck 已经在遍历 AST，已经知道所有符号的定义和引用位置
- LSP 单独实现 SemanticCollector = 重复遍历 + 维护两套逻辑
- **好品味**：一次遍历，多处使用

**实现目标**：
- 在 `src/frontend/typecheck/` 模块中扩展语义收集功能
- 类型检查时同时产出 `SemanticDB` 数据
- LSP 层直接查询复用，不重复遍历 AST

**收集由 typecheck 规则**（阶段产出）：
```
StmtKind::Fn        → SemanticTokenType::Function (定义)
StmtKind::TypeDef   → SemanticTokenType::Type (定义)
StmtKind::Var       → SemanticTokenType::Variable (定义)
StmtKind::MethodBind→ SemanticTokenType::Method (定义)
StmtKind::Use       → SemanticTokenType::Namespace (引用)
Param               → SemanticTokenType::Parameter (定义)
Expr::Var           → SemanticTokenType::Variable (引用)
Expr::Call          → SemanticTokenType::Function (引用)
Expr::FieldAccess   → SemanticTokenType::Property (引用)
Expr::Cast          → SemanticTokenType::Type (引用)
```

**验收标准**：
- [x] typecheck 阶段产出 SemanticDB
- [x] LSP 能查询 typecheck 产出的语义信息
- [x] 消除 LSP 层的重复 AST 遍历

---

### 1.3 作用域链收集

**实现目标**：
- 作用域信息也由 typecheck 阶段产出
- 记录每个作用域的起始和结束位置
- 记录作用域内的符号列表
- 支持嵌套作用域的正确父子关系
- 支持 4 种作用域类型：Global, Function, Block, Lambda

**注意**：这些信息已经在 typecheck 的 `TypeEnvironment` 中管理，现在需要导出给 SemanticDB 使用。

**验收标准**：
- [x] 全局作用域信息正确
- [x] 函数作用域信息正确
- [x] 块级作用域信息正确
- [x] 嵌套作用域父子关系正确

**测试项目**：
- [x] 单层作用域测试（全局作用域）
- [x] 嵌套作用域测试（全局 + 函数）
- [x] Lambda 作用域测试
- [x] 作用域最内层查找测试

---

### 1.4 World 扩展集成

**实现目标**：
- 扩展 `src/lsp/world.rs` 中的 World 结构
- 添加 SemanticDB 字段
- LSP 文档变更时，触发 typecheck 重新执行以更新语义信息
- LSP handlers 直接查询 typecheck 产出的 SemanticDB

**设计调整**：
- 不再需要在 LSP 层单独调用 SemanticCollector
- LSP 只需要在文档变更后触发 typecheck 重新执行
- World 持有对编译 pipeline 的引用，获取最新的 SemanticDB

**验收标准**：
- [x] World 包含 SemanticDB 字段
- [x] 文档变更时触发 typecheck 重新执行并更新语义信息
- [x] LSP handlers 能查询语义信息

**测试项目**：
- [x] World 更新语义信息测试（通过已有 server 测试验证）
- [x] 多文件语义信息管理测试
- [x] 语义信息查询接口测试

---

## 阶段 2：LSP 语义高亮

> **目标版本**：v0.10
> **依赖**：阶段 1 完成
> **状态**：✅ 已完成

### 2.1 Semantic Tokens Capability 声明

**实现目标**：
- 在 `src/lsp/capabilities.rs` 中声明 semanticTokensProvider
- 定义 token 类型映射（YaoXiang → LSP）
- 定义 token 修饰符映射

**Token 类型映射**：
```
YaoXiang SymbolKind::Function    → LSP TokenType::FUNCTION
YaoXiang SymbolKind::Type        → LSP TokenType::TYPE
YaoXiang SymbolKind::Variable    → LSP TokenType::VARIABLE
YaoXiang SymbolKind::GenericType  → LSP TokenType::TYPE
YaoXiang SymbolKind::Parameter    → LSP TokenType::PARAMETER
YaoXiang SymbolKind::Property     → LSP TokenType::PROPERTY
YaoXiang SymbolKind::Method       → LSP TokenType::METHOD
YaoXiang SymbolKind::Namespace    → LSP TokenType::NAMESPACE
```

**验收标准**：
- [x] capabilities 声明包含 semanticTokensProvider
- [x] token 类型映射正确
- [x] 支持 full 和 delta 模式

**测试项目**：
- [x] capability 声明测试
- [x] 协议兼容性测试

---

### 2.2 textDocument/semanticTokens/full Handler

**实现目标**：
- 实现 `handle_semantic_tokens_full` 处理函数
- 从 SemanticDB 获取文件的语义 tokens
- 转换为 LSP SemanticToken 格式
- 支持全量刷新

**LSP 响应格式**：
```json
{
  "data": [
    0,   // deltaLine (相对于上一 token)
    0,   // deltaStart (相对于上一 token)
    5,   // length
    0,   // tokenType (function)
    0    // tokenModifiers
  ]
}
```

**验收标准**：
- [x] 返回正确的 semantic tokens 数据
- [x] 行号列号从 0 开始
- [x] 响应时间 < 200ms（单文件 < 1000 行）
- [x] 空文件返回空数组

**测试项目**：
- [x] 简单函数语义高亮测试
- [x] 复杂嵌套结构测试
- [ ] 性能测试（1000 行文件）——待基准测试
- [x] 空文件测试

---

### 2.3 textDocument/semanticTokens/full/delta Handler

**实现目标**：
- 实现增量语义 tokens 更新
- 跟踪文档版本差异
- 只返回变化的 tokens

**验收标准**：
- [ ] 增量更新返回正确的 delta
- [ ] 版本号正确追踪
- [ ] 删除操作正确处理

**测试项目**：
- [ ] 添加 token 增量测试
- [ ] 删除 token 增量测试
- [ ] 修改 token 增量测试

---

### 2.4 VSCode 主题配置

**实现目标**：
- 在 language-pack 中添加语义高亮主题配置示例
- 文档化 token 类型与主题色的映射

**主题色映射建议**：
```json
{
  "tokenTypes": {
    "function": "entity.name.function",
    "type": "entity.name.type",
    "variable": "variable",
    "parameter": "variable.parameter",
    "property": "variable.property",
    "namespace": "namespace"
  }
}
```

**验收标准**：
- [ ] 主题配置示例完整
- [ ] 文档说明清晰

---

## 阶段 3：增量编译

> **目标版本**：v0.11
> **依赖**：阶段 1 完成

### 3.1 模块依赖图构建

**实现目标**：
- 实现 `ModuleDependencyGraph` 结构
- 解析 import/use 语句构建模块依赖关系
- 支持循环依赖检测

**数据结构**：
```rust
pub struct ModuleDependencyGraph {
    // 模块 ID -> 依赖的模块 ID 列表
    deps: HashMap<ModuleId, Vec<ModuleId>>,
    // 模块 ID -> 导出的符号列表
    exports: HashMap<ModuleId, Vec<SymbolId>>,
    // 符号定义位置
    symbol_defs: HashMap<SymbolId, SymbolLocation>,
}

pub struct ModuleId {
    pub name: String,
    pub path: PathBuf,
}
```

**验收标准**：
- [ ] 单文件项目依赖图正确
- [ ] 多文件项目依赖图正确
- [ ] 循环依赖检测正确
- [ ] 增量更新时依赖图正确更新

**测试项目**：
- [ ] 单文件依赖测试
- [ ] 多文件依赖测试
- [ ] 循环依赖检测测试
- [ ] 增量更新测试

---

### 3.2 编译缓存系统

**实现目标**：
- 实现编译产物缓存（AST、类型信息、IR）
- 基于文件内容哈希检测变更
- 实现缓存序列化/反序列化

**缓存内容**：
```rust
pub struct CompilationCache {
    // 文件路径 -> 文件缓存
    files: HashMap<PathBuf, FileCache>,
    // 缓存元数据
    metadata: CacheMetadata,
}

pub struct FileCache {
    pub content_hash: u64,
    pub ast: Option<Module>,
    pub type_info: Option<TypeInfo>,
    pub ir: Option<ModuleIR>,
    pub semantic_db: Option<SemanticDB>,
    pub timestamp: SystemTime,
}
```

**验收标准**：
- [ ] 未变更文件直接使用缓存
- [ ] 变更文件正确重新编译
- [ ] 缓存序列化正确
- [ ] 缓存清理机制正常

**测试项目**：
- [ ] 缓存命中测试
- [ ] 缓存未命中测试
- [ ] 缓存序列化测试
- [ ] 缓存清理测试

---

### 3.3 增量编译调度器

**实现目标**：
- 实现基于依赖图的编译调度
- 只编译受变更影响的文件
- 拓扑排序确定编译顺序

**调度策略**：
```
1. 检测变更文件列表
2. 找出所有依赖变更文件的模块（递归向上）
3. 拓扑排序确定编译顺序
4. 并行/串行调度编译
```

**验收标准**：
- [ ] 单文件变更只重编译必要文件
- [ ] 编译顺序正确（依赖在前）
- [ ] 并行编译无竞态条件

**测试项目**：
- [ ] 单文件变更测试
- [ ] 多文件变更测试
- [ ] 依赖链变更测试
- [ ] 并行编译测试

---

### 3.4 构建系统集成

**实现目标**：
- 实现 `yaoxiang build` 命令的增量编译支持
- 输出增量编译统计信息
- 支持 `--force` 强制全量编译

**验收标准**：
- [ ] 增量编译命令正常工作
- [ ] 全量编译命令正常工作
- [ ] 统计信息输出正确
- [ ] 错误处理正确

**测试项目**：
- [ ] 增量编译功能测试
- [ ] 全量编译功能测试
- [ ] 统计信息测试

---

## 阶段 4：死代码警告（整合到编译流程）

> **目标版本**：v0.11
> **依赖**：阶段 1 完成（typecheck 阶段的语义信息）

> **说明**：死代码警告依赖 typecheck 阶段的符号引用信息，是编译时分析功能，不是运行时特性。

> **架构调整**：死代码分析整合到 typecheck 阶段，因为两者都需要遍历 AST/SemanticDB

### 4.1 死代码分析器

**实现目标**：
- 实现 `DeadCodeAnalyzer` 结构
- 分析未使用的导出符号
- 分析未使用的导入
- 生成警告信息

**设计决策**：死代码分析应该在 **typecheck 阶段** 完成，因为：
- typecheck 已经知道所有符号的定义和引用
- 不需要额外遍历 AST
- 语义信息已经通过 SemanticDB 提供

**分析规则**：
```
1. 收集所有入口点（main, pub 函数）
2. 从入口点出发，标记所有可达的符号
3. 不可达的导出符号 -> 警告
4. 未使用的导入 -> 警告
```

**数据结构**：
```rust
pub struct DeadCodeAnalyzer {
    // 入口点
    entry_points: HashSet<SymbolId>,
    // 所有符号定义
    all_defs: HashMap<SymbolId, SymbolDef>,
    // 符号引用（从 SemanticDB 获取）
    references: HashMap<SymbolId, Vec<Location>>,
    // 导入列表
    imports: Vec<ImportInfo>,
}

pub struct SymbolDef {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub is_exported: bool,
}
```

**验收标准**：
- [ ] 未使用的导出函数能检测
- [ ] 未使用的导出类型能检测
- [ ] 未使用的导入能检测
- [ ] 警告信息格式正确

**测试项目**：
- [ ] 未使用导出函数测试
- [ ] 未使用导出类型测试
- [ ] 未使用导入测试
- [ ] 多层级依赖测试

---

### 4.2 警告系统集成

**实现目标**：
- 在编译过程中集成死代码检测
- 通过 `CompilationWarning` 事件发布警告
- 支持多种输出格式（终端、JSON）

**警告格式**：
```
warning: unused function `dead_function`
  --> src/utils.yx:10:1
   |
10 | fn dead_function() { }
   | ^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: function is never used
```

**验收标准**：
- [ ] 死代码警告正确触发
- [ ] 警告位置信息准确
- [ ] 警告可配置（启用/禁用）
- [ ] 终端输出格式美观

**测试项目**：
- [ ] 警告触发测试
- [ ] 警告位置测试
- [ ] 配置测试
- [ ] 输出格式测试

---

## 关于 DAG 并发的说明

**本计划不包含 DAG 并发编译**，原因如下：

| 概念 | 归属 | 说明 |
|------|------|------|
| **运行时 DAG** | RFC-008 Runtime | 惰性求值依赖图，控制运行时任务调度 |
| **模块依赖图** | 本计划阶段3 | 编译器层面的模块依赖，用于增量编译 |
| **模块级并行编译** | 构建系统 | 基于阶段3的依赖图实现，不属于 LSP |

**正确的位置**：
- 运行时 DAG 并发 → 参考 [RFC-008: 运行时并发模型](../design/rfc/accepted/008-runtime-concurrency-model.md)
- 模块依赖图 → 本计划阶段3（已完成/进行中）
- 模块级并行编译 → 应作为构建系统功能实现，可基于阶段3的依赖图

---

## 架构设计总结

### 统一数据流

```
┌─────────────────────────────────────────────────────────────────┐
│                      语义信息中台架构                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   源码                                                             │
│     │                                                              │
│     ▼                                                              │
│   ┌─────────────────┐                                            │
│   │  词法/解析       │ ──▶ AST                                    │
│   └────────┬────────┘                                            │
│            │                                                       │
│            ▼                                                       │
│   ┌─────────────────┐                                            │
│   │  类型检查        │ ──┬─▶ TypeResult + Bindings                │
│   │                  │   │                                        │
│   │  同时产出         │   │  ← 一次遍历，多处使用                  │
│   │  SemanticDB      │   │                                        │
│   └────────┬────────┘   │                                        │
│            │            │                                        │
│            ▼            │                                        │
│   ┌─────────────────┐  │                                        │
│   │  SemanticDB     │◄─┘  ← typecheck 产出                      │
│   │  - 符号定义     │                                            │
│   │  - 符号引用     │                                            │
│   │  - 作用域链     │                                            │
│   └────────┬────────┘                                            │
│            │                                                       │
│    ┌───────┴───────┐                                            │
│    ▼               ▼                                             │
│ ┌──────┐       ┌──────────┐                                    │
│ │ LSP  │       │ 增量编译 │                                    │
│ │语义高亮│       │ + 死代码 │                                    │
│ └──────┘       └──────────┘                                    │
│                                                                 │
│   ▲                                                         ▲    │
│   │                                                         │    │
│   │  DAG 并发 → RFC-008 运行时，不在本计划范围               │    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 设计原则

1. **一次遍历**：typecheck 阶段同时产出语义信息，不重复遍历 AST
2. **多处使用**：LSP 语义高亮、增量编译、死代码分析共享同一份数据
3. **好品味**：不为了"解耦"而增加不必要的抽象层

### 文件修改清单

| 阶段 | 新增文件 | 修改文件 | 状态 |
|------|----------|----------|------|
| 1 | `src/frontend/typecheck/semantic_db.rs` | `src/frontend/typecheck/mod.rs` | ✅ 已完成 |
| 1 | - | `src/lsp/world.rs` | ✅ 已完成 |
| 2 | - | `src/lsp/capabilities.rs` | ✅ 已完成 |
| 2 | `src/lsp/handlers/semantic_tokens.rs` | `src/lsp/handlers/mod.rs` | ✅ 已完成 |
| 2 | - | `src/lsp/server.rs` | ✅ 已完成（新增 semanticTokens/full 请求分发） |
| 3 | `src/frontend/module/dep_graph.rs` | `src/frontend/module/mod.rs` | 待开始 |
| 3 | `src/frontend/cache/compilation_cache.rs` | `src/frontend/pipeline.rs` | 待开始 |
| 4 | `src/frontend/typecheck/dead_code.rs` | `src/frontend/typecheck/mod.rs` | 待开始 |

**关键调整**：语义收集器从 `src/lsp/` 迁移到 `src/frontend/typecheck/`

---

## 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| typecheck 耦合语义信息 | 解耦设计，SemanticDB 作为独立数据结构 |
| 循环依赖处理 | 显式检测并警告 |
| 增量编译竞态 | 使用 Mutex 保护共享状态 |
| 缓存一致性 | 版本号追踪、哈希验证 |

---

## 参考资料

- [LSP Semantic Tokens Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens)
- [Rust Analyzer Semantic Highlighting](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/semantic-highlighting.md)
- [Incremental Compilation (Rustc)](https://rustc-dev-guide.rust-lang.org/inc-intro.html)
- [RFC-008: 运行时并发模型](../design/rfc/accepted/008-runtime-concurrency-model.md)
