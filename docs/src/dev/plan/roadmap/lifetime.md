---
title: "所有权检查器状态"
---

# 所有权检查器（Ownership）

> **模块状态**：迁移完成——前端霍尔命题管道已接管
> **新架构位置**：`src/frontend/core/typecheck/layers/ownership.rs`（～1600 行）
> **遗留位置**：`src/middle/passes/lifetime/`（保留，逐步清理）
> **最后更新**：2026-06-15
>
> **相关 RFC**：
> - [RFC-009: 所有权模型设计](../design/rfc/accepted/009-ownership-model.md) — 已接受
> - [RFC-009a: 令牌生命期分析——基于霍尔证明管道](../design/rfc/accepted/009a-borrow-proof-pipeline.md) — 已接受
>
> **已知问题**：[ongoing/ownership-known-issues.md](../ongoing/ownership-known-issues.md) — 6 项缺陷与精度取舍

---

## 模块概述

所有权检查器负责 YaoXiang 的所有权分析——Move 语义、借用令牌冲突、Drop 正确性、可变性违规、NLL 精确释放、闭包捕获、函数签名查询、ref 逃逸分析。

**当前架构**（v9 霍尔命题管道）：
- 品牌树（BrandTree）追踪令牌派生关系和冲突判断
- 消费者分析驱动 NLL 释放（ReleasePlan）
- 反向 BFS 活性分析（快速通道，覆盖 95%+ 场景）
- SMT 逻辑切断兜底（极罕见的 while + 路径条件场景）
- 作用域驱动 Drop（退出作用域时自动标记 VarState::Dropped）
- 闭包捕获分析（save/restore/diff → CapturesStore）
- 函数签名查询（TypeEnvironment → T/&T/&mut T → Move/ReadBorrow/WriteBorrow）
- ref 逃逸分析（spawn 内使用 → Arc，否则 → Rc）
- ir_gen 读取 ReleasePlan 插入 Drop 指令 + 按 escaped_refs 选 RcNew/ArcNew

---

## RFC 对齐状态

### RFC-009 五个核心概念

| 概念 | 用户可见行为 | 底层实现 |
|------|-------------|---------|
| **Move** | ✅ 已完成 | OwnershipChecker，UseAfterMove 检测 |
| **&T / &mut T** | ✅ 已完成 | 品牌树令牌冲突检测（快速通道 + SMT 兜底） |
| **ref** | ✅ 已完成 | 逃逸分析自动选 Rc/Arc |
| **clone()** | ✅ 已完成 | CloneChecker，0 tests |
| **unsafe + *T** | ✅ 已完成 | UnsafeChecker |

### RFC-009a 六阶段（新版——前端实现）

| 阶段 | 内容 | 状态 | 说明 |
|------|------|------|------|
| 1 | 品牌树数据结构 | ✅ 已完成 | `BrandTree` + `BrandNode` + `BrandId` |
| 2 | 消费者分析 | ✅ 已完成 | `BrandNode.consumers`，AST 遍历自动收集 |
| 3 | 反向 BFS 活性分析 | ✅ 已完成 | `fast_path_check()`，break 切断回边 |
| 4 | 作用域驱动 Release | ✅ 已完成 | `ReleasePlan` + `scope_vars` 栈，LIFO Drop |
| 5 | SMT 逻辑切断 | ✅ 已完成 | `smt_cut(path_cond, loop_cond)` via Z3 |
| 6 | 清理 | ✅ 已完成 | 遗留文件已删除，错误码格式未统一（P2） |

### 补充阶段

| 阶段 | 内容 | 状态 | 说明 |
|------|------|------|------|
| D.1 | ref 逃逸分析（Rc vs Arc） | ✅ 已完成 | `ref_vars` + `escaped_refs` + `inside_spawn`，ref 属性传播 |
| D.2 | 测试覆盖扩展 | ✅ 已完成 | 61 tests（原始 31 → 目标 50+） |
| D.3 | Drop 语义触发点 | ✅ 已完成 | `VarState::Dropped` 激活，作用域退出自动标记 |
| D.4 | 可变性检查 | ✅ 已完成 | `&mut` 和赋值检查 `var_mutability`，emit `mut_violation` |
| D.5 | 路线图同步 | ✅ 已完成 | 本文档 |
| — | 闭包捕获分析 | ✅ 已完成 | save→walk→diff→restore→CapturesStore |
| — | 函数签名查询 | ✅ 已完成 | TypeEnvironment.get_var → T/&T/&mut T |
| — | Spawn walk | ✅ 已完成 | save/restore 防止污染外层，检测 ref 逃逸 |

---

## 新架构核心组件

### `src/frontend/core/typecheck/layers/ownership.rs`（~1600 行）

| 组件 | 功能 |
|------|------|
| `BrandId` / `BrandTree` | 令牌标识 + 派生树 + 冲突判断 + 消费者追踪 |
| `ControlFlowGraph` | CFG 节点/边/路径条件，Break/BackEdge |
| `fast_path_check()` | 反向 BFS 活性分析（快速通道） |
| `smt_cut()` | SMT 逻辑切断（慢速通道，Z3） |
| 5 种系统谓词 | borrow_conflict / use_after_move / use_after_drop / double_drop / mut_violation |
| `OwnershipChecker` | AST 遍历 + 品牌树 + CFG + 谓词验证 |
| `ReleasePlan` | NLL 精确释放计划（消费者 + 作用域 Drop 双源合并） |
| `VarState` | Alive / Moved / Dropped 三态 |
| `Captures` / `CapturesStore` | 闭包捕获变量集合 + 存储 |
| `StateSnapshot` | save_state / restore_state / diff_captures |
| `ParamOwnership` | Move / ReadBorrow / WriteBorrow |
| `ref_vars` / `escaped_refs` / `inside_spawn` | ref 逃逸分析（含属性传播） |

### `src/middle/core/ir_gen.rs`

- 读取 `TypeCheckResult.release_plan` → 插入 `Drop` 指令（NLL 精确释放点）
- 读取 `TypeCheckResult.escaped_refs` → `Expr::Ref` 选 `RcNew` 或 `ArcNew`

### `src/middle/core/ir.rs` / `bytecode.rs` / `opcode.rs`

- 新增 `RcNew` 指令 + `Opcode::RcNew(0x89)`

---

## 当前 middle 层模块清单

> 注：`borrow_checker.rs`、`control_flow.rs`、`consume_analysis.rs`、`move_semantics.rs`、
> `drop_semantics.rs`、`mut_check.rs`、`ref_semantics.rs`、`clone.rs`、`empty_state.rs`、
> `send_sync.rs` 已删除。以下为保留的活跃模块。

| 子模块 | 文件 | 功能 |
|--------|------|------|
| **链式调用** | `chain_calls.rs` | 链式方法调用分析 |
| **跨任务环** | `cycle_check.rs` | 跨 spawn 循环引用 DFS |
| **任务内环** | `intra_task_cycle.rs` | 任务内 ref 循环追踪 |
| **生命周期** | `lifecycle.rs` | IR 级 Drop 位置追踪 |
| **所有权流** | `ownership_flow.rs` | 函数所有权流向分析 |
| **Unsafe** | `unsafe_check.rs` | unsafe 块绕过检查 |
| **错误类型** | `error.rs` | ValueState + Checker trait |

---

## 测试覆盖

**前端所有权检查器：61 个单元测试**

| 测试类别 | 测试数 | 覆盖内容 |
|----------|--------|----------|
| 基础（BrandId/冲突/级联/消费者/快速通道） | 17 | 令牌前缀、冲突判断、级联删除、消费者追踪、BFS 活性 |
| 系统谓词 | 6 | borrow_conflict / use_after_move / use_after_drop / double_drop / mut_violation |
| E2E 集成（基础） | 7 | use after move、valid move、argument move、借用冲突、写写冲突、读读安全 |
| E2E 可变性 | 5 | &mut 非 mut、&mut mut、赋值非 mut、赋值 mut、参数非 mut |
| E2E Drop | 2 | 作用域 Drop（ReleasePlan）、嵌套块 Drop |
| E2E Move+Borrow | 1 | move 后 borrow 检测 |
| E2E 控制流 | 2 | if/else 双分支冲突、while 循环内借用 |
| E2E Drop 排序 | 1 | 多变量同 span 释放 |
| E2E 返回值 | 2 | return Move、return 后 use |
| E2E 多重借用 | 2 | 三个 ReadToken、Read+Write 冲突 |
| E2E 块表达式 | 1 | 内层块变量作用域 |
| E2E 连续 Move | 2 | 连续 Move、double move 检测 |
| E2E 参数 | 2 | 参数 move 后 use、参数不在 ReleasePlan |
| E2E 闭包捕获 | 5 | Move 捕获、Read 捕获、无捕获、定义后调用前、二次调用 |
| E2E 函数签名 | 2 | 未知函数回退 Move、未注册函数回退 |
| E2E ref 逃逸 | 4 | 无 spawn 不逃逸、spawn 内逃逸、非 ref 不逃逸、嵌套 spawn |

**middle 层测试：53 个单元测试**

---

## 代码质量评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 未完成事项 | 4 | 错误信息格式统一 (P2) + 阶段 0 测试补齐 (5) + 已知问题 6 项 |
| 测试覆盖 | 良好 | 前端 61 tests + middle 53 tests = 114 tests |
| 文档质量 | 良好 | 模块/结构体/方法级别均有文档注释 |
| 代码架构 | 迁移完成 | 前端霍尔命题管道已接管核心逻辑；middle 层遗留文件已删除 |
