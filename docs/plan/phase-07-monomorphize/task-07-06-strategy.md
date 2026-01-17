# Task 7.6: 实例化策略

> **优先级**: P1
> **状态**: ⏳ 待实现
> **依赖**: task-07-03

## 功能描述

选择最优的单态化策略：特化 vs 通用实例化。

## 实例化策略

| 策略 | 描述 | 适用场景 |
|------|------|----------|
| `Eager` | 立即实例化所有可能类型 | 小型项目、编译时间不敏感 |
| `Lazy` | 按需实例化 | 大型项目、减少编译时间 |
| `Demand` | 仅实例化实际使用的 | 最小化二进制大小 |
| `Specialize` | 对已知类型特化 | 性能关键路径 |

## 策略选择算法

```rust
enum InstantiationStrategy {
    /// 立即实例化
    Eager,
    /// 延迟实例化
    Lazy,
    /// 按需实例化
    Demand,
    /// 类型特化
    Specialize(Vec<MonoType>),
}

impl MonoState {
    /// 选择最优实例化策略
    pub fn select_strategy(&self, generic_id: GenericId) -> InstantiationStrategy {
        match generic_id {
            GenericId::Function(fn_info) => {
                // 根据使用场景选择策略
                self.select_fn_strategy(fn_info)
            }
            GenericId::Type(type_info) => {
                // 根据类型使用场景选择策略
                self.select_type_strategy(type_info)
            }
        }
    }

    /// 为函数选择策略
    fn select_fn_strategy(&self, fn_info: &GenericFunctionInfo) -> InstantiationStrategy {
        // 热路径函数使用特化策略
        if self.is_hot_path(&fn_info.name) {
            return InstantiationStrategy::Specialize(self.common_types());
        }

        // 库函数使用延迟策略
        if fn_info.is_pub && !fn_info.is_main {
            return InstantiationStrategy::Lazy;
        }

        // 其他使用按需策略
        InstantiationStrategy::Demand
    }

    /// 常见类型列表（用于特化）
    fn common_types(&self) -> Vec<MonoType> {
        vec![
            MonoType::Concrete(self.type_id_of("Int")),
            MonoType::Concrete(self.type_id_of("Float")),
            MonoType::Concrete(self.type_id_of("String")),
            MonoType::Ref(Box::new(MonoType::Concrete(self.type_id_of("Int")))),
        ]
    }
}
```

## 策略配置

```rust
/// 单态化配置
struct MonoConfig {
    /// 全局策略
    global_strategy: InstantiationStrategy,
    /// 每个函数的策略覆盖
    per_fn_strategy: HashMap<String, InstantiationStrategy>,
    /// 最大实例化数量（防止无限实例化）
    max_instances: usize,
    /// 启用缓存
    enable_cache: bool,
}

impl Default for MonoConfig {
    fn default() -> Self {
        Self {
            global_strategy: InstantiationStrategy::Lazy,
            per_fn_strategy: HashMap::new(),
            max_instances: 10000,
            enable_cache: true,
        }
    }
}
```

## 相关文件

- **strategy.rs**: 策略选择逻辑
- **config.rs**: 配置管理
