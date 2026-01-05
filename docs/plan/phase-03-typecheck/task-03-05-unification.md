# Task 3.5: 类型统一

> **优先级**: P0
> **状态**: ⚠️ 部分实现

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

print("Unification tests passed!")
```

## 相关文件

- **types.rs**: TypeConstraintSolver 实现
