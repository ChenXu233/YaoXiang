---
title: "RFC-036: std.test 测试框架与 yaoxiang test 命令"
status: "草案"
author: "晨煦"
created: "2026-07-26"
updated: "2026-07-26"
issue: "#94, #95, #221"
---

# RFC-036: std.test 测试框架与 yaoxiang test 命令

## 摘要

为 YaoXiang 引入标准测试框架 `std.test` 模块和 `yaoxiang test` CLI 子命令。测试文件是普通的 `.yx` 文件，通过 `std.assert.assert` + exit code 判断通过/失败。`std.test` 模块用纯 YaoXiang 实现，是第一个 dogfooding 库。`yaoxiang test` 是 CLI 工具，非编译器特性——不涉及 parser、IR、字节码或执行器的任何改动。

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

1. **测试框架不是编译器特性，是 CLI 工具** — `yaoxiang run` 已经能"执行测试"了，`yaoxiang test` 只是帮你去跑所有文件并给你看报告
2. **零编译器改动** — 不引入 `@test` 注解扫描、字节码元数据段、执行器特殊入口
3. **自举** — `std.test` 模块用纯 YaoXiang 实现，底层调用 `std.assert.assert`
4. **测试文件是普通 `.yx` 文件** — 通过 exit code 判断通过/失败

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
  --no-progress       不显示进度条（CI 场景）
  --json              输出 JSON 格式结果（CI 集成用）
```

#### 输出格式

**默认输出**：
```
Running 5 tests from 3 files...

tests/math_test.yx ........................ PASS (0.002s)
tests/list_test.yx ........................ PASS (0.001s)
tests/string_test.yx ...................... FAIL (0.003s)
  `-- Expected "hello", got "world"
      at tests/string_test.yx:12:5

Results: 2 passed, 1 failed, 0 skipped (0.006s)
```

**JSON 输出**（`--json`）：
```json
{
  "summary": { "total": 3, "passed": 2, "failed": 1, "skipped": 0, "time_secs": 0.006 },
  "tests": [
    { "file": "tests/math_test.yx", "passed": true, "time_secs": 0.002 },
    {
      "file": "tests/string_test.yx", "passed": false, "time_secs": 0.003,
      "error": "Expected \"hello\", got \"world\"",
      "exit_code": 1
    }
  ]
}
```

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

use std.assert

assert_eq: (a: ?, b: ?) -> Void = (a, b) => {
    assert.assert(a == b, f"Expected {b}, got {a}")
}

assert_ne: (a: ?, b: ?) -> Void = (a, b) => {
    assert.assert(a != b, f"Expected not equal to {b}, got {a}")
}

assert_true: (cond: Bool) -> Void = (cond) => {
    assert.assert(cond, f"Expected true, got {cond}")
}

assert_false: (cond: Bool) -> Void = (cond) => {
    assert.assert(!cond, f"Expected false, got {cond}")
}
```

- 4 个断言函数，全部用 `f"..."` 做诊断信息
- `assert_eq` / `assert_ne` 的 `?` 泛型参数依赖泛型系统
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

模块加载器解析 `use std.test` 时：
1. 先查 Rust native 模块（现有机制，如 `std.assert`）
2. 未命中，查嵌入的 `STD_YX_FILES`，找到 `std/test.yx` 的源代码
3. 编译该源代码并注册到模块系统

优势：
- 单文件模式下 `use std.test` 也能工作
- 标准库版本与二进制严格绑定，不会版本错配
- 不需要用户配置标准库路径

**未来：文件系统标准库**

当 YaoXiang 项目模式成熟后，标准库将改为文件系统形式。详见 RFC-014 的更新。

### 5. 发现与执行

**发现阶段**：
1. 如果指定了 `[PATHS]`，直接使用指定的路径
2. 否则读取 `yaoxiang.toml` 的 `[tool.test].patterns`
3. 如果没有配置，默认 `tests/**/*.yx`
4. 应用 `--filter` 过滤（文件名包含）

**执行阶段**：
1. 对每个文件：`yaoxiang run <file>` 启动子进程
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

## 与现有系统关系

| 项目 | 关系 |
|------|------|
| Rust `#[test]` | 不动，编译器内部测试继续用 Rust |
| 现有 `.yx` 集成测试（`tests/yaoxiang/`） | 被 `yaoxiang test` 发现并执行 |
| `std.assert.assert(cond)` | 保留，`std.test` 底层依赖它 |
| `#200` 重构（`io.println` → `assert.assert`） | 与 `yaoxiang test` 完全一致的方向 |
| `@` 注解 | 不使用，不引入 `@test` |

## 实现策略

### Phase 1：核心功能

改动范围：
- `src/main.rs` — 新增 `Test` 子命令
- `src/std/test.yx` — 新增纯 YaoXiang 模块
- `build.rs` — 嵌入 `std/*.yx` 到二进制
- 模块加载器 — 支持从嵌入源加载 `.yx` 模块
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

### Phase 3：进阶

- `--parallel` 并行执行（依赖 spawn 并发模型完善）
- `[tool.test].exclude` 配置
- 更多断言函数（如 `assert_approx_eq` 用于 Float）

## 风险与缓解

| 风险 | 概率 | 缓解 |
|------|------|------|
| `f"..."` 在泛型类型上插值失败 | 低 | 已在 `std.assert.assert` 中验证基础类型可用 |
| 子进程启动开销影响测试速度 | 中 | Phase 1 串行执行，可接受；Phase 3 并行缓解 |
| `yaoxiang.toml` 配置解析不在当前 CLI 中 | 低 | 简单扩展，不影响核心功能 |
| 泛型 `?` 在 `std.test` 中不可用 | 低 | 可降级为 `Any` 类型或类型特化 |
| 嵌入 `.yx` 源文件到二进制增加体积 | 低 | `.yx` 源文件极小，可忽略 |

## 开放问题

- [ ] `std/test.yx` 中 `use std.assert` 的引用是否能在模块加载器中正确解析？需要验证嵌入源模块之间的依赖关系
- [ ] 测试输出中 `f"..."` 的泛型 `to_string` 是否会引入新的类型约束？需要验证

## 设计决策记录

| 决策 | 决定 | 日期 | 理由 |
|------|------|------|------|
| 测试标记方式 | 不使用 `@test` 注解，测试文件是普通 `.yx` | 2026-07-26 | 零编译器改动，子进程即隔离 |
| 断言方式 | `std.test` 模块纯 YaoXiang 函数 | 2026-07-26 | 自举，无 native 代码 |
| 测试执行模型 | 子进程 `yaoxiang run <file>` + exit code | 2026-07-26 | 进程级隔离，零编译器改动 |
| 标准库加载 | 当前嵌入二进制，未来文件系统 | 2026-07-26 | 版本绑定，单文件可用 |
| 泛型断言 | 依赖 `?` 泛型参数 | 2026-07-26 | 不引入特化，信任泛型系统 |

## 参考文献

- [RFC-014: 包管理系统设计](../accepted/014-package-manager.md) — 标准库目录结构
- [RFC-015: 配置系统](../accepted/015-configuration-system.md) — `[tool.test]` 配置段
- [RFC-030: assert 断言机制](../review/030-assert-mechanism.md) — 底层依赖
- [Rust `#[test]` 机制](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考设计