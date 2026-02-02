# Task 2.4: 函数解析

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

解析函数定义和函数类型注解。

## 函数语法

YaoXiang 使用赋值式语法定义函数，而非传统的 `fn` 关键字语法。

### 函数定义

```yaoxiang
# 完整形式（类型注解 + 实现）
add: (Int, Int) -> Int = (a, b) => a + b

# 简写形式（自动推断类型）
add(a, b) = a + b

# 单参数 lambda 语法
inc = x => x + 1

# 带类型注解的简写
mul: (Int, Int) -> Int = (a, b) => a * b

# 命名参数
greet(name: String) = "Hello, " + name
```

### 函数类型注解

```yaoxiang
# 函数类型
callback: (Int) -> Bool

# 多参数函数
merge(a: Int, b: Int, c: Int) = a + b + c

# 函数作为值
apply: ((Int) -> Int, Int) -> Int
apply(f, x) = f(x)
```

## 输入示例

```rust
// Token 序列
Identifier("add"), Colon, LParen, KwInt, Comma, KwInt, RParen, Arrow, KwInt,
Eq, LParen, Identifier("a"), Comma, Identifier("b"), RParen, FatArrow,
Identifier("a"), Plus, Identifier("b")
```

## 输出示例

```rust
Stmt {
    kind: StmtKind::Fn {
        name: "add",
        type_annotation: Some(Type::Fn {
            params: vec![Type::Int, Type::Int],
            return_type: Box::new(Type::Int),
        }),
        params: vec![
            Param { name: "a", ty: None, span },
            Param { name: "b", ty: None, span },
        ],
        body: (
            vec![],
            Some(Box::new(Expr::BinOp {
                op: BinOp::Add,
                left: Box::new(Expr::Var("a", span)),
                right: Box::new(Expr::Var("b", span)),
                span,
            })),
        ),
    },
    span,
}
```

## 函数定义语法规则

```ebnf
function_def = identifier ":" type_annotation "=" lambda
              | identifier ["(" param_list ")"] "=" lambda

lambda       = "(" param_list ")" "=>" body
              | identifier "=>" body

param_list   = param { "," param }
param        = identifier [":" type_annotation]

body         = block_expression | expression
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

# 单参数 lambda
inc = x => x + 1
assert(inc(5) == 6)

# 嵌套函数
outer(x) = inner(y) = x + y
assert(outer(10)(5) == 15)

# 高阶函数
apply(f, x) = f(x)
assert(apply(inc, 5) == 6)

# 递归函数
fact(n) = if n <= 1 { 1 } else { n * fact(n - 1) }
assert(fact(5) == 120)

print("Function parsing tests passed!")
```

## 相关文件

- **[`stmt.rs`](stmt.rs:943)**: `parse_fn_stmt()`, `parse_fn_params()`
- **[`ast.rs`](ast.rs:27)**: `Expr::FnDef`, `Param`
- **[`nud.rs`](nud.rs:227)**: lambda 表达式解析
