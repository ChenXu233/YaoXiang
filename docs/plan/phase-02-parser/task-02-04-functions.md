# Task 2.4: 函数解析

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

解析函数定义和函数类型注解。

## 函数语法

### 函数定义

```yaoxiang
# 完整形式
add: (Int, Int) -> Int = (a, b) => a + b

# 简写形式（自动推断类型）
add(a, b) = a + b

# 命名参数
greet(name: String) = "Hello, " + name

# 默认参数
config(path: String = "config.yaml") = path
```

### 函数类型

```yaoxiang
# 函数类型注解
callback: (Int) -> Bool

# 多参数函数
merge(a: Int, b: Int, c: Int) = a + b + c
```

## 输入示例

```rust
// Token 序列
Identifier("add"), Colon, LParen, KwInt, Comma, KwInt, RParen, Arrow, KwInt,
Eq, LParen, Identifier("a"), Comma, Identifier("b"), RParen, Arrow,
Identifier("a"), Plus, Identifier("b")
```

## 输出示例

```rust
Stmt::Function {
    name: "add",
    params: [
        Param { name: "a", ty: Type::Int, default: None },
        Param { name: "b", ty: Type::Int, default: None },
    ],
    return_ty: Some(Type::Int),
    body: Expr::Binary {
        op: BinaryOp::Add,
        left: Expr::Identifier("a"),
        right: Expr::Identifier("b"),
    },
}
```

## 验收测试

```yaoxiang
# test_functions.yx

# 基础函数
add(a, b) = a + b
assert(add(1, 2) == 3)

# 带类型注解
mul: (Int, Int) -> Int = (a, b) => a * b
assert(mul(3, 4) == 12)

# 嵌套函数
outer(x) = inner(y) = x + y
assert(outer(10)(5) == 15)

# 高阶函数
apply(f, x) = f(x)
assert(apply(add, 5) == 8)

# 递归函数
fact(n) = if n <= 1 { 1 } else { n * fact(n - 1) }
assert(fact(5) == 120)

print("Function parsing tests passed!")
```

## 相关文件

- **mod.rs**: parse_function(), parse_param_list()
- **ast.rs**: Function, Param, FunctionType
