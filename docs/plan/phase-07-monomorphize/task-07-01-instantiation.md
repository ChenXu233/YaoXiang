# Task 7.1: 泛型实例化

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

为每个泛型类型/函数生成具体的实例。

## 实例化算法

```rust
struct MonoState {
    /// 已生成的实例
    instances: HashMap<InstanceKey, FunctionId>,
    /// 实例化请求队列
    pending: Vec<InstanceKey>,
}

struct InstanceKey {
    /// 原始泛型函数/类型
    generic_id: Id,
    /// 类型参数
    type_args: Vec<MonoType>,
}
```

## 实例化示例

```yaoxiang
# 泛型函数
identity[T](x: T): T = x

# 使用时实例化
# identity[Int] -> identity_int
# identity[String] -> identity_string
# identity[Float] -> identity_float

# 泛型类型
List[T] = struct { elements: Array[T], length: Int }

# 使用时实例化
# List[Int] -> List_int
# List[String] -> List_string
```

## 相关文件

- **instantiate.rs**: Instantiator
