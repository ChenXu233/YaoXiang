# Task 3.5: 类型统一

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

实现类型统一算法，用于求解类型约束。

## 统一算法

```rust
impl TypeConstraintSolver {
    /// 求解所有类型约束
    pub fn solve(&mut self) -> Result<(), Vec<UnifyError>> { ... }

    /// 统一两个类型
    pub fn unify(&mut self, t1: &MonoType, t2: &MonoType, span: Span) -> Result<(), UnifyError> { ... }
}
```

## 统一规则

| 类型组合 | 统一结果 |
|----------|----------|
| `T1 == T2` | 如果相等则成功，否则失败 |
| `T == TypeVar` | 将 TypeVar 绑定为 T |
| `List[T1] == List[T2]` | 统一 T1 和 T2 |
| `Fn[P1] == Fn[P2]` | 统一参数和返回类型 |
| `Struct[S1] == Struct[S2]` | 结构体类型必须完全匹配 |
| `Union[T1, T2] == Union[T3, T4]` | 元素数量相同且一一兼容 |
| `Union[T1, T2] == T3` | T3 兼容任一成员即成功 |
| `Intersection[T1, T2] == T3` | T3 必须同时兼容所有成员 |
| `Intersection[T1, T2] == Intersection[T3, T4]` | 元素数量相同且一一兼容 |

## 代码实现

```rust
// 联合类型 unify：T1 | T2 == T3 分解为 (T1 == T3) | (T2 == T3)
(MonoType::Union(types), other) | (other, MonoType::Union(types)) => {
    let mut unified = false;
    for member in types {
        if self.unify(member, other).is_ok() {
            unified = true;
            break;
        }
    }
    if !unified {
        return Err(TypeMismatch { ... });
    }
    Ok(())
}

// 交集类型 unify：T1 & T2 == T3 分解为 (T1 == T3) & (T2 == T3)
(MonoType::Intersection(types), other) | (other, MonoType::Intersection(types)) => {
    for member in types {
        self.unify(member, other)?;
    }
    Ok(())
}
```

## 验收测试

```yaoxiang
# test_unification.yx

# 基本类型统一
x = 42
y = x  # Int == Int ✓

# 列表类型统一
nums = [1, 2, 3]
mixed = [1, 2.0]  # ✗ Int != Float

# 函数类型统一
apply(f: (Int) -> Int, x: Int): Int = f(x)
add_one(n: Int): Int = n + 1
result = apply(add_one, 5)  # ✓

# 联合类型统一
type Result[T] = Ok(T) | Err(Exception)
let r: Result[Int] = Ok(42)

# 交集类型统一
trait Printable = { print(self) }
trait Comparable = { compare(self, other) -> Int }

print("Unification tests passed!")
```

## 相关文件

- **types.rs**: TypeConstraintSolver 实现（包含 Union/Intersection unify 逻辑）
