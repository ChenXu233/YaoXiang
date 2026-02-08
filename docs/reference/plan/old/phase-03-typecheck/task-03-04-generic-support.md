# Task 3.4: 泛型支持

> **优先级**: P1
> **状态**: ✅ 已实现

## 功能描述

支持泛型类型和泛型函数的类型推断与检查。

## 泛型语法

```yaoxiang
# 泛型函数（新语法）
identity: [T](T) -> T = x => x

# 泛型类型
type List[T] = List(head: T, tail: Option[List[T]])

# 多泛型参数
pair: [A, B](A, B) -> (A, B) = (a, b) => (a, b)
```

## 泛型处理

泛型处理在 `TypeConstraintSolver` 中实现：

```rust
impl TypeConstraintSolver {
    /// 泛型实例化：将泛型变量替换为新类型变量
    pub fn instantiate(&mut self, poly: &PolyType) -> MonoType { ... }

    /// 泛化类型：将单态类型中的自由变量提取为泛型变量
    pub fn generalize(&self, ty: &MonoType) -> PolyType { ... }

    /// 泛型参数替换
    pub fn substitute(&self, ty: &MonoType, mapping: &HashMap<TypeVar, MonoType>) -> MonoType { ... }
}
```

## 验收测试

```yaoxiang
# test_generics.yx

# 泛型函数（新语法）
identity: [T](T) -> T = x => x
assert(identity(42) == 42)
assert(identity("hello") == "hello")

# 多泛型参数
pair: [A, B](A, B) -> (A, B) = (a, b) => (a, b)
result = pair(1, "hello")
assert(result[0] == 1)
assert(result[1] == "hello")

print("Generic support tests passed!")
```

## 相关文件

- **types.rs**: PolyType, TypeConstraintSolver（含泛型处理）
