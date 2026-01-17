# Task 7.1: 单态化数据结构

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

定义单态化所需的 core 数据结构。

## 数据结构

```rust
/// 单态化类型（替换泛型参数后的具体类型）
enum MonoType {
    /// 具体类型引用
    Concrete(TypeId),
    /// 泛型实例（如 `List_Int`）
    Instance(GenericId, Vec<MonoType>),
    /// 函数类型单态化
    Fn {
        params: Vec<MonoType>,
        return_type: Box<MonoType>,
        is_async: bool,
    },
}

/// 单态化实例键
struct InstanceKey {
    /// 原始泛型函数/类型 ID
    generic_id: Id,
    /// 类型参数列表
    type_args: Vec<MonoType>,
}

impl InstanceKey {
    /// 生成实例名称
    pub fn instance_name(&self) -> String {
        match self.generic_id {
            Id::Function(name) => {
                let args = self.type_args
                    .iter()
                    .map(|t| t.name())
                    .collect::<Vec<_>>()
                    .join("_");
                format!("{}_{}", name, args)
            }
            Id::Type(name) => {
                let args = self.type_args
                    .iter()
                    .map(|t| t.name())
                    .collect::<Vec<_>>()
                    .join("_");
                format!("{}_{}", name, args)
            }
        }
    }
}

/// 单态化状态
struct MonoState {
    /// 已生成的函数实例
    fn_instances: HashMap<InstanceKey, FunctionId>,
    /// 已生成的类型实例
    type_instances: HashMap<InstanceKey, TypeId>,
    /// 待实例化的泛型函数
    pending_functions: Vec<GenericFunctionId>,
    /// 待实例化的泛型类型
    pending_types: Vec<GenericTypeId>,
    /// Send/Sync 约束缓存
    constraint_cache: HashMap<MonoType, (bool, bool)>, // (Send, Sync)
}
```

## 相关文件

- **mod.rs**: MonoState 定义
- **types.rs**: MonoType 和 InstanceKey
