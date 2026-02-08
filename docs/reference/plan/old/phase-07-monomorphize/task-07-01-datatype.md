# Task 7.1: 单态化数据结构

> **优先级**: P0
> **状态**: ✅ 已完成
>
> **参考**: [009-ownership-model.md](../design/accepted/009-ownership-model.md) - Arc 类型支持

## 完成情况

| 类型 | 位置 | 状态 |
|------|------|------|
| `GenericTypeId` | [instance.rs:238-280](src/middle/monomorphize/instance.rs#L238-L280) | ✅ 已实现 |
| `TypeId` | [instance.rs:347-407](src/middle/monomorphize/instance.rs#L347-L407) | ✅ 已实现 |
| `TypeInstance` | [instance.rs:409-454](src/middle/monomorphize/instance.rs#L409-L454) | ✅ 已实现 |
| 单元测试 | [tests/instance.rs](src/middle/monomorphize/tests/instance.rs) | ✅ 28/28 通过 |

## 验证结果

```bash
cargo test --package yaoxiang middle::monomorphize::tests::instance
# test result: ok. 28 passed; 0 failed
```

## 功能描述

定义单态化所需的核心数据结构。**基于现有代码（instance.rs, types.rs）进行补充和调整**。

## 现有代码分析

### 已实现（无需修改）

| 类型 | 位置 | 说明 |
|------|------|------|
| `MonoType` | `types.rs:56-111` | 单态类型，已包含 `Arc` 支持 |
| `FunctionId` | `instance.rs:285-345` | 函数ID，含 `specialized_name()` |
| `GenericFunctionId` | `instance.rs:189-236` | 泛型函数ID |
| `SpecializationKey` | `instance.rs:56-187` | 特化缓存键，含完整 `Hash` 实现 |
| `InstantiationRequest` | `instance.rs:11-54` | 实例化请求 |
| `FunctionInstance` | `instance.rs:238-283` | 函数实例 |

### 需要补充

| 类型 | 说明 | 状态 |
|------|------|------|
| `GenericTypeId` | 泛型类型ID（如 `List<T>`） | ❌ 缺失 |
| `TypeInstance` | 泛型类型的实例化结果 | ❌ 缺失 |

## 数据结构设计

### 1. GenericTypeId（新增）

```rust
/// 泛型类型ID
///
/// 用于唯一标识一个泛型类型（如 `List<T>`、`Option<T>`）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericTypeId {
    /// 类型名称
    name: String,
    /// 泛型参数列表（用于区分重载的泛型类型）
    type_params: Vec<String>,
}

impl GenericTypeId {
    /// 创建新的泛型类型ID
    pub fn new(name: String, type_params: Vec<String>) -> Self {
        GenericTypeId { name, type_params }
    }

    /// 获取类型名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取泛型参数列表
    pub fn type_params(&self) -> &[String] {
        &self.type_params
    }
}

impl fmt::Display for GenericTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.type_params.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}<{}>", self.name, self.type_params.join(", "))
        }
    }
}
```

### 2. TypeInstance（新增）

```rust
/// 类型实例
///
/// 表示一个泛型类型被特化后的具体类型
#[derive(Debug, Clone)]
pub struct TypeInstance {
    /// 特化后的类型ID
    pub id: TypeId,

    /// 泛型类型ID
    pub generic_id: GenericTypeId,

    /// 使用的类型参数
    pub type_args: Vec<MonoType>,

    /// 实例化后的 MonoType（延迟生成）
    pub mono_type: Option<MonoType>,
}

impl TypeInstance {
    /// 创建新的类型实例
    pub fn new(
        id: TypeId,
        generic_id: GenericTypeId,
        type_args: Vec<MonoType>,
    ) -> Self {
        TypeInstance {
            id,
            generic_id,
            type_args,
            mono_type: None,
        }
    }

    /// 设置单态类型
    pub fn set_mono_type(&mut self, mono_type: MonoType) {
        self.mono_type = Some(mono_type);
    }

    /// 获取单态类型
    pub fn get_mono_type(&self) -> Option<&MonoType> {
        self.mono_type.as_ref()
    }
}

/// 类型ID
///
/// 用于唯一标识一个已特化的类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeId {
    /// 类型名称
    name: String,
    /// 类型参数（用于生成唯一名称）
    type_args: Vec<MonoType>,
}

impl TypeId {
    /// 创建新的类型ID
    pub fn new(name: String, type_args: Vec<MonoType>) -> Self {
        TypeId { name, type_args }
    }

    /// 获取类型名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取完整的特化名称
    pub fn specialized_name(&self) -> String {
        if self.type_args.is_empty() {
            self.name.clone()
        } else {
            let args_str = self
                .type_args
                .iter()
                .map(|t| t.type_name())
                .collect::<Vec<_>>()
                .join("_");
            format!("{}_{}", self.name, args_str)
        }
    }
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.specialized_name())
    }
}

impl std::hash::Hash for TypeId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        for ty in &self.type_args {
            ty.type_name().hash(state);
        }
    }
}
```

### 3. MonoState（基于现有 Monomorphizer 调整）

```rust
/// 单态化状态
///
/// 现有代码中对应 `Monomorphizer` 结构体，以下为补充的泛型类型支持：
#[derive(Debug)]
pub struct MonoState {
    /// 已实例化的函数
    fn_instances: HashMap<FunctionId, FunctionIR>,
    /// 已实例化的类型
    type_instances: HashMap<TypeId, TypeInstance>,
    /// 待实例化的泛型函数
    pending_functions: Vec<InstantiationRequest>,
    /// 待实例化的泛型类型
    pending_types: Vec<(GenericTypeId, Vec<MonoType>)>,
}
```

## 任务拆分

### 7.1.1: 添加 GenericTypeId（instance.rs）

- 在 `instance.rs` 中添加 `GenericTypeId` 结构体
- 实现 `Display`、`Hash`、`PartialEq`、`Eq` trait
- 单元测试

### 7.1.2: 添加 TypeId 和 TypeInstance（instance.rs）

- 添加 `TypeId` 结构体（含 `specialized_name()`）
- 添加 `TypeInstance` 结构体
- 单元测试

### 7.1.3: 更新 Monomorphizer 支持泛型类型（mod.rs）

- 在 `Monomorphizer` 中添加 `type_instances` 字段
- 添加 `type_specialization_cache` 缓存
- 实现泛型类型的实例化逻辑

## 相关文件

| 文件 | 修改内容 |
|------|----------|
| `instance.rs` | 添加 `GenericTypeId`、`TypeId`、`TypeInstance` |
| `mod.rs` | 更新 `Monomorphizer` 支持泛型类型 |
| `tests/instance.rs` | 添加新类型的单元测试 |
