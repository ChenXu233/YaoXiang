# LSP 支持实现计划

> **任务**：实现 YaoXiang 语言服务器协议（LSP）支持
> **基于 RFC**：RFC-017 语言服务器协议（LSP）支持设计
> **日期**：2026-02-23
> **状态**：进行中
> **目标版本**：v0.7 - v0.9

---

## 概述

本计划基于 RFC-017 文档，将 LSP 实现分解为 6 个阶段，共 20 个子步骤。每个步骤包含详细的实现目标、验收标准和测试项目。

### 依赖关系总览

```
阶段0（前置） ──────┐
    │               │
    ▼               │
阶段1 ──────────────┼──► 阶段2 ──► 阶段3 ──► 阶段4 ──► 阶段5
                    │         │         │         │
                    └─────────┴─────────┴─────────┘
                              (可并行开发)
```

---

## 阶段 0：编译器前置适配 ✅ 已完成

> **重要性**：此阶段是 LSP 实现的前提，必须先完成
> **目标版本**：v0.6（与 LSP 服务器开发并行）
> **完成日期**：2025-07

### 0.1 错误收集模式

**实现目标**：
- 修改 `src/frontend/typecheck/inference/` 模块，返回 `Result&lt;Type, Vec&lt;Error>>` 而非遇到错误立即返回
- 实现 `ErrorKind` 枚举，包含 `Error`（严重错误）、`Warning`（警告）、`Note`（附加信息）
- 错误收集器持续累积错误，检查完成后统一返回所有错误

**验收标准**：
- [x] 类型检查器对单个文件返回所有错误（非短路返回）
- [x] 错误包含 Severity 级别信息
- [x] 存在 Error 时 publishDiagnostics 显示错误
- [x] 仅 Warning 时继续编译并显示警告

**实现说明**：
- `StatementChecker` 新增 `collect_all_errors` 模式，错误不再短路返回而是累积到 `collected_errors: Vec&lt;Diagnostic>`
- `TypeChecker::check_module_collect_all()` 为 LSP 提供全量错误收集入口
- 复用已有的 `Severity` 枚举（Error/Warning/Info/Hint）
- 修改文件：`src/frontend/typecheck/inference/statements.rs`、`src/frontend/typecheck/mod.rs`

**测试项目**：
- [x] 单文件多错误收集测试（至少 3 个类型错误）
- [x] Error/Warning/Note 级别区分测试
- [x] 错误累积后统一返回测试
- [x] 回归测试：现有正确代码行为不变

---

### 0.2 Parser 错误恢复

**实现目标**：
- 解析出错时，插入 `MissingExpression`、`MissingStatement` 等 placeholder 节点
- 避免因 AST 不完整导致类型检查 panic
- 示例：`x = ;` → `x = MissingExpression`

**验收标准**：
- [x] 解析器遇到语法错误时生成 placeholder 节点而非 panic
- [x] placeholder 节点有合理的 Span 信息
- [x] 类型检查器能处理 placeholder 节点（报告错误但不 panic）

**实现说明**：
- AST 新增 `Expr::Error(Span)` 和 `StmtKind::Error(Span)` 占位变体
- `parse_with_recovery()` 函数始终返回 `ParseResult`（包含 Module + 错误列表），不会失败
- `ExpressionInferrer` 和 `StatementChecker` 均能处理 Error 变体（报告 `invalid_syntax` 错误但不 panic）
- 修改文件：`src/frontend/core/parser/ast.rs`、`src/frontend/core/parser/mod.rs`、`src/frontend/core/parser/parser_state.rs`、`src/frontend/typecheck/inference/expressions.rs`、`src/middle/core/ir_gen.rs`

**测试项目**：
- [x] 语法错误恢复测试（缺少表达式、分号、括号等）
- [x] 连续错误恢复测试
- [x] placeholder 节点 Span 正确性测试
- [x] 错误级联场景测试

---

### 0.3 符号表位置扩展

**实现目标**：
- 扩展 `SymbolEntry` 结构，添加 `location: Location` 字段（文件路径、行号、列号）
- 构建 `SymbolIndex` 反向索引（名称 → 位置列表）
- 支持快速查找符号定义位置

**验收标准**：
- [x] SymbolEntry 包含完整的位置信息
- [x] 能根据名称快速查询所有定义位置
- [x] 能根据文件查询该文件所有符号

**实现说明**：
- `SymbolEntry` 新增 `location: Option&lt;SymbolLocation>>` 字段，`SymbolLocation` 包含 `file_path` 和 `Span`
- `SymbolTable` 新增 `insert_with_location()` 和 `insert_full()` 方法
- 新增 `SymbolIndex` 反向索引结构，支持 `by_name` 和 `by_file` 双向查询
- 方法包括：`find_by_name()`、`find_by_file()`、`from_table()`、`remove_file()` 等
- 修改文件：`src/frontend/core/lexer/symbols.rs`

**测试项目**：
- [x] 符号位置信息正确性测试
- [x] 名称到位置的映射测试
- [x] 多文件符号索引测试
- [x] 符号重载/重名处理测试

---

### 0.4 文档缓存系统（DocumentCache）

**实现目标**：
- 实现 `DocumentCache` 结构，包含：
  - `version: u32` - LSP 文档版本号
  - `content: String` - 当前内容
  - `content_hash: u64` - 内容哈希（快速比较）
  - `ast: Option&lt;Ast>>` - 缓存的 AST
- 实现增量变更检测（对比 content_hash）
- 文件级缓存：变化时重新解析整个文件

**验收标准**：
- [x] DocumentCache 正确管理版本号
- [x] 哈希检测能快速识别未变化文档
- [x] 变化时正确重新解析
- [x] 内存占用合理（有清理机制）

**实现说明**：
- `DocumentCache` 结构：version、content、content_hash、ast (`Option&lt;Module>>`)、file_path、dirty
- `DocumentStore` 管理所有打开文档，`HashMap<String, DocumentCache>`，支持容量限制和自动清理
- 内容哈希使用 `DefaultHasher`，`update()` 仅在哈希变化时更新内容和使 AST 缓存失效
- 清理策略：超过 `max_documents`（默认 128）时移除版本号最低的文档
- 包含完整测试套件（7 个单元测试）
- 修改文件：`src/util/cache.rs`

**测试项目**：
- [x] 版本号递增测试
- [x] 哈希检测准确性测试
- [x] 增量变更应用测试
- [x] 缓存清理/过期测试
- [ ] 大文件缓存性能测试（后续阶段补充）

---

## 阶段 1：LSP 基础框架（v0.7）✅ 已完成

### 1.1 项目结构创建

**实现目标**：
- 创建 `src/lsp/` 目录结构
- 引入 `lsp-types` crate 依赖
- 配置 Cargo.toml

**目录结构**：
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
├── world.rs            # 编译世界
├── scroller.rs         # 符号索引构建
├── protocol.rs         # LSP 协议类型定义
└── cache/              # 增量缓存模块
    ├── mod.rs
    ├── document.rs     # 文档缓存
    └── incremental.rs  # 增量解析策略
```

**验收标准**：
- [x] 目录结构创建完成
- [x] 依赖正确引入（lsp-types 0.97, lsp-server 0.7, serde_json, tokio 等）
- [x] 基础模块编译通过

**实现说明**：
- 创建 `src/lsp/` 目录，包含 `mod.rs`、`server.rs`、`session.rs`、`capabilities.rs`、`protocol.rs`、`world.rs`、`handlers/`
- Cargo.toml 添加 `lsp-types = "0.97"` 和 `lsp-server = "0.7"` 依赖
- `lib.rs` 注册 `pub mod lsp`
- `main.rs` 添加 `yaoxiang lsp` 子命令入口
- handlers 子模块：initialize、text_document、diagnostics（已实现）；completion、definition、references、hover（占位）

**测试项目**：
- [x] 模块编译测试
- [x] 依赖版本兼容性测试

---

### 1.2 生命周期方法实现

**实现目标**：
- 实现 `initialize` 请求处理（返回 serverCapabilities）
- 实现 `initialized` 通知处理
- 实现 `shutdown` / `exit` 请求处理
- 声明支持的 LSP 协议版本（3.18）

**验收标准**：
- [x] initialize 返回正确的 serverCapabilities
- [x] 支持的标准方法全部响应正确
- [x] 正确处理客户端关闭连接

**实现说明**：
- `handle_initialize()`：返回 ServerCapabilities（当前支持 TextDocumentSync Full 模式）+ ServerInfo
- `handle_initialized()`：会话进入 Running 状态
- `handle_shutdown()`：清理文档缓存，会话进入 ShuttingDown 状态
- `exit` 通知结束主循环
- Session 状态机：Uninitialized → Initializing → Running → ShuttingDown
- 未知方法返回 MethodNotFound 错误
- 修改文件：`src/lsp/handlers/initialize.rs`、`src/lsp/server.rs`、`src/lsp/session.rs`

**测试项目**：
- [x] initialize 请求/响应测试
- [x] shutdown/exit 流程测试
- [x] capabilities 声明完整性测试

---

### 1.3 基础日志和错误处理

**实现目标**：
- 配置日志系统（env_logger 或 tracing）
- 实现 JSON-RPC 错误响应
- 错误信息格式化为可读日志

**验收标准**：
- [x] 启动时输出配置信息
- [x] 错误请求返回正确的 error response
- [x] 日志包含请求/响应关键信息

**实现说明**：
- 复用项目已有的 `tracing` 日志系统，每个请求/通知都记录 info 级别日志
- `protocol.rs` 实现 JSON-RPC 响应构建函数：`ok_response()`、`error_response()`、`method_not_found()`、`internal_error()`、`notification()`
- 支持 ErrorCode：MethodNotFound、InternalError、InvalidRequest 等
- 修改文件：`src/lsp/protocol.rs`

**测试项目**：
- [x] 日志输出测试
- [x] 错误响应格式测试
- [x] 异常请求处理测试

---

## 阶段 2：诊断支持（v0.7） ✅ 已完成

### 2.1 文本文档同步

**实现目标**：
- 实现 `textDocument/didOpen` 通知处理
- 实现 `textDocument/didChange` 通知处理
- 实现 `textDocument/didClose` 通知处理
- 集成 DocumentCache 管理文档状态

**验收标准**：
- [x] didOpen 正确解析文档并缓存
- [x] didChange 正确更新文档内容
- [x] didClose 正确清理文档缓存
- [x] 文档版本号正确管理

**测试项目**：
- [x] didOpen/didChange/didClose 完整流程测试
- [x] 增量变更测试
- [x] 多文档管理测试
- [x] 并发变更测试

---

### 2.2 诊断集成

**实现目标**：
- 复用 `util/diagnostic/` 诊断系统
- 将 YaoXiang Diagnostic 转换为 LSP Diagnostic
- 实现诊断格式转换函数

**转换规则**：
```
YaoXiang Severity::Error   → LSP DiagnosticSeverity::ERROR
YaoXiang Severity::Warning → LSP DiagnosticSeverity::WARNING
YaoXiang Severity::Info    → LSP DiagnosticSeverity::INFORMATION
```

**验收标准**：
- [x] 类型错误转换为正确 severity
- [x] 语法错误正确报告
- [x] 位置信息准确（行号 0-indexed）

**测试项目**：
- [x] 错误类型转换测试
- [x] 位置偏移正确性测试
- [x] 多错误诊断测试

---

### 2.3 publishDiagnostics 发布

**实现目标**：
- 实现 `textDocument/publishDiagnostics` 通知
- 在文档变更后自动触发诊断
- 支持增量诊断更新

**验收标准**：
- [x] 正确发送 publishDiagnostics 通知
- [x] 诊断包含文件 uri、版本号
- [x] 错误清除时发送空诊断

**测试项目**：
- [x] 诊断发布测试
- [x] 错误清除测试
- [x] 版本号匹配测试

---

## 阶段 3：补全支持（v0.8） ✅ 已完成

### 3.1 符号索引构建

**实现目标**：
- 实现 World 结构的符号索引
- 构建：名称 → 位置列表的反向索引
- 实现文件 → 符号列表索引

**验收标准**：
- [x] 能根据光标位置获取上下文符号
- [x] 补全响应时间 < 100ms
- [x] 索引支持增量更新

**测试项目**：
- [x] 符号索引构建测试
- [x] 索引查询性能测试
- [x] 增量更新测试

---

### 3.2 关键字补全

**实现目标**：
- 实现 YaoXiang 关键字补全
- 支持关键字建议排序

**关键字列表**（基于 language-spec.md 第 2.3 节，共 17 个）：
```
pub         # 公开声明
use         # 模块导入
spawn       # 并作函数标记
ref         # Arc 引用计数共享
mut         # 可变绑定
if          # 条件分支
elif        # 否则如果
else        # 否则分支
match       # 模式匹配
while       # 条件循环
for         # 迭代循环
return      # 函数返回
break       # 循环跳出
continue    # 循环继续
as          # 类型转换
in          # for 循环迭代
unsafe      # 不安全代码块
```

**保留字**（基于 language-spec.md 第 2.4 节，共 7 个）：
```
Type        # 元类型（用于类型定义）
true        # Bool 真值
false       # Bool 假值
void        # Void 空值
some(T)     # Option 值变体构造
ok(T)       # Result 成功变体构造
err(E)      # Result 错误变体构造
```

**函数注解**（基于 language-spec.md 第 6.9.1 节）：
```
@block      # 禁用并发优化
@eager      # 强制急切求值
```

**验收标准**：
- [x] 所有 17 个关键字出现在补全列表
- [x] 7 个保留字出现在补全列表
- [x] 2 个函数注解（@block, @eager）出现在补全列表
- [x] 关键字按类别正确分类（关键字/保留字/注解）

**测试项目**：
- [x] 关键字补全测试（pub, use, spawn, ref, mut, if, elif, else, match, while, for, return, break, continue, as, in, unsafe）
- [x] 保留字补全测试（Type, true, false, void, some, ok, err）
- [x] 函数注解补全测试（@block, @eager）
- [x] 上下文相关关键字测试（如 if/elif/else 成组出现）

---

### 3.3 标识符补全

**实现目标**：
- 基于当前作用域的符号补全
- 基于导入模块的符号补全
- 支持类型前缀过滤（如 `Vec::`）

**验收标准**：
- [x] 当前文件符号可补全
- [x] 导入模块符号可补全
- [x] 补全项包含 kind 信息（keyword, function, variable, type）

**测试项目**：
- [x] 变量名补全测试
- [x] 函数名补全测试
- [x] 类型名补全测试
- [x] 模块成员补全测试
- [x] 补全触发测试（输入字符后）

---

## 阶段 4：跳转支持（v0.8） ✅ 已完成

### 4.1 跳转到定义（definition）

**实现目标**：
- 实现 `textDocument/definition` 处理
- 基于 AST 查找标识符定义位置
- 支持函数、结构体、变量、类型定义跳转

**验收标准**：
- [x] 函数调用跳转到函数定义
- [x] 变量引用跳转到变量定义
- [x] 类型使用跳转到类型定义
- [x] 支持跨文件跳转

**测试项目**：
- [x] 函数定义跳转测试
- [x] 变量定义跳转测试
- [x] 类型定义跳转测试
- [x] 跨文件跳转测试
- [x] 多次定义（同名）处理测试

---

### 4.2 查找引用（references）

**实现目标**：
- 实现 `textDocument/references` 处理
- 查找符号的所有引用位置
- 排除定义本身

**验收标准**：
- [x] 返回所有引用位置
- [x] 不包含定义位置
- [x] 引用包含定义位置信息

**测试项目**：
- [x] 变量引用查找测试
- [x] 函数引用查找测试
- [x] 跨文件引用查找测试

---

### 4.3 悬停提示（hover）

**实现目标**：
- 实现 `textDocument/hover` 处理
- 显示符号类型信息
- 显示函数签名和文档注释

**验收标准**：
- [x] 变量显示推断类型
- [x] 函数显示函数签名
- [x] 常量显示计算值

**测试项目**：
- [x] 变量悬停测试
- [x] 函数悬停测试
- [x] 常量悬停测试
- [x] 跨文件悬停测试

---

## 阶段 5：高级功能（v0.9） ✅ 已完成

### 5.1 工作区符号搜索

**实现目标**：
- 实现 `workspace/symbol` 处理
- 支持模糊搜索
- 支持符号类型过滤

**验收标准**：
- [x] 模糊匹配搜索结果正确
- [x] 搜索响应时间 < 500ms
- [x] 支持文件过滤

**测试项目**：
- [x] 模糊搜索测试
- [x] 符号类型过滤测试
- [x] 性能测试（大工作区）

---

### 5.2 格式化支持（可选）

**实现目标**：
- 实现 `textDocument/formatting` 处理
- 实现 `textDocument/rangeFormatting` 处理
- 定义 YaoXiang 代码风格

**验收标准**：
- [x] 基本格式化正确（缩进、空格）
- [x] 范围格式化正确

**测试项目**：
- [x] 整文件格式化测试
- [x] 范围格式化测试
- [x] 格式化性能测试

---

### 5.3 重构支持（可选）

**实现目标**：
- 实现符号重命名（textDocument/rename）
- 实现代码操作（textDocument/codeAction）

**验收标准**：
- [x] 重命名更新所有引用
- [x] 预览更改内容

**测试项目**：
- [x] 符号重命名测试
- [x] 引用更新测试

---

## 高级特性（后续版本）

### 幽灵提示（Inlay Hints） ✅ 已完成

**优先级**：P0

| 特性 | 实现目标 |
|------|----------|
| 常量值提示 | 显示编译期已计算好的常量（如 `const MAX = 100 + 200` 旁显示 `300`）|
| 可变性提示 | 显示变量是否可变（如 `mut x` 有明显标记）|
| 所有权消费提示 | 显示函数参数是否被消费 |
| 类型推断提示 | 显示推断出的具体类型（如 `x = vec()` 旁显示 `Vec<i32>`）|

**验收标准**：
- [x] 各种 Inlay Hint 正确显示
- [x] 性能影响 < 50ms

---

### 所有权语义可视化

**优先级**：P2

**实现目标**：
- 显示变量的 move 路径（从定义位置到所有使用位置）
- 借用生命周期可视化

---

## 测试策略

### 单元测试
- 每个模块独立的单元测试
- 使用 mock 隔离依赖

### 集成测试
- LSP 协议兼容性测试
- 与真实 IDE 的集成测试（VS Code、Neovim）

### 性能测试
- 大文件解析性能
- 补全响应时间
- 跳转响应时间

---

## 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| 性能问题 | 增量解析、后台线程处理 |
| 内存占用 | LRU 缓存、延迟加载 |
| 协议兼容性 | 声明支持的协议版本 |

---

## 参考资料

- [Language Server Protocol 规范](https://microsoft.github.io/language-server-protocol/)
- [LSP 规范 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [lsp-types crate](https://crates.io/crates/lsp-types)
- [Rust Analyzer](https://rust-analyzer.github.io/) - 参考实现
