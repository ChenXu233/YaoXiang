# yaoxiang check 命令实现计划

## 概述

`yaoxiang check` 是一个命令行工具，用于对 YaoXiang 源码进行静态检查（类型检查、所有权检查、lint），不生成任何代码。它应提供快速的反馈，适合集成到 CI 流程或作为开发过程中的验证步骤。

## 目标

- 实现 `yaoxiang check` 命令，接受一个或多个文件/目录作为输入。
- 复用编译器的类型检查模块，输出错误和警告信息到标准错误。
- 支持 `--json` 选项，以 JSON 格式输出诊断信息，便于其他工具集成。
- 支持 `--watch` 模式，监视文件变化并重新检查。
- 错误信息应包含文件名、行号、列号、错误等级和描述。

## 方案选择

**采用方案 1：扩展现有 check 命令**

理由：
- 重用现有代码，改动最小
- 与现有 CLI 结构一致
- 复用已有的 `TextEmitter` 和 `JsonEmitter`

## 实现阶段

### 阶段 1：CLI 参数扩展（目标：1天）

#### 实现目标

- 扩展 `Commands::Check` 支持多个路径输入
- 添加 `--json`、`--watch`、`--color`、`--no-progress` 选项

#### 原子步骤

1. **修改 main.rs:95-99 的 Commands::Check 结构体**
   - 添加 `paths: Vec<PathBuf>`
   - 添加 `--json`、`--watch`、`--color`、`--no-progress` 标志

2. **添加文件收集函数**
   - 复用 `format` 命令的 `collect_yx_files()` 逻辑
   - 支持目录递归收集 `.yx` 文件

3. **修改 match 分支**
   - 调用新的检查函数并传递参数

#### 预期结果

- `yaoxiang check file.yx` 工作
- `yaoxiang check dir/` 递归检查目录
- `yaoxiang check --json file.yx` 输出 JSON

---

### 阶段 2：诊断收集器重构（目标：1天）

#### 实现目标

- 将 `check_file_with_diagnostics` 改为返回诊断列表
- 支持多文件检查并聚合所有错误

#### 原子步骤

1. **在 util/diagnostic/mod.rs 添加 check_files_with_diagnostics**

   返回结构 `CheckResult`:
   ```rust
   pub struct CheckResult {
       pub diagnostics: Vec<Diagnostic>,
       pub source_files: HashMap<String, SourceFile>,
       pub error_count: usize,
       pub warning_count: usize,
   }
   ```

2. **修改检查逻辑**
   - 遍历所有文件，收集错误而非遇到错误即返回
   - 对每个文件调用 `Compiler::compile()`
   - 收集所有编译错误

3. **处理跨文件引用**
   - 需要解析所有文件并构建全局符号表
   - 处理标准库的路径和内置定义

#### 预期结果

- 单文件检查返回所有错误
- 多文件检查聚合所有错误

---

### 阶段 3：输出格式化（目标：1天）

#### 实现目标

- 支持人类可读的彩色输出
- 支持 JSON 格式输出
- 统一错误计数统计

#### 原子步骤

1. **修改 check 命令处理逻辑**
   - 根据 `--json` 参数选择输出格式
   - 使用已有的 `TextEmitter` 和 `JsonEmitter`

2. **实现 Rustc 风格的文本输出**
   ```
   error[E1001]: Unknown variable
    --> file.yx:5:7
     |
   5 |     print(a)
     |            ^
   ```

3. **实现 JSON 格式**
   - 参考 LSP 诊断结构
   - 输出 JSON 数组

#### 预期结果

- 文本输出美观易读
- JSON 输出可被其他工具解析

---

### 阶段 4：Watch 模式（目标：2天）

#### 实现目标

- 使用 `notify` 监视文件变化
- 增量检查而非全量重检

#### 原子步骤

1. **添加 --watch 参数处理**

2. **使用 notify crate**
   - 监视所有 `.yx` 文件
   - 监听 `Create`, `Modify`, `Delete` 事件

3. **实现增量检查**
   - 只检查变化的文件
   - 防抖处理（debounce）避免频繁检查
   - 保持已检查文件的结果缓存

4. **终端输出优化**
   - 清除旧结果
   - 显示检查状态

#### 预期结果

- `yaoxiang check --watch .` 监视文件变化
- 文件保存后自动重新检查

---

### 阶段 5：测试与集成（目标：1天）

#### 实现目标

- 编写单元测试和集成测试
- 验证输出格式正确

#### 原子步骤

1. **单元测试**
   - 测试各种错误类型的输出格式
   - 测试 JSON 输出结构
   - 测试多文件收集逻辑

2. **集成测试**
   - 在 `tests/integration/` 添加 check 命令测试
   - 测试实际 CLI 调用

3. **退出码验证**
   - 有错误：退出码 1
   - 无错误：退出码 0

#### 预期结果

- 测试覆盖关键路径
- CI 流程可集成

---

## 验收标准

| 验收项 | 标准 |
|--------|------|
| 单文件无错 | 退出码 0，无输出（或 "Type check passed"） |
| 单文件有错 | 退出码非 0，输出文件名、行号、列号、描述 |
| 多文件 | 检查所有 .yx 文件，聚合错误 |
| --json | 输出有效 JSON 数组 |
| --watch | 文件变化自动重新检查 |
| 检查速度 | 与编译器类型检查速度相当 |
| 跨文件引用 | 能正确处理项目中的多个文件，识别跨文件引用 |

## 测试项目

1. **test_check_no_errors**: 无错误文件，验证退出码 0
2. **test_check_type_error**: 类型错误，验证错误信息完整
3. **test_check_multi_files**: 多文件模块依赖，验证跨文件错误检测
4. **test_check_json_output**: JSON 格式，验证输出结构正确
5. **test_check_watch_mode**: Watch 模式，验证文件变化自动重新检查
6. **test_check_exit_code**: 退出码，验证有错/无错的退出码正确
7. **test_check_warning**: 警告信息，验证警告输出正确
8. **test_check_directory**: 目录检查，验证递归检查所有 .yx 文件

## 依赖

- 类型检查模块必须支持从文件系统读取文件并构建全局环境
- 标准库的定义需要可访问（可能内置于编译器或通过路径指定）

## 技术储备

- 项目已有 `clap` 作为命令行解析（cargo.toml:74）
- 项目已有 `notify` 用于文件监视（cargo.toml:61）
- 项目已有 `TextEmitter` 和 `JsonEmitter` 用于诊断输出
- 项目已有 `Compiler::compile()` 执行类型检查
- 项目已有 `collect_yx_files()` 函数可复用
