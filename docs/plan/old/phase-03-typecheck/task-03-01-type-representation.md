# Task 3.1: 类型表示

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

定义类型的内部表示，包括单态类型、多态类型和类型约束求解器。

## 类型层次

```rust
// 单态类型 (MonoType) - 具体类型
enum MonoType {
    Int(u8),           // int8, int16, int32, int64
    Float(u8),         // float32, float64
    Bool,
    Char,
    String,
    Void,
    Bytes,
    List(Box<MonoType>),
    Dict(Box<MonoType>, Box<MonoType>),
    Set(Box<MonoType>),              // 集合类型
    Tuple(Vec<MonoType>),
    Struct(StructType),
    Enum(EnumType),
    Fn {
        params: Vec<MonoType>,
        return_type: Box<MonoType>,
        is_async: bool,
    },
    TypeVar(TypeVar),  // 类型变量
    Range {
        elem_type: Box<MonoType>,
    },
}

// 多态类型 (PolyType) - 泛型多态
struct PolyType {
    quantifiers: Vec<TypeVar>,  // 泛型参数
    body: MonoType,             // 类型体
}
```

## 类型约束求解器

```rust
struct TypeConstraintSolver {
    constraints: Vec<TypeConstraint>,
    substitutions: HashMap<TypeVar, MonoType>,
}

enum TypeConstraint {
    Equal(MonoType, MonoType, Span),
}
```

## 相关文件

- **types.rs**: MonoType, PolyType, TypeConstraintSolver 定义
