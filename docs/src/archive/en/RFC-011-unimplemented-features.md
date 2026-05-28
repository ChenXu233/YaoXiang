# RFC-011 Generic System - Not Implemented Features List

> **Created**: 2026-02-03
> **Last Updated**: 2026-02-04

> **Status**: In Progress
> **Based on RFC**: [RFC-011 Generic System Design](../accepted/011-generic-type-system.md)

## Abstract

This document records the completed and unimplemented functional modules in the RFC-011 generic system design. Based on analysis of the compiler implementation, the current system's capability boundaries and areas requiring improvement are identified.

---

## Implementation Status Overview

| Phase | Feature Module | Status | Completion | Key Files |

|-------|---------------|--------|------------|-----------|
| Phase 1 | Basic Generics | ✅ Partial Implementation | 70% | `src/middle/passes/mono/mod.rs` |
| Phase 2 | Type Constraint System | ⚠️ Basic Structure | 30% | `src/frontend/type_level/` |
| Phase 3 | Associated Types | ⚠️ Basic Structure | 5% | `src/frontend/typecheck/gat/` |

| Phase 4 | Compile-time Generics | ⚠️ Basic Structure | 40% | `src/frontend/type_level/const_generics/` |
| Phase 5 | Conditional Types | ✅ Basic Implementation | 65% | `src/frontend/type_level/type_match.rs` |
| - | Function Overload Specialization | ✅ Implemented | 75% | `src/frontend/typecheck/overload.rs` |
| - | Platform-specific Optimization | ⚠️ Basic Implementation | 50% | `src/middle/passes/mono/platform_specializer.rs` |

| - | Complete DCE | ✅ Partial Implementation | 90% | `src/middle/passes/mono/` |

---

---
## Unimplemented Features Details

### 1. Function Overload

## 未实现功能详述

### 1. 函数重载特化机制


#### 1.1 功能描述

RFC-011 设计使用**函数重载**实现特化：

```yaoxiang
# 具体类型特化
sum: (arr: Array[Int]) -> Int = (arr) => {
    native_sum_int(arr.data, arr.length)
}

sum: (arr: Array[Float]) -> Float = (arr) => {
    simd_sum_float(arr.data, arr.length)
}

# 泛型实现（自动选择）
sum: [T](arr: Array[T]) -> T = (arr) => { ... }
```


#### 1.2 当前状态

- ✅ 数据结构支持重载 (`instance.rs`)

- ✅ Overload resolver module exists (`overload.rs`)
- ✅ Type environment supports overload candidate storage
- ✅ Function call overload resolution integrated (`expressions.rs`)
- ⚠️ Generic fallback integration (pending improvement)

#### 1.3 Required Implementation

```
src/frontend/typecheck/overload.rs              # ✅ 重载解析器（完成）
src/frontend/typecheck/mod.rs                  # ✅ 类型环境扩展（完成）
src/middle/passes/mono/instance.rs             # ✅ 实例化ID扩展（完成）
src/frontend/typecheck/inference/expressions.rs  # ✅ 重载解析集成（完成）
src/frontend/typecheck/checking/mod.rs          # ✅ BodyChecker扩展（完成）
```

#### 1.4 Acceptance Criteria

- [x] Can parse different type signatures of functions with the same name (data structure support)
- [x] Automatically select optimal match based on argument types at call site (integrated)
- [x] Compile error: ambiguous call or no matching definition (implemented)
- [x] Integration with generic system: generics as fallback (completed)

### 2. Platform-Specific Optimization

---

### 2. 平台特定优化


#### 2.1 Feature Description

RFC-011 designs support for platform specialization through predefined generic parameter `P` (without using `#[cfg]`):

```yaoxiang
# 通用实现（所有平台可用）
sum: [T: Add](arr: Array[T]) -> T = { ... }

# 平台特化：P 是预定义泛型参数，代表当前平台
sum: [P: X86_64](arr: Array[Float]) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: [P: AArch64](arr: Array[Float]) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

#### 2.2 Current Status

- ✅ `platform_info.rs` implemented (80%)

- TargetPlatform: X86_64, AArch64, RiscV64, Arm, X86, Wasm32
  - PlatformDetector: Detects from target triple/environment variables
  - Predefined generic parameter `P` support

- ✅ `platform_specializer.rs` implemented (50%)
  - PlatformConstraint: `[P: X86_64]` constraint
  - PlatformSpecializer: Platform specialization selection
  - Supports multi-platform specialization version registration and selection

- ❌ No `#[cfg]` attribute parsing (RFC design does not use this approach)
- ❌ Platform specialization integration with monomorphizer (pending implementation)
- ❌ Platform-aware code generation (pending implementation)

#### 2.3 Required Implementation

```
src/frontend/core/parser/attr.rs                    # 可选：属性解析（RFC设计不使用 #[cfg]）
src/middle/passes/mono/platform_info.rs             # ✅ 已实现
src/middle/passes/mono/platform_specializer.rs      # ✅ 已实现
src/middle/passes/mono/mod.rs                       # 修改：集成平台特化
```

#### 2.4 Acceptance Criteria

- [x] Can detect target platform (X86_64, AArch64, etc.)

- [x] Predefined generic parameter `P` recognition
- [ ] Platform specialization correctly integrated with monomorphizer
- [ ] Only generate specialized code matching the current platform
- [ ] Automatically select specialized version based on target platform at compile time

---

### 3. Complete Dead Code Elimination (DCE) Implementation

#### 3.1 Feature Description

RFC-011 designs multi-level DCE:

1. **Instantiation Graph Analysis**: Build a generic instantiation dependency graph and perform reachability analysis from entry points
2. **Usage Point Analysis**: Only instantiate generics that are actually called
3. **Cross-module DCE**: Analyze inter-module dependencies and eliminate unused exports

4. **LLVM-level DCE**: Utilize LLVM's optimization passes

```rust
// 编译器内部：构建泛型实例化依赖图
struct InstantiationGraph {
    nodes: HashMap<InstanceKey, InstanceNode>,
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}
```

#### 3.2 Current Status

- ✅ Basic monomorphizer exists (`mono/mod.rs`)

- ✅ On-demand specialization basic implementation
- ✅ Instantiation graph construction completed (`instantiation_graph.rs`)
- ✅ Complete reachability analysis completed (`reachability.rs`)
- ⚠️ Cross-module DCE basic implementation (needs production environment validation)

- ✅ Code bloat control completed (`dce.rs`)

#### 3.3 Implementation Needed

```rust
// 新增模块
src/middle/passes/mono/instantiation_graph.rs      # 实例化图构建
src/middle/passes/mono/reachability.rs              # 可达性分析
src/middle/passes/mono/cross_module_dce.rs          # 跨模块DCE
src/middle/passes/mono/code_bloat_control.rs        # 代码膨胀控制
```

#### 3.4 Acceptance Criteria

- [x] Build complete instantiation dependency graph

- [x] Perform reachability analysis from main entry point
- [x] Eliminate unused generic instances
- [x] Cross-module dependency analysis (production environment verification)
- [x] Code bloat threshold control

- [x] Statistics output (detailed version + JSON format)

---

### 4. Complete Trait System

#### 4.1 Feature Description

RFC-011 designs a type constraint system (similar to Rust Trait):
------

```yaoxiang
# Trait 定义
type Clone = { clone: (Self) -> Self }
type Add = { add: (Self, Self) -> Self }

# 使用约束
clone: [T: Clone](value: T) -> T = value.clone()
combine: [T: Clone + Add](a: T, b: T) -> T = a.clone() + b
```

#### 4.2 Current Status

- ⚠️ Type-level computation module exists (`type_level/`)

- ⚠️ Basic `Some`/`None` wrapper implementation
- ❌ No Trait definition syntax parsing
- ❌ No Trait implementation verification
- ❌ No Trait inheritance/derivation

- ❌ Constraint solver incomplete

#### 4.3 Required Implementation

```
src/frontend/core/parser/trait_def.rs               # 新增：Trait 定义解析
src/frontend/typecheck/trait_resolution.rs          # 新增：Trait 约束求解
src/frontend/typecheck/trait_impl.rs                # 新增：Trait 实现检查
src/frontend/type_level/trait_bounds.rs             # 新增：Trait 边界表示
```

#### 4.4 Acceptance Criteria

- [ ] Can parse `type TraitName = { ... }

- [ ] 能解析 `[T: Trait]` 约束语法
- [ ] 验证类型是否满足 Trait 约束
- [ ] 支持多重约束 `[T: A + B]`
- [ ] 错误信息指出缺失的 Trait 实现

### 5. Associated Types (GAT)

#### 5.1 Feature Requirements
Description

```# 关联类型定义
type Iterator[T] = {
    Item: T,                           # 关联类型
    next: (Self) -> Option[T],
    has_next: (Self) -> Bool,
}

# 使用关联类型
collect: [T, I: Iterator[T]](iter: I) -> List[T] = { ... }
```

#### 5.2 Current Status

- ❌ No associated type parsing

- ❌ No associated type constraint checking
- ❌ No GAT support

#### 5.3 Required Implementation
------



```
src/frontend/type_level/associated_types.rs         # 新增：关联类型表示
src/frontend/typecheck/gat_check.rs                 # 新增：GAT 类型检查
```

#### 5.4 Acceptance Criteria

- [ ] Can parse Trait definitions with associated types

- [ ] Can use associated types as constraints
- [ ] Type checking correctly resolves associated types
- [ ] Supports generic associated types

---

### 6. Full Compile-time Generics Implementation

#### 6.1 Feature Description

```yaoxiang
# 编译期常量参数
type Array[T, N: Int] = { data: T[N] }

# 编译期函数：使用字面量类型约束
factorial: [n: Int](n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

# 编译期计算（编译器在编译期计算）
SIZE: Int = factorial(5)  # 120
```

#### 6.2 Current Status

- ⚠️ `const_generics/` module exists

- ⚠️ Basic `GenericSize` representation
- ⚠️ Basic constant expression evaluation
- ❌ No literal type parameter parsing `[n: Int](n:

- ❌ 无编译期维度验证
- ⚠️ `static_assert` 由条件类型标准库实现（见 7. 条件类型）

#### 6.3 需要的实现



```
src/frontend/core/parser/literal_param.rs           # 新增：字面量类型参数解析
src/frontend/typecheck/const_eval.rs                 # 新增：编译期表达式求值
src/middle/passes/mono/compile_time_monomorphization.rs  # 新增：编译期泛型特化
```


#### 6.4 验收标准

- [ ] 能解析 `[n: Int](n: n)` 字面量类型参数语法

- [ ] 能解析 `[N: Int]` 编译期泛型参数
- [ ] 编译期求值字面量类型参数的函数调用
- [ ] 支持编译期泛型实例化
- [ ] 注：`Assert` 由标准库利用条件类型实现（见 7.4 验收标准）

### 7. Complete Conditional Type Implementation

#### 7.1 Feature Description

```yaoxiang
# 类型级If
type If[C: Bool, T, E] = match C {
    True => T,
    False => E,
}

# 类型族
type Add[A: Nat, B: Nat] = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}
```

#### 7.2 Current Status

- ✅ `type_families.rs` implemented (60%)

- Bool type family: `True`, `False`
  - Nat type family: `Zero`, `Succ[N]`
  - Conditional types: `IsTrue`, `IsFalse`, `IsZero`, `IsSucc`
  - Unified handling with TypeFamily trait

- ✅ `type_match.rs` implemented (70%)
  - MatchPattern: literal/constructor/tuple/wildcard patterns
  - PatternMatcher: pattern matching engine

- MatchType: Complete type matching
- PatternBuilder: Streaming API builder pattern

- ✅ `type_eval.rs` is implemented (65%)

- If conditional evaluation: `If<True, Int, String> => Int`
- Nat operations: `Add`,


- ⚠️ `conditional_types.rs` 存在（基础框架）

- ❌ 与类型归一化器完整集成（待实现）

- ❌ 标准库 `Assert` 实现（待实现）

#### 7.3 需要的实现


```
src/frontend/type_level/type_match.rs               # ✅ 已实现
src/frontend/type_level/type_families.rs            # ✅ 已实现
src/frontend/typecheck/type_eval.rs                 # ✅ 已实现
src/frontend/type_level/evaluation/mod.rs          # 修改：集成求值器
```


#### 7.4 验收标准

- [x] 支持 `If[C, T, E]` 条件类型

- [x] Support Bool type family (True, False)
- [x] Support Nat type family (Zero, Succ)
- [x] Support type-level match expressions
- [x] Compile-time type computation (If, Nat arithmetic)
- - [ ] Full integration with type normalizer
- [ ] Standard library `Assert` implementation (compile-time assertions)
- - - - - -

### 8. Inline Optimization and Specialization Combined

#### 8.1 Feature Description

RFC-011 designs function overloading and inline optimization to naturally combine:

- [ ] 与类型归一化器完整集成
- [ ] 标准库 `Assert` 实现（编译期断言）

```yaoxiang
type Assert[C: Bool] = match C {
    True => Void,
    False => compile_error("Assertion failed"),
}
```


---

### 8. 内联优化与特化结合


#### 8.1 功能描述

RFC-011 设计函数重载与内联优化天然结合：



```yaoxiang
sum: (arr: Array[Int]) -> Int = (arr) => {
    native_sum_int(arr.data, arr.length)
}

# 使用时编译器自动选择并内联
result = sum(int_arr)  # => native_sum_int(int_arr.data, int_arr.length)
```

#### 8.2 Current Status

- ❌ No

- ❌ 优化器无特化感知
- ❌ 内联决策不考虑特化

#### 8.3 需要的实现



```
src/middle/optimizer/specialization_aware_inlining.rs  # 新增：特化感知内联
src/middle/passes/opt/size_analysis.rs                 # 新增：函数大小分析
```


#### 8.4 验收标准

- [ ] 特化后的代码能进一步内联

- [ ] 小特化体自动内联到调用点
- [ ] 生成代码等价于手写优化

---

## Priority Ranking (Updated

|--------|------|----------|------|------|
| **P0** | 完整DCE | 1周 | 基础单态化器 | 90% - 收尾 |
| **P1** | 函数重载特化集成 | 2周 | 重载解析 | 75% - 完善 |
| **P2** | 条件类型集成 | 2周 | 类型归一化器 | 65% - 中期 |

| **P3** | 平台特化集成 | 2周 | 单态化器 | 50% - 中期 |
| **P4** | 编译期泛型完整 | 3周 | Phase 4 | 40% - 中期 |
| **P5** | 完整Trait系统 | 4周 | Phase 2 | 10% - 长期 |
| **P6** | 关联类型 | 4周 | Trait系统 | 5% - 长期 |

| **P7** | 特化感知内联 | 2周 | P1 + 优化器 | 0% - 长期 |
| **P8** | 宏替代 | 3周 | 泛型+Trait | 0% - 长期 |

### 下一步建议


**短期 (1-2周)**：
1. 完成 DCE 收尾工作
2. 集成条件类型到类型归一化器

3. Integration Platform Specialized to Monomorphizers

**Mid-term (1 month)**:
1. Complete integration of function overloading and generics

2. Complete compile-time generics (literal parameters)
3. Begin basic Trait system implementation

**Long-term (2-3 months)**:

1. Associated types (GAT)
2. Specialization-aware inlining
3. Macro substitution capabilities

---

## Implementation Suggestions

### Short-term Goals (1-2 months)

1. **Complete basic DCE**
   - Build instantiation graph

- Implement reachability analysis
- This will eliminate most useless code bloat

2. **Implement function overload specialization**

- This is the core feature of RFC-011
- Supporting subsequent specialization optimizations

### Medium-term goals (3-4 months)

1. **Complete Trait system**
- Supporting generic constraints
- Providing foundation for standard library

2. **Compile-time generics**
- Supporting compile-time computation
- Supporting static array optimization

- No `const` keyword needed

### Long-term goals (5-6 months)

1. **Conditional Types**
   - Type-level programming
   - More powerful generic capabilities

2. **Platform Specialization**
   - SIMD optimization
   - Architecture-specific code

---

## Related File List

### Implemented Modules (partial/basic)

| File Path | Status | Description |
|----------|------|------|

| `src/middle/passes/mono/mod.rs` | ⚠️ 70% | Monomorphizer main body |
| `src/middle/passes/mono/function.rs` | ⚠️ 70% | Function monomorphization |
| `src/middle/passes/mono/type_mono.rs` | ⚠️ 50% | Type monomorphization |
| `src/middle/passes/mono/closure.rs` | ⚠️ 50% | Closure monomorphization |

| `src/middle/passes/mono/platform_info.rs` | ✅ 80

| `src/frontend/type_level/const_generics/mod.rs` | ⚠️ 40% | 编译期泛型框架 |
| `src/frontend/type_level/evaluation/compute.rs` | ⚠️ 30% | 类型级计算 |
| `src/frontend/type_level/type_match.rs` | ✅ 70% | 类型级 match |
| `src/frontend/type_level/type_families.rs` | ✅ 60% | 类型族 (Bool/Nat) |

| `src/frontend/typecheck/type_eval.rs` | ✅ 65% | 编译期类型求值器 |
| `src/frontend/typecheck/gat/mod.rs` | ⚠️ 5% | GAT 基础结构 |
| `src/frontend/typecheck/traits/mod.rs` | ⚠️ 10% | Trait 基础结构 |


### 需要新增/完善的模块

| 文件路径 | 功能 | 状态 |
|----------|------|------|

| `src/frontend/core/parser/overload.rs` | 函数重载解析 | 已存在 |
| `src/frontend/core/parser/trait_def.rs` | Trait定义解析 | ❌ 未实现 |
| `src/frontend/core/parser/literal_param.rs` | 字面量类型参数解析 | ❌ 未实现 |
| `src/frontend/core/parser/attr.rs` | 属性解析（可选） | ❌ 未实现 |

| `src/frontend/typecheck/trait_resolution.rs` | Trait constraint solving | ⚠️ Partial |
| `src/frontend/typecheck/trait_impl.rs` | Trait implementation checking | ⚠️ Partial |
| `src/frontend/typecheck/const_eval.rs` | Compile-time expression evaluation | ⚠️ Partial |
| `src/frontend/type_level/associated_types.rs` | Associated types | ❌ Not implemented |
------
| `src/middle/passes/mono/instantiation_graph.rs` | Instantiation graph | Exists |
| `src/middle/passes/mono/reachability.rs` | Reachability analysis | Exists |
| `src/middle/passes/mono/cross_module_dce.rs` | Cross-module DCE | Exists |
| `src/middle/optimizer/specialization_aware_inlining.rs` | Specialization-aware inlining | ❌ Not implemented |
------

---

## Appendix: RFC-011 Design Review
------

### Core Features Checklist (Updated 2026-02-04) |

| `src/middle/passes/mono/instantiation_graph.rs` | 实例化图 | 已存在 |
| `src/middle/passes/mono/reachability.rs` | 可达性分析 | 已存在 |
| `src/middle/passes/mono/cross_module_dce.rs` | 跨模块DCE | 已存在 |
| `src/middle/optimizer/specialization_aware_inlining.rs` | 特化感知内联 | ❌ 未实现 |


---

## 附录：RFC-011 设计回顾


### 核心特性清单（2026-02-04 更新）

| 特性 | RFC设计 | 当前实现 | 差距 | 优先级 |

|------|---------|----------|------|--------|
| 基础泛型 `[T]` | ✅ | ✅ 70% | 需完善 | P1 |
| 类型推导 | ✅ | ⚠️ 基础 | 需扩展 | P1 |
| 类型约束 (Trait) | ✅ | ⚠️ 10% | 需实现 | P2 |

| Feature | Impl | Progress | Notes | Priority |
|---------|------|----------|-------|----------|
| Generic Associated Types (GAT) | ✅ | ⚠️ 5% | Needs implementation | P4 |
| Compile-time Generics | ✅ | ⚠️ 40% | Needs improvement | P3 |
| Conditional Types | ✅ | ✅ 65% | Conditional type framework complete | P2 |
| Function Specialization | ✅ | ✅ 75% | Overload mechanism complete | P1 |

| Platform Specialization | ✅ | ⚠️ 50% | Basic structure complete | P2 |
| Full DCE | ✅ | ⚠️ 90% | Near completion | P0 |
| Macro Substitution | ✅ | ❌ 0% | Needs implementation | P5 |
| Specialization-aware Inlining | ✅ | ❌ 0% | Needs implementation | P5 |

> **Update Notes (2026-02-04)**:
> - Conditional types from 35% → **65%**: `type_match.rs`, `type_families.rs`, `type_eval.rs` implemented
> - Platform specialization from 0% → **50%**: `platform_info.rs`, `platform_specializer.rs` implemented

> - Generic associated types from 0% → **5%**: Basic structure created in `src/frontend/typecheck/gat/`

### Dependency Graph

```
基础泛型 ([T])
    │
    ├──> 类型约束系统 (Trait)
    │        │
    │        ├──> 关联类型 (GAT)
    │        │
    │        └──> Trait继承
    │
    ├──> 编译期泛型
    │        │
    │        ├──> 字面量类型参数
    │        │
    │        └──> 编译期计算
    │
    └──> 条件类型
             │
             └──> 类型级编程

函数重载特化 ─────────────> 平台特定优化
                             │
                             └──> 特化感知内联

完整DCE ──────────────────> 跨模块DCE
                             │
                             └──> 代码膨胀控制
```



