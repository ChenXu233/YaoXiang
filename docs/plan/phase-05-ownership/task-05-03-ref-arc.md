# Task 5.3: ref 关键字（Arc 引用计数）

> **优先级**: P0
> **状态**: ✅ 已完成
> **模块**: `src/middle/lifetime/`

## 功能描述

`ref` 关键字创建 Arc（原子引用计数），用于安全共享所有权：

- **`ref` = Arc**：原子引用计数，线程安全
- **自动 Send + Sync**：Arc 自动满足并发约束
- **跨 spawn 安全**：可安全捕获到闭包中

> **RFC-009 v7 核心设计**：`ref` 替代借用检查器，通过 Arc 实现安全共享。

## 设计原则

**编译器职责**（不做什么）：
- ❌ 不维护引用计数（运行时负责）
- ❌ 不做原子操作（运行时负责）

**编译器职责**（做什么）：
- ✅ 解析 `ref` 表达式
- ✅ 类型推断（推断为 `Arc<T>`）
- ✅ 生成 IR 指令（`ArcNew`, `ArcClone`, `ArcDrop`）
- ✅ 所有权检查（`RefNonOwner`）

**运行时职责**：
- ✅ 原子计数增减
- ✅ 计数归零时释放内存

## ref 规则

### ref 创建 Arc

```yaoxiang
# ref 创建 Arc（原子引用计数）
p: Point = Point(1.0, 2.0)
shared = ref p    # p 的引用计数 = 1

# 多个共享引用
shared2 = ref p   # p 的引用计数 = 2
shared3 = ref p   # p 的引用计数 = 3

# 当所有 Arc 释放时，值自动释放
# shared, shared2, shared3 释放后，p 自动释放
```

### 跨 spawn 边界安全

```yaoxiang
# ✅ ref 可安全跨 spawn 边界
p: Point = Point(1.0, 2.0)
shared = ref p    # Arc，线程安全

spawn(() => {
    print(shared.x)   # ✅ 安全访问
})
# spawn 自动检查 Send 约束

# ✅ 多个任务共享
task1 = spawn(() => print(shared.x))
task2 = spawn(() => print(shared.y))

# 两个任务都通过 Arc 安全访问同一值
```

### ref 与 Move 对比

```yaoxiang
# Move：值转移
data: List[Int] = [1, 2, 3]
new_owner = data    # data 不再可用

# ref：共享访问（Arc）
data: List[Int] = [1, 2, 3]
shared = ref data   # data 和 shared 都可用

# 原值仍可访问
print(data.length)  # ✅
print(shared.length) # ✅

# Arc 引用计数
# shared 释放时计数减少
# 计数归零时 data 自动释放
```

## IR 指令设计

```rust
// ArcNew: 创建 Arc
ArcNew { dst: Operand, src: Operand }

// ArcClone: 克隆 Arc（引用计数+1）
ArcClone { dst: Operand, src: Operand }

// ArcDrop: 释放 Arc（引用计数-1）
ArcDrop(Operand)
```

## 错误类型

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipError {
    // ... 已有错误 ...

    /// ref 应用于非所有者
    RefNonOwner {
        ref_span: Span,
        target_span: Span,
        target_value: String,
    },
}
```

## 实现步骤

| 步骤 | 文件 | 说明 |
|------|------|------|
| 1 | `src/frontend/parser/ast.rs` | 添加 `Expr::Ref` 变体 |
| 2 | `src/frontend/parser/nud.rs` | 添加 `ref` 前缀解析 |
| 3 | `src/middle/ir.rs` | 添加 `ArcNew`, `ArcClone`, `ArcDrop` 指令 |
| 4 | `src/middle/lifetime/error.rs` | 添加 `RefNonOwner` 错误 |
| 5 | `src/middle/lifetime/mod.rs` | 添加 `RefChecker` |
| 6 | `src/middle/codegen/expr.rs` | 生成 Arc 指令 |
| 7 | `src/vm/instructions.rs` | 添加运行时支持 |
| 8 | 测试 | 验收测试 |

## 与 RFC-009 v7 对照

| RFC-009 v7 设计 | 实现状态 |
|----------------|---------|
| ref 关键字创建 Arc | ✅ 已实现 |
| Arc 自动 Send + Sync | ✅ 隐式满足 |
| 跨 spawn 安全捕获 | ✅ 类型系统保证 |
| 引用计数管理 | ✅ 运行时 |
| 跨任务循环检测 | ❌ 见 task-05-06 |

## 验收测试

```yaoxiang
# test_ref.yx

# === ref 创建 Arc ===
p: Point = Point(1.0, 2.0)
shared = ref p
assert(p.x == 1.0)     # ✅ 原值仍可用
assert(shared.x == 1.0) # ✅ Arc 可访问

# === 多个 ref ===
shared2 = ref p
shared3 = ref p
# 引用计数 = 3

# === 跨 spawn 安全 ===
p: Point = Point(1.0, 2.0)
shared = ref p

task1 = spawn(() => {
    print(shared.x)   # ✅ 安全
})

task2 = spawn(() => {
    print(shared.y)   # ✅ 安全
})

# === ref 计数归零释放 ===
p: Point = Point(1.0, 2.0)
shared = ref p
# shared 释放后，p 可被释放

print("ref (Arc) tests passed!")
```

## 相关文件

- **src/frontend/parser/ast.rs**: 添加 `Expr::Ref`
- **src/frontend/parser/nud.rs**: `ref` 解析
- **src/frontend/typecheck/types.rs**: `MonoType::Arc`
- **src/frontend/typecheck/infer.rs**: `infer_ref` 类型推断
- **src/middle/ir.rs**: `ArcNew`, `ArcClone`, `ArcDrop` 指令
- **src/middle/lifetime/error.rs**: `RefNonOwner` 错误
- **src/middle/lifetime/ref_semantics.rs**: `RefChecker` 所有权检查
- **src/middle/codegen/expr.rs**: `generate_ref` 代码生成
- **src/vm/opcode.rs**: `ArcNew(0x79)`, `ArcClone(0x7A)`, `ArcDrop(0x7B)`
- **src/vm/executor.rs**: `ArcValue` 运行时实现

## 测试覆盖

- **src/frontend/parser/tests/ref_test.rs**: Parser 测试 (5个)
- **src/frontend/typecheck/tests/ref_test.rs**: TypeCheck 测试 (6个)
- **src/middle/codegen/tests/ref_test.rs**: Codegen 测试 (5个)
- **src/middle/lifetime/tests/ref_semantics.rs**: Lifetime 测试 (5个)
- **src/vm/tests/arc.rs**: VM 测试 (9个)
