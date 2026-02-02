# Task 3.2: 类型推断

> **优先级**: P0
> **状态**: ✅ 已实现

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
| match 表达式 | 所有 arm 类型必须一致；模式类型必须与被匹配值类型兼容 |
| 模式 | 推断模式绑定的变量类型，支持字面量、结构体、元组、枚举模式 |

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

    /// 推断模式类型
    pub fn infer_pattern(&mut self, pattern: &ast::Pattern) -> TypeResult<MonoType> { ... }

    /// 推断类型转换 (as)
    fn infer_cast(...) -> TypeResult<MonoType> { ... }

    /// 推断元组类型
    fn infer_tuple(...) -> TypeResult<MonoType> { ... }

    /// 推断列表类型
    fn infer_list(...) -> TypeResult<MonoType> { ... }

    /// 推断字典类型
    fn infer_dict(...) -> TypeResult<MonoType> { ... }
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

# 模式匹配推断
type Option[T] = some(T) | none
first_opt = some(42)      # 推断为 Option[Int]
result = match first_opt {
    some(n) => n * 2      # n 推断为 Int
    none => 0
}  # 推断为 Int

# 结构体模式匹配
type Point = Point(x: Int, y: Int)
p = Point(x: 1, y: 2)     # 推断为 Point
match p {
    Point(x, y) => x + y  # x, y 推断为 Int
}  # 推断为 Int

print("Type inference tests passed!")
```

## 相关文件

- **infer.rs**: TypeInferrer 实现
