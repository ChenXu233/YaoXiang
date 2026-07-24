---
title: "RFC-036: std.test 测试框架与 yaoxiang test 命令"
status: "草案"
author: "晨煦"
created: "2026-07-25"
updated: "2026-07-25"
issue: "#94"
---

# RFC-036: std.test 测试框架与 yaoxiang test 命令

## 摘要

为 YaoXiang 引入标准测试框架 `std.test` 模块和 `yaoxiang test` CLI 子命令。测试发现基于现有 `@test` 注解标记函数（**零新语法**），断言使用 std.test 模块导出的纯函数（`assert_eq`、`assert_ne`、`assert_ok`、`assert_err`），测试执行由编译器扫描注解 + 运行时调度。不引入任何新的关键字、保留字或语法结构。

## 动机

### 为什么需要测试框架？

当前 YaoXiang 的测试分散在 Rust 侧的 `#[test]` 和 `tests/` 集成测试中，对 YaoXiang 语言本身的测试覆盖依赖 Rust 写。这意味着：

1. YaoXiang 标准库（std.math / std.list / std.dict / std.convert / std.io）的单元测试无法用 YaoXiang 编写
2. `#117 标准库各模块单元测试覆盖` 被阻塞，因为没有可用的测试基础设施
3. 语言特性的回归测试（如 RFC-032 spawn 语义变更）缺乏自动化手段

### 关键约束：零新语法

YaoXiang 仅保留 17 个核心关键字（`pub`、`use`、`spawn`、`if`、`else`、`match`、`for`、`in`、`while`、`return`、`break`、`continue`、`as`、`ref`、`true`、`false`、`None`），这是设计宣言中明确的不可妥协原则。

**测试框架绝不能引入新关键字（如 `test`、`assert` 关键字）或新语法结构（如 `test` 代码块）。**

## 提案

### 核心设计

```
┌─────────────────────────────────────────────────────────────────┐
│                      测试架构（三层）                             │
│                                                                 │
│  ① 发现层：编译器扫描 @test 注解 → 生成测试清单                 │
│      不新增语法，@ 注解已存在                                     │
│                                                                 │
│  ② 执行层：yaoxiang test 子命令                                  │
│      发现 → 编译 → 并行执行 → 报告                               │
│                                                                 │
│  ③ 断言层：std.test 模块的纯函数                                 │
│      assert_eq / assert_ne / assert_ok / assert_err             │
│      零新语法，纯函数调用                                         │
└─────────────────────────────────────────────────────────────────┘
```

### 使用方式示例

```yaoxiang
# 测试文件：math_test.yx
use std.test
use std.math

# @test 注解标记测试函数（@ 是现有 Token，非新语法）
@test
fn test_add() -> Void = {
    test.assert_eq(2 + 3, 5)
    test.assert_eq(-1 + 1, 0)
    test.assert_eq(0 + 0, 0)
}

@test
fn test_subtract() -> Void = {
    test.assert_eq(10 - 3, 7)
    test.assert_ne(10 - 3, 8)
}

@test
fn test_divide_by_zero() -> Void = {
    test.assert_err(1 / 0)
}
```

```bash
# 运行所有测试
yaoxiang test

# 运行单个文件
yaoxiang test math_test.yx

# 运行匹配名称的测试
yaoxiang test --filter "add"
```

### 语法变化

**无。** 本提案不引入任何新的语法结构。

| 项目 | 状态 |
|------|------|
| 新关键字 | ❌ 无 |
| 新保留字 | ❌ 无 |
| 新语法结构 | ❌ 无 |
| @test 注解 | ✅ 复用现有 `@` Token |
| std.test 模块 | ✅ 新增模块，纯函数 |

## 详细设计

### 1. 测试发现机制

#### 1.1 @test 注解

`@test` 使用 YaoXiang 已有的 `@` 注解语法。注解机制在 parser 层已支持（RFC-008 中 `@block`/`@eager` 注解的使用证明了该机制可用）。

**注解格式**：

```yaoxiang
@test
fn test_name() -> Void = { ... }
```

**规则**：
- `@test` 紧跟在函数定义之前
- 被注解的函数必须是 `() -> Void` 签名
- 注解只标记，不改变函数行为——`@test` 函数在普通 `run` 下仍是可调用的普通函数（但不会被自动调用）
- 测试发现仅在 `yaoxiang test` 子命令下生效

#### 1.2 测试发现流程

```
yaoxiang test
    │
    ▼
┌─────────────────────┐
│ 扫描所有 .yx 文件    │  ← 默认扫描 src/ 和 tests/ 目录
│ 收集 @test 注解函数  │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ 过滤：               │  ← --filter 参数匹配函数名
│ --filter 模式匹配    │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ 编译：每个测试文件    │  ← 复用已有编译器，测试标记嵌入字节码
│ 注入测试注册代码      │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ 执行：并行运行测试    │  ← 复用已有 spawn 并发模型
│ 收集结果             │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ 报告：通过/失败/耗时  │
│ JUnit XML（可选）    │
└─────────────────────┘
```

### 2. std.test 模块设计

#### 2.1 模块结构

```
src/std/test.rs
    ├── assert_eq(actual, expected)     → Void
    ├── assert_ne(actual, expected)     → Void
    ├── assert_ok(value)                → Void
    ├── assert_err(value)               → Void
    ├── assert_true(cond)               → Void
    ├── assert_false(cond)              → Void
    ├── assert_passes()                 → Void
    └── assert_fails()                  → Void（显式失败）
```

#### 2.2 断言函数规范

所有断言函数在失败时抛出 `ExecutorError::TestAssertionFailed`，包含：
- 文件路径（从注解函数的源码位置获取）
- 行号
- 失败信息（actual vs expected）
- 测试函数名

```yaoxiang
# std.test.assert_eq 实现伪代码
fn assert_eq(actual, expected) -> Void = {
    if actual == expected {
        return
    }
    raise TestAssertionFailed(
        "assertion failed: {actual} != {expected}",
        file, line, test_name
    )
}
```

#### 2.3 模块注册

通过现有 `StdModule` trait 注册，与 `std.io` / `std.math` 等模块完全一致：

```rust
impl StdModule for TestModule {
    fn module_path(&self) -> &str { "std.test" }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            NativeExport::new("assert_eq",  "std.test.assert_eq",  "(a: T, b: T) -> Void", native_assert_eq),
            NativeExport::new("assert_ne",  "std.test.assert_ne",  "(a: T, b: T) -> Void", native_assert_ne),
            NativeExport::new("assert_ok",  "std.test.assert_ok",  "(r: Result(T)) -> Void", native_assert_ok),
            NativeExport::new("assert_err", "std.test.assert_err", "(r: Result(T)) -> Void", native_assert_err),
            NativeExport::new("assert_true", "std.test.assert_true", "(b: Bool) -> Void", native_assert_true),
            NativeExport::new("assert_false","std.test.assert_false","(b: Bool) -> Void", native_assert_false),
            NativeExport::new("fail",       "std.test.fail",       "(msg: String) -> Void", native_fail),
        ]
    }
}
```

**关键**：`assert_eq` / `assert_ne` 的 `T` 需要泛型支持。当前编译器对泛型函数的运行时分发已有基础设施（RFC-011）。如果泛型分发尚未完备，第一版可限制为 `Int` / `Float` / `String` / `Bool` 的显式重载，后续升级。

### 3. yaoxiang test 子命令

#### 3.1 CLI 参数

在 `Commands` 枚举中添加：

```rust
/// Run YaoXiang tests
Test {
    /// Source files or directories to test (default: all .yx files in src/ and tests/)
    #[arg(value_name = "PATH", num_args = 0..)]
    paths: Vec<PathBuf>,

    /// Filter test names (substring match)
    #[arg(short, long, value_name = "PATTERN")]
    filter: Option<String>,

    /// Run tests serially (disable parallel execution)
    #[arg(long)]
    serial: bool,

    /// Number of parallel workers (0 = auto)
    #[arg(long, default_value = "0")]
    workers: usize,

    /// Output JUnit XML report
    #[arg(long, value_name = "FILE")]
    junit: Option<PathBuf>,

    /// Stop on first failure
    #[arg(long)]
    fail_fast: bool,
}
```

#### 3.2 执行流程

```rust
match command {
    Commands::Test { paths, filter, serial, workers, junit, fail_fast } => {
        // 1. 发现测试文件
        let test_files = discover_test_files(&paths);

        // 2. 扫描 @test 注解，收集测试函数
        let test_cases = scan_test_functions(&test_files, &filter);

        // 3. 编译每个测试文件（复用 compiler.compile）
        let compiled = compile_tests(&test_cases);

        // 4. 并行执行测试（复用 runtime spawn）
        let results = execute_tests(&compiled, serial, workers);

        // 5. 生成报告
        print_test_report(&results, junit);
    }
}
```

#### 3.3 测试隔离

每个测试函数在独立的运行时上下文中执行：
- 独立的 Heap（列表/字典不会被测试间共享污染）
- 独立的寄存器文件
- 独立的 FFI Registry 快照（标准库状态不被测试间污染）

实现方式：每次 `execute_function` 时创建新的 `Frame` + `Heap`，复用已有的隔离机制（见 `src/backends/interpreter/executor/tests/execute.rs`）。

### 4. 字节码层面改动

#### 4.1 测试元数据

编译器在扫描 `@test` 注解时，在生成的字节码文件头部追加测试元数据段：

```
[Bytecode Header]
    ...
[Test Metadata Section]
    test_count: u16
    tests: [
        { name: str, file: str, line: u32, param_count: u8 }
    ]
```

#### 4.2 执行器改动

`Executor` 新增测试执行入口：

```rust
impl Executor<'_> {
    /// 执行标记为测试的函数
    fn run_test(&mut self, test: &TestMeta) -> TestResult {
        let start = Instant::now();
        match self.execute_function(&self.func_by_name(&test.name), &[]) {
            Ok(RuntimeValue::Unit) => TestResult { name: test.name.clone(), passed: true, elapsed, error: None },
            Ok(_) => TestResult { name: test.name.clone(), passed: false, elapsed, error: Some("test returned non-Void".to_string()) },
            Err(e) if is_assertion_error(&e) => TestResult { name: test.name.clone(), passed: false, elapsed, error: Some(e.to_string()) },
            Err(e) => TestResult { name: test.name.clone(), passed: false, elapsed, error: Some(format!("unexpected: {}", e)) },
        }
    }
}
```

### 5. 输出格式

#### 5.1 默认输出

```
Running 5 tests...

PASS test_add (0.002s)
PASS test_subtract (0.001s)
FAIL test_divide_by_zero (0.003s)
  └── assertion failed: 1 / 0
      Expected: Error
      Actual:   ExecutorError: division by zero
PASS test_max_value (0.001s)
PASS test_min_value (0.001s)

Results: 4 passed, 1 failed, 0 skipped (0.007s)
```

#### 5.2 JUnit XML

```xml
<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="math_test" tests="5" failures="1" time="0.007">
    <testcase name="test_add" classname="math_test" time="0.002"/>
    <testcase name="test_divide_by_zero" classname="math_test" time="0.003">
      <failure message="assertion failed: 1 / 0">ExecutorError: division by zero</failure>
    </testcase>
  </testsuite>
</testsuites>
```

## 类型系统影响

**无。** 测试框架不引入新类型、类型约束或类型系统变更。所有断言函数返回 `Void`，参数使用现有的泛型基础设施。

## 运行时行为

- `@test` 注解的函数在 `yaoxiang run` 下行为不变——它们仍是普通函数
- 仅 `yaoxiang test` 子命令触发测试发现和自动调用
- 测试失败抛出的 `TestAssertionFailed` 与普通 `ExecutorError` 共享同一错误处理管道

## 编译器改动

| 模块 | 改动 |
|------|------|
| `src/frontend/core/parser/` | 复用已有注解解析（`@` Token），无需改动 |
| `src/middle/core/ir_gen.rs` | 扫描 `@test` 注解，标记 AST 节点 |
| `src/middle/passes/codegen/` | 在字节码头部写入测试元数据段 |
| `src/backends/interpreter/executor/` | 新增 `run_test` 入口 |
| `src/std/test.rs` | 新增 `std.test` 模块实现 |
| `src/main.rs` | 新增 `Test` 子命令分支 |

## 向后兼容性

- ✅ **完全向后兼容**：新增 `yaoxiang test` 子命令，不改变任何现有子命令行为
- ✅ **现有代码不受影响**：`@test` 注解是可选的，未注解的函数不受任何影响
- ✅ **std 模块注册独立**：`std.test` 作为新的 `StdModule` 注册，不影响现有模块

## 权衡

### 优点

1. **零新语法**：完全符合 YaoXiang 的 17 关键字约束
2. **利用现有机制**：`@` 注解、`StdModule` trait、`NativeExport`、spawn 并发模型全部复用
3. **与 Rust 测试体验对齐**：`#[test]` → `@test`，`assert_eq!` → `test.assert_eq()`
4. **可扩展**：后续可添加 `@benchmark`、`@ignore` 等注解，无需语法变更

### 缺点

1. `assert_eq(a, b)` 是函数调用，不是宏——不能像 Rust 那样打印 `a = 42, b = 43` 的精确值（需要在 native 实现中捕获值）
2. 泛型断言（`assert_eq<T>`）依赖泛型运行时分发，如果该能力未完备，第一版需做类型特化
3. 注解扫描在编译期进行，无法动态注册测试

## 替代方案

| 方案 | 描述 | 优点 | 缺点 |
|------|------|------|------|
| **A: 本方案（@test + std.test 模块）** | 注解 + 纯函数 | 零新语法，利用现有机制 | 函数调用而非宏，诊断信息略弱 |
| **B: 测试块语法 `test { }`** | 引入新关键字 `test` | 语法简洁 | ❌ 违反 17 关键字约束，需 parser 大改 |
| **C: 约定命名（`test_*` 前缀）** | 纯命名约定，扫描 `test_` 前缀函数 | 零注解改动 | 命名空间污染，易误匹配，无显式标记 |
| **D: 外部测试运行器** | 独立的 Rust 工具编译并执行 .yx 测试 | 不改动编译器 | 架构分裂，两套编译流程 |

**选择 A**：在满足零新语法约束的前提下，注解方案是最明确的标记方式，同时命名空间污染最小。

## 实现策略

### Phase 1：基础设施（v0.7.8）

- `std.test` 模块：`assert_eq` / `assert_ne` / `assert_ok` / `assert_err` / `fail`
- 初始版本限制为 `Int` / `Float` / `String` / `Bool` 特化
- 注解解析确认（确保 `@` 在 parser 层正确传递给 AST）

### Phase 2：测试发现与执行（v0.7.8）

- IR 层扫描 `@test` 注解并标记
- 字节码测试元数据段
- `yaoxiang test` 子命令骨架
- 基础测试执行 + 默认输出

### Phase 3：完善（v0.7.9）

- `assert_true` / `assert_false`
- `--filter` / `--serial` / `--fail-fast` 参数
- JUnit XML 输出
- 测试隔离（独立 Heap 上下文）
- 泛型断言支持（如果泛型运行时分发已完备）

### 依赖关系

| 依赖 | 状态 | 说明 |
|------|------|------|
| `@` 注解解析 | ✅ 已有 | lexer 已有 `TokenKind::At`，parser 需确认传递 |
| `StdModule` trait | ✅ 已有 | `src/std/mod.rs` 已有完整实现 |
| `NativeExport` | ✅ 已有 | 标准库函数注册机制 |
| 泛型运行时分发 | ⚠️ 部分实现 | 影响 `assert_eq<T>` 泛型版本，Phase 1 可用类型特化绕过 |
| `yaoxiang` CLI (clap) | ✅ 已有 | `Commands` 枚举可直接扩展 |
| 并发执行模型 | ⚠️ 草案 | RFC-024 已接受但实现中，Phase 2 可用单线程实现 |

### 风险

1. **注解解析不完整**：如果 parser 层对 `@test` 的处理不完整（只识别了 `@block`/`@eager`），需要扩展注解解析。**缓解**：注解本质上是 `@` + 标识符，扩展为通用注解解析工作量小
2. **泛型分发不成熟**：`assert_eq<T>` 需要泛型运行时支持。**缓解**：Phase 1 用 `Int` / `Float` / `String` / `Bool` 四个显式函数，不依赖泛型分发
3. **测试隔离不够**：如果 `std` 模块的全局状态被测试间污染。**缓解**：每次测试执行创建新的 `NativeContext` + `Heap`

## 开放问题

- [ ] `@test` 注解是否支持参数（如 `@test(reason = "flaky")`）？目前暂不支持，保持简单
- [ ] 测试模块命名规范？建议约定：测试文件与源文件同名加 `_test` 后缀，如 `math.rs` → `math_test.yx`
- [ ] 是否支持 `@ignore` 注解跳过特定测试？Phase 1 不做，Phase 3 可考虑
- [ ] 断言诊断信息格式？需确定 `actual` / `expected` 的展示方式（当前 RuntimeValue 已支持 `format_value_with_prefix`）

## 附录A：与已有测试基础设施的关系

| 项目 | 现有 | 本 RFC | 关系 |
|------|------|--------|------|
| Rust `#[test]` | `src/**/tests/` 目录 | 不动 | 编译器内部测试继续用 Rust |
| YaoXiang `.yx` 集成测试 | `tests/yaoxiang/` | 不动 | 现有 .yx 回归测试文件继续作为集成测试 |
| `std.assert.assert(cond)` | 已有 | 不动 | 保留，供普通代码使用 |
| `TestAssertionFailed` 错误 | 无 | 新增 | 新增错误类型，与现有错误代码规范（RFC-013）对齐 |

## 附录B：设计决策记录

| 决策 | 决定 | 日期 | 理由 |
|------|------|------|------|
| 测试标记方式 | `@test` 注解 | 2026-07-25 | 零新语法，@ 已存在 |
| 断言方式 | `std.test` 模块函数 | 2026-07-25 | 纯函数，无宏，无语法变更 |
| 测试执行模型 | 独立 Heap 上下文 | 2026-07-25 | 避免测试间状态污染 |
| 泛型断言 | Phase 1 类型特化，Phase 3 泛型 | 2026-07-25 | 绕过泛型分发未完成的风险 |

## 参考文献

- [RFC-008: Runtime 并发模型](../accepted/008-runtime-concurrency-model.md) — `@` 注解机制参考
- [RFC-013: 错误代码规范](../accepted/013-error-code-specification.md) — `TestAssertionFailed` 错误编码
- [RFC-030: assert 断言机制](../review/030-assert-mechanism.md) — 现有 `assert(cond)` 的运行时实现
- [RFC-011: 泛型系统](../accepted/011-generic-type-system.md) — 泛型断言的类型约束
- [Rust `#[test]` 机制](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考设计
- [Go `testing` 包](https://pkg.go.dev/testing) — 参考设计
