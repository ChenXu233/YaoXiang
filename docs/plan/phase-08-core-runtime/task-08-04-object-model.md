# Task 8.4: 对象模型

> **优先级**: P1
> **状态**: ⚠️ 需重构

## 功能描述

定义运行时对象模型，包括类型信息和对象表示。

## 类型信息

```rust
/// 类型信息
struct TypeInfo {
    /// 类型名称
    name: String,
    /// 类型 ID
    type_id: TypeId,
    /// 类型大小
    size: usize,
    /// 对齐要求
    align: usize,
    /// 基础类型
    kind: TypeKind,
    /// 方法表
    methods: MethodTable,
    /// 类型标志
    flags: TypeFlags,
}

enum TypeKind {
    /// 标量类型
    Scalar(ScalarKind),
    /// 结构体类型
    Struct(StructLayout),
    /// 枚举类型
    Enum(EnumLayout),
    /// 函数类型
    Function(FunctionLayout),
    /// 泛型类型
    Generic {
        base: TypeId,
        type_params: Vec<TypeParam>,
    },
    /// 类型别名
    Alias(TypeId),
}
```

## 对象布局

```rust
/// 结构体布局
struct StructLayout {
    /// 字段
    fields: Vec<FieldLayout>,
    /// 字段数量
    field_count: usize,
    /// 是否有可变部分
    has_variants: bool,
}

/// 字段布局
struct FieldLayout {
    name: String,
    offset: usize,
    ty: TypeId,
}

/// 枚举布局
struct EnumLayout {
    /// 变体
    variants: Vec<VariantLayout>,
    /// 判别式类型
    discriminant_type: PrimitiveType,
}
```

## 相关文件

- `src/runtime/core/object.rs`
