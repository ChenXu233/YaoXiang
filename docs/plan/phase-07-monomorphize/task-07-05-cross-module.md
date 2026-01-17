# Task 7.5: 跨模块实例化

> **优先级**: P1
> **状态**: ⏳ 待实现
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

## 跨模块实例化算法

```rust
struct CrossModuleInstantiator {
    /// 当前模块
    current_module: ModuleId,
    /// 已导入的泛型类型
    imported_generics: HashMap<ModuleId, Vec<GenericTypeId>>,
    /// 已导入的泛型函数
    imported_functions: HashMap<ModuleId, Vec<GenericFunctionId>>,
    /// 模块依赖图
    module_graph: ModuleGraph,
}

impl CrossModuleInstantiator {
    /// 实例化跨模块的泛型类型
    pub fn instantiate_imported_type(
        &mut self,
        module: ModuleId,
        type_name: &str,
        type_args: &[MonoType],
    ) -> Result<TypeId, MonoError> {
        // 查找泛型类型定义
        let generic_type = self.find_generic_type(module, type_name)?;

        // 确保类型已导入
        self.ensure_imported(module, &generic_type)?;

        // 在定义模块中实例化
        let defining_module = self.module_graph.get_defining_module(module, type_name);
        self.instantiate_in_module(defining_module, generic_type, type_args)
    }

    /// 确保泛型类型已导入
    fn ensure_imported(&mut self, from_module: ModuleId, generic_type: &GenericTypeId) -> Result<(), MonoError> {
        if self.imported_generics.contains_key(&from_module) {
            return Ok(());
        }

        // 从模块加载泛型类型定义
        let types = self.module_loader.load_public_types(from_module)?;
        self.imported_generics.insert(from_module, types);
        Ok(())
    }

    /// 在指定模块中实例化
    fn instantiate_in_module(
        &mut self,
        module: ModuleId,
        generic_type: GenericTypeId,
        type_args: &[MonoType],
    ) -> Result<TypeId, MonoError> {
        // 获取该模块的单态化器
        let mono_state = self.module_states.entry(module)
            .or_insert_with(MonoState::new);

        mono_state.monomorphize_type(generic_type, type_args)
    }
}
```

## 模块导出规则

| 场景 | 导出内容 |
|------|----------|
| 泛型类型 | 类型定义 + 所有实例化版本 |
| 泛型函数 | 函数定义 + 所有实例化版本 |
| 私有泛型 | 仅在定义模块内实例化 |

## 相关文件

- **cross_module.rs**: 跨模块实例化逻辑
- **imports.rs**: 导入处理
