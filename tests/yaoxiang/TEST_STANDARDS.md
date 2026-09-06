# YaoXiang 测试规范

> 版本: 3.0
> 适用分支: all (assert.assert pattern)
> 体系设计: RFC-036（§7 套件收集 / §8 负向三层 / §9 测试体系分层）

---

## 一、测试体系

YaoXiang 测试按两个正交维度组织。

**按被测对象分两层（归属维度，RFC-036 §9）**：

```
┌─────────────────────────────────────────────┐
│  第一层: 语言可用性语料                       │
│  tests/yaoxiang/                             │
│  被测对象: 语言本身（解析器/类型系统/所有权/  │
│  编译期拒绝/运行期语义）                      │
│  std 只作断言工具，永不被测                   │
├─────────────────────────────────────────────┤
│  第二层: 库测试（随库走）                     │
│  std → src/std/tests/                        │
│  被测对象: 库的公开 API 契约                  │
│  未来用户包：测试在包内，随包配置发现          │
└─────────────────────────────────────────────┘
```

**按测试机制分三层（实现维度）**：

```
┌─────────────────────────────────────┐
│  第一层: E2E .yx 测试               │
│  tests/yaoxiang/ 与库测试 .yx       │
│  验证: 完整编译+执行流水线          │
├─────────────────────────────────────┤
│  第二层: 集成测试                    │
│  tests/integration/                 │
│  验证: 模块间交互、公开 API         │
├─────────────────────────────────────┤
│  第三层: 单元测试                    │
│  src/*/tests/                       │
│  验证: 单个模块内部逻辑             │
└─────────────────────────────────────┘
```

两层语料的判定机制完全共用（标记解析 `src/util/test_markers.rs`、断言库 `std.test`），
差异只在**被测对象**与**发现路径**（RFC-036 §9）。

---

## 二、语言可用性语料规范（tests/yaoxiang/）

### 2.1 目录结构

按语言规范章节组织（对齐 `docs/src/reference/language-spec/`）：

```
tests/yaoxiang/
├── 00-smoke/             # 冒烟测试
├── 01-syntax/            # 语法规范（对应 syntax.md）
│   ├── basics/           #   基本语法
│   ├── functions/        #   函数定义与调用
│   └── control-flow/     #   控制流
├── 02-type-system/       # 类型系统（对应 type-system.md）
├── 03-modules/           # 模块系统（对应 modules.md）
├── 03-semantics/         # 语义（return/尾表达式）
├── 04-concurrency/       # 并发模型（对应 concurrency.md）
├── 05-ownership/         # 所有权（独立章节）
├── 06-compile-errors/    # 编译期错误检测
├── 99-demos/             # 论文演示（非规范测试）
└── TEST_STANDARDS.md     # 本文件
```

**收录边界**：被测对象必须是语言本身。库的 API 行为测试放库自己的包（见第四章）；
语料中 std 只允许作为断言工具出现（`std.assert` / `std.test`）。甄别原则：换个
等价 API 不影响测试结论的，是库测试；结论依赖编译器/运行时行为本身的，是语言测试。

### 2.2 测试分类与判定契约（RFC-036 §8 分流判定）

语料内测试按失败发生层分三类，runner 按类别走不同判定路径：

| 类别 | 声明方式 | 判定契约 |
| ---- | -------- | -------- |
| 行为测试 | 无指令（默认） | 编译通过 + 运行退出 0 |
| 编译期拒绝测试 | `// expect: compile-error EXXXX` | `check` 必须失败且输出含 `[EXXXX]`；编译通过 = FAIL，码不符 = FAIL |
| 运行期失败测试 | `// expect: runtime-error EXXXX` | `check` 必须成功（编译期无辜）+ `run` 必须失败且输出含 `[EXXXX]` |

- 语法报错（E1xxx 解析段）与语义报错（E2xxx+）不设独立类别——预期码本身钉死阶段
- 指令解析失败（unknown kind / 缺码 / 多余 token / 重复声明 / 非法 mode）=
  不执行直接 FAIL——无静默退化通道（构造期拒绝）
- 分流判定与指令文法已落地（RFC-036 §8.2，2026-09-06）；报告层给出类别计数
  （`Categories:` 行与 JSON `by_kind`）

### 2.3 文件头格式

每个 `.yx` 文件必须以以下格式开头：

```yaoxiang
// <章节>/<文件名>.yx
// 覆盖: 规范 §X.X
// 验证: 一句话描述此文件验证的内容
// 状态: ✅ 可运行 | ⚠️ 待修复 | 🔴 未实现
```

状态说明：

- `✅ 可运行` — 当前实现完全支持
- `⚠️ 待修复` — 已知有 bug，测试已注释或跳过
- `🔴 未实现` — 编译器暂未实现该特性

### 2.4 断言约定

使用 `assert.assert` 验证值，不再使用 `io.println("ALL TESTS PASSED")` 哨兵字符串：

```yaoxiang
// 🟢 新模式（强制）
use std.assert

main = {
    x = 42
    assert.assert(x == 42, "x should be 42")
}
```

测试框架通过 exit code 判断（0=通过，非0=断言失败），不再依赖 stdout 字符串匹配。
文件内多测试用 `std.test` 套件收集（RFC-036 §7）：测试函数零参返回
`Result(Void, String)`，以 `test.suite([("名字", () => test_fn())])` 收集。

### 2.5 文件头指令

文件头注释可声明测试性质，`yaoxiang test` 与 cargo test 语料 runner（yx_runner）
据此判定——两者共用同一解析实现（`src/util/test_markers.rs`，RFC-036 §8.2）。
指令扫描窗口为**前 16 行**，文法为 `// key: value` 严格 token 匹配：

```yaoxiang
// expect: compile-error E2018
// expect: runtime-error E6008
// skip: <原因>
// mode: standard
```

- `// expect: compile-error EXXXX` — 本文件应被编译期拒绝：runner 单步 `check`，
  退出码非 0 且全部预期码实际出现 = PASS；编译通过 = FAIL（该报的没报），码不符
  = FAIL 并指明实际出现的码。`expect:` 是期望的唯一声明（RFC-036 §8.2，
  2026-09-06 定案弃用 `[test:error]` 布尔标记与中文 `预期:` 行）
- `// expect: runtime-error EXXXX` — 编译期无辜 + 运行期正确失败：`check` 必须
  通过，`run` 必须失败且码相符；check 就失败 = FAIL（语料分类错误必须暴露）
- `// skip: <原因>` — 本文件被跳过，计入报告的 skipped（追踪 issue 编号，
  见提交检查清单）
- `// mode: <模式>` — 子进程运行时模式（`standard` / `embedded` / `full`），
  runner 透传 `--runtime`（仅 run 步消费；非法值 = 解析失败直接 FAIL）

### 2.6 已知 Bug 的处理

对于已知有 bug 的语法特性，测试文件可以包含被注释的代码，并标记 TODO：

```yaoxiang
// TODO: 修复 match bug 后取消注释
// desc = match 1 {
//     1 => "one",
//     _ => "other"
// }
```

### 2.7 未实现特性的处理

不存在的功能不写测试。等实现后再补充 `.yx` 文件。

---

## 三、库测试规范（随库走）

库的公开 API 契约测试写在**库自己的包里**，不进语言语料（RFC-036 §9）：

- **位置**：std 的包即 `src/std/`，yx 级测试归 `src/std/tests/`，与实现体同处一地。
  `std.test` 自身的测试也在其中（用 std.test 测 std.test，自举闭环）。目录与
  Rust 单元测试共存（文件类型不相交，互不干扰）。
  **2026-09-06 已落地**：原 `tests/yaoxiang/07-std/` 的 19 个文件按甄别原则
  整体迁入（被测对象均为 std 模块 API 契约），07-std 目录撤销
- **未来用户包**：测试在包内，随包的 `[tool.test].patterns` 发现——std 是该惯例的
  预演（RFC-014）
- **发现**：不进默认 patterns；CLI 经显式路径运行（`yaoxiang test src/std/tests`），
  cargo test 侧 yx_runner 双根发现（`tests/yaoxiang/` + `src/std/tests/`），
  CI 分层运行。标记系统与断言库与语料层完全共用
- **甄别**：迁入时逐个判断被测对象——纯 API 行为的迁入；结论依赖编译器/运行时
  行为本身的（如 native `&T` 自动借用边界）留在语言语料并归入对应规范章节

---

## 四、集成测试规范

### 4.1 测试位置

`tests/integration/` 和 `tests/yx_runner.rs`

### 4.2 断言要求

- 必须验证实际输出值，不能只 `assert!(result.is_ok())`
- 对 `yaoxiang::run()`，通过返回值判断编译执行成功
- 对 E2E 测试，通过 `std::process::Command` 捕获 stdout 验证输出

### 4.3 禁用测试处理

禁用的测试必须注明原因和跟踪 issue：

```rust
// TODO: 修复 stack overflow (#XXX)
// #[test]
// fn test_disabled() { ... }
```

---

## 五、单元测试规范

### 5.1 测试位置

每个模块下的 `tests/` 子目录，保持与源码相同的层级结构。

### 5.2 命名规范

```
测试模块：mod_<描述>_tests
测试函数：test_<特性>_<场景>
```

示例：

```rust
mod mod_parser_fn_def_tests {
    fn test_parse_fn_def_with_params() { ... }
    fn test_parse_fn_def_block_body() { ... }
}
```

### 5.3 断言要求

- 验证具体的返回值或数据结构字段，不使用 `assert!(result.is_ok())` 作为唯一断言
- 对解析测试：验证 AST 节点字段值
- 对类型检查测试：验证推断出的 MonoType

### 5.4 文件大小上限

单个测试文件建议不超过 500 行。超过时应拆分到多个文件或合并重复用例。

---

## 六、运行方式

```bash
# 全部测试
cargo test

# 语言语料 E2E 测试
cargo test --test yx_runner

# 库测试层（std 包内 yx 测试，2026-09-06 已迁移；yx_runner 同样双根发现）
yaoxiang test src/std/tests

# 集成测试
cargo test --test integration

# 单元测试（lib）
cargo test --lib

# 单个模块测试
cargo test -p yaoxiang --lib -- <module>::tests::

# 手动运行单个 .yx 文件
cargo run -- run tests/yaoxiang/01-syntax/basics/variables.yx
```

---

## 七、提交检查清单

提交前确认：

- [ ] `cargo test` 全部通过
- [ ] E2E 测试文件有正确的文件头（// 覆盖: + // 验证: + // 状态:）
- [ ] 使用 `assert.assert` 而非 `io.println("ALL TESTS PASSED")`
- [ ] 每个 `assert.assert` 有自定义错误消息
- [ ] `// skip:` 文件有追踪 issue 编号
- [ ] 负向文件带 `// expect:` 指令（compile-error / runtime-error + 真实预期码）
- [ ] 库的 API 行为测试放库包内（`src/std/tests/`），不进语言语料
