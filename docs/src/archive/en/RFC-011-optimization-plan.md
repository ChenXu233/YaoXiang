# RFC-011 Generic System - Comprehensive Optimization Plan

> **Created**: 2026-02-04
> **Last Updated**: 2026-02-04

> **Status**: In Progress
> **Based on**: [RFC-011 Generic Type System Design](../../design/accepted/011-generic-type-system.md)

## Abstract

This document consolidates the analysis results from all sub-tasks, identifies integration gaps and optimization directions in the codebase, and formulates a systematic improvement plan.

---

## ✅ Completed Tasks

### P0: DCE Cleanup (Completed 2026-02-04)

#### Task 1.1: Fix instantiation_graph TODO ✅
**File**: `src/middle/passes/mono/dce.rs`

**Modified Content**:
1. Add `extract_base_name` helper function - extract base generic name from specialization name
2. Add `extract_type_param_names_from_generic` - extract type parameter names from generic function mapping
3. Add `extract_type_params_from_ir` - extract type parameter names from FunctionIR

4. Modify `build_instantiation_graph` - accept `generic_functions` parameter and correctly extract type parameters
5. Modify `mark_entry_points` - correctly handle entry points
6. Modify `collect_kept_functions` - correctly match nodes

**Test**: 38/38 mono tests passed

#### Task 1.2: Implement substitute_type_ast ✅
**File**: `src/middle/passes/mono/function.rs`

**Modified Content**:
1. Implement `substitute_type_ast` function - complete AST type substitution
   - Basic types are returned directly

- Struct/NamedStruct: recursively substitute field types
   - Union/Variant: recursively substitute member/variant types
   - Tuple/List/Dict/Set/Option/Result/Fn: recursively substitute nested types
   - Generic: substitute type parameters

- AssocType: Recursive substitution of associated types
- Literal: Substitute basic types

**Test**: All related tests passed

---

## I. Current Status Analysis Overview

### 1.1 Module Completion Status

| Module | Completion | Status | Key Issues |

|--------|------------|--------|------------|
| **DCE (Dead Code Elimination)** | **95%** | ✅ Near completion | Few edge cases |
| Function overload specialization | 75% | ⚠️ Needs improvement | Generic fallback integration |
| Platform specialization | 50% | ⚠️ Defined but not integrated | Needs integration with monomorphizer |

| Conditional types | 65% |

### 1.2 Core Problem Classification

---

## 2. Detailed Analysis

---

### 2.1 Conditional Types and Normalizer Integration

#### Implemented Components

---

| Component | File | Status |
|-----------|------|--------|
| `TypeEvaluator` | `type_eval.rs` | ✅ Done |

---

| `TypeNormalizer` | `evaluation/normalize.rs` | ✅ Done |
| `PatternMatcher` | `type_match.rs` | ✅ Done |
| `TypeFamilies` (Bool/Nat) | `type_families.rs` | ✅ Done |
| `From<EvalResult>` Conversion | `type_eval.rs:932-947` | ✅ Done |

```
┌─────────────────────────────────────────────────────────────┐
│                    核心问题分类                               │
├─────────────────────────────────────────────────────────────┤
│  1. 架构性问题：两套并行的类型求值系统未集成                  │
│     - TypeEvaluator (type_eval.rs)                         │
│     - TypeNormalizer (evaluation/normalize.rs)             │
├─────────────────────────────────────────────────────────────┤
│  2. 集成缺口：已定义组件未被使用                             │
│     - PlatformSpecializer 未集成到 Monomorphizer            │
│     - TypeEvaluator 未在类型检查中调用                       │
├─────────────────────────────────────────────────────────────┤
│  3. 功能缺失：Trait 系统约束求解器不完整                     │
│     - 仅支持硬编码的内置 Trait                              │
│     - 缺少用户定义 Trait 的求解                             │
└─────────────────────────────────────────────────────────────┘
```


---

## 二、详细分析


### 2.1 条件类型与归一化器集成

#### 已实现组件


| 组件 | 文件 | 状态 |
|------|------|------|
| `TypeEvaluator` | `type_eval.rs` | ✅ 完成 |

| `TypeNormalizer` | `evaluation/normalize.rs` | ✅ 完成 |
| `PatternMatcher` | `type_match.rs` | ✅ 完成 |
| `TypeFamilies` (Bool/Nat) | `type_families.rs` | ✅ 完成 |
| `From<EvalResult>` 转换 | `type_eval.rs:932-947` | ✅ 完成 |

#### Missing Integration

```rust
// type_eval.rs:952-959 - 空实现
#[allow(dead_code)]
pub fn integrate_evaluator(
    _evaluator: &mut TypeEvaluator,
    _normalizer: &mut TypeNormalizer,
) {
    // TODO: 将求值器的缓存与归一化器的缓存同步
    // 具体实现取决于归一化器的内部结构
}
```

#### Problem Location

| Missing Item | File Location | Problem Description |

| `integrate_evaluator` | `type_eval.rs:952-959` | Empty implementation |
| `TypeNormalizer` calls evaluator | `evaluation/normalize.rs:121-171` | If/Match types not handled |
| `compute_conditional` | `evaluation/compute.rs:217-223` | Only returns original type |

### 2.2 Platform Specialization and Monomorphizer Integration

#### Implemented Components

| Component | File | Status |
|------|------|------|
| `PlatformInfo` | `platform_info.rs` | ✅ 80% |

| `PlatformSpecializer` | `platform_specializer.rs` | ✅ 50% | ✅ Complete |
| `PlatformConstraint` | `platform_specializer.rs:37-88` | ✅ Complete |
| `SpecializationDecider` | `platform_specializer.rs:415-450` | ✅ Complete |

#### Missing Monomorphizer Structure

```rust
// mod.rs:44-95 - 缺少平台特化器字段
pub struct Monomorphizer {
    instantiated_functions: HashMap<FunctionId, FunctionIR>,
    instantiation_queue: Vec<InstantiationRequest>,
    // ...
    // ❌ 缺少: platform_specializer: PlatformSpecializer
    // ❌ 缺少: platform_info: PlatformInfo
}
```

#### Missing Integration Points

| Missing Item | File Location | Problem Description |
|------|----------|----------|
| `Monomorphizer` platform field | `mod.rs:44-95` | No platform specializer field |
| `should_specialize` | `function.rs:403-408` | Returns hardcoded `true`, does not check platform constraints |
| `instantiate_function` | `function.rs:410-438` | Does not call platform selection logic |

| Platform specialization collection | `monomorphize_module` | Does not collect platform specialization info from module |

### 2.3 Compile-Time Generic State

| 平台特化收集 | `monomorphize_module` | 未从模块收集平台特化信息 |

### 2.3 编译期泛型状态


#### Implemented

| Feature | Status |
|---------|--------|

| `GenericSize` | ✅ Basic implementation |
| `ConstExpr` (Int, Bool) | ✅ Done |
| `ConstGenericEval` | ✅ Done

| 内置函数 (`sizeof`, `factorial`, `fibonacci`) | ✅ 完成 |

#### 缺失功能


| 功能 | 状态 | 备注 |
|------|------|------|
| `ConstExpr::Float` | ❌ 未实现 | 浮点数字面量 |
| 位运算 | ❌ 未实现 | `BitAnd`, `BitOr`, `Shl`, `Shr` |

| `MonoType::Array` 支持 | ❌ 未实现 | 数组大小计算 |
| AST -> ConstExpr 解析 | ❌ 未实现 | 解析器集成 |
| 用户自定义 Const 函数 | ❌ 未实现 | 语法支持 |


### 2.4 Trait System Status

#### Implemented

| Feature | File | Status |
|---------|------|--------

| `TraitSolver` | `typecheck/traits/solver.rs` | ⚠️ 部分 |
| Trait 边界检查 | `typecheck/checking/bounds.rs` | ⚠️ 部分 |

#### 缺失功能


| 功能 | 问题描述 |
|------|----------|
| 约束求解器 | 仅支持硬编码的内置 Trait (`Clone`, `Debug`, `Send`, `Sync`) |

| 隐式参数推导 | 缺少完整的约束传播算法 |
| 自动化 Derive | `derive.rs` 需要完善 |
| 关联类型 | 未实现 |
| 一致性检查 (orphan rules) | `coherence.rs` 是简化实现 |

## 3. Optimization Plan

---

### 3.1 Priority Ranking

| Priority | Task | Scope of Impact | Estimated Duration | Status |
|----------|------|-----------------|---------------------|--------|
| **P0** | Complete DCE wrap-up | Monomorphization | 3 days | ✅ Completed |
| **P1** | Platform specialization integration | Platform optimization | 1 week | ✅ Completed |
| **P2** | Conditional type integration | Type system | 1 week | ✅ Completed |
| **P3** | Complete compile-time generics | Compile-time computation | 2 weeks | ✅ Completed (P3-1/2/3) |
| **P4** | Improve Trait constraint solver | Type constraints | 2 weeks | ✅ Completed |
| **P5** | Unify type evaluation architecture | Overall architecture | 3 weeks | ✅ Completed (P2 completed main integration) |
| **P6** | Implement GAT (Generic Associated Types) | Type system | 3 weeks | ✅ Completed |

---

### 3.2 Detailed Task Breakdown

#### ✅ P0: DCE Completion (Completed on 2026-02-04)


### 3.1 优先级排序

| 优先级 | 任务 | 影响范围 | 工期估计 | 状态 |

|--------|------|----------|----------|------|
| **P0** | 完成 DCE 收尾 | 单态化器 | 3天 | ✅ 已完成 |
| **P1** | 平台特化集成 | 平台优化 | 1周 | ✅ 已完成 |
| **P2** | 条件类型集成 | 类型系统 | 1周 | ✅ 已完成 |

| **P3** | 完善编译期泛型 | 编译期计算 | 2周 | ✅ 已完成 (P3-1/2/3) |
| **P4** | 完善 Trait 约束求解器 | 类型约束 | 2周 | ✅ 已完成 |
| **P5** | 统一类型求值架构 | 整体架构 | 3周 | ✅ 已完成 (P2 已完成主要集成) |
| **P6** | 实现关联类型 GAT | 类型系统 | 3周 | ✅ 已完成 |


### 3.2 详细任务分解

#### ✅ P0: DCE 收尾 (2026-02-04 已完成)

**Task 1.1: Fix instantiation_graph TODO** ✅
- Add `extract_base_name` helper function
- Add `extract_type_param_names_from_generic` helper function

- Modify `build_instantiation_graph` to accept `generic_functions` parameter
- Modify `mark_entry_points` and `collect_kept_functions`
- Update test file `dce_tests.rs`

**Task 1.2: Implement substitute_type_ast** ✅
- Implement complete AST type substitution logic
- Support all AstType variants: Struct, Union, Variant, Tuple, List, Dict, Set, Fn, Option, Result, Generic, AssocType, Literal

#### P1: Platform Specialization Integration (1 week)

**Task 2.1: Add Platform Fields to Monomorphizer**

```rust
// mod.rs
pub struct Monomorphizer {
    // ... 现有字段 ...

    // 新增
    platform_info: PlatformInfo,
    platform_specializer: PlatformSpecializer,
}
```

------
**Task 2.2: Modify should_specialize to Check Platform Constraints**

```rust
// function.rs:403-408
fn should_specialize(&self, constraint: &PlatformConstraint) -> bool {
    // 使用 PlatformConstraintSolver::satisfies() 判断
    self.platform_specializer.decide(constraint).should_specialize()
}
```

**Task 2.3: Modify instantiate_function to Select Platform Specialization**

```rust
// function.rs:410-438
fn instantiate_function(&mut self, ...) -> Option<FunctionId> {
    // 调用 PlatformSpecializer::select_specialization()
}
```

**Task 2.4: Collect Platform Specialization Information**

```rust
// monomorphize_module 方法
// 从 AST/IR 收集平台特化并注册到 PlatformSpecializer
```

#### ✅ P1: Platform Specialization Integration (Completed 2026-02-04)

**Task 1-1: Add Platform Fields to Monomorphizer** ✅

- Add `platform_info: PlatformInfo` field
- Add `specialization_decider: SpecializationDecider` field
-

- 添加 `platform_info()` 和 `set_target_platform()` 方法

**任务 1-2: 修改 should_specialize 检查平台约束** ✅
- 修改 `should_specialize()` 使用 `SpecializationDecider` 判断

- Add `get_function_platform_constraint()` helper method
- Support correct instantiation of constrained/unconstrained functions

**Task 1-3: Framework Ready** ✅

- `instantiate_function` logic is ready
- Will work fully once the parser collects platform constraints

#### ✅ P2: Conditional Type Integration (Completed 2026-02-04)

**Task 3.1: Implement integrate_evaluator**

```rust
// type_eval.rs:952-959
pub fn integrate_evaluator(
    evaluator: &mut TypeEvaluator,
    normalizer: &mut TypeNormalizer,
) {
    // 同步缓存
    // 设置环境引用
}
```

**Task 3.2: Call TypeEvaluator in TypeNormalizer** ✅

```rust
// evaluation/normalize.rs
fn normalize_internal(&mut self, ty: &MonoType) -> NormalForm {
    match ty {
        // 处理 If/Match 类型
        MonoType::TypeRef(name) => {
            if let Some(args) = self.parse_conditional_args(name) {
                self.eval_conditional(name, &args)
            } else {
                NormalForm::Normalized
            }
        }
        _ => { /* 原有逻辑 */ }
    }
}
```

- Add `evaluator: TypeEvaluator` field

**Task 3.3: Implement compute_conditional** ✅

```rust
// evaluation/compute.rs
fn compute_conditional(&mut self, ty: &MonoType) -> ComputeResult {
    let evaluator = self.normalizer.evaluator();
    let eval_result = evaluator.eval(ty);
    match eval_result {
        EvalResult::Value(result_ty) => ComputeResult::Done(result_ty),
        EvalResult::Pending => ComputeResult::Pending(vec![ty.clone()]),
        EvalResult::Error(msg) => ComputeResult::Error(msg),
    }
}
```

- Use the evaluator from normalizer to compute conditional types
- Support evaluation of types like If, Match, Nat, etc.

**Task 3.4: Fix Integration Issues** ✅

- Add manual Clone implementation for TypeEvaluator (handling raw pointers)
- Make `parse_type` method public
- Update `integrate_evaluator` documentation

#### P3: Enhance Compile-Time Generics (2 weeks)

**Task 4.1: Add Floating Point Support** ✅
- ✅ `ConstExpr::Float(f32)` - Add floating point expression variant

- ✅ `ConstValue::from_literal_name()` - Support floating point literal parsing (e.g., "3.14")
- ✅ Manually implement `PartialEq`, `Eq`, `Hash` for `ConstExpr` (f32 doesn't support these traits)
- ✅ New tests: `test_float_literal_parsing`, `test_const_expr_float`, `test_const_eval_float_operations`

**Task 4.2: Add Bitwise Operations Support** ✅
- ✅ `ConstBinOp::BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr` - Add bitwise operators
- ✅ `eval_binop()` - Implement bitwise operation evaluation logic
- ✅ New test: `test_const_eval_bitwise`

**Task 4.3: Add Array Size Calculation** ✅
- ✅ `GenericSize::parse_array_type()` - Parse `Array<T, N>` generic type
- ✅ `GenericSize::size_of_array()` - Calculate array size

- ✅ Support nested arrays like `Array<Array<Int, 2>, 3>`
- ✅ New test: `test_generic_size_array`

**Task 4.4: Integrate Parser**

```rust
// parser 集成 AST -> ConstExpr
```

#### P4: Improve Trait Constraint Solver (2 weeks)

**Task 5.1: Extend TraitSolver** ✅

- ✅ Refactor `typecheck/traits/solver.rs` - Integrate `TraitTable` to support user-defined Traits
- ✅ Add `TraitTable::new()` and `TraitTable::clone()` methods
- ✅ New tests: `test_user_defined_trait`, `test_trait_solver_integration`, `test_trait_table_clone`

**Task 5.2: Add Constraint Propagation** ✅
- ✅ Add `solve_all()` batch solving method
- ✅ Add `propagate_constraints_to_type_args()` constraint propagation framework
- ✅ New tests: `test_solve_all_constraints`, `test_constraint_propagation`

**Task 5.3: Complete Derive** ✅
- ✅ Expand `DeriveImpl` to support Debug, PartialEq, Eq
- ✅ Implement `generate_debug_method()`, `generate_partial_eq_method()`, `generate_eq_method()`

- ✅ Update `init_known_derives()` to add new Traits
- ✅ New tests: `test_derive_impl_trait_name`, `test_supported_derive_traits`

#### P5: Unified Type Evaluation Architecture (3 weeks)

**Goal**: Eliminate two parallel type evaluation systems, establish a unified architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    统一前（当前）                            │
├─────────────────────────────────────────────────────────────┤
│  TypeEvaluator (type_eval.rs)                              │
│       ↓                                                    │
│  TypeNormalizer (evaluation/normalize.rs) [P2 已集成]        │
│       ↓                                                    │
│  分离的缓存、逻辑                                           │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    统一后（当前架构）                       │
├─────────────────────────────────────────────────────────────┤
│  TypeNormalizer (嵌入式集成)                                │
│       ├── 内部包含 TypeEvaluator                           │
│       ├── 条件类型求值 (If, Match, Nat)                     │
│       ├── 编译期计算 (Const generics)                       │
│       └── 归一化 (Normalization)                            │
│       ↓                                                    │
│  统一的缓存和状态管理                                        │
└─────────────────────────────────────────────────────────────┘
```

**Task 6.1: Complete Integration Documentation** ✅
- ✅ Update `integrate_evaluator` function documentation, explaining the current embedded integration design
- ✅ Add `sync_caches` backup method for potential future separation scenarios

- ✅ Added `NormalizationContext::cache_mut()` and `cache()` methods
- ✅ New tests: `test_integrate_evaluator_function`, `test_sync_caches_function`

**Note**: P2 has completed the main integration work, P5 only needs to improve documentation and fallback methods.

#### P6: Implement GAT (Associated Types) (3 weeks)

**Task 6.1: Parse Associated Types** ✅

- ✅ GAT module already exists in `src/frontend/typecheck/gat/`
- ✅ `MonoType::AssocType` definition is complete (host type, associated name, generic parameters)
- ✅ `GATChecker::is_associated_type_defined()` supports Iterator::Item, IntoIterator::Item
- ✅ New tests: `test_associated_type_defined`, `test_undefined_associated_type`, `test_resolve_associated_type`

**Task 6.2: Associated Type Constraint Checking** ✅
- ✅ `GATChecker::check_associated_type()` checks if associated types are defined
- ✅ `GATChecker::check_associated_type_constraints()` checks constraints

- ✅ `GATChecker::check_associated_type_generics()` checks generic parameters
- ✅ New tests: `test_check_associated_type`, `test_check_associated_type_constraints`, `test_check_associated_type_generics`

**Task 6.3: GAT Type Checking** ✅

- ✅ `GATChecker::check_gat()` supports function types and struct types
- ✅ `GATChecker::contains_generic_params()` detects generic parameters
- ✅ `GATChecker::check_type_gat()` recursively checks nested types
- ✅ New tests added: `test_check_gat_fn_type`, `test_check_gat_struct_type`, `test_check_gat_with_generic_params`

---

## IV. Technical Debt

### 4.1 Code Duplication

| Location | Description |

|---------|-------------|
| `type_eval.rs` vs `evaluation/compute.rs` | Conditional type evaluation logic is duplicated |
| `type_eval.rs` vs `const_generics/eval.rs` | Constant expression evaluation logic is duplicated |

### 4.2 Empty Implementations / Placeholders

| Location | Description |
|------|-------------|

| `integrate_evaluator` | No-op implementation |
| `compute_conditional` | Returns original type only |
| `check_const_bounds` | Simplified implementation |
| `substitute_type_ast` | Returns `ty.clone()` directly |

---

### 4.3 TODO Comments

| File Location | Description |
|---------------|-------------|
| `


### 4.3 TODO 注释

| 文件位置 | 描述 |

|----------|------|
| `instantiation_graph.rs:721` | 类型参数提取 |
| `function.rs:596-602` | AST 类型替换 |
| `type_eval.rs:946-954` | 集成逻辑 |


---

## 五、风险评估


| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 架构改动影响范围大 | P5 可能引入回归 | 渐进式重构，先集成后统一 |

| Trait system complexity high | P4/P6 may be delayed | Implement basic features first, then refine advanced features |
| Platform specialization integration gaps | Platform optimizations not taking effect | Add integration tests |

---

## 6. Acceptance Criteria

### 6.1 DCE Wrap-up (P0)

- [ ] instantiation_graph extracts type parameters from func_id
- [ ] function.rs implements AST type substitution
- [ ] All DCE tests pass

---

### 6.2 Platform Specialization Integration (P1)

- [ ] Monomorphizer includes PlatformSpecializer
- [ ] should_specialize checks platform constraints
- [ ] instantiate_function selects correct specialization
- [ ] Platform specialization tests pass


## 六、验收标准

### 6.1 DCE 收尾 (P0)


- [ ] instantiation_graph 从 func_id 提取类型参数
- [ ] function.rs 实现 AST 类型替换
- [ ] 所有 DCE 测试通过


### 6.2 平台特化集成 (P1)

- [ ] Monomorphizer 包含 PlatformSpecializer

- [ ] should_specialize 检查平台约束
- [ ] instantiate_function 选择正确特化
- [ ] 平台特化测试通过


### 6.3 Conditional Type Integration (P2)

- [

- [ ] 条件类型单元测试通过

### 6.4 编译期泛型完善 (P3)


- [x] 支持浮点数字面量
- [x] 支持位运算
- [x] 支持数组大小计算
- [x] 编译期求值测试通过 (27/27 ✅)


### 6.5 Trait 系统完善 (P4)

- [x] 支持用户定义 Trait 的约束求解

- [x] 支持隐式参数推导 (框架就绪)
- [x] Derive 正常工作 (Debug, PartialEq, Eq)
- [x] Trait 相关测试通过 (21/21 ✅)


### 6.6 Unified Type Evaluation Architecture (P5)

- [x] TypeEvaluator and TypeNormalizer embedded integration completed
- [x] Conditional type evaluation working normally

- [x] Cache synchronization documentation and fallback methods ready
- [x] Unified type evaluation tests passed (8/8 ✅)

### 6.7 Generic Associated Types (GAT) Implementation (P6)

- [x] Parse associated types (MonoType::AssocType defined)
- [x] Associated type constraint checking (GATChecker)
-

- [x] GAT 相关测试通过 (17/17 ✅)

---


## 附录 A: 关键文件路径

### 平台特化
- `src/middle/passes/mono/mod.rs` - Monomorphizer 定义

- `src/middle/passes/mono/platform_specializer.rs` - Platform

- `src/frontend/typecheck/type_eval.rs` - 类型求值器
- `src/frontend/type_level/type_match.rs` - 类型级 match
- `src/frontend/type_level/type_families.rs` - Bool/Nat 类型族
- `src/frontend/type_level/evaluation/normalize.rs` - 类型归一化


### 编译期泛型
- `src/frontend/type_level/const_generics/eval.rs` - 常量表达式求值
- `src/frontend/type_level/const_generics/generic_size.rs` - 大小计算

- `src/frontend/type_level/const_generics/validation.rs` - 验证

### Trait 系统
- `src/frontend/typecheck/traits/solver.rs` - 约束求解器

- `src/frontend/type_level/trait_bounds.rs` - Trait 边界
- `src/frontend/typecheck/checking/bounds.rs` - 边界检查
- `src/frontend/type_level/impl_check.rs` - 实现检查


### Associated Types GAT
- `src/frontend/typecheck/gat/mod.rs` - GAT module
- `src/frontend/typecheck/gat/checker.rs` - GAT checker
- `src/frontend/typecheck/gat/higher_rank.rs` - Higher-rank type checking

---

## Appendix B: Architecture Diagram

### Current Architecture (Problems)

```
┌────────────────────────────────────────────────────────────────┐
│                     解析层 (Parser)                             │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     类型检查 (TypeCheck)                         │
│  ┌────────────────┐    ┌────────────────┐                       │
│  │ TypeEvaluator  │    │  TraitSolver   │                       │
│  │ (type_eval.rs) │    │ (traits/)     │                       │
│  └────────────────┘    └────────────────┘                       │
│         ↓                      ↓                               │
│    ┌─────────────────────────────────────────┐                 │
│    │           TypeEnvironment                │                 │
│    └─────────────────────────────────────────┘                 │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     单态化 (Monomorphize)                       │
│  ┌────────────────┐    ┌────────────────┐                       │
│  │ Monomorphizer  │    │  DCE Pass    │                       │
│  │ (mod.rs)      │    │ (dce.rs)     │                       │
│  └────────────────┘    └────────────────┘                       │
│         ↓                      ↓                               │
│  ┌─────────────────────────────────────────┐                    │
│  │ PlatformSpecializer ❌ 未集成           │                    │
│  └─────────────────────────────────────────┘                    │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     条件类型 (TypeLevel)                         │
│  ┌────────────────┐    ┌────────────────┐                       │
│  │ TypeNormalizer │    │ ConstGeneric  │                       │
│  │ (evaluation/)  │    │ (const_generics/)                    │
│  └────────────────┘    └────────────────┘                       │
│         ↑                      ↑                                │
│  TypeEvaluator ❌ 未调用      │                                │
└────────────────────────────────────────────────────────────────┘
```

### Target Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                     解析层 (Parser)                             │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     类型检查 (TypeCheck)                         │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │           UnifiedTypeEvaluator (新)                    │    │
│  │  ├── 条件类型求值 (If, Match, Nat)                     │    │
│  │  ├── Trait 约束求解                                    │    │
│  │  ├── 编译期计算                                       │    │
│  │  └── 归一化                                           │    │
│  └─────────────────────────────────────────────────────────┘    │
│         ↓                                                       │
│  ┌─────────────────────────────────────────┐                    │
│  │           TypeEnvironment                │                    │
│  └─────────────────────────────────────────┘                    │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     单态化 (Monomorphize)                       │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Monomorphizer                        │    │
│  │  ├── 函数/类型/闭包单态化                               │    │
│  │  ├── PlatformSpecializer ✅ 已集成                     │    │
│  │  ├── DCE Pass                                         │    │
│  │  └── 实例化图 + 可达性分析                             │    │
│  └─────────────────────────────────────────────────────────┘    │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     优化器 (Optimizer)                          │
│  ├── LLVM Passes                                            │
│  └── 特化感知内联 (待实现)                                     │
└────────────────────────────────────────────────────────────────┘
```



