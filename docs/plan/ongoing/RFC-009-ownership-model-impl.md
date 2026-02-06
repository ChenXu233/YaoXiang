# RFC-009 所有权模型实现路径

> **文档版本**: v1.0
> **基于设计**: `docs/design/accepted/009-ownership-model.md`
> **生成日期**: 2025-02-05

## 实现概述

本文档将 RFC-009 设计规范转化为可执行的实现步骤，基于 YaoXiang 现有架构进行扩展。

### 现有基础

| 模块 | 位置 | 状态 |
|------|------|------|
| 所有权系统 | `src/middle/passes/lifetime/` | ✅ 已完成基础 |
| Move 语义 | `move_semantics.rs` | ✅ 已实现 |
| ref 语义 | `ref_semantics.rs` | ✅ 已实现 |
| 循环检测 | `cycle_check.rs` | ✅ 已实现 |
| mut 检查 | `mut_check.rs` | ✅ 已实现 |

---

## Phase 1: 字段级不可变性 (P0)

### 目标

在类型定义中支持 `mut` 字段标记，实现三层次可变性模型：
- 绑定可变性（变量级别）
- 字段可变性（结构体级别）
- 方法参数可变性（函数级别）

### 实现状态：✅ 已完成（2026-02-05）

#### 已完成变更（2026-02-05 更新）

1. **AST 扩展** (`ast.rs`)
   - ✅ 创建 `StructField` 结构体：`name: String, is_mut: bool, ty: Type`
   - ✅ `Type::Struct(Vec<StructField>)` 替换 `Type::Struct(Vec<(String, Type)>)`
   - ✅ `Type::NamedStruct { name, fields: Vec<StructField> }`
   - ✅ `Pattern::Struct { name, fields: Vec<(String, bool, Box<Pattern>)> }`

2. **Parser 扩展** (`statements/declarations.rs`)
   - ✅ `parse_struct_type` 支持 `{ x: Float, mut y: Float }` 语法
   - ✅ `parse_named_struct_type` 支持 `Point(x: Float, mut y: Float)` 语法

3. **类型系统** (`type_system/mono.rs`)
   - ✅ `StructType` 添加 `field_mutability: Vec<bool>`
   - ✅ 实现 `field_is_mut(&self, field_name: &str) -> Option<bool>` 方法
   - ✅ 更新 `MonoType::from(ast::Type)` 转换逻辑

4. **模式匹配** (`typecheck/inference/patterns.rs`)
   - ✅ 模式推断支持 `is_mut` 标记

5. **Parser 模式解析** (`parser/pratt/nud.rs`)
   - ✅ 结构体模式语法解析支持 `mut` 关键字

6. **错误类型** (`lifetime/error.rs`)
   - ✅ 添加 `ImmutableFieldAssign` 错误变体
   - ✅ 添加 Display 实现

7. **IR 指令扩展** (`middle/core/ir.rs`)
   - ✅ `StoreField` 添加 `type_name: Option<String>` 和 `field_name: Option<String>`

8. **IR 生成** (`middle/core/ir_gen.rs`)
   - ✅ `get_field_mutability` 返回类型名
   - ✅ StoreField 指令携带类型信息

9. **可变性检查** (`lifetime/mut_check.rs`)
   - ✅ 绑定级可变性检查
   - ✅ 字段级可变性检查（传入类型表）
   - ✅ `ImmutableFieldAssign` 错误检测

10. **代码生成** (`codegen/translator.rs`)
    - ✅ StoreField 模式匹配修复（使用 `..` 忽略额外字段）

### 涉及文件

| 类型 | 文件 | 状态 |
|------|------|------|
| 修改 | `src/frontend/core/parser/ast.rs` | ✅ 已完成 |
| 修改 | `src/frontend/core/parser/statements/declarations.rs` | ✅ 已完成 |
| 修改 | `src/frontend/core/parser/pratt/nud.rs` | ✅ 已完成 |
| 修改 | `src/frontend/core/type_system/mono.rs` | ✅ 已完成 |
| 修改 | `src/frontend/typecheck/inference/patterns.rs` | ✅ 已完成 |
| 修改 | `src/frontend/typecheck/mod.rs` | ✅ 已完成 |
| 修改 | `src/frontend/type_level/auto_derive.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/error.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/mut_check.rs` | ✅ 已完成 |
| 修改 | `src/middle/core/ir_gen.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/codegen/mod.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/mono/cross_module.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/mono/function.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/mono/module_state.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/mono/type_mono.rs` | ✅ 已完成 |
| 修改 | `src/frontend/core/type_system/solver.rs` | ✅ 已完成 |
| 修改 | `src/frontend/core/type_system/substitute.rs` | ✅ 已完成 |
| 修改 | `src/frontend/typecheck/specialization/algorithm.rs` | ✅ 已完成 |
| 修改 | `src/frontend/typecheck/specialize.rs` | ✅ 已完成 |
| 修改 | `src/frontend/typecheck/overload.rs` | ✅ 已完成 |
| 修改 | `src/frontend/typecheck/inference/expressions.rs` | ✅ 已完成 |

### 验收标准

- [x] `type Point { x: Float, mut y: Float }` 语法解析正确
- [x] `type Point(x: Float, mut y: Float)` 命名结构体语法解析正确
- [x] `NamedStruct(Point(x: Float, mut y: Float))` 构造函数支持 mut 字段
- [x] `mut p: Point = Point(1.0, 2.0); p.y = 3.0` 编译通过（绑定可变，字段可变）
- [x] `p.y = 3.0` 在非 mut 绑定下编译通过（绑定不可变，字段可变）
- [x] `p.x = 3.0` 在非 mut 绑定下编译失败（绑定不可变，字段不可变）→ `ImmutableFieldAssign`
- [x] `p.x = 3.0` 在 mut 绑定下编译通过（绑定可变，字段可写）

### 实现说明

1. **数据结构变更**（已完成）
   - `StructField` 结构体：`name, is_mut, ty`
   - `StructType.field_mutability: Vec<bool>`
   - `Pattern::Struct` 字段支持 `is_mut` 标记

2. **Parser 层**（已完成）
   - `parse_struct_type` 支持 `{ x: Float, mut y: Float }`
   - `parse_named_struct_type` 支持 `Point(x: Float, mut y: Float)`

3. **IR 生成**（已完成）
   - 字段赋值 `p.y = value` 生成 `StoreField` 指令
   - `get_field_mutability` 方法查询字段可变性
   - `StoreField` 携带 `type_name` 和 `field_name` 用于检查

4. **MutChecker**（已完成）
   - 绑定级可变性检查：检查变量是否声明为 `mut`
   - 字段级可变性检查：检查字段是否声明为 `mut`
   - 规则：**绑定可变 OR 字段可变** → 允许赋值
   - 架构：传入 `HashMap<String, StructType>` 类型表
   - 添加解析器：`parse_let_stmt` 和 `parse_pattern`
   - IR 生成：`generate_pattern_ir` 处理模式解构

### 待后续优化

（当前 Phase 1 已完成）

---

## Phase 2: 空状态重用 (P1) ✅ 已完成

### 目标

实现 Move 后变量进入 `empty` 状态，可重新赋值复用变量名。

### 实现状态：✅ 已完成（2026-02-05）

#### 已完成变更（2026-02-05 更新）

1. **ValueState 扩展** (`error.rs`)
   - ✅ `ValueState::Owned(Option<TypeId>)` 添加类型追踪
   - ✅ `ValueState::Empty` 新增空状态变体
   - ✅ 添加 `TypeId` 类型标识符
   - ✅ 添加 `EmptyStateTypeMismatch` 和 `ReassignNonEmpty` 错误类型

2. **空状态追踪** (新建 `empty_state.rs`)
   - ✅ 创建 `EmptyStateTracker` 结构体
   - ✅ 实现状态追踪和类型检查
   - ✅ 实现分支状态合并（保守策略）

3. **控制流分析** (新建 `control_flow.rs`)
   - ✅ 创建 `ControlFlowAnalyzer` 结构体
   - ✅ 实现 `merge_states` 保守合并策略
   - ✅ 提供活跃变量分析辅助函数

4. **Move 检查器扩展** (`move_semantics.rs`)
   - ✅ Move 后变量进入 Empty 状态（而非 Moved）
   - ✅ 空状态变量允许重新赋值
   - ✅ 类型一致性检查
   - ✅ 函数调用参数进入 Empty 状态

5. **其他检查器适配**
   - ✅ `clone.rs`: 更新以适配 Empty 状态
   - ✅ `drop_semantics.rs`: Drop Empty 状态合法
   - ✅ `ref_semantics.rs`: 更新以适配 Empty 状态

6. **模块注册** (`mod.rs`)
   - ✅ 注册 `empty_state` 和 `control_flow` 模块

### 涉及文件

| 类型 | 文件 | 状态 |
|------|------|------|
| 修改 | `src/middle/passes/lifetime/error.rs` | ✅ 已完成 |
| 新建 | `src/middle/passes/lifetime/empty_state.rs` | ✅ 已完成 |
| 新建 | `src/middle/passes/lifetime/control_flow.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/move_semantics.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/clone.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/drop_semantics.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/ref_semantics.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/mod.rs` | ✅ 已完成 |

### 验收标准

- [x] `p = Point(1.0); p2 = p; p = Point(2.0)` 编译通过
- [x] `p = Point(1.0); p2 = p; print(p)` 编译失败（UseAfterMove）
- [x] if 分支正确追踪空状态（保守分析）
- [x] `p = "hello"` 在 Point 类型后报错（EmptyStateTypeMismatch）

### 实现说明

1. **状态设计**
   - `Owned(Option<TypeId>)`: 有效值，携带类型信息
   - `Empty`: 空状态，可重新赋值
   - `Moved`: 已移动（保留用于兼容）
   - `Dropped`: 已释放

2. **状态转换**
   ```
   Owned ──Move──► Empty ──(Store, 类型一致)──► Owned
                         ▲
                         │
                    报错：类型不匹配
   ```

3. **保守分支合并**
   - 任一分支为 Empty → 汇合后为 Empty
   - 任一分支为 Moved → 汇合后为 Moved
   - 都是 Owned → 保留第一个

4. **类型检查**
   - 重新赋值时检查类型一致性
   - 类型不匹配时报告 `EmptyStateTypeMismatch`

### 待后续优化

（当前 Phase 2 已完成）

---

## Phase 3: 所有权回流 (P1) ✅ 已完成

### 目标

实现函数参数被修改后返回，形成所有权闭环，支持链式调用。

### 实现状态：✅ 已完成（2026-02-06）

#### 已完成变更（2026-02-06 更新）

1. **消费模式枚举** (`ownership_flow.rs`)
   - ✅ 创建 `ConsumeMode` 枚举：`Returns | Consumes | Undetermined`
   - ✅ `Returns`: 参数在返回值中返回，所有权回流
   - ✅ `Consumes`: 参数被消费，不返回
   - ✅ `Undetermined`: 无法确定（保守分析）

2. **所有权回流分析器** (`ownership_flow.rs`)
   - ✅ 创建 `OwnershipFlowAnalyzer` 结构体
   - ✅ `analyze_function()` 分析函数消费模式
   - ✅ `operand_references_param()` 检查返回值是否引用参数
   - ✅ `returns_param_directly()` 快速检测 `return p;` 模式
   - ✅ 保守估计：临时变量可能引用参数

3. **链式调用分析器** (`chain_calls.rs`)
   - ✅ 创建 `ChainCallAnalyzer` 结构体
   - ✅ `analyze_chain()` 分析方法链所有权流动
   - ✅ `extract_chain_calls()` 提取连续的虚方法调用
   - ✅ `infer_consume_mode()` 基于使用方式推断消费模式
   - ✅ `check_ownership_closure()` 验证所有权闭合

4. **错误类型扩展** (`error.rs`)
   - ✅ 添加 `ConsumedNotReturned` 错误变体
   - ✅ 用于参数被消费但未返回的诊断

5. **模块注册** (`mod.rs`)
   - ✅ 注册 `ownership_flow` 和 `chain_calls` 模块

### 涉及文件

| 类型 | 文件 | 状态 |
|------|------|------|
| 新建 | `src/middle/passes/lifetime/ownership_flow.rs` | ✅ 已完成 |
| 新建 | `src/middle/passes/lifetime/chain_calls.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/error.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/mod.rs` | ✅ 已完成 |

### 验收标准

- [x] `p = p.process()` 推断为 Returns 模式
- [x] `consume(p)` 推断为 Consumes 模式
- [x] `p = p.rotate(90).scale(2.0).translate(1.0)` 链式调用正确
- [x] 回流推断错误给出准确提示

### 实现说明

1. **ConsumeMode 设计**
   ```
   ConsumeMode::Returns     → 参数在返回值中返回
   ConsumeMode::Consumes   → 参数被消费，不返回
   ConsumeMode::Undetermined → 无法确定，保守分析
   ```

2. **参数引用检测**
   - 直接引用：`Operand::Arg(idx)` → 检查索引匹配
   - 临时变量：保守估计可能引用参数
   - 常量/全局：不引用参数

3. **链式调用分析**
   ```ignore
   p.rotate(90)    // Method 1: rotate
     .scale(2.0)   // Method 2: scale (obj = temp_1)
     .translate(1.0); // Method 3: translate (obj = temp_2)
   ```

4. **所有权闭合检查**
   - Consumes 模式 → 所有权正确闭合
   - Returns 模式 → 返回值应该被使用
   - Undetermined → 保守返回 true

### 测试覆盖

| 模块 | 测试数 | 说明 |
|------|--------|------|
| `ownership_flow` | 10 | 参数引用检测、模式推断 |
| `chain_calls` | 13 | 链式调用、所有权闭合 |

### 待后续优化

（当前 Phase 3 已完成）

---

## Phase 4: 消费分析 (P1) ✅ 已完成

### 目标

实现完整的消费标记系统，追踪每个变量的 Consumes/Returns 状态。

### 实现状态：✅ 已完成（2026-02-06）

#### 已完成变更（2026-02-06 更新）

1. **消费分析器** (新建 `consume_analysis.rs`)
   - ✅ 复用 Phase 3 的 `ConsumeMode` 和 `OwnershipFlowAnalyzer`
   - ✅ `ConsumeAnalyzer` 提供跨函数消费模式查询
   - ✅ 内置函数特殊处理（consume, clone 等）
   - ✅ 消费模式缓存机制

2. **生命周期追踪器** (新建 `lifecycle.rs`)
   - ✅ 创建 `LifecycleTracker` 结构体
   - ✅ 变量生命周期事件记录（创建/消费/移动/释放/返回）
   - ✅ 消费次数和读取次数统计
   - ✅ 生命周期问题检测（未消费释放/多次消费/消费后使用）

3. **MoveChecker 扩展** (`move_semantics.rs` 扩展)
   - ✅ 添加 `ConsumeAnalyzer` 字段
   - ✅ `check_call` 根据函数消费模式决定参数状态
   - ✅ Returns 模式：参数所有权回流，不进入 Empty
   - ✅ Consumes 模式：参数进入 Empty

4. **模块注册** (`mod.rs`)
   - ✅ 注册 `consume_analysis` 和 `lifecycle` 模块

### 涉及文件

| 类型 | 文件 | 状态 |
|------|------|------|
| 新建 | `src/middle/passes/lifetime/consume_analysis.rs` | ✅ 已完成 |
| 新建 | `src/middle/passes/lifetime/lifecycle.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/move_semantics.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/mod.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/ownership_flow.rs` | ✅ 已完成 |

### 验收标准

- [x] 赋值/传参/返回正确标记为 Move
- [x] `consume(x)` 后 x 变空（Consumes 模式）
- [x] `x = modify(x)` 推断为 Returns（复用 OwnershipFlowAnalyzer）
- [x] `clone()` 正确复制，不影响原变量（内置函数处理）

### 实现说明

1. **复用 Phase 3 成果**
   - 直接使用 `ownership_flow.rs` 中的 `ConsumeMode` 枚举
   - `OwnershipFlowAnalyzer` 进行函数级消费模式分析

2. **消费分析器设计**
   ```
   ConsumeMode::Returns     → 参数所有权回流，保持 Owned
   ConsumeMode::Consumes   → 参数被消费，进入 Empty
   ConsumeMode::Undetermined → 保守估计进入 Empty
   ```

3. **生命周期追踪**
   ```
   事件：Created → Consumed → Moved → Dropped → Returned
   检测：未消费释放 / 多次消费 / 消费后使用 / 从未使用
   ```

4. **MoveChecker 集成**
   - `check_call` 查询被调用函数的消费模式
   - Returns 模式：参数状态不变
   - Consumes 模式：参数进入 Empty

---

## Phase 5: ref 关键字 = Arc (P1) ✅ 已完成

### 目标

`ref` 关键字实现为 Arc，线程安全引用计数。

### 实现状态：✅ 已完成（2026-02-06）

#### 已完成变更（2026-02-06 更新）

1. **ref 语法解析** (已有)
   - ✅ `parser/expr.rs`: `parse_ref` 解析 `ref expression` 语法
   - ✅ `ast.rs`: `Expr::Ref { expr, span }` AST 节点

2. **类型推断** (已有)
   - ✅ `typecheck/infer.rs`: `ref T` 推断为 `Arc[T]`

3. **所有权处理** (已有)
   - ✅ `ref_semantics.rs`: ArcNew/Clone/Drop 所有权检查

4. **IR 生成** (新增)
   - ✅ `ir_gen.rs`: 添加 `Expr::Ref` → `ArcNew` 指令生成

### 涉及文件

| 类型 | 文件 | 状态 |
|------|------|------|
| 修改 | `src/frontend/core/parser/expr.rs` | ✅ 已有 |
| 修改 | `src/frontend/typecheck/infer.rs` | ✅ 已有 |
| 修改 | `src/middle/passes/lifetime/ref_semantics.rs` | ✅ 已有 |
| 修改 | `src/middle/core/ir_gen.rs` | ✅ 本次新增 |

### 验收标准

- [x] `ref p` 类型推断为 `Arc[Point]`
- [x] `ref p` 不消费 p，p 仍可用
- [x] `spawn(() => print(shared.x))` 编译通过
- [x] `ref` 表达式可嵌套

### 实现说明

1. **IR 生成** (本次实现)
   ```rust
   Expr::Ref { expr, span: _ } => {
       let src_reg = self.next_temp_reg();
       self.generate_expr_ir(expr, src_reg, instructions, constants)?;
       instructions.push(Instruction::ArcNew {
           dst: Operand::Local(result_reg),
           src: Operand::Local(src_reg),
       });
   }
   ```

2. **所有权语义**
   - `ArcNew`: 创建 Arc，不影响原值状态
   - `ArcClone`: 克隆 Arc，不影响原值状态
   - `ArcDrop`: 释放 Arc，不影响原值状态

### 待后续优化

（当前 Phase 5 已完成）

---

## Phase 6: 循环引用检测 (P1) ✅ 已完成

### 目标

跨任务循环引用检测，任务内循环允许。

### 实现状态：✅ 已完成（2026-02-06）

#### 已完成变更（2026-02-06 更新）

1. **错误类型扩展** (`error.rs`)
   - ✅ `IntraTaskCycle` 警告变体（任务内循环，不阻断编译）
   - ✅ `UnsafeBypassCycle` 信息变体（unsafe 绕过记录）
   - ✅ Display 实现

2. **CycleChecker 增强** (`cycle_check.rs`)
   - ✅ 深度限制常量 `MAX_DETECTION_DEPTH = 1`（只检测单层边界）
   - ✅ `unsafe_ranges` 字段追踪 unsafe 块范围
   - ✅ `unsafe_bypasses` 字段记录绕过信息
   - ✅ `is_in_unsafe()` 方法检查位置是否在 unsafe 块内
   - ✅ `find_spawn_result_direct()` 方法实现深度限制
   - ✅ `collect_unsafe_ranges()` 预留 Phase 7 接口
   - ✅ 优化错误消息，包含解决建议

3. **任务内循环追踪器** (新建 `intra_task_cycle.rs`)
   - ✅ `IntraTaskCycleTracker` 结构体
   - ✅ `RefEdge` 结构体追踪 ref 边
   - ✅ `track_function()` 追踪函数内循环
   - ✅ `collect_ref_info()` 收集 ArcNew/Move/StoreField
   - ✅ `build_ref_graph()` 构建引用图
   - ✅ `detect_intra_task_cycles()` DFS 检测循环
   - ✅ 警告模式输出，不阻断编译

4. **OwnershipChecker 集成** (`mod.rs`)
   - ✅ 添加 `intra_task_tracker` 字段
   - ✅ `check_function()` 调用任务内循环追踪
   - ✅ `intra_task_warnings()` 方法返回警告
   - ✅ `unsafe_bypasses()` 方法返回绕过记录

### 涉及文件

| 类型 | 文件 | 状态 |
|------|------|------|
| 修改 | `src/middle/passes/lifetime/error.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/cycle_check.rs` | ✅ 已完成 |
| 新建 | `src/middle/passes/lifetime/intra_task_cycle.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/mod.rs` | ✅ 已完成 |

### 验收标准

- [x] spawn 参数和返回值之间的 ref 循环检测
- [x] 任务内循环不报错（泄漏可控）
- [x] 跨任务循环报错位置准确
- [x] unsafe 块可绕过检测（接口预留，Phase 7 完善）

### 实现说明

1. **深度限制设计**
   - 只检测单层 spawn 边界（深度 = 1）
   - `find_spawn_result_direct()` 最多追踪一层 Move
   - 不递归检测嵌套 spawn 的间接引用

2. **循环检测分工**
   ```
   CycleChecker        → 跨 spawn 循环（报错）
   IntraTaskCycleTracker → 任务内循环（警告）
   ```

3. **unsafe 绕过机制**
   - `collect_unsafe_ranges()` 收集 unsafe 块范围
   - `is_in_unsafe()` 检查指令位置
   - unsafe 块内的 spawn 跳过检测
   - 当前版本预留接口，Phase 7 实现 unsafe 语法后完善

4. **错误消息优化**
   ```
   跨任务循环引用: temp_0 → temp_1 → temp_0 (形成环).
   建议: 使用 Weak 打破循环，或在 unsafe 块中绕过检测
   ```

### 测试覆盖

| 模块 | 测试数 | 说明 |
|------|--------|------|
| `cycle_check` | 22 | 跨任务循环、深度限制、状态重置 |
| `intra_task_cycle` | 7 | 任务内循环、自引用、警告位置 |

### 待后续优化

- Phase 7 实现 unsafe 语法后，完善 `collect_unsafe_ranges()` 解析

---

## Phase 7: unsafe + 裸指针 (P2) ✅ 已完成

### 目标

支持 `unsafe` 块中的 `*T` 裸指针操作。

### 实现状态：✅ 已完成（2026-02-06）

#### 已完成变更（2026-02-06 更新）

1. **关键字和 Token** (`tokens.rs`, `state.rs`)
   - ✅ 添加 `KwUnsafe` 关键字
   - ✅ `state.rs`: 添加 `"unsafe" => Some(TokenKind::KwUnsafe)`

2. **AST 扩展** (`ast.rs`)
   - ✅ `Expr::Unsafe { body: Box<Block>, span }` - unsafe 块表达式
   - ✅ `Type::Ptr(Box<Type>)` - 裸指针类型 `*T`
   - ✅ `UnOp::Deref` - 解引用运算符

3. **Parser 扩展** (`pratt/nud.rs`, `statements/declarations.rs`)
   - ✅ `parse_unsafe()` - 解析 `unsafe { ... }` 语法
   - ✅ `parse_unary()` - 支持 `*expr` 解引用语法
   - ✅ `parse_type_annotation()` - 支持 `*T` 类型注解

4. **IR 指令扩展** (`ir.rs`)
   - ✅ `Instruction::UnsafeBlockStart` - unsafe 块开始标记
   - ✅ `Instruction::UnsafeBlockEnd` - unsafe 块结束标记
   - ✅ `Instruction::PtrFromRef { dst, src }` - `&value → *T`
   - ✅ `Instruction::PtrDeref { dst, src }` - `*ptr → value`
   - ✅ `Instruction::PtrStore { dst, src }` - `*ptr = value`
   - ✅ `Instruction::PtrLoad { dst, src }` - 加载指针

5. **IR 生成** (`ir_gen.rs`)
   - ✅ `Expr::Unsafe` → `UnsafeBlockStart/End` 指令包裹
   - ✅ `UnOp::Deref` → `PtrDeref` 指令

6. **类型系统** (`mono.rs`, `cross_module.rs`, `function.rs`, `module_state.rs`, `type_mono.rs`)
   - ✅ `Type::Ptr` → `MonoType::TypeRef("*{...}")`
   - ✅ 类型名称转换支持裸指针

7. **类型推断** (`expressions.rs`)
   - ✅ `infer_unary()` 支持 `Deref` 类型推断
   - ✅ `infer_expr()` 支持 `Expr::Unsafe` 类型推断

8. **Unsafe 范围收集** (`cycle_check.rs`)
   - ✅ `collect_unsafe_ranges()` 解析 `UnsafeBlockStart/End` 指令

9. **Unsafe 检查器** (新建 `unsafe_check.rs`)
   - ✅ `UnsafeChecker` 结构体
   - ✅ `check_function()` - 检查 unsafe 块外解引用
   - ✅ `UnsafeDeref` 错误类型

10. **错误类型扩展** (`error.rs`)
    - ✅ `OwnershipError::UnsafeDeref` 变体
    - ✅ Display 实现

11. **代码生成** (`translator.rs`)
    - ✅ unsafe 块和指针指令跳过的占位实现

### 涉及文件

| 类型 | 文件 | 状态 |
|------|------|------|
| 修改 | `src/frontend/core/lexer/tokens.rs` | ✅ 已完成 |
| 修改 | `src/frontend/core/lexer/state.rs` | ✅ 已完成 |
| 修改 | `src/frontend/core/parser/ast.rs` | ✅ 已完成 |
| 修改 | `src/frontend/core/parser/pratt/nud.rs` | ✅ 已完成 |
| 修改 | `src/frontend/core/parser/statements/declarations.rs` | ✅ 已完成 |
| 修改 | `src/middle/core/ir.rs` | ✅ 已完成 |
| 修改 | `src/middle/core/ir_gen.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/cycle_check.rs` | ✅ 已完成 |
| 新建 | `src/middle/passes/lifetime/unsafe_check.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/error.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/lifetime/mod.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/codegen/translator.rs` | ✅ 已完成 |
| 修改 | `src/frontend/core/type_system/mono.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/mono/cross_module.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/mono/function.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/mono/module_state.rs` | ✅ 已完成 |
| 修改 | `src/middle/passes/mono/type_mono.rs` | ✅ 已完成 |
| 修改 | `src/frontend/typecheck/inference/expressions.rs` | ✅ 已完成 |

### 验收标准

- [x] `unsafe { ... }` 语法解析正确
- [x] `*T` 裸指针类型注解解析正确
- [x] `*ptr` 解引用语法解析正确
- [x] `unsafe { *ptr }` 编译通过
- [x] unsafe 块外 `*ptr` 报错 `UnsafeDeref`
- [x] 裸指针类型表示为 `*{type}`
- [x] unsafe 块生成 `UnsafeBlockStart/End` IR 标记
- [x] `collect_unsafe_ranges()` 正确收集 unsafe 范围

### 实现说明

1. **AST 设计**
   ```rust
   Expr::Unsafe {
       body: Box<Block>,
       span: Span,
   }
   Type::Ptr(Box<Type>)  // *T
   UnOp::Deref           // *expr
   ```

2. **IR 设计**
   ```
   UnsafeBlockStart
   // 块内指令...
   UnsafeBlockEnd
   ```

3. **解引用类型推断**
   ```rust
   UnOp::Deref => {
       if let MonoType::TypeRef(inner) = expr {
           // 去掉 * 前缀获取内部类型
           let inner_type = inner.trim_start_matches('*').to_string();
           Ok(MonoType::TypeRef(inner_type))
       } else {
           Err(Diagnostic::error("Dereference requires pointer type"))
       }
   }
   ```

4. **裸指针类型表示**
   - 解析：`*T` → `Type::Ptr(Box<Type>)`
   - IR：`PtrFromRef`, `PtrDeref`, `PtrStore`, `PtrLoad`
   - MonoType：`*{type_name}`

### 测试覆盖

| 模块 | 测试数 | 说明 |
|------|--------|------|
| Parser | - | unsafe/deref/ptr 语法解析 |
| TypeCheck | - | 指针类型推断 |
| IR Gen | - | unsafe 块和指针 IR 生成 |
| UnsafeCheck | - | unsafe 块外解引用检查 |

### 待后续优化

- Phase 8+ 实现裸指针的代码生成（wasm 地址操作）
- 添加 `UnsafeBlock` 作用域追踪

---

## Phase 8: Rc/Arc/Weak 标准库 (P1)

### 目标

实现 `std.rc` 和 `std.sync` 模块。

### 实现步骤

1. **Rc/Weak 实现** (新建 `std/rc.rs`)
   - `Rc::new()`, `Rc::clone()`
   - `Weak::new()`, `Weak::upgrade()`
   - 非原子计数，单线程安全

2. **Arc 实现** (新建 `std/sync.rs`)
   - `Arc::new()`, `Arc::clone()`
   - 原子计数，线程安全
   - 自动满足 Send + Sync

### 涉及文件

| 类型 | 文件 |
|------|------|
| 新建 | `src/std/rc.rs` |
| 新建 | `src/std/sync.rs` |

### 验收标准

- [ ] `use std.rc.{Rc, Weak}` 导入正确
- [ ] `rc.clone()` 增加计数
- [ ] `weak.upgrade()` 返回 Some/None
- [ ] `Arc` 使用原子操作

---

## 依赖关系

```
Phase 1 (字段不可变性)
    │
    ├─► Phase 2 (空状态重用)
    │       │
    │       └─► Phase 3 (所有权回流)
    │
    ├─► Phase 4 (消费分析)
    │       │
    │       └─► Phase 5 (ref = Arc)
    │               │
    │               └─► Phase 6 (循环检测)
    │
    ├─► Phase 7 (unsafe + 裸指针)
    │
    └─► Phase 8 (Rc/Arc/Weak)
```

---

## 文件清单

### 新建文件

| 文件 | Phase | 描述 |
|------|-------|------|
| `src/middle/passes/lifetime/empty_state.rs` | P2 | 空状态追踪 |
| `src/middle/passes/lifetime/control_flow.rs` | P2 | 控制流分析 |
| `src/middle/passes/lifetime/ownership_flow.rs` | P3 ✅ | 所有权回流推断 |
| `src/middle/passes/lifetime/chain_calls.rs` | P3 ✅ | 链式调用分析 |
| `src/middle/passes/lifetime/consume_analysis.rs` | P4 ✅ | 消费标记系统 |
| `src/middle/passes/lifetime/lifecycle.rs` | P4 ✅ | 变量生命周期追踪 |
| `src/middle/passes/lifetime/unsafe_check.rs` | P7 | unsafe 检查 |
| `src/middle/passes/lifetime/intra_task_cycle.rs` | P6 ✅ | 任务内循环处理 |
| `src/std/rc.rs` | P8 | Rc/Weak 实现 |
| `src/std/sync.rs` | P8 | Arc 实现 |

### 修改文件

| 文件 | Phase | 修改内容 |
|------|-------|----------|
| `src/frontend/core/parser/ast.rs` | P1 | 创建 StructField，修改 Type/Pattern |
| `src/frontend/core/parser/statements/declarations.rs` | P1 | Parser 支持 mut 字段 |
| `src/frontend/core/parser/pratt/nud.rs` | P1 | 结构体模式解析支持 mut |
| `src/frontend/core/type_system/mono.rs` | P1 | StructType 添加 field_mutability |
| `src/frontend/typecheck/inference/patterns.rs` | P1 | 模式推断支持 is_mut |
| `src/frontend/typecheck/mod.rs` | P1 | 适配 StructField |
| `src/frontend/type_level/auto_derive.rs` | P1 | 适配 StructField |
| `src/frontend/core/type_system/solver.rs` | P1 | 适配 field_mutability |
| `src/frontend/core/type_system/substitute.rs` | P1 | 适配 field_mutability |
| `src/frontend/typecheck/specialization/algorithm.rs` | P1 | 适配 field_mutability |
| `src/frontend/typecheck/specialize.rs` | P1 | 适配 field_mutability |
| `src/frontend/typecheck/overload.rs` | P1 | 适配 field_mutability |
| `src/middle/passes/lifetime/error.rs` | P1 | 添加 ImmutableFieldAssign |
| `src/middle/passes/lifetime/mut_check.rs` | P1 | StoreField 检查扩展 |
| `src/middle/core/ir_gen.rs` | P1 | 适配 StructField |
| `src/middle/passes/codegen/mod.rs` | P1 | 适配 StructField |
| `src/middle/passes/mono/cross_module.rs` | P1 | 适配 field_mutability |
| `src/middle/passes/mono/function.rs` | P1 | 适配 StructField |
| `src/middle/passes/mono/module_state.rs` | P1 | 适配 StructField |
| `src/middle/passes/mono/type_mono.rs` | P1 | 适配 field_mutability |
| `src/middle/passes/lifetime/move_semantics.rs` | P2, P4 ✅ | 空状态检查、消费分析 |
| `src/middle/passes/lifetime/error.rs` | P3 | 回流错误诊断 |
| `src/middle/passes/lifetime/ownership_flow.rs` | P4 | ConsumeMode 添加 Copy |
| `src/frontend/core/parser/expr.rs` | P5 | ref 表达式解析 |
| `src/frontend/typecheck/infer.rs` | P5 | ref 类型推断 |
| `src/middle/passes/lifetime/ref_semantics.rs` | P5 | ref 所有权处理 |
| `src/middle/passes/lifetime/cycle_check.rs` | P6 ✅ | 跨任务循环检测、深度限制、unsafe 绕过 |
| `src/middle/passes/lifetime/error.rs` | P6 ✅ | IntraTaskCycle, UnsafeBypassCycle |
| `src/middle/passes/lifetime/mod.rs` | P6 ✅ | 集成 IntraTaskCycleTracker |
| `src/frontend/core/parser/block.rs` | P7 | unsafe 语法解析 |

---

## 时间估计

| Phase | 复杂度 | 估计工期 |
|-------|--------|----------|
| P1: 字段不可变性 | 中 | 3-4 天 |
| P2: 空状态重用 | 中 | 2-3 天 |
| P3: 所有权回流 | 低 | 1-2 天 |
| P4: 消费分析 | 中 | 2-3 天 |
| P5: ref = Arc | 低 | 1 天（已有基础） |
| P6: 循环检测 | 中 | 2 天（已有基础） |
| P7: unsafe + 裸指针 | 中 | 2-3 天 |
| P8: Rc/Arc/Weak | 中 | 3-4 天 |

**总计**: 约 16-22 个工作日
