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
  --parallel          并行执行（每核一个 worker；与 [tool.test].parallel 取或）
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
Categories: 2 behavior, 0 compile-error, 0 runtime-error
```

**JSON 输出**（`--json`）：

```json
{
  "summary": { "total": 3, "passed": 2, "failed": 1, "skipped": 0, "by_kind": { "behavior": 3, "compile-error": 0, "runtime-error": 0, "invalid": 0 }, "time_secs": 0.006 },
  "files": [
    { "file": "tests/math_test.yx", "kind": "behavior", "passed": true, "time_secs": 0.002 },
    {
      "file": "tests/list_test.yx",
      "kind": "behavior",
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
- 每文件附 `kind`（behavior / compile-error / runtime-error / invalid，§8.2），
  summary 附 `by_kind` 执行计数（固定四键，不含 skipped）；人类汇总附
  `Categories:` 类别分布行
- `--parallel` 下人类进度行按**完成序**流式输出（整块不交错），JSON `files`
  按 `file` 路径排序保证输出稳定（CI diff 友好）
- 文件内 per-test `tests` 数组来自 §7 套件收集，随值化模型落地生效（#319）
- 官方 CI（`.github/workflows/ci.yml` test job）即按此消费：cargo 侧跑
  `--test integration`（CLI 集成）与 `--test yx_runner`（双根语料守卫），随后
  `yaoxiang test --json --parallel` 分层实跑——默认模式（语言语料）与显式
  `src/std/tests`（库层）各产出一份报告；汇总表（total / passed / failed /
  skipped / time_secs 与 by_kind）写入 job summary，失败文件打印 `kind` /
  `exit_code` / `stderr` 取证，任一套件非零退出即判红

### 2. yaoxiang.toml 配置

放置在 `[tool.test]` 下，符合 RFC-015 的 `[tool.*]` 第三方扩展约定：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # 命中者从发现集剔除（--list 同样剔除）
parallel = true                   # 并行执行（与 --parallel flag 取或）
```

- 默认 `patterns = ["tests/**/*.yx"]` — 用户零配置开箱即用
- `exclude` 与 `patterns` 同形态（字面路径或 `root/**…`，一律按路径前缀命中）；
  excluded 意味着不是测试，需要运行时行为验证的夹具走 `yaoxiang run` 直跑
- **单文件模式（`yaoxiang test foo.yx`）直接跑，不读配置**——显式 paths 下
  `exclude`/`parallel` 配置键均不生效（flag 除外）
- 未来可能拆成独立仓库（`[tool.test]` 位置不变）

### 3. std.test 模块（纯 YaoXiang）

```yaoxiang
// std/test.yx — 纯 YaoXiang 测试断言库（值语义标准形态，2026-09-03 落地）
// 第一个 dogfooding 库：YaoXiang 的测试库用 YaoXiang 写。

use std.result

assert_eq: (a: Any, b: Any) -> Result(Void, String) = (a, b) => {
    if a == b { return result.ok(void) }
    return result.err(f"Expected {b}, got {a}")
}

assert_ne: (a: Any, b: Any) -> Result(Void, String) = (a, b) => {
    if a != b { return result.ok(void) }
    return result.err(f"Expected not equal to {b}, got {a}")
}

assert_true: (cond: Bool) -> Result(Void, String) = (cond) => {
    if cond { return result.ok(void) }
    return result.err(f"Expected true, got {cond}")
}

// assert_not 与 assert_false 同体；assert_err / assert_err_code 见 §8.1
```

- 断言函数为**值语义**：返回 `Result(Void, String)`，失败以 `Err(诊断信息)` 表达，
  不 abort 进程——§7 套件据此收集 per-test 判定。`std.assert.assert` 的进程级
  abort 语义保留给运行时守卫，不进入测试断言路径。Ok 载荷为 `Void`
  （type-system.md 规范 unit；`()` 是空 Tuple，两者不混用——2026-09-03 定案）
- **函数族 7 个（2026-09-03 已交付，abort 过渡版删除）**：值化
  `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not`
  （与 assert_false 同体，为 `!assert` 换装预留）+ `assert_err`（§8.1）
  - `assert_err_code`（§8.1 错误码断言）
  - `assert_approx_eq(a: Float, b: Float, eps: Float)`（Phase 3，2026-09-07
    交付）：`|a - b| <= eps` 判定，eps 由调用方**显式给出**——容差是测试契约的
    一部分，不设隐藏默认值；负 eps 声明处即 Err，NaN 恒 Err
- `assert_eq` / `assert_ne` 用 **Any 标注参数**——`==`/`!=` 与 f-string 插值在
  Any 上工作正常，不依赖泛型系统。注意参数**必须显式标注**：无标注参数
  过不了 native 泛型 `&Result(T, E)` 的调用检查（R1 探针实证）
- `assert_false` / `assert_not` 用 `cond == false` 表达取反（`not` 一元语法未落地，
  稳定后可迁移；`!assert` 一元形态同此依赖，见 §8.1）
- 块体 + 显式 `return` 形态：if 表达式的 then 臂类型在检查中被丢弃，
  两臂 Result 的 if 表达式是检查盲区，实现规避之
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
5. 发现范围即测试分层（§9）：默认 patterns 只覆盖语言可用性语料；库测试层
   （如 `src/std/tests/`）经显式路径或包配置发现，不混入默认扫描

**执行阶段**：

1. 对每个文件按头部指令分流执行（指令文法见 §8.2，经 `src/util/test_markers.rs`
   解析，与 yx_runner 共用）：
   - 行为测试：`yaoxiang run <file>` 子进程
     （#327 起运行时错误默认携带源码位置与栈帧——debug_map 默认生成，
     stack trace 输出 `file:line:col`）；`// mode:` 声明子进程 `--runtime` 模式
   - 编译期拒绝类：单步 `yaoxiang check <file>`
   - 运行期失败类：`check`（必须通过）+ `run`（必须失败）两步
2. `// skip: <原因>` 的文件跳过执行，计入报告的 skipped
3. 判定与预期码比对按 §8.2 判定矩阵；指令解析失败不执行直接 FAIL（构造期拒绝）
4. 捕获 stdout/stderr 用于报告
5. 默认串行；`--parallel`（或 `[tool.test].parallel`）按可用核数起 worker 池，
   每文件仍是一个独立子进程——skip/invalid 按发现序先行处理，执行结果按完成序
   流式输出，JSON 按路径排序（Phase 3，2026-09-07 交付）
6. 如果 `--fail-fast`，遇到第一个 FAIL 立即停止调度新文件；并行模式下在途文件
   跑完并计入

### 6. 测试隔离

测试隔离通过进程级边界自然实现：

- 每个测试文件运行在独立的子进程中
- 每个子进程有独立的 Heap、Frame、NativeContext
- 一个测试文件的 panic 不会影响其他测试文件
- 不需要额外的独立 Heap 上下文机制
- **并行执行（Phase 3）不扩展隔离边界**：子进程间的工作目录（CWD）是共享的，
  并行测试不得占用 CWD 内相同路径的文件——文件 I/O 类测试用独立文件名并在
  结束时清理

### 7. 套件与多测试（值化模型）

一个测试文件可包含多个测试。文件内组织方式（2026-09-03 已落地）：

```yaoxiang
// tests/list_test.yx
use std.list
use std.result
use std.test

push_grows_len: () -> Result(Void, String) = () => {
    xs = [1]
    extended = list.push(xs, 2)
    test.assert_eq(list.len(extended), 2)
}

pop_returns_last: () -> Result(Void, String) = () => {
    mut xs = [1, 2]
    last = list.pop(xs)
    test.assert_eq(last, 2)
}

main = {
    test.suite([
        ("push_grows_len", () => push_grows_len()),
        ("pop_returns_last", () => pop_returns_last()),
    ])
}
```

- 每个测试是返回 `Result(Void, String)` 的零参函数；断言失败以 `Err` 表达（§3 值语义
  断言族），不中断进程——后续测试照常运行
- `test.suite` 逐个调用并收集：某测试非 Ok 即记录名字与诊断，Ok 静默；全部跑完后
  任一 Err 即以 `std.assert.assert` 中止并附失败明细（`N of M test(s) failed` +
  每项 `[FAIL] 名字: 诊断`）——文件退出码非 0（§5 判定不变）。这里的 abort 是
  测试二进制的运行时守卫，不是断言路径；全 Ok 静默退出 0
- 顶层测试函数以**闭包形式入列**（`("name", () => test_fn())`）：顶层函数名作为
  值引用暂不支持（IR 层限制，`E3006`）——闭包体调用全局函数不受影响
- runner 只见文件，不做函数级扫描：per-test 判定完全来自套件内收集，文件内部结构
  对 runner 透明——零编译器改动原则不受影响
- 明确不采用：进程内 catch 边界（17 关键字铁律）；runner 逐函数入口调用（仅限
  §8.2 编译失败等内部场景）
- API 形态已定案（2026-09-03，#319）：`suite(tests: List((String, () -> Result(Void, String)))) -> Void`；
  重名不检测（名字仅用于报告展示）；`--filter` 按文件名过滤、不感知套件内测试名

### 8. 负向测试（预期失败）三层设计

负向测试按失败发生层拆分，各层归位：

#### 8.1 值级反向（通用，面向用户）

被测操作返回 `Result`，测试以普通断言表达预期失败：

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// 或一句封装（码只存在于 std Error 载体上，E 钉死为 Error）：
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` 已随值语义族交付（2026-09-03，§3）；
  `!assert` 一元形态待 not 语法落地后提供（与 `assert_false` 的 `cond == false`
  同款约束）
- 错误码断言依赖 `Error` 值携带机器可读 `code` 字段——已由 #323 M4 落地：
  `Error = { code, message }`（native `error_new(code, message)`），读取走
  `result.unwrap_err(r)` 取载体 + `result.code(e)` / `result.message(e)` 访问器
  （设计阶段预估的 `error_new_with_code` 命名、码常量导出与 `err.code` 字段访问
  均未采用——语言无 Struct 字段访问，码常量未导出）
- 随 Result 化推进（#301、#316），可失败的操作逐个返回 `Result`，语料中的文件级
  负向标记随之迁为文件内断言

#### 8.2 文件级负向指令（仅限语言设计者内部使用）

编译是全文件全有全无，无法在文件内表达"这行不该编译"；运行期失败同样需要文件级
表达（如套件含必失败测试）。文件头部以**结构化指令**声明期望，runner 按期望类别
**分流判定**（2026-09-03 定案分流；2026-09-06 指令文法定案并落地）。

头部指令文法（`// key: value`，前 16 行内，严格 token 匹配）：

```text
// expect: compile-error E1002 [E1003 ...]   编译期拒绝测试
// expect: runtime-error E6003 [...]         运行期失败测试
// skip: <原因>                              跳过执行，计入 skipped
// mode: embedded|standard|full              子进程 --runtime 模式（仅 run 步消费）
```

- 无 `expect:` 指令 = 行为测试。`expect:` 是期望的**唯一声明**——2026-09-06
  定案弃用 `[test:error]` 布尔标记与中文 `预期:` 散文抠码：布尔标记与期望行是
  两个松耦合事实，靠纪律保持一致必然漂移；英文严格 token 文法让 runner 机械
  解析（kind + 码全为固定 token，码后跟任何多余 token 即解析失败），解析失败
  = 不执行直接 FAIL——指令声明错误的静默退化通道不存在（构造期拒绝）
- **编译错误类**：单步 `check`——必须失败且输出含全部 `[EXXXX]`；编译通过
  = FAIL（该报的没报），拒绝但码不符 = FAIL。语法报错（E1xxx 解析段）与语义
  报错（E2xxx+）不设独立类别——预期码本身钉死了阶段
- **运行时错误类**：两步判定——`check` 必须**成功**（编译期无辜），`run` 必须
  失败且输出含全部 `[EXXXX]`。编译期就炸 = FAIL（与编译错误类方向相反的关键
  判定，防止「编译意外通过、运行侥幸失败」漏检）
- 与业界同构：Rust compiletest `//~ ERROR`、Go `// ERROR "regexp"`、GCC
  `dg-error`、Clang `expected-error` 均在 fixture 注释内声明期望并由 harness
  双向比对；它们采用行级锚定是因为多诊断编译器需要区分同文件多期望，本编译器
  首错即停、一文件一诊断，文件级即与编译器现实同构——错误恢复落地后可在文法
  上追加行锚形态（Cranelift filetests 的文件头指令 + 函数级期望是同款混合形态）
- 已知渲染债：parse 期诊断当前以 Debug 形态输出（`code: "E0012"` 而非
  `[E0012]`），码扫描对两种形态都接受；诊断渲染统一后收回严格形态
- **仅服务本仓库语料，不是用户测试框架的一部分**；双 runner 判定约定已收口
  （2026-09-03，#319）：yx_runner（cargo test）与 `yaoxiang test` 共用
  `src/util/test_markers.rs` 解析头部指令，06-compile-errors 的目录约定废弃。
  报告层给出类别计数：人类汇总附 `Categories:` 行，JSON summary 附
  `by_kind`（behavior / compile-error / runtime-error / invalid，skipped 另计），
  每文件附 `kind`

#### 8.3 运行时硬失败（归入 Result 化）

不设独立机制——会失败的操作按语言方向返回 `Result`（#301、#316），测试统一走
§8.1 表达。进程级 abort（如断言违例、运行时参数错位）随 Result 化逐步收敛为值，
测试框架不为其提供专门语义。（注意：§8.2 的「运行时错误类」标记判定是 runner 对
**尚不可 Result 化操作**的文件级验证通道，与本节的语义方向不矛盾——后者是终点，
前者是迁移期通道）

### 9. 测试体系分层：语言语料与库测试（2026-09-03 定案）

测试按**被测对象**分两层，各有归属与维护方；标记系统（§8.2）与断言库（§3）两级通用：

**第一层：语言可用性语料（`tests/yaoxiang/`）**

- 被测对象是**语言本身**——解析器、类型系统、模块、并发、所有权、编译期拒绝、
  运行期语义；目录按语言规范章节组织
- std 在语料中只作**断言工具**（`std.assert` / `std.test`），永不被测——库的
  API 行为不属于语言可用性
- 语料内按 §8 分流为行为测试 / 编译期拒绝测试 / 运行期失败测试三类判定

**第二层：库测试（随库走）**

- 被测对象是**库的公开 API 契约**（如 `list.push` 行为、`result.code` 语义）
- 测试写在**库自己的包里**：std 的包即 `src/std/`，其 yx 级测试归 `src/std/tests/`
  （与实现体同处一地；目录与 Rust 单元测试共存，文件类型不相交）；`std.test`
  自身的测试也在其中（用 std.test 测 std.test，自举闭环）
- 未来用户包沿用同一惯例：测试在包内，随包的 `[tool.test]` 发现（RFC-014 包管理
  的测试布局由此预演）
- 发现不进默认 patterns（默认 `tests/**/*.yx` 只覆盖语言层）：库测试层经显式路径
  （`yaoxiang test src/std/tests`）或包配置发现；CI 分层运行

迁移注记：**已迁移（2026-09-06）**——原 `tests/yaoxiang/07-std/` 的 19 个文件
逐个甄别后全部为库测试（被测对象均为 std 模块的 API 契约；`?` 传播、自动借用、
泛型实例化等语言特性在其中的角色是载体而非被测对象），整体迁入 `src/std/tests/`
并撤销 07-std 目录；yx_runner 改双根发现（`tests/yaoxiang/` + `src/std/tests/`），
默认 patterns 不含库层（集成测试固化该契约）。语言语料自此零 std-API 测试。

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
- 子进程执行 + 报告

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

### Phase 3：进阶（2026-09-07 已交付）

- `--parallel` 并行执行（worker 池 + 每文件独立子进程；`[tool.test].parallel` 配置键同效）
- `[tool.test].exclude` 配置（前缀命中剔除，`--list` 同样剔除）
- `assert_approx_eq`（Float 显式 eps 断言，§3）

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
| 报告源码位置 | 运行时错误默认携带（#327）                | 2026-09-07 | debug_map 默认生成，stack trace 输出 `file:line:col`；经嵌入模块（std.test）中转的帧归属不在此保证内，属 #289 + RFC-034 |
| 负向测试分层 | 值级反向通用 / 编译失败 runner 结构化标记（仅内部）/ 硬失败归 Result 化 | 2026-09-02 | #319 定案；取代隐式 [test:error] 约定 |
| 文件内多测试 | 值化标准模型：测试函数返回 Result，套件收集 per-test 判定 | 2026-09-02 | 无 catch、非入口调用（入口仅限内部场景） |
| Error 码 | Error 增加机器可读 `code` 字段            | 2026-09-02 | 支撑错误码断言；编译期码走 runner 比对 |
| 断言库形态 | 值语义族 7 函数落地，`Result(Void, String)` 契约；abort 过渡版删除 | 2026-09-03 | Void 是规范 unit（`()` 是空 Tuple 不混用）；嵌套位 Any 刚性、无标注参数过不了 native 泛型检查——参数必须显式标注（R1 探针实证） |
| 测试体系分层 | 语言语料（`tests/yaoxiang/`）与库测试（随库走，std → `src/std/tests/`）分两层；std 在语料中只作断言工具 | 2026-09-03 | 被测对象决定归属与维护方；库测试随包布局为 RFC-014 预演 |
| 负向标记分流判定 | 按期望类别分流：编译错误类 `check` 必须失败、运行时错误类 `check` 必须过 + `run` 必须失败；报告给出类别计数 | 2026-09-03（09-06 落地） | 类别混判会让「编译意外通过、运行侥幸失败」漏检；预期码钉死阶段，语法报错不单设类别 |
| 头部指令文法 | 期望以英文结构化指令声明（`// expect:` / `// skip:` / `// mode:`，严格 token 文法，解析失败直接 FAIL）；弃用 `[test:error]` 布尔标记与中文 `预期:` 散文抠码 | 2026-09-06 | 期望是 fixture 内容的属性，in-fixture 声明与业界同构（compiletest / Go / GCC / Clang 均如此），中央清单必腐烂；布尔标记 + 期望行双事实靠纪律耦合是缺陷面；结构化文法让 runner 机械判定无人工参与 |
| 并行执行模型 | `--parallel` 起每核 worker 池（每文件仍独立子进程），skip/invalid 先行按发现序处理、执行结果按完成序流式输出、JSON 按路径排序；`--fail-fast` 停止调度（在途跑完计入）；CWD 仍共享、不扩展隔离边界 | 2026-09-07 | 子进程模型下并行 = OS 线程调度 spawn，无需 yx 层并发；耗时主项是每文件全量编译（风险表），并行只缓解进程侧——#293 缓存切片才是主缓解 |

## 参考文献

- [RFC-014: 包管理系统设计](../accepted/014-package-manager.md) — 标准库目录结构
- [RFC-015: 配置系统](../accepted/015-configuration-system.md) — `[tool.test]` 配置段
- [RFC-030: assert 断言机制](../review/030-assert-mechanism.md) — 底层依赖
- [Rust `#[test]` 机制](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考设计
