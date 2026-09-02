---
title: 'RFC-036: std.test 测试框架与 yaoxiang test 命令'
status: '已接受'
author: '晨煦'
created: '2026-07-26'
updated: '2026-09-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test 测试框架与 yaoxiang test 命令

## 摘要

为 YaoXiang 引入标准测试框架 `std.test` 模块和 `yaoxiang test` CLI 子命令。测试文件是普通的 `.yx`
文件，以子进程 exit code 判定整体通过/失败；文件内部支持多个测试函数——断言失败以 `Err`
值表达（值语义），由套件收集 per-test 判定（§7）。`std.test` 模块用纯 YaoXiang 实现，是第一个
dogfooding 库。`yaoxiang test` 是 CLI 工具，非编译器特性——不涉及 parser、IR、字节码或执行器的任何改动。

## 动机

### 为什么需要测试框架？

当前 YaoXiang 的测试覆盖依赖 Rust 侧的 `#[test]` 和 `tests/` 集成测试。这意味着：

1. 标准库（std.math / std.list / std.dict / std.convert / std.io）的单元测试无法用 YaoXiang 编写
2. `#117 标准库各模块单元测试覆盖` 被阻塞，因为没有可用的测试基础设施
3. 语言特性的回归测试（如 RFC-032 spawn 语义变更）缺乏自动化手段

### 关键约束

- **17 关键字铁律**：不引入任何新关键字或语法结构
- **零编译器改动**：不碰 parser、IR、字节码、执行器
- **自举优先**：测试库用 YaoXiang 写，第一个 dogfooding 库

## 架构

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI 层:  yaoxiang test [--filter --fail-fast --json ...]    │
│              │                                               │
│  发现层:    读取 yaoxiang.toml → [tool.test] patterns         │
│              默认: tests/**/*.yx                              │
│              │                                               │
│  执行层:    对每个文件: yaoxiang run <file>                    │
│              检查 exit code → 串行执行                        │
│              │                                               │
│  报告层:    PASS/FAIL → 汇总                                  │
│              支持 --json / --verbose / --fail-fast            │
│                                                              │
│  断言层:    std.test (纯 YaoXiang，自举)                      │
│              底层: std.assert.assert                          │
│              诊断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### 核心原则

1. **测试框架不是编译器特性，是 CLI 工具** — `yaoxiang run` 已经能"执行测试"了，`yaoxiang test`
   只是帮你去跑所有文件并给你看报告
2. **零编译器改动** — 不引入 `@test` 注解扫描、字节码元数据段、执行器特殊入口
3. **自举** — `std.test` 模块用纯 YaoXiang 实现，底层能力来自 `std.assert` / `std.result`
4. **测试文件是普通 `.yx` 文件** — 文件以子进程运行，exit code 判定整体通过/失败
5. **断言失败是值，不是进程事件** — 测试函数返回 `Result`，断言失败以 `Err` 表达，
   套件逐个收集 per-test 判定（§7）；进程级 abort 只属于运行时守卫，不用于测试断言

## 详细设计

### 1. CLI 设计

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      指定测试文件或目录（默认: 从 yaoxiang.toml 读取，否则 tests/）

Options:
  --filter <NAME>     只跑文件名包含 <NAME> 的测试
  --fail-fast         遇到第一个失败就停止
  --verbose, -v       显示每个测试的详细 stdout/stderr
  --list              只列出测试文件，不跑
  --no-progress       不显示进度输出（表头与 PASS 行）；FAIL 明细与汇总保留（CI 场景）
  --json              输出 JSON 格式结果（CI 集成用）
```

#### 输出格式

**默认输出**（per-test 判定来自文件内套件收集，见 §7）：

```
Running 3 test files...

tests/math_test.yx ........................ PASS (0.002s)
tests/list_test.yx ........................ FAIL (0.003s)
  `-- [FAIL] push_grows_len: Expected 3, got 2
  `-- [ ok ] pop_returns_last
Results: 2 files passed, 1 file failed, 0 skipped (0.006s)
```

**JSON 输出**（`--json`）：

```json
{
  "summary": { "total": 3, "passed": 2, "failed": 1, "skipped": 0, "time_secs": 0.006 },
  "files": [
    { "file": "tests/math_test.yx", "passed": true, "time_secs": 0.002 },
    {
      "file": "tests/list_test.yx",
      "passed": false,
      "time_secs": 0.003,
      "exit_code": 1,
      "stderr": "error [E1024]: one is not two",
      "tests": [
        { "name": "push_grows_len", "passed": false, "error": "Expected 3, got 2" },
        { "name": "pop_returns_last", "passed": true }
      ]
    }
  ]
}
```

- 失败文件额外携带 `exit_code` 与 `stderr`（ANSI 剥离后的子进程诊断，CI 取证用）；
  `--verbose` 与 `--json` 组合时全部文件携带 `stdout` / `stderr`
- `--no-progress` 只抑制进度输出（表头与 PASS 行）——FAIL 明细与汇总始终输出，
  失败不可静默；`--list` 每行输出一个测试文件路径，不执行
- 文件内 per-test `tests` 数组来自 §7 套件收集，随值化模型落地生效（#319）

### 2. yaoxiang.toml 配置

放置在 `[tool.test]` 下，符合 RFC-015 的 `[tool.*]` 第三方扩展约定：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
# 未来可扩展:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- 默认 `patterns = ["tests/**/*.yx"]` — 用户零配置开箱即用
- 单文件模式（`yaoxiang test foo.yx`）直接跑，不读配置
- 未来可能拆成独立仓库（`[tool.test]` 位置不变）

### 3. std.test 模块（纯 YaoXiang）

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library
// First dogfooding library: YaoXiang's test library written in YaoXiang

use std.result

assert_eq = (a, b) => {
    if a == b { result.ok(()) } else { result.err(f"Expected {b}, got {a}") }
}

assert_ne = (a, b) => {
    if a != b { result.ok(()) } else { result.err(f"Expected not equal to {b}, got {a}") }
}

assert_true = (cond: Bool) => {
    if cond { result.ok(()) } else { result.err(f"Expected true, got {cond}") }
}

assert_false = (cond: Bool) => {
    if cond == false { result.ok(()) } else { result.err(f"Expected false, got {cond}") }
}
```

- 断言函数为**值语义**：返回 `Result((), String)`，失败以 `Err(诊断信息)` 表达，
  不 abort 进程——§7 套件据此收集 per-test 判定。`std.assert.assert` 的进程级
  abort 语义保留给运行时守卫，不进入测试断言路径
- 过渡说明：Phase 1 落地的 4 函数基于 `std.assert.assert`（abort 语义），是自举
  过渡实现；值语义族为本 RFC 标准形态，落地后替换（#319）
- `assert_eq` / `assert_ne` 用**无标注参数**（`Any`）——2026-08-02 实证：`==`/`!=` 与
  f-string 插值在 Any 上工作正常（Int/String 均验证通过），**不依赖泛型系统**。
  未来泛型就绪后可补标注
- `assert_false` 用 `cond == false` 表达取反（`not` 一元语法未落地，稳定后可迁移；
  `!assert` 一元形态同此依赖，见 §8.1）
- 错误码断言（`assert(err.code == "E3017")`）依赖 `Error` 值携带机器可读 `code`
  字段（§8.1）
- `std.test` 不依赖任何 native 代码，纯 YaoXiang 实现

### 4. 标准库加载机制（关键设计）

**Phase 1：嵌入二进制**

`std/test.yx`（以及未来所有用 YaoXiang 写的标准库模块）在构建时嵌入二进制：

```rust
// build.rs 或构建脚本，自动生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // 源代码文本
    // 未来更多
];
```

模块系统（RFC-029，2026-08-02 已完整落地）提供接入点：Registry 同时持有 native 模块与源模块，
orchestrator 负责多文件编排。`use std.test` 的解析顺序：

1. 先查 Rust native 模块（现有机制，如 `std.assert`）
2. 未命中，查嵌入的 `STD_YX_FILES`——命中则以**虚拟路径**（如 `<std>/test.yx`）作为种子模块
   注入 orchestrator，走正常前端管道（parse → typecheck → IR）
3. 未命中，走文件系统发现（用户模块）

嵌入源模块内部的 `use std.assert` 由 resolver 正常解析到 native registry——native 与源模块在
Registry 中共存，跨种类依赖天然成立。嵌入模块**按需编译**：仅被 import 时进入管道。

优势：

- 单文件模式下 `use std.test` 也能工作
- 标准库版本与二进制严格绑定，不会版本错配
- 不需要用户配置标准库路径

**未来：文件系统标准库**

当 YaoXiang 项目模式成熟后，标准库将改为文件系统形式。详见 RFC-014 的更新。

### 5. 发现与执行

**前置条件（2026-08-02 审核决议）**：CLI `run` 接入 orchestrator。现状 CLI `run` 走单文件管道
（`run_file_with_diagnostics`），无法解析用户模块导入；而 `yaoxiang test` 的子进程模型继承
CLI 能力，测试文件导入项目模块是核心场景。因此 Phase 1 先将 CLI `Run` 的源码分支委托给
`run_project`（orchestrator，目录递归发现）；#247（沿 use 按需发现）之后作为纯性能优化叠加。
无 import 的单文件经 orchestrator 行为等价，字节码分支不变。

**发现阶段**：

1. 如果指定了 `[PATHS]`，直接使用指定的路径
2. 否则读取 `yaoxiang.toml` 的 `[tool.test].patterns`
3. 如果没有配置，默认 `tests/**/*.yx`
4. 应用 `--filter` 过滤（文件名包含）

**执行阶段**：

1. 对每个文件：`yaoxiang run --debug-info <file>` 启动子进程
   （`--debug-info` 使运行时错误带源码位置——2026-08-02 实证 stack trace 输出 `file:line:col`）
2. 检查 exit code：0 为 PASS，非 0 为 FAIL
3. 捕获 stdout/stderr 用于报告
4. 仅串行执行（Phase 1），未来支持 `--parallel`
5. 如果 `--fail-fast`，遇到第一个 FAIL 立即停止

### 6. 测试隔离

测试隔离通过进程级边界自然实现：

- 每个测试文件运行在独立的子进程中
- 每个子进程有独立的 Heap、Frame、NativeContext
- 一个测试文件的 panic 不会影响其他测试文件
- 不需要额外的独立 Heap 上下文机制

### 7. 套件与多测试（值化模型）

一个测试文件可包含多个测试。文件内组织方式：

```yaoxiang
// tests/list_test.yx
use std.test
use std.list

push_grows_len = () => {
    xs = []
    list.append(xs, 1)
    test.assert_eq(list.len(xs), 1)
}

pop_returns_last = () => {
    xs = [1, 2]
    test.assert_eq(list.pop(xs), 2)
}

main = {
    test.suite([
        ("push_grows_len", push_grows_len),
        ("pop_returns_last", pop_returns_last),
    ])
}
```

- 每个测试是返回 `Result((), String)` 的零参函数；断言失败以 `Err` 表达（§3 值语义
  断言族），不中断进程——后续测试照常运行
- `test.suite` 逐个调用并收集：某测试非 Ok 即打印该测试的名字与诊断信息，Ok 静默
- 文件退出码：套件全 Ok → 0；任一 Err → 非 0（§5 执行阶段的 exit code 判定不变）
- runner 只见文件，不做函数级扫描：per-test 判定完全来自套件内收集，文件内部结构
  对 runner 透明——零编译器改动原则不受影响
- 明确不采用：进程内 catch 边界（17 关键字铁律）；runner 逐函数入口调用（仅限
  §8.2 编译失败等内部场景）
- `test.suite` 的具体 API 形态（签名、重复名处理、`--filter` 与套件名的交互）是
  实现细节，落地时在 #319 定案

### 8. 负向测试（预期失败）三层设计

负向测试按失败发生层拆分，各层归位：

#### 8.1 值级反向（通用，面向用户）

被测操作返回 `Result`，测试以普通断言表达预期失败：

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
test.assert_eq(result.err_code(r), "E3017")
```

- std.test 补 `assert_not` / `assert_err` 函数族；`!assert` 一元形态待 not 语法
  落地后提供（与 `assert_false` 的 `cond == false` 同款约束）
- 错误码断言依赖 `Error` 值扩展机器可读 `code` 字段：由 `Struct { message }` 增至
  `{ code, message }`（native `error_new_with_code`，std 导出码常量），使
  `err.code == "E3017"` 可断言
- 随 Result 化推进（#301、#316），可失败的操作逐个返回 `Result`，语料中的文件级
  负向标记随之迁为文件内断言

#### 8.2 编译失败（仅限语言设计者内部使用）

编译是全文件全有全无，无法在文件内表达"这行不该编译"。保留文件级特殊标记：

- `[test:error]` 标记由 runner 读取并反向判定（run 退出码非 0 = PASS）
- 标记升级为**结构化预期码**：runner 解析头部 `预期: 编译错误 EXXXX`，与编译器
  stderr 输出的 `[EXXXX]` 实际比对，码不符 = FAIL（trybuild stderr 快照思想的
  轻量版，零编译器改动）
- **仅服务本仓库语料，不是用户测试框架的一部分**；实现侧需统一双 runner 判定
  约定（目录约定 vs 头部标记），见 #319

#### 8.3 运行时硬失败（归入 Result 化）

不设独立机制——会失败的操作按语言方向返回 `Result`（#301、#316），测试统一走
§8.1 表达。进程级 abort（如断言违例、运行时参数错位）随 Result 化逐步收敛为值，
测试框架不为其提供专门语义。

## 与现有系统关系

| 项目                                          | 关系                              |
| --------------------------------------------- | --------------------------------- |
| Rust `#[test]`                                | 不动，编译器内部测试继续用 Rust   |
| 现有 `.yx` 集成测试（`tests/yaoxiang/`）      | 被 `yaoxiang test` 发现并执行     |
| `std.assert.assert(cond)`                     | 保留给运行时守卫；`std.test` 值语义断言族改基于 `std.result`（§3、§7） |
| 模块系统（RFC-029）                            | 嵌入源模块经 Registry/orchestrator 接入；CLI `run` 接 orchestrator 是前置 |
| `#200` 重构（`io.println` → `assert.assert`） | 与 `yaoxiang test` 完全一致的方向 |
| `@` 注解                                      | 不使用，不引入 `@test`            |

## 实现策略

### Phase 1：核心功能

改动范围：

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` 源码分支委托 `run_project`（多文件运行前置）
- `src/main.rs` — 新增 `Test` 子命令
- `src/std/test.yx` — 新增纯 YaoXiang 模块
- `build.rs` — 嵌入 `std/*.yx` 到二进制
- orchestrator / Registry — 支持从嵌入源以虚拟路径加载 `.yx` 模块
- RFC-015 配置解析 — `[tool.test]` 段
- 子进程执行（`--debug-info`）+ 报告

交付物：

- `yaoxiang test` 基本可用
- `std.test` 4 个断言函数
- 默认 `tests/**/*.yx` 发现
- 串行执行 + 默认输出格式

### Phase 2：完善

- `--filter` / `--fail-fast` / `--verbose` 参数
- `--json` 输出（CI 集成）
- `--list` 选项
- `--no-progress` 选项

### Phase 3：进阶

- `--parallel` 并行执行（依赖 spawn 并发模型完善）
- `[tool.test].exclude` 配置
- 更多断言函数（如 `assert_approx_eq` 用于 Float）

## 风险与缓解

| 风险                                    | 概率 | 缓解                                        |
| --------------------------------------- | ---- | ------------------------------------------- |
| `f"..."` 在 Any 上插值失败              | 无   | 2026-08-02 已实证（Int/String 均正常）      |
| `yaoxiang.toml` 配置解析不在当前 CLI 中 | 低   | 简单扩展，不影响核心功能                    |
| CLI run 接 orchestrator 引入行为回归    | 低   | 无 import 单文件路径等价；集成测试已覆盖 orchestrator |
| 嵌入 `.yx` 源文件到二进制增加体积       | 低   | `.yx` 源文件极小，可忽略                    |
| 测试回路耗时随语料增长                  | 高   | 主项是每文件全量编译（185 文件实测 11.3s），非子进程启动；`--parallel` 只缓解进程侧，编译成本需测试回路缓存（#251/#293 切片） |

## 开放问题

- [x] `std/test.yx` 中 `use std.assert` 的引用能否正确解析？——**已解决（2026-08-02）**。
      模块系统（RFC-029）落地后 native 与源模块在 Registry 中共存，resolver 统一解析，跨种类依赖天然成立
- [x] 测试输出中 `f"..."` 的泛型 `to_string` 是否引入新类型约束？——**已解决（2026-08-02）**。
      实证无标注参数（Any）上 `==`/`!=` 与 f-string 插值均工作（Int/String 验证通过），不引入新约束
- [x] `?` 泛型参数可行性？——**已解决（2026-08-02）**：`?` 类型语法当前不存在（且会被静默吞掉，
      已单开 issue 跟踪），Phase 1 断言函数用无标注参数，不依赖泛型系统

## 设计决策记录

| 决策         | 决定                                      | 日期       | 理由                       |
| ------------ | ----------------------------------------- | ---------- | -------------------------- |
| 测试标记方式 | 不使用 `@test` 注解，测试文件是普通 `.yx` | 2026-07-26 | 零编译器改动，子进程即隔离 |
| 断言方式     | `std.test` 模块纯 YaoXiang 函数           | 2026-07-26 | 自举，无 native 代码       |
| 测试执行模型 | 子进程 `yaoxiang run <file>` + exit code  | 2026-07-26 | 进程级隔离，零编译器改动   |
| 标准库加载   | 当前嵌入二进制，未来文件系统              | 2026-07-26 | 版本绑定，单文件可用       |
| 断言参数类型 | 无标注参数（Any），不依赖泛型系统         | 2026-08-02 | `?` 类型语法不存在；Any 实证可比较、可插值 |
| 多文件运行   | CLI `run` 委托 `run_project`（orchestrator）作为前置 | 2026-08-02 | 子进程模型继承 CLI 能力；#247 退化为纯性能优化 |
| 报告源码位置 | 子进程带 `--debug-info`                   | 2026-08-02 | 实证 stack trace 输出 `file:line:col`；经嵌入模块（std.test）中转的帧归属不在此保证内，属 #289 + RFC-034 |
| 负向测试分层 | 值级反向通用 / 编译失败 runner 结构化标记（仅内部）/ 硬失败归 Result 化 | 2026-09-02 | #319 定案；取代隐式 [test:error] 约定 |
| 文件内多测试 | 值化标准模型：测试函数返回 Result，套件收集 per-test 判定 | 2026-09-02 | 无 catch、非入口调用（入口仅限内部场景） |
| Error 码 | Error 增加机器可读 `code` 字段            | 2026-09-02 | 支撑错误码断言；编译期码走 runner 比对 |

## 参考文献

- [RFC-014: 包管理系统设计](../accepted/014-package-manager.md) — 标准库目录结构
- [RFC-015: 配置系统](../accepted/015-configuration-system.md) — `[tool.test]` 配置段
- [RFC-030: assert 断言机制](../review/030-assert-mechanism.md) — 底层依赖
- [Rust `#[test]` 机制](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考设计
