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

## Phase 2: 空状态重用 (P1)

### 目标

实现 Move 后变量进入 `empty` 状态，可重新赋值复用变量名。

### 实现步骤

1. **变量状态追踪** (新建 `empty_state.rs`)
   - 定义 `VariableState: Valid | Empty` 枚举
   - 追踪每个变量的状态变化

2. **空状态检查** (`move_semantics.rs` 扩展)
   - 使用 Move 后报错，提示可重新赋值
   - 空状态变量允许重新赋值（类型必须一致）

3. **控制流分析** (新建 `control_flow.rs`)
   - if/match 分支的变量状态合并
   - 跨分支状态追踪

### 涉及文件

| 类型 | 文件 |
|------|------|
| 新建 | `src/middle/passes/lifetime/empty_state.rs` |
| 新建 | `src/middle/passes/lifetime/control_flow.rs` |
| 修改 | `src/middle/passes/lifetime/move_semantics.rs` |

### 验收标准

- [ ] `p = Point(1.0); p2 = p; p = Point(2.0)` 编译通过
- [ ] `p = Point(1.0); p2 = p; print(p)` 编译失败
- [ ] if 分支正确追踪空状态
- [ ] `p = "hello"` 在 Point 类型后报错

---

## Phase 3: 所有权回流 (P1)

### 目标

实现函数参数被修改后返回，形成所有权闭环，支持链式调用。

### 实现步骤

1. **回流推断** (新建 `ownership_flow.rs`)
   - 分析 return 语句中是否包含参数
   - 推断 Consumes / Returns 模式

2. **链式调用支持** (新建 `chain_calls.rs`)
   - 分析方法链的所有权流动
   - 支持 `p.rotate(90).scale(2.0)`

3. **错误诊断** (`lifetime/error.rs`)
   - 参数被消费但未返回的报错
   - 提供正确的使用建议

### 涉及文件

| 类型 | 文件 |
|------|------|
| 新建 | `src/middle/passes/lifetime/ownership_flow.rs` |
| 新建 | `src/middle/passes/lifetime/chain_calls.rs` |
| 修改 | `src/middle/passes/lifetime/error.rs` |

### 验收标准

- [ ] `p = p.process()` 推断为 Returns 模式
- [ ] `consume(p)` 推断为 Consumes 模式
- [ ] `p = p.rotate(90).scale(2.0).translate(1.0)` 链式调用正确
- [ ] 回流推断错误给出准确提示

---

## Phase 4: 消费分析 (P1)

### 目标

实现完整的消费标记系统，追踪每个变量的 Consumes/Returns 状态。

### 实现步骤

1. **消费标记系统** (新建 `consume_analysis.rs`)
   - 定义 `ConsumeMode: Consumes | Returns` 枚举
   - 函数调用时标记参数消费模式

2. **变量生命周期追踪** (新建 `lifecycle.rs`)
   - 追踪变量从创建到消费的完整生命周期
   - 作用域进入/退出时清理

3. **跨函数分析** (`move_semantics.rs` 扩展)
   - 分析函数调用对变量的影响
   - 追踪跨函数的所有权流动

### 涉及文件

| 类型 | 文件 |
|------|------|
| 新建 | `src/middle/passes/lifetime/consume_analysis.rs` |
| 新建 | `src/middle/passes/lifetime/lifecycle.rs` |
| 修改 | `src/middle/passes/lifetime/move_semantics.rs` |

### 验收标准

- [ ] 赋值/传参/返回正确标记为 Move
- [ ] `consume(x)` 后 x 变空
- [ ] `x = modify(x)` 推断为 Returns
- [ ] `clone()` 正确复制，不影响原变量

---

## Phase 5: ref 关键字 = Arc (P1)

### 目标

`ref` 关键字实现为 Arc，线程安全引用计数。

### 实现步骤

1. **ref 语法解析** (`parser/expr.rs`)
   - 解析 `ref expression` 语法
   - 生成 `Expr::Ref` AST 节点

2. **类型推断** (`typecheck/infer.rs`)
   - `ref T` 推断为 `Arc[T]`
   - 自动添加 Send + Sync 约束

3. **所有权处理** (`ref_semantics.rs` 扩展)
   - ref 不消费原值
   - Arc 计数增加逻辑

### 涉及文件

| 类型 | 文件 |
|------|------|
| 修改 | `src/frontend/core/parser/expr.rs` |
| 修改 | `src/frontend/typecheck/infer.rs` |
| 修改 | `src/middle/passes/lifetime/ref_semantics.rs` |

### 验收标准

- [ ] `ref p` 类型推断为 `Arc[Point]`
- [ ] `ref p` 不消费 p，p 仍可用
- [ ] `spawn(() => print(shared.x))` 编译通过
- [ ] `ref` 表达式可嵌套

---

## Phase 6: 循环引用检测 (P1)

### 目标

跨任务循环引用检测，任务内循环允许。

### 实现步骤

1. **任务边界追踪** (`cycle_check.rs` 扩展)
   - 追踪 spawn 的参数和返回值边界
   - 构建跨任务引用图

2. **循环检测算法** (`cycle_check.rs`)
   - 检测参数和返回值之间的 ref 环
   - 报错时给出位置和建议

3. **任务内循环处理** (新建 `intra_task_cycle.rs`)
   - 任务内循环允许，只记录警告
   - 泄漏在任务结束后释放

### 涉及文件

| 类型 | 文件 |
|------|------|
| 新建 | `src/middle/passes/lifetime/intra_task_cycle.rs` |
| 修改 | `src/middle/passes/lifetime/cycle_check.rs` |

### 验收标准

- [ ] spawn 参数和返回值之间的 ref 循环检测
- [ ] 任务内循环不报错（泄漏可控）
- [ ] 跨任务循环报错位置准确
- [ ] unsafe 块可绕过检测

---

## Phase 7: unsafe + 裸指针 (P2)

### 目标

支持 `unsafe` 块中的 `*T` 裸指针操作。

### 实现步骤

1. **unsafe 语法解析** (`parser/block.rs`)
   - 解析 `unsafe { ... }` 块语法
   - 解析 `*Type` 裸指针类型

2. **unsafe 语义检查** (新建 `unsafe_check.rs`)
   - 限制 unsafe 块内的操作
   - 解引用、指针运算必须在 unsafe 内
   - 裸指针不满足 Send + Sync

### 涉及文件

| 类型 | 文件 |
|------|------|
| 修改 | `src/frontend/core/parser/block.rs` |
| 新建 | `src/middle/passes/lifetime/unsafe_check.rs` |

### 验收标准

- [ ] `unsafe { ptr: *Point = &p }` 语法正确
- [ ] unsafe 块外解引用报错
- [ ] 裸指针 Send + Sync 为 false

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
| `src/middle/passes/lifetime/ownership_flow.rs` | P3 | 所有权回流推断 |
| `src/middle/passes/lifetime/chain_calls.rs` | P3 | 链式调用分析 |
| `src/middle/passes/lifetime/consume_analysis.rs` | P4 | 消费标记系统 |
| `src/middle/passes/lifetime/lifecycle.rs` | P4 | 变量生命周期追踪 |
| `src/middle/passes/lifetime/unsafe_check.rs` | P7 | unsafe 检查 |
| `src/middle/passes/lifetime/intra_task_cycle.rs` | P6 | 任务内循环处理 |
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
| `src/middle/passes/lifetime/move_semantics.rs` | P2, P4 | 空状态检查、消费分析 |
| `src/middle/passes/lifetime/error.rs` | P3 | 回流错误诊断 |
| `src/frontend/core/parser/expr.rs` | P5 | ref 表达式解析 |
| `src/frontend/typecheck/infer.rs` | P5 | ref 类型推断 |
| `src/middle/passes/lifetime/ref_semantics.rs` | P5 | ref 所有权处理 |
| `src/middle/passes/lifetime/cycle_check.rs` | P6 | 跨任务循环检测 |
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
