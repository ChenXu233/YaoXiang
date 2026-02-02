# Task 7.5: 跨模块实例化

> **优先级**: P1
> **状态**: ✅ 已完成
> **实现文件**:
>   - [cross_module.rs](../../../../src/middle/monomorphize/cross_module.rs) - CrossModuleMonomorphizer
>   - [module_state.rs](../../../../src/middle/monomorphize/module_state.rs) - ModuleMonoState
> **测试文件**: [cross_module.rs](../../../../src/middle/monomorphize/tests/cross_module.rs)
> **依赖**: task-07-03

## 功能描述

处理模块间泛型类型的实例化可见性。

## 跨模块实例化规则

```yaoxiang
# === Module A ===
# a.yx
pub type Box[T] = Box(value: T)

# === Module B ===
# b.yx
use a.{Box}

# 使用 Box
my_box: Box[Int] = Box(42)

# 单态化时需要访问 Module A 的 Box 定义
```

## 核心数据结构

```rust
/// 跨模块单态化器
pub struct CrossModuleMonomorphizer {
    /// 模块依赖图
    module_graph: ModuleGraph,
    /// 模块单态化状态
    module_states: HashMap<ModuleId, ModuleMonoState>,
    /// 全局实例缓存
    global_instance_cache: HashMap<GlobalInstanceKey, CachedInstance>,
    /// 泛型定义位置表
    generic_definitions: HashMap<(ModuleId, String), GenericInfo>,
}
```

## 模块导出规则

| 场景 | 导出内容 |
|------|----------|
| 泛型类型 | 类型定义 + 所有实例化版本 |
| 泛型函数 | 函数定义 + 所有实例化版本 |
| 私有泛型 | 仅在定义模块内实例化 |

## 相关文件

- [cross_module.rs](../../../../src/middle/monomorphize/cross_module.rs): 跨模块实例化逻辑
- [module_state.rs](../../../../src/middle/monomorphize/module_state.rs): 模块单态化状态
