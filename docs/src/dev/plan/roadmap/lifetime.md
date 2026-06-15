---
title: "所有权检查器状态"
---

# 所有权检查器（Ownership）

> **模块状态**：迁移完成——前端霍尔命题管道已接管，middle 层仅保留遗留代码
> **新架构位置**：`src/frontend/core/typecheck/layers/ownership.rs`（～1300 行）
> **遗留位置**：`src/middle/passes/lifetime/`（保留，逐步清理）
> **最后更新**：2026-06-14
>
> **相关 RFC**：
> - [RFC-009: 所有权模型设计](../design/rfc/accepted/009-ownership-model.md) — 已接受
> - [RFC-009a: 令牌生命期分析——基于霍尔证明管道](../design/rfc/accepted/009a-borrow-proof-pipeline.md) — 已接受

---

## 模块概述

所有权检查器负责 YaoXiang 的所有权分析——Move 语义、借用令牌冲突、Drop 正确性、可变性违规、NLL 精确释放。

**当前架构**（v9 霍尔命题管道）：
- 品牌树（BrandTree）追踪令牌派生关系和冲突判断
- 消费者分析驱动 NLL 释放（ReleasePlan）
- 反向 BFS 活性分析（快速通道，覆盖 95%+ 场景）
- SMT 逻辑切断兜底（极罕见的 while + 路径条件场景）
- 作用域驱动 Drop（退出作用域时自动标记 VarState::Dropped）
- ir_gen 读取 ReleasePlan 插入 Drop 指令（NLL 精确释放点）

**遗留架构**（v8 线性扫描，`src/middle/passes/lifetime/`）：
- ir_gen 硬编码插入 Borrow/Release 指令（词法作用域）
- BorrowChecker 线性扫描 IR，被动验证令牌冲突
- ControlFlowAnalyzer 存在但核心逻辑为空
- 用户可见行为基本正确，但底层不是 RFC-009a 的霍尔命题管道

**代码量**：前端 ~1300 行 + middle 约 300KB 源码（15 个子文件，遗留）

---

## RFC 对齐状态

### RFC-009 五个核心概念

| 概念 | 用户可见行为 | 底层实现 |
|------|-------------|---------|
| **Move** | ✅ 已完成 | OwnershipChecker，UseAfterMove 检测 |
| **&T / &mut T** | ✅ 已完成 | 品牌树令牌冲突检测（快速通道 + SMT 兜底） |
| **ref** | ⚠️ 环检测完成，逃逸分析缺失 | ref_semantics + cycle_check + intra_task_cycle；当前统一用 Arc |
| **clone()** | ✅ 已完成 | CloneChecker，0 tests |
| **unsafe + *T** | ✅ 已完成 | UnsafeChecker |

### RFC-009a 六阶段（新版——前端实现）

| 阶段 | 内容 | 状态 | 说明 |
|------|------|------|------|
| 1 | 品牌树数据结构 | ✅ 已完成 | `BrandTree` + `BrandNode` + `BrandId`，前缀匹配冲突判断 |
| 2 | 消费者分析 | ✅ 已完成 | `BrandNode.consumers`，AST 遍历自动收集 |
| 3 | 反向 BFS 活性分析 | ✅ 已完成 | `fast_path_check()`，break 切断回边，覆盖 95%+ 场景 |
| 4 | 作用域驱动 Release | ✅ 已完成 | `ReleasePlan` + `scope_vars` 栈，作用域退出时 LIFO Drop |
| 5 | SMT 逻辑切断 | ✅ 已完成 | `smt_cut(path_cond, loop_cond)` via Z3 backend |
| 6 | 清理 | ⏳ 进行中 | 前端已接管，middle 遗留代码待删除 |

### 补充阶段（Phase D——远期补全）

| 阶段 | 内容 | 状态 | 说明 |
|------|------|------|------|
| D.1 | ref 逃逸分析（Rc vs Arc） | ⬜ 未开始 | 当前统一用 Arc，正确但非最优 |
| D.2 | 测试覆盖扩展 | ⏳ 部分完成 | 37 tests（目标 50+） |
| D.3 | Drop 语义触发点 | ✅ 已完成 | `VarState::Dropped` 激活，作用域退出自动标记 |
| D.4 | 可变性检查 | ✅ 已完成 | `&mut` 和赋值检查 `var_mutability`，emit `mut_violation` |
| D.5 | 路线图同步 | ✅ 已完成 | 本文档 |

---

## 新架构核心组件

### `src/frontend/core/typecheck/layers/ownership.rs`

| 组件 | 功能 | 行数 |
|------|------|------|
| `BrandId` | 编译期唯一令牌标识（`#0`、`#0.x`） | ~50 |
| `BrandTree` | 令牌派生树，冲突判断，消费者追踪 | ~200 |
| `ControlFlowGraph` | CFG 节点/边/路径条件，支持 Break/BackEdge | ~100 |
| `fast_path_check()` | 反向 BFS 活性分析（快速通道） | ~80 |
| `smt_cut()` | SMT 逻辑切断（慢速通道，Z3） | ~60 |
| 5 种系统谓词 | `emit_borrow_predicate` / `emit_move_predicate` / `emit_drop_predicate` / `emit_double_drop_predicate` / `emit_mut_predicate` | ~80 |
| `OwnershipChecker` | AST 遍历 + 品牌树构建 + CFG 控制流 + 谓词验证 | ~400 |
| `ReleasePlan` | NLL 精确释放计划（品牌树消费者 + 作用域 Drop 双源合并） | ~60 |
| `VarState` | Alive / Moved / Dropped 三态追踪 | ~30 |

### `src/middle/core/ir_gen.rs`

- 读取 `TypeCheckResult.release_plan`
- 在语句边界插入 `Instruction::Drop` 指令（NLL 精确释放点）

---

## 当前 middle 层模块清单

> 注：`borrow_checker.rs`、`control_flow.rs`、`consume_analysis.rs`、`move_semantics.rs`、
> `drop_semantics.rs`、`mut_check.rs`、`ref_semantics.rs`、`clone.rs`、`empty_state.rs`、
> `send_sync.rs` 已删除。以下为保留的活跃模块。

### 核心模块（14 个文件，约 120KB）

| 子模块 | 文件 | 功能 | 归宿 |
|--------|------|------|------|
| **链式调用** | `chain_calls.rs` | 链式方法调用分析 | 保留 |
| **跨任务环** | `cycle_check.rs` | 跨 spawn 循环引用 DFS | 保留 |
| **任务内环** | `intra_task_cycle.rs` | 任务内 ref 循环追踪 | 保留 |
| **生命周期** | `lifecycle.rs` | IR 级 Drop 位置追踪 | 保留（与前端 ReleasePlan 互补） |
| **所有权流** | `ownership_flow.rs` | 函数所有权流向分析（consumes/returns） | 保留 |
| **Unsafe** | `unsafe_check.rs` | unsafe 块绕过检查 | 保留 |
| **错误类型** | `error.rs` | ValueState + Checker trait | 保留（middle 层状态追踪） |

### 测试模块（5 个文件）

| 文件 | 测试数 | 覆盖情况 |
|------|--------|----------|
| `tests/chain_calls.rs` | 13 | 充分 |
| `tests/ownership_flow.rs` | 10 | 充分 |
| `tests/lifecycle.rs` | 10 | 充分 |
| `tests/cycle_check.rs` | 8 | 良好 |
| `tests/intra_task_cycle.rs` | 7 | 良好 |

---

## 实现路线图（旧架构 → 新架构迁移）

### 阶段 0：补齐测试（可立即开始，阻塞重构）

> 在动架构之前，先把现有行为的测试网铺好。

| # | 任务 | 文件 |
|---|------|------|
| 0.1 | 补充 Drop 语义测试 | `tests/drop_semantics.rs` |
| 0.2 | 补充 Clone 语义测试 | `tests/clone.rs` |
| 0.3 | 补充可变性检查测试 | `tests/mut_check.rs` |
| 0.4 | 补充 Ref 语义测试 | `tests/ref_semantics.rs` |
| 0.5 | 补充 Unsafe 检查测试 | `tests/unsafe_check.rs` |

### 阶段 1-5：✅ 已完成（前端 OwnershipChecker + 品牌树 + 快速通道 + SMT + ReleasePlan）

| 阶段 | 内容 | 实现位置 |
|------|------|----------|
| 1 | 品牌树数据结构 | `ownership.rs:BrandTree` |
| 2 | 消费者分析 | `ownership.rs:BrandNode.consumers` |
| 3 | 反向 BFS 活性分析 | `ownership.rs:fast_path_check()` |
| 4 | 作用域驱动 Release | `ownership.rs:ReleasePlan + scope_vars` |
| 5 | SMT 逻辑切断 | `ownership.rs:smt_cut()` |

### 阶段 6：清理（大部分已完成）

| # | 任务 | 状态 | 说明 |
|---|------|------|------|
| 6.1 | 删除 `borrow_checker.rs`（线性扫描架构） | ✅ 已完成 | 前端品牌树 + fast_path_check 已替代 |
| 6.2 | 删除 `control_flow.rs` → `ControlFlowAnalyzer` | ✅ 已完成 | 前端 CFG 替代 |
| 6.3 | 删除 `consume_analysis.rs` | ✅ 已完成 | 前端 BrandNode.consumers 替代 |
| 6.4 | 删除 `move_semantics.rs` / `drop_semantics.rs` / `mut_check.rs` / `ref_semantics.rs` / `clone.rs` / `empty_state.rs` / `send_sync.rs` | ✅ 已完成 | 前端 OwnershipChecker 替代 |
| 6.5 | 更新错误信息格式 | ⬜ 待完成 | 与新架构错误码对齐 |
| 6.6 | 删除 `ir_gen.rs` 硬编码 Borrow/Release 指令 | ✅ 已完成 | ReleasePlan + NLL Drop 替代 |

---

## 独立任务

| # | 任务 | 状态 | 说明 |
|---|------|------|------|
| I.1 | ref 逃逸分析（Rc vs Arc 自动选择） | ⬜ 未开始 | 当前编译器不区分跨任务与否，统一用 Arc |
| I.2 | 删除 `control_flow.rs` 的 `ControlFlowAnalyzer` 之前，不要往里加新代码 | — | — |
| I.3 | 嵌套函数递归检查 | ⬜ 未开始 | `StmtKind::Binding` 当前跳过，需独立作用域 |
| I.4 | 函数签名查询（T vs &T） | ⬜ 未开始 | 用于确定 Call 参数是 Move 还是 Borrow |

---

## 测试覆盖

**前端所有权检查器：37 个单元测试**（`src/frontend/core/typecheck/layers/tests/ownership.rs`）

| 测试类别 | 测试数 | 覆盖内容 |
|----------|--------|----------|
| BrandId 前缀匹配 | 3 | `is_prefix_of`、root_id 提取 |
| 冲突判断 | 5 | read/read、read/write、write/write、派生令牌冲突 |
| 级联删除 | 2 | remove 级联到子令牌 |
| 消费者追踪 | 2 | consumer 收集、未知令牌 |
| 冲突查询 | 1 | `conflicting_with` |
| 快速通道 | 4 | 线性代码、消费者前后、break 切断、loop unsafe |
| 系统谓词 | 6 | 5 种谓词（move/drop/double_drop/mut/both） |
| E2E 集成（基础） | 7 | use after move、valid move、argument move、借用冲突、写写冲突、读读安全 |
| E2E 可变性 | 5 | &mut 非 mut、&mut mut、赋值非 mut、赋值 mut、参数非 mut |
| E2E Drop | 2 | 作用域 Drop（ReleasePlan）、嵌套块 Drop |
| E2E Move+Borrow | 1 | move 后 borrow 检测 |
| E2E 控制流 | 2 | if/else 双分支冲突、while 循环内借用 |
| E2E Drop 排序 | 1 | 多变量同 span 释放 |
| E2E 返回值 | 2 | return Move、return 后 use |
| E2E 多重借用 | 2 | 三个 ReadToken、Read+Write 冲突 |
| E2E 块表达式 | 1 | 内层块变量作用域 |
| E2E 连续 Move | 2 | 连续 Move x→y→z、double move 检测 |
| E2E 参数 | 2 | 参数 move 后 use、参数不在 ReleasePlan |

**middle 层测试：53 个单元测试**

| 文件 | 测试数 | 覆盖情况 |
|------|--------|----------|
| `tests/chain_calls.rs` | 13 | 充分 |
| `tests/ownership_flow.rs` | 10 | 充分 |
| `tests/lifecycle.rs` | 10 | 充分 |
| `tests/cycle_check.rs` | 8 | 良好 |
| `tests/intra_task_cycle.rs` | 7 | 良好 |
| 其他（已删除模块的遗留测试） | 5 | 待迁移或删除 |

---

## 代码质量评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 未完成事项 | 3 | 阶段 0 测试 (5) + 错误信息格式更新 (1) + ref 逃逸分析 (1) + 嵌套函数检查 (1) + 函数签名查询 (1) |
| 测试覆盖 | 良好 | 前端 50 tests + middle 53 tests = 103 tests；middle 层 5 个子模块充分覆盖 |
| 文档质量 | 良好 | 模块/结构体/方法级别均有文档注释 |
| 代码架构 | 迁移完成 | 前端霍尔命题管道已接管所有权检查核心逻辑；middle 层遗留文件已删除 |
