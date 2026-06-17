---
title: "RFC-030: assert 断言机制"
status: "草案"
author: "晨煦"
created: "2026-06-15"
---

# RFC-030: assert 断言机制

## 摘要

为 YaoXiang 引入 `assert` 断言机制，用于测试和前置条件检查。本 RFC 提出两种实现方案——**native 函数**与**关键字**——分析各自权衡，供决策。

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

## 方案 A：native 函数

以 native 函数实现 `assert`，不引入新关键字。

```yaoxiang
use std.assert

main = {
    assert(1 + 1 == 2, "math is broken")
    assert(get_name() == "YaoXiang", "name mismatch")
}
```

### 类型签名

```
assert: (Bool, String) -> ()
```

### 运行时行为

1. 评估第一个参数 `condition: Bool`
2. 若为 `true`，返回 `Unit`
3. 若为 `false`，触发运行时 panic：
   - 输出 `message` 内容
   - 输出调用栈（debug 模式下）
   - 终止当前执行

### 编译器改动

**无需改动 parser、AST、typecheck、IR gen。**

仅需在 `src/std/` 下添加 native 函数注册：

1. 新增 `src/std/assert.rs`
2. 注册 `std.assert` native 函数
3. 内部调用已有的运行时 panic 机制

### 优点

- **零语法变更**：不需要改 parser、AST、IR、codegen
- **实现简单**：约 20 行 native 函数注册代码
- **一等公民**：可以作为参数传递、赋值给变量、用于高阶函数
- **渐进增强**：未来可在不破坏代码的前提下添加编译期内联优化

### 缺点

- **无编译期分析**：编译器无法对 `assert(false)` 后的代码做死代码消除
- **错误信息受限**：源码位置需通过运行时调用栈获取，不如关键字精确
- **可被遮蔽**：用户可以定义同名函数覆盖 `assert`
- **需要 import**：必须写 `use std.assert` 才能使用

---

## 方案 B：关键字

引入 `assert` 作为语言关键字，编译器在 IR 层特殊处理。

```yaoxiang
main = {
    assert(1 + 1 == 2, "math is broken")
    assert(get_name() == "YaoXiang", "name mismatch")
}
```

### 语法

```
assert '(' Expr ',' StringLit ')'
```

`assert` 是关键字，不需要 `use` 导入。

### 编译器改动

需要改动全链路：

| 阶段 | 改动 |
|------|------|
| Lexer | 添加 `KwAssert` token |
| Parser | 添加 `assert(expr, "msg")` 语法 |
| AST | 添加 `StmtKind::Assert { condition, message }` |
| IR Gen | 将 Assert 编译为条件跳转 + Throw |
| Codegen | 生成 `JmpIfNot` + `Throw` 指令序列 |

### 优点

- **内置可用**：不需要 import，任何位置直接使用
- **精确的源码位置**：编译器在 AST 层就知道 assert 的位置，错误信息精确到行号列号
- **不可遮蔽**：关键字不能被用户代码覆盖
- **未来可扩展**：可以做编译期分析（配合常量折叠 pass）
- **语义明确**：`assert` 在语法层就有明确语义，不会被误用

### 缺点

- **实现成本高**：需要改动 parser、AST、IR、codegen 全链路
- **不是一等公民**：不能作为参数传递、不能赋值给变量
- **关键字膨胀**：每加一个关键字都增加语言复杂度
- **可扩展性差**：新增签名变体（如 `assert_eq`）需要改语法

---

## 对比

| 维度 | 方案 A：函数 | 方案 B：关键字 |
|------|-------------|---------------|
| 实现成本 | 低（~20 行） | 高（~200 行，4 个模块） |
| 语法变更 | 无 | 有（新增关键字 + 语法） |
| 需要 import | 是 | 否 |
| 一等公民 | 是 | 否 |
| 源码位置精确度 | 运行时调用栈 | 编译期精确到行列 |
| 不可遮蔽 | 否 | 是 |
| 编译期分析 | 不可（需要常量折叠） | 可（配合常量折叠） |
| 可扩展性 | 高（重载、高阶函数） | 低（需改语法） |

### 编译期分析的现实约束

方案 B 的核心优势——编译期分析——需要**常量折叠 pass** 才能生效。即编译器需要在编译期求值 `assert(false)` 中的 `false`，才能知道这是死代码。

YaoXiang 当前没有常量折叠 pass。即使采用方案 B，`assert(x > 0)` 这类常见写法在编译期仍然无法分析。只有 `assert(true)` / `assert(false)` 这种字面量才能被分析。

因此方案 B 的编译期优势**在当前阶段是理论性的，不是实际的**。

---

## 开放问题

- [ ] 选择方案 A 还是方案 B？
- [ ] `assert` 是否需要支持不带 `message` 的简化形式 `assert(cond)` ？
- [ ] 是否需要 `assert_eq`、`assert_ne` 等变体？
- [ ] panic 输出是否包含源码位置？（方案 A 依赖 debug info，方案 B 编译期可得）

---

## 附录 B：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| （待定） | | | |

## 参考文献

- [RFC-007: 函数定义语法统一方案](007-function-syntax-unification.md) — `name: type = value` 模型
- [RFC-010: 统一类型语法](010-unified-type-syntax.md) — 类型系统基础
- [RFC-026: FFI 核心机制](../review/026-ffi-core-mechanism.md) — native 函数注册机制
