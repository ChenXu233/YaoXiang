# Task 7.4: Send/Sync 约束传播与特化

> **优先级**: P0
> **状态**: ✅ 已实现
> **依赖**: task-07-02, task-07-03
> **完成时间**: 2026-01-18

## 功能描述

根据 Send/Sync 约束生成特化版本。约束从 `spawn` 点自然产生，沿类型结构传播到泛型参数。

## 核心设计

### 约束来源

```yaoxiang
# spawn 闭包捕获的变量必须 Send
fn process[T](x: T) {
    spawn(|| use(x))  # x 必须 Send → T 必须 Send
}

# 泛型容器类型的 Send 约束传播
fn test[T](v: Vec[T]) {
    spawn(|| use(v))  # Vec[T]: Send → T: Send
}
```

### Send/Sync 派生规则 (RFC-009)

| 类型 | Send | Sync | 派生规则 |
|------|------|------|----------|
| 值类型 (Int, Float, Bool, String) | ✅ | ✅ | 自动满足 |
| `ref T` (Arc) | ✅ | ✅ | 自动满足 |
| `*T` 裸指针 | ❌ | ❌ | unsafe 用户负责 |
| `Rc[T]` | ❌ | ❌ | 非线程安全 |
| `Arc[T]` | ✅ | ✅ | 原子引用计数 |
| `Vec[T]` | ⇐ T: Send | ⇐ T: Sync | 元素约束传播 |
| `Box[T]` | ⇐ T: Send | ⇐ T: Sync | 元素约束传播 |

## 实现架构

```
┌─────────────────────────────────────────────────────────────────┐
│                    约束传播流程                                   │
├─────────────────────────────────────────────────────────────────┤
│  1. 类型推断阶段                                                 │
│     └─ TypeInferrer 检测 spawn → 添加 Send 约束                 │
│         └─ SendSyncConstraintSolver 求解约束                    │
│                                                                 │
│  2. 约束传播阶段                                                 │
│     └─ SendSyncPropagator 沿类型结构传播约束                    │
│         └─ 泛型参数获得 Send 约束                                │
│                                                                 │
│  3. 单态化阶段                                                   │
│     └─ Monomorphizer 根据约束生成特化版本                        │
│         ├─ 普通版本：T 无 Send 约束                              │
│         ├─ Send 版本：T: Send，无法 Send 则报错                  │
│         └─ Sync 版本：T: Sync                                    │
└─────────────────────────────────────────────────────────────────┘
```

## 核心文件

### 1. 类型系统扩展 (src/frontend/typecheck/types.rs)

```rust
/// Send/Sync 约束
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SendSyncConstraint {
    pub require_send: bool,
    pub require_sync: bool,
}

/// Send/Sync 约束求解器
pub struct SendSyncConstraintSolver {
    constraints: HashMap<TypeVar, SendSyncConstraint>,
}

impl SendSyncConstraintSolver {
    /// 添加 Send 约束（传播到类型参数）
    pub fn add_send_constraint(&mut self, ty: &MonoType);

    /// 检查类型是否满足 Send
    pub fn is_send(&self, ty: &MonoType) -> bool;

    /// 检查类型是否满足 Sync
    pub fn is_sync(&self, ty: &MonoType) -> bool;
}
```

### 2. 类型推断集成 (src/frontend/typecheck/infer.rs)

```rust
impl<'a> TypeInferrer<'a> {
    /// 标记当前函数需要 Send 约束（spawn 函数）
    pub fn mark_current_fn_requires_send(&mut self);

    /// 检查泛型参数是否满足 Send 约束
    pub fn check_send_for_generic_params(&self) -> Vec<(MonoType, &'static str)>;

    /// 为闭包捕获的变量添加 Send 约束
    pub fn add_send_constraint_to_captured_vars(&mut self);
}
```

### 3. 约束传播器 (src/middle/lifetime/send_sync.rs)

```rust
/// Send/Sync 约束传播器
pub struct SendSyncPropagator {
    constraints: Vec<(MonoType, SendSyncConstraint)>,
}

impl SendSyncPropagator {
    /// 约束沿类型结构传播
    pub fn propagate(&self) -> Vec<(MonoType, SendSyncConstraint)>;

    /// 验证约束是否可满足
    pub fn verify_constraints(&self, checker: &SendSyncChecker) -> SendSyncPropagationResult;
}
```

### 4. 单态化器扩展 (src/middle/monomorphize/mod.rs)

```rust
impl Monomorphizer {
    /// 根据 Send/Sync 约束单态化泛型函数
    pub fn monomorphize_with_send_sync_constraints(
        &mut self,
        generic_id: &GenericFunctionId,
        type_args: &[MonoType],
        send_constraints: &[(MonoType, bool)],
        sync_constraints: &[(MonoType, bool)],
    ) -> Option<FunctionId>;

    /// 生成 Send 特化版本
    pub fn generate_send_specialization(
        &mut self,
        generic_id: &GenericFunctionId,
        type_args: &[MonoType],
    ) -> Option<FunctionId>;
}
```

### 5. 约束传播模块 (src/middle/monomorphize/constraint.rs)

```rust
/// 约束传播引擎
pub struct ConstraintPropagationEngine {
    collector: ConstraintCollector,
    propagator: SendSyncPropagator,
    checker: SendSyncChecker,
}

impl ConstraintPropagationEngine {
    /// 添加 spawn 约束
    pub fn add_spawn_constraint(&mut self, closure_type: &MonoType, span: Span);

    /// 传播约束
    pub fn propagate(&mut self) -> ConstraintPropagationResult;
}

/// 特化请求
pub struct SpecializationRequest {
    pub generic_name: String,
    pub type_args: Vec<MonoType>,
    pub constraints: SendSyncConstraint,
    pub span: Span,
}
```

## 测试覆盖

测试文件: [src/middle/monomorphize/tests/constraint.rs](../../../../src/middle/monomorphize/tests/constraint.rs)

### 测试用例

1. **基本类型 Send/Sync**
   - `test_basic_type_is_send`
   - `test_checker_basic_send`

2. **约束收集与传播**
   - `test_constraint_propagation`
   - `test_send_constraint_propagation`
   - `test_nested_type_constraint_propagation`

3. **泛型函数约束**
   - `test_generic_function_constraint_propagation`
   - `test_full_propagation_flow`

4. **特化请求**
   - `test_specialization_request_collector`
   - `test_filter_send_requests`

## 使用示例

```yaoxiang
# 示例 1: spawn 捕获变量约束
fn process[T](x: T) {
    spawn(|| {
        print(x)  # x 必须 Send → T 必须 Send
    })
}

# 示例 2: 泛型容器约束传播
fn multi_test[T](v: Vec[T], s: String) {
    spawn(|| {
        print(v)  # Vec[T] 必须 Send → T 必须 Send
    })
    spawn(|| {
        print(s)  # String 总是 Send
    })
}

# 示例 3: 无法 Send 的类型报错
fn bad_example[T](x: *T) {
    spawn(|| {
        print(x)  # 编译错误: *T 不是 Send
    })
}
```

## 后续优化

1. **自动 Arc 包装**：对于无法 Send 但需要 Send 版本的类型，自动生成 Arc 包装版本
2. **约束优化**：避免生成不必要的特化版本
3. **错误信息改进**：提供更精确的约束违反位置和信息
