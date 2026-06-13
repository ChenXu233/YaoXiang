---
title: "借用检查器状态"
---

# 借用检查器（Lifetime）

> **模块状态**：过渡期——v8 线性扫描架构 → v9 霍尔命题管道
> **位置**：`src/middle/passes/lifetime/`
> **最后更新**：2026-06-13
>
> **相关 RFC**：
> - [RFC-009: 所有权模型设计](../design/rfc/accepted/009-ownership-model.md) — 已接受
> - [RFC-009a: 令牌生命期分析——基于霍尔证明管道](../design/rfc/accepted/009a-borrow-proof-pipeline.md) — 已接受

---

## 模块概述

借用检查器模块负责 YaoXiang 的所有权分析——Move 语义、借用令牌冲突、Drop/Clone 正确性、ref 环检测、可变性违规。

**当前架构**（过渡期）：
- ir_gen 硬编码插入 Borrow/Release 指令（词法作用域）
- BorrowChecker 线性扫描 IR，被动验证令牌冲突
- ControlFlowAnalyzer 存在但核心逻辑为空
- 用户可见行为基本正确，但底层不是 RFC-009a 的霍尔命题管道

**目标架构**（RFC-009 + RFC-009a）：
- 品牌树追踪令牌派生关系
- 消费者分析驱动 NLL 释放
- 反向 BFS 活性分析（快速通道覆盖 95%+ 场景）
- SMT 逻辑切断兜底（极罕见的 while + 路径条件场景）
- Release 由作用域分析驱动，不在 ir_gen 硬编码

**代码量**：约 300KB 源码（15 个子文件）

---

## RFC 对齐状态

### RFC-009 五个核心概念

| 概念 | 用户可见行为 | 底层实现 |
|------|-------------|---------|
| **Move** | ✅ 已完成 | MoveChecker，UseAfterMove 检测 |
| **&T / &mut T** | ✅ 已完成 | BorrowChecker 线性扫描，被动响应 Borrow/Release 指令 |
| **ref** | ⚠️ 环检测完成，逃逸分析缺失 | ref_semantics + cycle_check + intra_task_cycle |
| **clone()** | ✅ 已完成 | CloneChecker，0 tests |
| **unsafe + *T** | ✅ 已完成 | UnsafeChecker |

### RFC-009a 六阶段

| 阶段 | 内容 | 状态 | 说明 |
|------|------|------|------|
| 1 | 品牌树数据结构 | ❌ 未开始 | 替换 HashMap<String, BorrowToken>，品牌 ID + 父节点 + 消费者列表 |
| 2 | 消费者分析 | ❌ 未开始 | DAG 构建时自动收集令牌消费者，NLL 基础 |
| 3 | 反向 BFS 活性分析 | ❌ 未开始 | 品牌树 + 消费者 + break 切断 → 覆盖 95%+ 场景 |
| 4 | 作用域驱动 Release | ❌ 未开始 | 删除 ir_gen 硬编码，作用域出口点 LIFO 插入，? 自动处理 |
| 5 | SMT 逻辑切断 | ❌ 未开始 | 阻塞于 RFC-027 Phase 2，仅 while + 路径条件触发 |
| 6 | 清理 | ❌ 未开始 | BorrowChecker → BorrowPredicateEmitter，删 ControlFlowAnalyzer |

---

## 当前模块清单

### 核心检查器

| 子模块 | 文件 | 功能 | 测试 |
|--------|------|------|------|
| **Move 语义** | `move_semantics.rs` | UseAfterMove 检测，空状态重赋值 | 6 |
| **Drop 语义** | `drop_semantics.rs` | UseAfterDrop、DropMovedValue、DoubleDrop | 0 |
| **可变性检查** | `mut_check.rs` | 不可变变量赋值/变异方法/字段赋值 | 0 |
| **Ref 语义** | `ref_semantics.rs` | RefNonOwner 检测 | 0 |
| **Clone 语义** | `clone.rs` | CloneMovedValue、CloneDroppedValue | 0 |
| **借用令牌** | `borrow_checker.rs` | 令牌冲突检测（线性扫描架构） | 16 |
| **跨任务环** | `cycle_check.rs` | 跨 spawn 循环引用 DFS 检测 | 8 |
| **任务内环** | `intra_task_cycle.rs` | 任务内 ref 循环追踪（警告模式） | 7 |

### 辅助模块

| 子模块 | 文件 | 归宿 |
|--------|------|------|
| **所有权回流** | `ownership_flow.rs` | 保留 |
| **消费分析** | `consume_analysis.rs` | → Phase 2 整合进品牌树 |
| **链式调用** | `chain_calls.rs` | 保留 |
| **生命周期追踪** | `lifecycle.rs` | 保留——Drop 插入需要 |
| **空状态** | `empty_state.rs` | 保留 |
| **控制流** | `control_flow.rs` | → Phase 6 删除 |
| **Unsafe 检查** | `unsafe_check.rs` | 保留 |
| **Send/Sync** | `send_sync.rs` | 保留（独立使用） |

---

## 实现路线图

### 阶段 0：补齐测试（可立即开始，阻塞重构）

> 在动架构之前，先把现有行为的测试网铺好。

| # | 任务 | 文件 |
|---|------|------|
| 0.1 | 补充 Drop 语义测试 | `tests/drop_semantics.rs` |
| 0.2 | 补充 Clone 语义测试 | `tests/clone.rs` |
| 0.3 | 补充可变性检查测试 | `tests/mut_check.rs` |
| 0.4 | 补充 Ref 语义测试 | `tests/ref_semantics.rs` |
| 0.5 | 补充 Unsafe 检查测试 | `tests/unsafe_check.rs` |

### 阶段 1：品牌树数据结构（RFC-009a Phase 1）

| # | 任务 | 产出 |
|---|------|------|
| 1.1 | 定义 `BrandTree`、`BrandNode` 结构体 | `brand_tree.rs` |
| 1.2 | 实现前缀匹配冲突判断 | `conflicts()` |
| 1.3 | 实现 DAG 构建时品牌节点注册 | 集成到 ir_gen |
| 1.4 | 单元测试 | `tests/brand_tree.rs` |

### 阶段 2：消费者分析（RFC-009a Phase 2）

| # | 任务 | 产出 |
|---|------|------|
| 2.1 | DAG 构建时自动收集每个令牌的消费者列表 | `BrandNode.consumers` |
| 2.2 | 系统谓词生成器定义（Borrow/Move/Drop/Mut → `{P} op {Q}`） | 接口定义 |

### 阶段 3：反向 BFS 活性分析（RFC-009a Phase 3）

| # | 任务 | 产出 |
|---|------|------|
| 3.1 | 实现反向 BFS 算法（break 切断回边） | 快速通道 |
| 3.2 | 接入 RFC-027 证明管道接口（Proved/Disproved） | 管道接入 |
| 3.3 | NLL 迭代边界规则实现 | 循环内令牌跨迭代语义 |
| 3.4 | 替换 BorrowChecker 线性扫描 | 删除 `check_instruction` 里的 Borrow/Release match |

### 阶段 4：作用域驱动 Release（RFC-009a Phase 4-5）

| # | 任务 | 产出 |
|---|------|------|
| 4.1 | 作用域出口点收集（`}`、`?`、显式 return） | ir_gen |
| 4.2 | LIFO Release 插入（品牌树父子关系自动级联） | ir_gen |
| 4.3 | 删除 `ir_gen.rs` Call 后硬编码 Release | 代码清理 |

### 阶段 5：SMT 逻辑切断（RFC-009a Phase 5，依赖 RFC-027 Phase 2）

| # | 任务 | 产出 |
|---|------|------|
| 5.1 | 路径条件收集集成 | 从 RFC-027 管道获取 |
| 5.2 | SMT fallback：`path_cond ⇒ !loop_cond` | 慢速通道 |
| 5.3 | 激活 while 循环体内借用检查 | 当前保守拒绝的场景 |

### 阶段 6：清理（RFC-009a Phase 6）

| # | 任务 | 产出 |
|---|------|------|
| 6.1 | `BorrowChecker` → `BorrowPredicateEmitter` | 重命名，职责明确 |
| 6.2 | 删除 `ControlFlowAnalyzer` | 管道统一处理 |
| 6.3 | `consume_analysis.rs` 消费者信息迁移到品牌树 | 去重 |
| 6.4 | 更新错误信息格式 | 与 RFC-009a §错误信息设计对齐 |

---

## 独立任务（不阻塞主线）

| # | 任务 | 说明 |
|---|------|------|
| I.1 | ref 逃逸分析（Rc vs Arc 自动选择） | 当前编译器不区分跨任务与否，统一用 Arc |
| I.2 | 删除 `control_flow.rs` 的 `ControlFlowAnalyzer` 之前，不要往里加新代码 | — |

---

## 测试覆盖

**当前：83 个单元测试**

| 文件 | 测试数 | 覆盖情况 |
|------|--------|----------|
| `borrow_checker.rs` | 16 | 充分 |
| `chain_calls.rs` | 13 | 充分 |
| `consume_analysis.rs` | 11 | 充分 |
| `ownership_flow.rs` | 10 | 充分 |
| `lifecycle.rs` | 10 | 充分 |
| `cycle_check.rs` | 8 | 良好 |
| `intra_task_cycle.rs` | 7 | 良好 |
| `move_semantics.rs` | 6 | 基本 |
| `control_flow.rs` | 1 | 不足 |
| `empty_state.rs` | 1 | 不足 |
| 其他 | 0 | **缺失**：drop_semantics, clone, mut_check, ref_semantics, unsafe_check |

---

## 代码质量评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 未完成事项 | 10 | 阶段 0 测试 (5) + 阶段 1-6 架构 (6) + ref 逃逸分析 (1) |
| 测试覆盖 | 待补强 | 5 个子模块 0 tests，重构前必须补齐 |
| 文档质量 | 良好 | 模块/结构体/方法级别均有文档注释 |
| 代码架构 | 过渡期 | 当前线性扫描架构能跑但对不齐 RFC-009a |
