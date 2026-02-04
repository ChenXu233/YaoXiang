# RFC-011 泛型系统 - 未实现功能清单

> **创建日期**: 2026-02-03
> **状态**: 进行中
> **基于 RFC**: [RFC-011 泛型系统设计](../accepted/011-generic-type-system.md)

## 摘要

本文档记录 RFC-011 泛型系统设计中已完成和未实现的功能模块。基于对编译器实现的分析，识别出当前系统的能力边界和待完善部分。

---

## 实现状态总览

| Phase | 功能模块 | 状态 | 完成度 | 关键文件 |
|-------|---------|------|--------|----------|
| Phase 1 | 基础泛型 | ✅ 部分实现 | 70% | `src/middle/passes/mono/mod.rs` |
| Phase 2 | 类型约束系统 | ⚠️ 基础结构 | 30% | `src/frontend/type_level/` |
| Phase 3 | 关联类型 | ❌ 未实现 | 0% | - |
| Phase 4 | 编译期泛型 | ⚠️ 基础结构 | 40% | `src/frontend/type_level/const_generics/` |
| Phase 5 | 条件类型 | ⚠️ 基础结构 | 35% | `src/frontend/type_level/conditional_types.rs` |
| - | 函数重载特化 | ✅ 已实现 | 75% | `src/frontend/typecheck/overload.rs` |
| - | 平台特定优化 | ❌ 未实现 | 0% | - |
| - | 完整DCE | ✅ 部分实现 | 90% | `src/middle/passes/mono/` |

---

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
- ✅ 重载解析器模块存在 (`overload.rs`)
- ✅ 类型环境支持重载候选存储
- ✅ 函数调用重载解析集成 (`expressions.rs`)
- ⚠️ 泛型fallback集成（待完善）

#### 1.3 需要的实现

```
src/frontend/typecheck/overload.rs              # ✅ 重载解析器（完成）
src/frontend/typecheck/mod.rs                  # ✅ 类型环境扩展（完成）
src/middle/passes/mono/instance.rs             # ✅ 实例化ID扩展（完成）
src/frontend/typecheck/inference/expressions.rs  # ✅ 重载解析集成（完成）
src/frontend/typecheck/checking/mod.rs          # ✅ BodyChecker扩展（完成）
```

#### 1.4 验收标准

- [x] 能解析同名函数的不同类型签名（数据结构支持）
- [x] 调用时根据实参类型自动选择最优匹配（已集成）
- [x] 编译错误：歧义调用或无匹配定义（已实现）
- [x] 与泛型系统集成：泛型作为fallback（已完成）

---

### 2. 平台特定优化

#### 2.1 功能描述

RFC-011 设计支持 `#[cfg]` 属性实现平台特化：
```yaoxiang
sum: [T](arr: Array[T]) -> T = (arr) => { basic_sum_iter(arr) }

#[cfg(target_arch = "x86_64")]
sum: (arr: Array[T]) -> T = (arr) => { avx2_sum(arr.data, arr.length) }

#[cfg(target_arch = "aarch64")]
sum: (arr: Array[T]) -> T = (arr) => { neon_sum(arr.data, arr.length) }
```

#### 2.2 当前状态

- ❌ 无 `#[cfg]` 属性解析
- ❌ 无平台检测逻辑
- ❌ 无条件编译选择

#### 2.3 需要的实现

```
src/frontend/core/parser/attr.rs                    # 新增：属性解析
src/frontend/config/platform_detect.rs              # 新增：平台检测
src/middle/passes/mono/platform_specializer.rs     # 新增：平台特化器
```

#### 2.4 验收标准

- [ ] 能解析 `#[cfg(...)]` 属性
- [ ] 编译时检测目标平台
- [ ] 只生成匹配当前平台的代码
- [ ] 支持 `target_arch`、`target_os` 等条件

---

### 3. 死代码消除(DCE)完整实现

#### 3.1 功能描述

RFC-011 设计了多层次的 DCE：

1. **实例化图分析**：构建泛型实例化依赖图，从入口点进行可达性分析
2. **使用点分析**：只实例化实际被调用的泛型
3. **跨模块DCE**：分析模块间依赖，消除未使用的导出
4. **LLVM层面DCE**：利用LLVM的优化pass

```rust
// 编译器内部：构建泛型实例化依赖图
struct InstantiationGraph {
    nodes: HashMap<InstanceKey, InstanceNode>,
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}
```

#### 3.2 当前状态

- ✅ 基础单态化器存在 (`mono/mod.rs`)
- ✅ 按需特化基本实现
- ✅ 实例化图构建完成 (`instantiation_graph.rs`)
- ✅ 完整可达性分析完成 (`reachability.rs`)
- ⚠️ 跨模块DCE基本实现（需生产环境验证）
- ✅ 代码膨胀控制完成 (`dce.rs`)

#### 3.3 需要的实现

```rust
// 新增模块
src/middle/passes/mono/instantiation_graph.rs      # 实例化图构建
src/middle/passes/mono/reachability.rs              # 可达性分析
src/middle/passes/mono/cross_module_dce.rs          # 跨模块DCE
src/middle/passes/mono/code_bloat_control.rs        # 代码膨胀控制
```

#### 3.4 验收标准

- [x] 构建完整的实例化依赖图
- [x] 从main入口进行可达性分析
- [x] 消除未使用的泛型实例
- [x] 跨模块依赖分析（生产环境验证）
- [x] 代码膨胀阈值控制
- [x] 统计信息输出（详细版+JSON格式）

---

### 4. 完整 Trait 系统

#### 4.1 功能描述

RFC-011 设计了类型约束系统（类似 Rust Trait）：

```yaoxiang
# Trait 定义
type Clone = { clone: (Self) -> Self }
type Add = { add: (Self, Self) -> Self }

# 使用约束
clone: [T: Clone](value: T) -> T = value.clone()
combine: [T: Clone + Add](a: T, b: T) -> T = a.clone() + b
```

#### 4.2 当前状态

- ⚠️ 类型级计算模块存在 (`type_level/`)
- ⚠️ 基础 `Some`/`None` 包装器实现
- ❌ 无 Trait 定义语法解析
- ❌ 无 Trait 实现验证
- ❌ 无 Trait 继承/派生
- ❌ 约束求解器不完整

#### 4.3 需要的实现

```
src/frontend/core/parser/trait_def.rs               # 新增：Trait 定义解析
src/frontend/typecheck/trait_resolution.rs          # 新增：Trait 约束求解
src/frontend/typecheck/trait_impl.rs                # 新增：Trait 实现检查
src/frontend/type_level/trait_bounds.rs             # 新增：Trait 边界表示
```

#### 4.4 验收标准

- [ ] 能解析 `type TraitName = { ... }` 语法
- [ ] 能解析 `[T: Trait]` 约束语法
- [ ] 验证类型是否满足 Trait 约束
- [ ] 支持多重约束 `[T: A + B]`
- [ ] 错误信息指出缺失的 Trait 实现

---

### 5. 关联类型 (GAT)

#### 5.1 功能yaoxiang
描述

```# 关联类型定义
type Iterator[T] = {
    Item: T,                           # 关联类型
    next: (Self) -> Option[T],
    has_next: (Self) -> Bool,
}

# 使用关联类型
collect: [T, I: Iterator[T]](iter: I) -> List[T] = { ... }
```

#### 5.2 当前状态

- ❌ 无关联类型解析
- ❌ 无关联类型约束检查
- ❌ 无 GAT 支持

#### 5.3 需要的实现

```
src/frontend/type_level/associated_types.rs         # 新增：关联类型表示
src/frontend/typecheck/gat_check.rs                 # 新增：GAT 类型检查
```

#### 5.4 验收标准

- [ ] 能解析带关联类型的 Trait 定义
- [ ] 能使用关联类型作为约束
- [ ] 类型检查正确解析关联类型
- [ ] 支持泛型关联类型

---

### 6. 编译期泛型完整实现

#### 6.1 功能描述

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

#### 6.2 当前状态

- ⚠️ `const_generics/` 模块存在
- ⚠️ 基础 `GenericSize` 表示
- ⚠️ 基础常量表达式求值
- ❌ 无字面量类型参数解析 `[n: Int](n: n)`
- ❌ 无编译期函数实例化
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

---

### 7. 条件类型完整实现

#### 7.1 功能描述

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

#### 7.2 当前状态

- ⚠️ `conditional_types.rs` 存在
- ⚠️ 基础类型级计算框架
- ❌ 无 `match` 类型匹配
- ❌ 无 Bool/Nat 类型族
- ❌ 编译期类型计算不完整

#### 7.3 需要的实现

```
src/frontend/type_level/type_match.rs               # 新增：类型级match
src/frontend/type_level/type_families.rs            # 新增：类型族(Bool, Nat)
src/frontend/typecheck/type_eval.rs                 # 新增：类型级求值器
```

#### 7.4 验收标准

- [ ] 能解析类型级 `match` 表达式
- [ ] 支持 `If[C, T, E]` 条件类型
- [ ] 支持 Bool 类型族 (True/False)
- [ ] 支持 Nat 类型族 (Zero/Succ)
- [ ] 编译期类型计算
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

#### 8.2 当前状态

- ❌ 无特化+内联联动
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

## 优先级排序

| 优先级 | 功能 | 预估工期 | 依赖 |
|--------|------|----------|------|
| **P0** | 完整DCE | 2周 | 基础单态化器 |
| **P1** | 函数重载特化 | 3周 | 泛型解析 |
| **P2** | 完整Trait系统 | 4周 | Phase 2类型约束 |
| **P3** | 编译期泛型完整 | 3周 | Phase 4 |
| **P4** | 条件类型完整 | 3周 | Phase 5 |
| **P5** | 平台特定优化 | 2周 | 函数重载 |
| **P6** | 关联类型 | 4周 | Trait系统 |
| **P7** | 特化感知内联 | 2周 | P1 + 优化器 |

---

## 实现建议

### 短期目标 (1-2个月)

1. **完成基础DCE**
   - 构建实例化图
   - 实现可达性分析
   - 这将消除大部分无用代码膨胀

2. **实现函数重载特化**
   - 这是 RFC-011 的核心特性
   - 支撑后续的特化优化

### 中期目标 (3-4个月)

1. **完整Trait系统**
   - 支撑泛型约束
   - 为标准库提供基础

2. **编译期泛型**
   - 支撑编译期计算
   - 支持静态数组优化
   - 无需 `const` 关键字

### 长期目标 (5-6个月)

1. **条件类型**
   - 类型级编程
   - 更强大的泛型能力

2. **平台特化**
   - SIMD优化
   - 架构特定代码

---

## 相关文件清单

### 已实现模块（部分/基础）

| 文件路径 | 状态 | 说明 |
|----------|------|------|
| `src/middle/passes/mono/mod.rs` | ⚠️ 70% | 单态化器主体 |
| `src/middle/passes/mono/function.rs` | ⚠️ 70% | 函数单态化 |
| `src/middle/passes/mono/type_mono.rs` | ⚠️ 50% | 类型单态化 |
| `src/middle/passes/mono/closure.rs` | ⚠️ 50% | 闭包单态化 |
| `src/frontend/type_level/mod.rs` | ⚠️ 40% | 类型级计算入口 |
| `src/frontend/type_level/conditional_types.rs` | ⚠️ 35% | 条件类型框架 |
| `src/frontend/type_level/const_generics/mod.rs` | ⚠️ 40% | 编译期泛型框架 |
| `src/frontend/type_level/evaluation/compute.rs` | ⚠️ 30% | 类型级计算 |

### 需要新增模块

| 文件路径 | 功能 |
|----------|------|
| `src/frontend/core/parser/overload.rs` | 函数重载解析 |
| `src/frontend/core/parser/trait_def.rs` | Trait定义解析 |
| `src/frontend/core/parser/literal_param.rs` | 字面量类型参数解析 |
| `src/frontend/core/parser/attr.rs` | 属性解析 |
| `src/frontend/typecheck/overload_resolution.rs` | 重载类型检查 |
| `src/frontend/typecheck/trait_resolution.rs` | Trait约束求解 |
| `src/frontend/typecheck/trait_impl.rs` | Trait实现检查 |
| `src/frontend/typecheck/const_eval.rs` | 编译期表达式求值 |
| `src/frontend/type_level/associated_types.rs` | 关联类型 |
| `src/frontend/type_level/type_match.rs` | 类型级match |
| `src/middle/passes/mono/instantiation_graph.rs` | 实例化图 |
| `src/middle/passes/mono/reachability.rs` | 可达性分析 |
| `src/middle/passes/mono/cross_module_dce.rs` | 跨模块DCE |
| `src/middle/passes/mono/platform_specializer.rs` | 平台特化器 |

---

## 附录：RFC-011 设计回顾

### 核心特性清单

| 特性 | RFC设计 | 当前实现 | 差距 |
|------|---------|----------|------|
| 基础泛型 `[T]` | ✅ | ✅ 70% | 需完善 |
| 类型推导 | ✅ | ⚠️ 基础 | 需扩展 |
| 类型约束 | ✅ | ❌ 0% | 需实现 |
| 关联类型 | ✅ | ❌ 0% | 需实现 |
| 编译期泛型 | ✅ | ⚠️ 40% | 需完善（无 const 关键字） |
| 条件类型 | ✅ | ⚠️ 35% | 需完善 |
| 函数特化 | ✅ | ❌ 0% | 需实现 |
| 平台特化 | ✅ | ❌ 0% | 需实现 |
| 完整DCE | ✅ | ⚠️ 50% | 需完善 |
| 宏替代 | ✅ | ❌ 0% | 需实现 |

### 依赖关系图

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
