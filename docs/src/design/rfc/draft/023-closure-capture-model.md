---
title: RFC-023：闭包捕获模型
---

# RFC-023: 闭包捕获模型

> **状态**: 草案
> **作者**: 晨煦
> **创建日期**: 2026-05-29
> **最后更新**: 2026-05-29

> **参考**:
> - [RFC-007: 函数语法统一](./accepted/007-function-syntax-unification.md)
> - [RFC-009: 所有权模型 v9](./accepted/009-ownership-model.md)
> - [RFC-011: 泛型系统设计](./accepted/011-generic-type-system.md) — 第 2.4 节：Dup/Clone 内置 marker trait

## 摘要

本 RFC 定义 YaoXiang 语言的**闭包捕获模型**。编译器自动分析闭包体引用的外部变量，根据变量类型（Dup/非Dup）和闭包是否逃逸，自动选择捕获方式——Dup 类型直接复制、非 Dup 不逃逸则借用、非 Dup 逃逸则 Move。用户零标注，与函数调用的自动借用选择共用一套规则。

## 动机

### 为什么需要？

当前闭包捕获是**空实现**——`MakeClosure` 指令的 `env` 字段永远是空的，lambda 不能引用任何外部变量。借用令牌系统要求闭包能捕获 `&T` 令牌（零成本复制），这是一个核心使用场景。

### 当前的问题

```yaoxiang
# 这种代码目前无法编译——lambda 不能引用 threshold
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)  # ❌ threshold 无法捕获
}
```

## 提案

### 核心设计

闭包捕获由编译器全自动判断。规则和函数调用的自动借用选择**完全相同**：

```
变量类型    闭包是否逃逸    捕获方式
─────────────────────────────────────────
Dup         任意            复制（比特拷贝或零成本）
非 Dup      不逃逸          自动借用（&T 或 &mut T）
非 Dup      逃逸            Move（所有权转移）
```

**逃逸判定**：

```
spawn { || ... }           → 逃逸
return || ...              → 逃逸
let x = || ... ;  x 存字段 → 逃逸
items.filter(|p| ...)      → 不逃逸（sync 高阶函数调用）
||.method()                → 不逃逸（当场调用）
```

保守原则：无法确定时按逃逸处理。

### 示例

```yaoxiang
# 1. Dup 令牌——直接复制（零成本）
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    # threshold: &Float → Dup → 编译器复制令牌进闭包
    # 零大小令牌，零运行时开销
    items.filter(|p| p.x > threshold)
}

# 2. 非 Dup + 不逃逸——自动借用
process: (buf: Buffer) -> Void = {
    # buf 不 Dup，filter 不逃逸 → 自动创建 &Buffer 令牌
    transform(|b| b.read())
    # 闭包返回后令牌释放，buf 恢复可用
}

# 3. 闭包逃逸——Move
spawn_worker: (data: Data) -> Void = {
    # data 不 Dup，spawn → 逃逸 → Move
    spawn { use(data) }
}

# 4. 混合捕获
complex: (items: List(Point), config: &Config, buf: Buffer) -> List(Point) = {
    # config: &Config → Dup → 复制令牌
    # buf: Buffer → 不 Dup，不逃逸 → &mut Buffer 借用
    items.filter(|p| {
        let threshold = config.get_threshold()
        buf.update(p)
        p.x > threshold
    })
}

# 5. 借用冲突检测
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf 已被闭包借用，此处冲突
}
```

### 语法变化

**零语法变化**。捕获方式由编译器自动决定，用户不需要标注。

## 详细设计

### 类型系统影响

Lambda 的类型签名保持不变：`(params) -> Return`。捕获的变量不体现在类型签名中，由编译器在 IR 生成阶段处理。

### 编译器改动

| 组件 | 改动 | 说明 |
|------|------|------|
| `capture.rs`（新建） | 捕获分析 + 逃逸分析 + 模式选择 | ~150 行 |
| `expressions.rs` | lambda 类型推断调用捕获分析 | ~10 行 |
| `ir_gen.rs` | MakeClosure env 填充；ZST 跳过 | ~80 行 |
| `ir.rs` | MakeClosure env 类型可能需要调整 | ~5 行 |

**捕获分析流程**：

```
1. 遍历 lambda body AST
2. 收集所有 Expr::Var(name) 引用
3. 过滤：只保留闭包外部作用域的变量
4. 分类：Read（只读）/ Write（读写）/ Move（被转移）
5. 查类型属性：是否 Dup
6. 判逃逸：闭包的使用方式
7. 选择捕获模式：
   Dup → Copy
   非Dup + 不逃逸 + Read → Borrow（&T）
   非Dup + 不逃逸 + Write → BorrowMut（&mut T）
   非Dup + 逃逸 → Move
```

**IR 生成**：

```rust
// 当前（空）
Instruction::MakeClosure { dst, func, env: Vec::new() }

// 改为
Instruction::MakeClosure { dst, func, env: captured_env }

// captured_env 的生成逻辑：
for captured in captures {
    match captured.mode {
        Copy if is_zst(captured.ty) => {
            // 零大小类型——不生成任何指令
            // 闭包体直接引用外层（编译期消除）
        }
        Copy => {
            // 生成 Move dst, src（Dup 类型的浅复制）
        }
        Borrow => {
            // 生成 Borrow dst, src（创建 ReadToken）
        }
        BorrowMut => {
            // 生成 Borrow dst, src（创建 WriteToken）
        }
        Move => {
            // 生成 Move dst, src（所有权转移）
        }
    }
}
```

### 运行时行为

捕获方式不影响运行时性能：

- **Dup + ZST**（如 `&T` 令牌）→ 零指令，闭包体直接引用外层变量
- **Dup + 非 ZST**（如 Int）→ 一次寄存器复制
- **Borrow/BorrowMut**→ 创建令牌（编译期概念，零开销）
- **Move** → 和普通 Move 同样的成本

### 向后兼容性

完全兼容。当前所有 lambda 都不能捕获外部变量，本 RFC 只会增加表达能力，不破坏任何现有代码。

## 权衡

### 优点

1. **零标注**：用户不需要写任何捕获标注
2. **和函数调用统一**：捕获规则 = 函数调用自动借用规则
3. **零成本**：Dup 令牌的捕获完全在编译期消除
4. **安全**：逃逸分析防止 use-after-free

### 缺点

1. **逃逸分析保守**：无法确定时按逃逸处理，可能不必要地 Move
2. **隐式**：捕获方式不体现在代码中，调试时需要看编译输出

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| Rust 式显式 `move` 关键字 | 引入新语法，增加用户认知负担 |
| 全部 Move | 无法表达零成本令牌借用 |
| 全部借用 | 闭包逃逸会导致悬垂引用 |
| 用户手动标注捕获方式 | 违背"编译器全自动"的设计哲学 |

## 实现策略

### 阶段划分

1. **Phase 1**：捕获分析（仅识别外部变量引用，不区分捕获方式）
2. **Phase 2**：逃逸分析 + 模式选择
3. **Phase 3**：IR 生成 + ZST 优化
4. **Phase 4**：借用冲突检测集成

### 依赖关系

- 依赖 RFC-011（泛型系统，第 2.4 节 Dup/Clone trait）——需要 Dup trait 判断变量是否可复制
- 依赖 RFC-009 v9（借用令牌）——Borrow/BorrowMut 捕获模式需要令牌类型
- RFC-023 和本 RFC 实现后，借用令牌系统（RFC-009 v9 实现）即可开工

### 风险

- 逃逸分析可能过于保守，导致不必要的 Move；可后续优化
- 泛型闭包的捕获分析可能需要额外处理

## 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| 捕获方式选择 | 全自动 | 和函数调用规则统一 | 2026-05-29 |
| 逃逸分析 | 保守原则 | 无法确定时按逃逸，安全优先 | 2026-05-29 |
| ZST 优化 | IR 生成时跳过 | 比后续优化 pass 更简单 | 2026-05-29 |
| 捕获不体现在类型签名 | 编译器内部处理 | 保持 lambda 类型简洁 | 2026-05-29 |

## 参考文献

### YaoXiang 官方文档

- [RFC-007: 函数语法统一](./accepted/007-function-syntax-unification.md)
- [RFC-009: 所有权模型 v9](./accepted/009-ownership-model.md)
- [RFC-011: 泛型系统设计](./accepted/011-generic-type-system.md) — 第 2.4 节：Dup/Clone 内置 marker trait

### 外部参考

- [Rust 闭包捕获规则](https://doc.rust-lang.org/reference/types/closure.html#capture-modes)
- [Swift 闭包捕获语义](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/closures/#Capturing-Values)
