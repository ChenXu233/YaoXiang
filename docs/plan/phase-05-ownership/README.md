# Phase 5: 所有权系统

> **模块路径**: `src/middle/lifetime/`
> **状态**: ✅ 已实现
> **设计依据**: [RFC-009 所有权模型 v7.1](../../design/accepted/009-ownership-model.md)

## 概述

所有权系统是 YaoXiang 语言的核心安全机制，确保内存安全和并发安全。

> **核心设计决策**：YaoXiang **不实现生命周期标注 `'a`** 和**借用检查器**
> - 默认 Move（值传递），零拷贝
> - 共享用 `ref` 关键字（Arc 引用计数）
> - 副本用 `clone()` 显式复制
> - 循环引用：任务内允许，跨任务检测
> - 系统级用 `unsafe` + `*T` 裸指针（Phase 6）

## 所有权规则

### 1. 默认 Move（零拷贝）

```yaoxiang
# 每个值有一个所有者
data: List[Int] = [1, 2, 3]  # data 是所有者

# 当所有者离开作用域，值被释放（RAII）

# 所有权可以转移（Move）
new_owner = data  # data 不再可用，零拷贝
```

### 2. ref = Arc（安全共享）

```yaoxiang
# ref 关键字创建 Arc（引用计数）
p: Point = Point(1.0, 2.0)
shared = ref p    # Arc，线程安全

spawn(() => print(shared.x))   # ✅ 安全
# spawn 自动检查 Send 约束
```

**循环引用规则**：
- 任务内循环：允许（泄漏可控，任务结束后释放）
- 跨任务循环：编译器检测并报错

### 3. clone() 显式复制

```yaoxiang
# 需要保留原值时使用 clone()
p: Point = Point(1.0, 2.0)
p2 = p.clone()   # p 和 p2 独立

p.x = 0.0        # ✅
p2.x = 0.0       # ✅ 互不影响
```

### 4. 标准库：Rc / Arc / Weak

```yaoxiang
use std::rc.{Rc, Weak}
use std::sync::Arc

# Rc：非线程安全引用计数
rc: Rc[Node] = Rc::new(node)

# Arc：线程安全引用计数
arc: Arc[Node] = Arc::new(node)

# Weak：打破循环（不增加计数）
weak: Weak[Node] = Weak::new(arc)
```

## 任务列表

根据 RFC-009 v7.1 设计，所有任务已完成：

| 顺序 | 任务 | 名称 | 依赖 | 状态 |
|------|------|------|------|------|
| 1 | task-05-01 | Move 语义（转移/释放） | 无 | ✅ 已实现 |
| 2 | task-05-02 | mut 检查 | task-05-01 | ✅ 已实现 |
| 3 | task-05-03 | ref 关键字（Arc） | task-05-01 | ✅ 已实现 |
| 4 | task-05-04 | clone() 显式复制 | task-05-01 | ✅ 已实现 |
| 5 | task-05-05 | Send/Sync 约束 | task-05-03 | ✅ 已实现 |
| 6 | task-05-06 | 跨任务循环引用检测 | task-05-03 | ✅ 已实现 |

> **说明**：RFC-009 v7.1 明确**不实现生命周期标注**和**借用检查器**。

### 任务说明

#### Task 5.1: Move 语义（基础模块）✅

- Move 语义（转移后原所有者失效）
- Drop 规则（RAII 资源释放）
- 值传递语义
- 实现：`src/middle/lifetime/move_semantics.rs`

> **依赖**：无（此任务是所有权系统的**基础模块**）

#### Task 5.2: mut 检查 ✅

- 变量可变性声明
- mut 字段访问规则
- 实现：`src/middle/lifetime/mut_check.rs`

> **依赖**：task-05-01（需要所有权状态信息）

#### Task 5.3: ref 关键字（Arc）✅

- `ref` 关键字解析为 Arc
- 引用计数管理
- 跨 spawn 边界安全
- 实现：`src/middle/lifetime/ref_semantics.rs`

> **依赖**：task-05-01（需要所有权状态信息）

#### Task 5.4: clone() 语义 ✅

- `clone()` 方法调用
- 值复制语义
- 实现：`src/middle/lifetime/clone.rs`

> **依赖**：task-05-01（需要所有权状态信息）

#### Task 5.5: Send/Sync 约束检查 ✅

- 跨线程所有权转移安全
- spawn 参数/返回值检查
- ref Arc 的 Send/Sync 自动满足
- 实现：`src/middle/lifetime/send_sync.rs`

> **依赖**：task-05-03（需要 ref Arc 信息）

#### Task 5.6: 跨任务循环引用检测 ✅

- 追踪 spawn 参数和返回值边界
- 检测跨任务边是否形成环
- 任务内循环不检测（泄漏可控）
- 实现：`src/middle/lifetime/cycle_check.rs`

> **依赖**：task-05-03（需要 ref Arc 信息）

## 不实现

| 特性 | 原因 |
|------|------|
| 生命周期 `'a` | 无引用概念，无需生命周期 |
| 借用检查器 | ref = Arc 替代 |
| `&T` 借用语法 | 使用 Move 语义 |

## 相关文件

### 任务文件

| 任务 | 文件 | 说明 |
|------|------|------|
| task-05-01 | [task-05-01-move-transfer.md](task-05-01-move-transfer.md) | Move 语义（转移/释放） |
| task-05-02 | [task-05-02-mut-check.md](task-05-02-mut-check.md) | mut 检查 |
| task-05-03 | [task-05-03-ref-arc.md](task-05-03-ref-arc.md) | ref 关键字（Arc） |
| task-05-04 | [task-05-04-clone.md](task-05-04-clone.md) | clone() 显式复制 |
| task-05-05 | [task-05-05-send-sync.md](task-05-05-send-sync.md) | Send/Sync 约束 |
| task-05-06 | [task-05-06-cycle-detection.md](task-05-06-cycle-detection.md) | 跨任务循环引用检测 |

### 源码文件

- **src/middle/lifetime/mod.rs**: 所有权检查器主实现（OwnershipChecker）
- **src/middle/lifetime/move_semantics.rs**: Move 语义
- **src/middle/lifetime/drop_semantics.rs**: Drop 规则
- **src/middle/lifetime/mut_check.rs**: mut 检查
- **src/middle/lifetime/ref_semantics.rs**: ref 关键字（Arc）
- **src/middle/lifetime/clone.rs**: clone() 语义
- **src/middle/lifetime/send_sync.rs**: Send/Sync 检查
- **src/middle/lifetime/cycle_check.rs**: 循环引用检测

## 参考文档

- [RFC-009 所有权模型 v7](../../design/accepted/009-ownership-model.md)
- [YaoXiang 语言规范](../../design/language-spec.md)
- [RFC-001 并发模型与错误处理](../../design/rfc/001-concurrent-model-error-handling.md)
- [RFC-008 运行时并发模型](../../design/accepted/008-runtime-concurrency-model.md)
