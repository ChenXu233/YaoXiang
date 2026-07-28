# Changelog

## :bookmark: V0.7.11: Curry 代码生成与语法清理

> 发布日期: 2026-07-29

### 📦 版本信息

| 项目     | 值                  |
| -------- | ------------------- |
| 发布日期 | 2026-07-29          |
| 版本变更 | `0.7.10` → `0.7.11` |
| 提交数   | 45 个 commit        |

### 📋 本次更新概要

本次发版的核心是 curry（柯里化）函数代码生成的完整落地，从中间层 IR 拆分、调度方法到字节码解码全链路打通，让高阶函数的部分应用得以正确编译执行。同时对语言语法做了一次减法清理：废弃 `elif` 关键字改用 `else if` 平铺语法、移除已废弃的 `|` 变体语法，让语法面更收敛一致。此外修复了 if 表达式值绑定、逻辑非运算符、I64Add 编码等多处代码生成缺陷，并将运行时空值统一命名为 Void，配合文档自动翻译与 lint 工具链的完善，整体提升了语言的健壮性与可维护性。

### ✨ 新功能

#### Curry 函数代码生成

完整实现了 curry（柯里化）函数的代码生成链路（issue #227）。柯里化允许对多参数函数进行部分应用，此前缺少从源码到字节码的完整生成支持。本次引入分层 IR 生成结构，将 curry 函数拆分为多层中间函数，逐层接收参数并在最后一层调用原函数，使部分应用场景可以正确编译并端到端执行。

- 新增 `CurryLayer` 结构与 `split_curry` 拆分函数，描述 curry 的分层形态
- 新增 `generate_curry_intermediate_func` 中间层函数生成
- 新增 `generate_curry_function_ir` 调度方法，统一协调 curry IR 生成

#### 模块加载魔数探针

模块加载改用魔数（magic number）探针替代扩展名判定来识别二进制格式。相比依赖文件扩展名，读取文件头魔数更可靠，避免了扩展名被误改或省略时加载失败的问题。

- 以文件头魔数判定模块二进制格式，替代扩展名判定

### 🐛 Bug 修复

#### 代码生成稳定性

修复了多个代码生成路径上的缺陷，这些问题会导致生成的字节码在特定语法场景下产生错误的运行结果，影响范围涵盖条件表达式、逻辑运算、算术编码与 curry 解码。

- 修复 if 表达式作为值绑定时分支返回值丢失的问题（补充回归测试）
- 修复 `!`（逻辑非）运算符 IR 生成错误——此前被直接编译为常量 0
- 修复 curry desugaring 分层 IR 生成与 CallDyn 字节码解码问题（issue #227）
- 修复 I64Add 编码不一致，并在调试输出中显示操作数 hex 便于排查

#### 运行时 Frame 结构

- 合并 Frame 的 `registers`/`locals` 为统一的 `slots`，简化栈帧模型

### ♻️ 重构优化

#### 语言语法收敛

对语法做减法，移除历史遗留的冗余写法，让语法面更一致、降低解析与学习成本。

- 删除 `elif` 关键字，改用 `else if` 平铺语法
- 移除已废弃的 `|` 变体语法，并补充废弃声明与文档同步（issue #203）

#### 运行时空值命名统一

- 统一运行时空值命名为 `Void`，并在 middle 层移除 `Int(0)` 中转改为直传 Void，使空值语义更清晰

### 📝 文档与工具链

#### 文档

- 新增 RFC-037 工业化分发方案草案及设计决策记录
- 更新 RFC-036 std.test 测试框架、RFC-014 模块解析设计文档
- 同步多语言文档翻译与内容精炼，多次自动翻译 locale 与文档

#### 构建与工具

- 添加 markdownlint 钩子并启用 fail_fast，文档工作流补充 lint/format 步骤
- 添加 editorconfig 及 markdownlint 配置、文档 lint/format 脚本
- 配置 Dependabot 聚合 npm 依赖更新
- 调整本地构建并行数

#### 测试

- 补全 CLI 子命令集成测试并移除冗余 new 命令
- 新增 curry 端到端执行、IR 形态与非 curry 回归测试（issue #227）
- 测试合规修正：补 RFC 引用、AAA 分段与自定义断言消息

### 📋 提交记录

|   Hash    | 描述                                                          |
| :-------: | ------------------------------------------------------------- |
| `ab1e0dee` | chore: 调整本地构建并行数为 8                                   |
| `6da18144` | refactor(middle): 空值移除 Int(0) 中转直传 Void                |
| `4051ccf5` | refactor(backends): 统一运行时空值命名为 Void                  |
| `5e1d8b14` | docs: auto-translate documentation                             |
| `a90d2c07` | docs(docs): 同步多语言文档翻译与内容精炼                       |
| `addf2a4f` | ci(ci): 添加 markdownlint 钩子并启用 fail_fast                 |
| `a3147bef` | style(docs): 统一文档格式与表格对齐                            |
| `d3c34356` | ci(docs): 自动翻译工作流添加 markdown lint/format 步骤         |
| `8b9806f5` | chore(docs): 添加文档 lint/format 脚本                         |
| `f2eed0bd` | chore(build): 添加 editorconfig 及 markdownlint 配置           |
| `ad8e3dc7` | refactor(parser): 删除 elif 关键字，改用 else if 平铺语法       |
| `a2887dc1` | docs: auto-translate documentation                             |
| `2da0a85d` | test(codegen): if 表达式值绑定回归测试                          |
| `b8c34d19` | docs(design): 更新 RFC-037 工业化分发方案设计决策              |
| `d92201ed` | fix(codegen): 修复 if 表达式作为值绑定时分支返回值丢失          |
| `584959e1` | fix(codegen): 修复 if 表达式作为值绑定时分支返回值丢失          |
| `7f9cb0cd` | feat(codegen): 魔数探针替代扩展名判定                           |
| `87d3fcad` | docs(test): curry 测试合规修正 (issue #227)                    |
| `6eb4f268` | docs: auto-translate documentation                             |
| `2c30dc9c` | fix(codegen): curry desugaring 分层 IR 生成 + CallDyn 解码 (issue #227) |
| `953f096b` | fix(codegen): ! (逻辑非) 运算符 IR 生成错误修复                |
| `6b1b40eb` | test(codegen): 非 curry 回归测试 (issue #227)                  |
| `3475af91` | test(codegen): curry 端到端执行测试 (issue #227)               |
| `b1b32b02` | docs(rfc): 添加 RFC-037 工业化分发方案草案                     |
| `1fdc7e8d` | docs(design): 更新 RFC-036 测试框架与 RFC-014 模块解析         |
| `3f24ce68` | test(codegen): curry IR 形态测试 (issue #227)                  |
| `5b0dcdf9` | feat(codegen): 新增 generate_curry_function_ir 调度方法 (issue #227) |
| `59951f81` | feat(codegen): 新增 generate_curry_intermediate_func 中间层生成 (issue #227) |
| `6e93c580` | feat(codegen): 新增 CurryLayer 结构和 split_curry 拆分函数 (issue #227) |
| `447313b1` | docs: auto-translate documentation                             |
| `adb1499e` | test(backends): 测试合规修正                                   |
| `d573eeae` | docs(test): declarations.rs 文件头补充 RFC-010 引用 (issue #203) |
| `976c4aa4` | docs(design): 同步 \| 变体语法废弃后的文档示例 (issue #203)     |
| `0394dca4` | refactor(parser): 移除已废弃的 \| 变体语法 (issue #203)         |
| `8ffb079f` | docs(rfc): 添加 \| 变体语法废弃声明 (issue #203)                |
| `32e614c5` | fix(backends): 合并 Frame 的 registers/locals 为 slots          |
| `2c91463c` | chore(meta): 删除误提交的根目录临时文件                         |
| `f150bbd1` | docs: auto-translate documentation                             |
| `4383ff05` | i18n: auto-translate locale files                              |
| `40e5c97d` | fix(codegen): 显示操作数 hex，修复 I64Add 编码不一致            |
| `cd07f4e7` | docs: auto-translate documentation                             |
| `1e56e3ba` | test(package): 补全 CLI 子命令集成测试并移除冗余 new 命令        |
| `97a9771c` | docs(design): 添加 RFC-036 std.test 测试框架草案               |
| `29df5d94` | chore(build): 配置 Dependabot 聚合 npm 依赖更新                |
| `4ae6a860` | chore(build): 聚合 Dependabot 依赖更新                         |
