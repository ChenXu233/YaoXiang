# Task 3.2: 类型推断

> **优先级**: P0
> **状态**: ⚠️ 部分实现

## 功能描述

使用 Hindley-Milner 算法推断表达式的类型。

## 推断规则

| 表达式类型 | 推断规则 |
|------------|----------|
| 字面量 | 直接返回对应类型 |
| 变量 | 从作用域查找并实例化 |
| 二元运算 | 约束左右操作数类型，返回结果类型 |
| 函数调用 | 约束参数类型，返回函数返回类型 |
| if 表达式 | 所有分支类型必须一致 |
| match 表达式 | 所有 arm 类型必须一致 |

## 推断方法

```rust
impl<'a> TypeInferrer<'a> {
    /// 推断表达式类型
    pub fn infer_expr(&mut self, expr: &ast::Expr) -> TypeResult<MonoType> { ... }

    /// 推断函数定义类型
    fn infer_fn_def_expr(...) -> TypeResult<MonoType> { ... }

    /// 推断函数调用类型
    pub fn infer_call(...) -> TypeResult<MonoType> { ... }

    /// 推断 if 表达式类型
    fn infer_if(...) -> TypeResult<MonoType> { ... }

    /// 推断 match 表达式类型
    fn infer_match(...) -> TypeResult<MonoType> { ... }
}
```

## 验收测试

```yaoxiang
# test_type_inference.yx

# 字面量推断
x = 42        # 推断为 Int
y = 3.14      # 推断为 Float
z = "hello"   # 推断为 String

# 表达式推断
sum = x + y   # 推断为 Float
result = x > 0  # 推断为 Bool

# 函数调用推断
add(a, b) = a + b  # 推断为 (T, T) -> T
double(n) = n * 2  # 推断为 (Int) -> Int

# if 表达式推断
sign(n) = if n > 0 { "positive" } else { "negative" }
# 推断为 (Int) -> String

print("Type inference tests passed!")
```

## 相关文件

- **infer.rs**: TypeInferrer 实现
