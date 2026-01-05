# Task 8.1: Value 类型定义

> **优先级**: P0
> **状态**: ⚠️ 需重构

## 功能描述

定义运行时值的统一表示，包括所有内建类型。

## Value 结构

```rust
/// 运行时值类型
enum Value {
    /// 空值
    Null,

    /// 布尔值
    Bool(bool),

    /// 整数（i8, i16, i32, i64）
    Int(i8, i16, i32, i64),  // 存储实际值和类型宽度

    /// 浮点数（f32, f64）
    Float(f32, f64),  // 存储实际值和类型宽度

    /// 字符
    Char(char),

    /// 字符串
    String(Handle<StringObject>),

    /// 字节数组
    Bytes(Handle<BytesObject>),

    /// 列表
    List(Handle<ListObject>),

    /// 字典
    Dict(Handle<DictObject>),

    /// 元组
    Tuple(Handle<TupleObject>),

    /// 结构体实例
    Struct(Handle<StructObject>),

    /// 枚举值
    Enum(Handle<EnumObject>),

    /// 函数闭包
    Fn(Handle<FnObject>),

    /// 类型值
    Type(Handle<TypeObject>),

    /// 范围
    Range(Handle<RangeObject>),

    /// 通道（用于并发）
    Channel(Handle<ChannelObject>),

    /// 互斥锁
    Mutex(Handle<MutexObject>),

    /// 错误
    Error(Handle<ErrorObject>),
}
```

## 内存布局

```rust
struct ValueHeader {
    /// 引用计数（用于 GC）
    ref_count: AtomicUsize,
    /// 类型指针
    type_ptr: *const TypeInfo,
    /// 对象标志
    flags: u8,
}

struct HeapObject {
    header: ValueHeader,
    data: [u8],  // 类型特定数据
}
```

## 相关文件

- `src/runtime/core/value.rs` (目标位置)
- `src/vm/mod.rs` (当前位置 - 需迁移)
