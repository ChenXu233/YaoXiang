# yaoxiang check 命令实现记录（已落地）

## 概述

`yaoxiang check` 已扩展为可用于 CI 和本地快速校验的静态检查命令：支持多路径输入、文本/JSON 输出、watch 重检和统一错误计数。

本文档由“实现计划”更新为“实现记录 + 后续改进项”，与当前代码保持一致。

## 已实现能力

- 支持一个或多个输入路径（文件或目录）：`yaoxiang check <PATH...>`
- 目录递归收集 `.yx` 文件
- 支持 `--json` 输出结构化诊断
- 支持 `--watch` 监控并自动重新检查
- 支持 `--color auto|always|never`
- 支持 `--no-progress` 关闭进度/汇总信息
- 错误时退出码 `1`，成功时退出码 `0`，输入无有效文件时退出码 `2`

## CLI 参数（当前）

`yaoxiang check [OPTIONS] <PATH>...`

- `--json`: 输出 JSON
- `--watch` / `-w`: 监控文件变化
- `--color <auto|always|never>`: 控制彩色输出
- `--no-progress`: 不输出进度与汇总

## 核心实现

### 1) CLI 扩展

- `Commands::Check` 已从单文件参数升级为多路径参数 `paths: Vec<PathBuf>`
- 增加 `json/watch/color/no_progress` 选项
- 检查流程拆分为：
  - `run_check_once(...)`
  - `run_check_watch(...)`

### 2) 诊断聚合

在 `util/diagnostic/mod.rs` 新增：

```rust
pub struct CheckDiagnostic {
    pub file: String,
    pub diagnostic: Diagnostic,
}

pub struct CheckResult {
    pub diagnostics: Vec<CheckDiagnostic>,
    pub source_files: HashMap<String, SourceFile>,
    pub error_count: usize,
    pub warning_count: usize,
}

pub fn check_files_with_diagnostics(files: &[PathBuf]) -> anyhow::Result<CheckResult>
```

行为：
- 对所有输入文件逐个编译并聚合诊断
- 不再“遇错即停”
- 保留 `check_file_with_diagnostics` 兼容入口（内部复用多文件实现）

### 3) 输出格式

- 文本输出：使用 `TextEmitter`，支持颜色开关
- JSON 输出：包含
  - `error_count`
  - `warning_count`
  - `diagnostics[]`（含 `file/severity/code/message/line/column/...`）
  - `lsp` 字段（由 `JsonEmitter` 转换）

### 4) Watch 模式

- 基于 `notify` 的 `RecommendedWatcher`
- 监听输入路径（目录递归、文件非递归）
- 只对 `.yx` 相关事件触发检查
- 加入防抖窗口（250ms）

## 与原计划差异

1. 当前 watch 采用“防抖后全量重检”，尚未实现“仅重检变更文件 + 缓存增量结果”。
2. 当前编译管线对每个失败文件主要暴露首个结构化诊断，尚未做到“单文件返回完整诊断列表”。
3. 跨文件全局符号联合分析尚未在 `check` 命令中额外实现（依赖现有编译器行为）。

## 验证结果

已完成验证：

- `cargo check --bin yaoxiang` 通过
- 单元测试通过：
  - `test_check_files_with_diagnostics_ok`
  - `test_check_files_with_diagnostics_error`
- 冒烟测试通过：
  - `cargo run -- check tests/yaoxiang/list_test.yx --json --no-progress`
  - 临时错误文件检查返回 exit code `1`，且输出文件名、行列、级别与消息

## 后续建议

1. 为 `check --watch` 增加增量缓存，避免每次全量扫描。
2. 在前端/管线层扩展错误收集，支持每文件多诊断完整输出。
3. 增加 CLI 集成测试（进程级）覆盖退出码、JSON 结构、目录输入和 watch 行为。
