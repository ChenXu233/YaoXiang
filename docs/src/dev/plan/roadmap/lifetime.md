---
title: "借用检查器状态"
---

# 借用检查器（Lifetime）

> **模块状态**：稳定（4 项待改进）
> **位置**：`src/middle/passes/lifetime/`
> **最后更新**：2026-06-01

---

## 模块概述

借用检查器模块是一个完整的**所有权分析与生命周期管理系统**，负责检查 Move 语义、借用冲突、可变性违规等所有权相关问题。

**代码量**：约 300KB 源码（15 个子文件）

---

## 功能清单

### 核心检查器（已集成到 OwnershipChecker 统一入口）

| 子模块 | 文件 | 功能 | 状态 |
|--------|------|------|------|
| **Move 语义** | `move_semantics.rs` (575行) | UseAfterMove 检测，支持空状态(Empty)重赋值 | ✅ 已完成 |
| **Drop 语义** | `drop_semantics.rs` (143行) | UseAfterDrop、DropMovedValue、DoubleDrop 检测 | ✅ 已完成 |
| **可变性检查** | `mut_check.rs` (395行) | 不可变变量赋值、不可变对象变异方法、不可变字段赋值 | ✅ 已完成 |
| **Ref 语义** | `ref_semantics.rs` (145行) | RefNonOwner 检测——ref 只能应用于有效所有者 | ✅ 已完成 |
| **Clone 语义** | `clone.rs` (173行) | CloneMovedValue、CloneDroppedValue 检测 | ✅ 已完成 |
| **借用令牌** | `borrow_checker.rs` (503行) | 借用令牌冲突检测：MutableBorrowConflict、BorrowAfterMove、UseWhileFrozen | ✅ 已完成 |
| **跨 spawn 循环** | `cycle_check.rs` (616行) | 跨任务循环引用检测，DFS 环检测 | ✅ 已完成 |
| **任务内循环** | `intra_task_cycle.rs` (406行) | 任务内 ref 循环追踪（警告模式） | ✅ 已完成 |

### 辅助分析器

| 子模块 | 文件 | 功能 | 状态 |
|--------|------|------|------|
| **所有权回流** | `ownership_flow.rs` (426行) | 分析函数参数是否在返回值中返回 | ✅ 已完成 |
| **消费分析** | `consume_analysis.rs` (363行) | 跨函数消费模式查询，支持缓存 | ✅ 已完成 |
| **链式调用** | `chain_calls.rs` (652行) | 方法链所有权流动分析 | ✅ 已完成 |
| **生命周期追踪** | `lifecycle.rs` (1037行) | 变量完整生命周期追踪 | ✅ 已完成 |
| **空状态** | `empty_state.rs` (513行) | Move 后变量空状态追踪 | ✅ 已完成 |
| **控制流** | `control_flow.rs` (353行) | 分支状态合并分析 | ⚠️ 骨架完成，核心分析逻辑为空实现 |
| **Unsafe 检查** | `unsafe_check.rs` (113行) | unsafe 块外解引用裸指针报错 | ✅ 已完成 |
| **Send/Sync** | `send_sync.rs` (401行) | 类型级 Send/Sync 约束检查和约束传播 | ✅ 已完成（独立使用） |

---

## 测试覆盖

**83 个单元测试**，分布如下：

| 文件 | 测试数 | 覆盖情况 |
|------|--------|----------|
| `borrow_checker.rs` | 16 | 最充分：单元测试+端到端测试 |
| `chain_calls.rs` | 13 | 充分：链提取、消费模式推断、长链、混合调用 |
| `consume_analysis.rs` | 11 | 充分：Returns/Consumes 模式、缓存、多参数 |
| `ownership_flow.rs` | 10 | 充分：直接返回、间接返回、多参数部分返回 |
| `lifecycle.rs` | 10 | 充分：创建/消费/释放追踪、问题检测 |
| `cycle_check.rs` | 8 | 良好：无循环/单向链/深度限制/unsafe 绕过 |
| `intra_task_cycle.rs` | 7 | 良好：无循环/简单循环/自引用/多循环 |
| `move_semantics.rs` | 6 | 基本：状态追踪、UseAfterMove |
| `control_flow.rs` | 1 | 不足：仅测试状态合并函数 |
| `empty_state.rs` | 1 | 不足：仅测试状态合并 |
| 其他 | 0 | 无测试：drop_semantics, clone, mut_check, ref_semantics, unsafe_check, send_sync |

---

## RFC 对比（RFC-009 所有权模型）

| RFC 设计要点 | 实现状态 | 说明 |
|-------------|---------|------|
| Move 语义（默认） | ✅ 已实现 | MoveChecker 检测 UseAfterMove |
| &T/&mut T 借用令牌 | ✅ 已实现 | BorrowChecker 实现令牌冲突检测 |
| &T 可复制（Dup） | ✅ 已实现 | 多个 &T 令牌可同时存在 |
| &mut T 线性 | ✅ 已实现 | 同一来源 &mut T 只能有一个活跃 |
| 令牌冲突检测（流敏感活性分析） | ✅ 已实现 | 函数体内追踪令牌状态 |
| ref 关键字（Rc/Arc 自动选择） | ⚠️ 部分实现 | ref 语义检查器存在 |
| clone() 显式深拷贝 | ✅ 已实现 | CloneChecker 检测 clone 移动/释放的值 |
| unsafe + *T | ✅ 已实现 | UnsafeChecker 检查 unsafe 块外的裸指针操作 |
| 任务内循环：静默允许 | ✅ 已实现 | IntraTaskCycleTracker 以警告模式追踪 |
| 跨任务循环：lint | ✅ 已实现 | CycleChecker 检测跨 spawn 循环引用 |
| 无生命周期 'a | ✅ 符合设计 | 没有实现生命周期参数 |
| Send/Sync 约束 | ✅ 已实现 | SendSyncChecker 独立于 OwnershipChecker |

---

## 代码质量评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 未完成事项 | 3 | 补充测试、control_flow 逻辑、ref 逃逸分析 |
| 测试覆盖 | 良好 | 83 个测试，borrow_checker/chain_calls/consume_analysis 测试充分 |
| 文档质量 | 良好 | 模块/结构体/方法级别均有文档注释 |
| 代码架构 | 优秀 | OwnershipChecker 统一编排，职责分离清晰 |
| RFC 合规 | 高度符合 | RFC-009 v9 设计高度符合 |

---

## 待改进项

1. **补充 5 个子模块的单元测试**：drop_semantics, clone, mut_check, ref_semantics, unsafe_check
2. **实现 control_flow 分析器的核心逻辑**（当前为空骨架）
3. **完善 ref 自动选择 Rc/Arc 的逃逸分析**
