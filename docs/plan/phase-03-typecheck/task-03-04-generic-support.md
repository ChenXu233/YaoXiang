# Task 3.4: 泛型支持

> **优先级**: P1
> **状态**: ⚠️ 部分实现

## 功能描述

支持泛型类型和泛型函数的类型推断与检查。

## 泛型语法

```yaoxiang
# 泛型函数
identity[T](x: T): T = x

# 泛型类型
List[T] = struct {
    elements: List[T],
    length: Int,
}

# 泛型约束
print_value[T: Printable](value: T) = value.to_string()
```

## 泛型处理

```rust
// 泛型特化
impl TypeSpecializer {
    /// 特化泛型类型
    pub fn specialize(ty: &PolyType, args: &[MonoType]) -> Result<MonoType, TypeError> { ... }

    /// 泛型实例化
    pub fn instantiate(poly: &PolyType) -> MonoType { ... }
}

// 类型约束求解
impl TypeConstraintSolver {
    /// 泛化类型
    pub fn generalize(&self, ty: &MonoType) -> PolyType { ... }

    /// 泛型参数替换
    pub fn substitute(&self, ty: &MonoType, mapping: &HashMap<TypeVar, MonoType>) -> MonoType { ... }
}
```

## 验收测试

```yaoxiang
# test_generics.yx

# 泛型函数
identity[T](x: T): T = x
assert(identity(42) == 42)
assert(identity("hello") == "hello")

# 泛型类型
list = List[Int]([1, 2, 3])
assert(list.length == 3)

# 多泛型参数
pair[A, B](a: A, b: B): (A, B) = (a, b)
result = pair(1, "hello")
assert(result[0] == 1)
assert(result[1] == "hello")

# 泛型约束
has_repr[T](x: T): String = x.to_string()
assert(has_repr(42) == "42")

print("Generic support tests passed!")
```

## 相关文件

- **specialize.rs**: TypeSpecializer 实现
