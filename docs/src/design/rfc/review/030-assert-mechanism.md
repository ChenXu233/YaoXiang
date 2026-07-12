---
title: "RFC-030: assert 断言机制"
status: "审核中"
author: "晨煦"
created: "2026-06-15"
updated: "2026-07-11"
decision: "Assert 与 assert 编译期断言与运行时断言。"
issue: "#97"
issues_impl:
  - "#155"
---

# RFC-030: assert 断言机制

## 摘要

为 YaoXiang 引入 `assert` 断言机制，用于测试和前置条件检查。`assert` 是 YaoXiang 唯一的运行时 panic 机制——不需要单独的 `raise`/`throw` 关键字。编译期条件类型 `Assert(C)` 是另一个独立功能（见 RFC-011 §4.3），放在 `std.assert` 同一模块下，但语义和实现完全分开。

## 动机

### 为什么需要这个特性？

当前 YaoXiang 的 E2E 测试只能通过 `if` + `io.println` + `return` 模拟断言：

```yaoxiang
val = some_func()
if val != 42 {
    io.println("FAIL: expected 42")
    return
}
```

这种写法存在三个问题：

1. **样板代码多**：每个断言需要 4 行，测试文件膨胀
2. **错误信息弱**：手动拼接字符串，缺少源码位置
3. **不可组合**：无法批量注册断言、无法作为参数传递给测试框架

### 当前的问题

- 没有统一的断言机制
- 测试代码充斥 `if` + 打印 + `return` 的模式
- 字节码层已有 `Throw` 指令但语言层未暴露
- RFC-011 定义了编译期 `Assert(C)` 条件类型，但运行时 `assert()` 尚未实现

### 设计原则

`assert` 是 YaoXiang 唯一的用户态 panic 机制。`assert(false, "msg")` 等同于 `raise`，不需要单独的 `throw`/`raise` 关键字。`assert` 函数本身就是 `if raise` 的最佳封装。

**不引入新关键字，不引入新语法。一切皆函数调用。**

## 方案 A：native 函数

以 native 函数实现 `assert`，不引入新关键字。

```yaoxiang
use std.assert.assert

main = {
    assert(1 + 1 == 2, "math is broken")
    assert(get_name() == "YaoXiang", "name mismatch")
}
```

### 重载签名

`assert` 有两个重载：

```
// 重载 1：条件断言
assert(cond: Bool, ?msg: String | Error)

// 重载 2：Result 断言
assert(cond: Result)
```

#### 重载 1：条件断言 `(Bool, ?String | Error)`

`Bool` + 可选消息。消息可以是 `String` 或 `Error` 值：

```yaoxiang
assert(1 + 1 == 2)                    // 无消息，默认 panic 信息
assert(1 + 1 == 2, "math is broken")   // 字符串消息
assert(x > 0, my_error)                // 直接抛 Error 值
```

`assert(false, "msg")` 是 YaoXiang 的 `raise`/`throw` 等价体——不需要单独关键字。

#### 重载 2：Result 断言 `(Result)`

单个 `Result` 参数，自动检查是否 `Err`：

### 优点

- **零语法变更**：纯函数，不需新关键字
- **零新概念**：复用现有 native 函数注册机制
- **高可扩展性**：函数重载自然支持多签名
- **自文档**：`std.assert` 命名空间本身就是文档

### 缺点

- 无。assert 的类型签名正确时，编译器可通过函数可达性分析推断死代码。无需额外 pass。

### 运行时行为

1. 评估第一个参数 `condition: Bool`
2. 若为 `true`，返回 `Unit`
3. 若为 `false`，触发运行时 panic：
   - 输出 `message` 内容（如果有）
   - 输出调用栈（debug 模式下）
   - 终止当前执行

#### 各重载的失败行为

| 签名 | 失败时行为 |
|------|-----------|
| `assert(false)` | 默认 panic 信息 |
| `assert(false, "msg")` | 输出字符串消息后 panic |
| `assert(false, error_val)` | 抛 Error 值 |
| `assert(Err(x))` | 提取 Err 内容并 panic |

### 与编译期 Assert 的关系

`std.assert` 模块包含两个完全独立的功能：

| 功能 | 类型 | 命名 | 时机 | 失败行为 | 运行时开销 |
|------|------|------|------|----------|-----------|
| 运行时断言 | 函数 | `assert()` | 运行时 | panic + 调用栈 | 始终存在 |
| 编译期条件类型 | 类型 | `Assert(C)` | 编译期 | 编译报错 | 零 |

```yaoxiang
use std.assert

// 运行时 assert：测试用，检查运行时的值
assert.assert(result == 42, "expected 42")

// 编译期 Assert：精化类型，验证泛型参数
length: assert.Assert(N > 0)
```

两者放在同一模块是因为都是"断言"概念的不同表现，但实现完全独立。`assert()` 不需要 `Assert(C)`，反之亦然。

### 编译器改动

**无需改动 parser、AST、typecheck、IR gen。**

仅需在 `src/std/` 下添加 native 函数注册：

1. 新增 `src/std/assert.rs`
2. 注册 `std.assert.assert` 和 `std.assert.Assert`（后者是编译期条件类型，见 #155）
3. 内部调用已有的 `BytecodeInstr::Throw` 指令

### 优点

- **零语法变更**：纯函数，不需新关键字
- **零新概念**：复用现有 native 函数注册机制
- **高可扩展性**：函数签名可扩展至 `assert_eq` 等变体（未来）
- **自文档**：`std.assert` 命名空间本身就是文档

### 缺点

- 编译期不可知：与方案 B（关键字）不同，无法在编译期做死代码消除
- debug 模式下才能获取调用栈

## 方案 B：内置关键字

> 已弃用，仅作历史记录。

以 `assert` 关键字实现，编译期可获取源码位置和常量折叠。

```yaoxiang
assert(1 + 1 == 2, "math is broken")
```

### 类型签名

无独立类型签名——关键字由 parser 处理。

### 运行时行为

同方案 A。

### 编译器改动

需要改动 parser、AST、typecheck、IR gen：
1. parser：新增 `Expr::Assert` 变体
2. AST：新增 `Expr::Assert` 节点
3. typecheck：校验参数类型
4. IR gen：生成 `BytecodeInstr::Throw`

### 优点

- 编译期知源码位置（不依赖 debug info）
- 编译期可常量折叠：`assert(true)` → 空操作，`assert(false)` → 编译错误

### 缺点

| 缺点 | 影响 |
|------|------|
| 需要解析器改动 | 引入新语法节点，增加维护成本 |
| 关键字不可扩展 | `assert_eq` 等变体仍需函数 |
| 编译期优势不实际 | 见下文分析 |

### 对比

| 维度 | 方案 A（函数） | 方案 B（关键字） |
|------|---------------|-----------------|
| 实现成本 | ~20 行 | parser + AST + typecheck + IR gen |
| 语法变更 | 无 | 新关键字 |
| 可扩展性 | 函数重载 | 需要配套宏 |
| 源码位置 | debug info | 编译期可得 |
| 常量折叠 | 需 pass 支持 | 编译期可得 |
| 运行时开销 | 函数调用 | 极小 |

### 编译期分析的现实约束

方案 B 的核心优势——编译期分析——需要 **常量折叠 pass** 才能生效。即编译器需要在编译期求值 `assert(false)` 中的 `false`，才能知道这是死代码。

YaoXiang 当前没有常量折叠 pass。即使采用方案 B，`assert(x > 0)` 这类常见写法在编译期仍然无法分析。只有 `assert(true)` / `assert(false)` 这种字面量才能被分析。

因此方案 B 的编译期优势**在当前阶段是理论性的，不是实际的**。

---

## 开放问题

- [x] ~~选择方案 A 还是方案 B？~~ → **方案 A：native 函数**
- [x] ~~`assert` 是否需要支持不带 `message` 的简化形式 `assert(cond)` ？~~ → **支持。`assert(cond, ?msg)`，message 可选。**
- [x] ~~是否需要 `assert_eq`、`assert_ne` 等变体？~~ → **不需要。YAGNI。等测试框架成型再说。**
- [x] ~~panic 输出是否包含源码位置？~~ → 方案 A 依赖 debug info（调用栈）。
- [ ] ~~assert / Assert 统一问题~~ → **已拆出为独立讨论 #156**。理想状态是 `assert: (cond: Bool) -> Assert(cond)` 让两者统一，但 `Assert(C)` 作为精化类型要求 `C` 编译期可知，与运行时断言冲突。详见 [#156](https://github.com/ChenXu233/YaoXiang/issues/156)。
---

## 附录 A：设计讨论记录

### 2026-07-05：选择方案 A

方案 A 的 20 行实现在价值和成本上胜出。方案 B 的编译期优势在无常量折叠 pass 的当下是理论性的。

### 2026-07-11：assert 重载设计

**问题**：为什么 `assert` 需要两个重载，而不是统一的 `(Bool, ?String)`？

**解答**：

运行时 `assert()` 是 YaoXiang 唯一的用户态 panic 机制。`assert(false, "msg")` 等价于其他语言的 `raise`/`throw`。因此它需要覆盖三种场景：
1. 条件 + 简单消息：`assert(cond, "msg")`
2. 条件 + 自定义 Error：`assert(cond, my_error)`
3. Result 检查：`assert(result)` — 最简洁的 `if is_err { panic }`

Result 重载的合理性在于：这是错误传播最短的路径——"Result 应当为 Ok，否则死"。不需要先 `.is_ok()` 再单独处理错误。

### 2026-07-11：std.assert 模块结构

`std.assert` 包含两个功能：

- `assert()`：运行时断言函数（本 RFC）
- `Assert(C)`：编译期条件类型（RFC-011 §4.3，#155）

两者放在同一模块，因为都是"断言"概念的不同表现。作为命名空间，`std.assert` 自文档——开发者看到 `use std.assert` 就知道这里引入了断言相关功能。

## 附录 B：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| 选择方案 A 还是方案 B | **方案 A：native 函数** — 20 行实现，一等公民，零语法变更。方案 B 编译期优势在当前阶段是理论性的 | 2026-07-03 | 晨煦 |
| message 是否可选 | **是**：`assert(cond, ?msg)`，String 或 Error | 2026-07-11 | 晨煦 |
| 是否需要 assert_eq 等变体 | **不需要**。YAGNI，等测试框架再说 | 2026-07-11 | 晨煦 |
| 是否需要单独的 raise/throw 关键字 | **不需要**。`assert(false, msg)` 等价于 raise | 2026-07-11 | 晨煦 |
| assert 和 Assert 的关系 | **完全独立**。运行时函数 vs 编译期条件类型，同一模块不同概念 | 2026-07-11 | 晨煦 |

## 参考文献

- [RFC-007: 函数定义语法统一方案](007-function-syntax-unification.md) — `name: type = value` 模型
- [RFC-010: 统一类型语法](010-unified-type-syntax.md) — 类型系统基础
- [RFC-011: 泛型系统设计 §4.3](../accepted/011-generic-type-system.md) — 编译期验证与 `Assert(C)` 条件类型
- [RFC-026: FFI 核心机制](../review/026-ffi-core-mechanism.md) — native 函数注册机制
- [RFC-027: 编译期谓词与统一静态验证](../accepted/027-compile-time-evaluation-types.md) — 编译期求值系统