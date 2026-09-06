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
| 行为测试 | 无标记（默认） | 编译通过 + 运行退出 0 |
| 编译期拒绝测试 | `[test:error]` + `预期: 编译错误 EXXXX` | `check` 必须失败且输出含 `[EXXXX]`；编译通过 = FAIL，码不符 = FAIL |
| 运行期失败测试 | `[test:error]` + `预期: 运行时错误 EXXXX` | `check` 必须成功（编译期无辜）+ `run` 必须失败且输出含 `[EXXXX]` |

- 语法报错（E1xxx 解析段）与语义报错（E2xxx+）不设独立类别——预期码本身钉死阶段
- 无 `预期:` 行的 `[test:error]` 回退 exit≠0 判定（未分类形态，新语料不鼓励）
- 分流判定的 runner 落地状态见 RFC-036 §8.2（设计定案 2026-09-03）

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

### 2.5 文件头标记

文件头注释可声明测试性质，`yaoxiang test` 与 cargo test 语料 runner（yx_runner）
据此判定——两者共用同一解析实现（`src/util/test_markers.rs`，RFC-036 §8.2）。
标记扫描窗口为**前 16 行**：

```yaoxiang
// [test:error]: <应失败的原因>
// 预期: 编译错误 E2018
// 预期: 运行时错误 E6008
// [test:ignore]: <原因>
// [test:runtime]: standard
```

- `[test:error]` — 本文件**应失败**（编译期或运行期），配合 `预期:` 行进入 §2.2
  的分流判定：编译错误类验证编译器正确拒绝、运行时错误类验证编译无辜 + 运行期
  正确失败；期望码与输出 `[EXXXX]` 实际比对，码不符 = FAIL 并指明实际出现的码
- `[test:ignore]: <原因>` — 本文件被跳过，计入报告的 skipped（追踪 issue 编号，
  见提交检查清单）
- `[test:runtime]: <模式>` — 子进程运行时模式（`standard` / `embedded` / `full`），
  runner 透传 `--runtime`

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
  `std.test` 自身的测试也在其中（用 std.test 测 std.test，自举闭环）
- **未来用户包**：测试在包内，随包的 `[tool.test].patterns` 发现——std 是该惯例的
  预演（RFC-014）
- **发现**：不进默认 patterns；经显式路径运行（`yaoxiang test src/std/tests`），
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

# 库测试层（std 包内 yx 测试；布局迁移后生效，见 RFC-036 §9）
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
- [ ] `[test:ignore]` 文件有追踪 issue 编号
- [ ] `[test:error]` 文件带 `预期:` 类别行（编译错误 / 运行时错误）
- [ ] 库的 API 行为测试放库包内（`src/std/tests/`），不进语言语料
