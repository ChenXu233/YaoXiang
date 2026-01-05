# Task 2.1: 基础解析

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

实现基础的 Token 到 AST 节点转换，包括表达式和语句的初步解析框架。

## 输入

```rust
// Token 序列
[Identifier("x"), Eq, IntLiteral(42), Semicolon]
```

## 输出

```rust
// AST 节点
Stmt::Assignment {
    target: Expr::Identifier("x"),
    value: Expr::Literal(Literal::Int(42)),
}
```

## 解析规则

### 运算符优先级（从低到高）

| 优先级 | 运算符 |
|--------|--------|
| 1 | `=`, `:=` |
| 2 | `\|\|` |
| 3 | `&&` |
| 4 | `==`, `!=` |
| 5 | `<`, `<=`, `>`, `>=` |
| 6 | `+`, `-` |
| 7 | `*`, `/`, `%` |
| 8 | 函数调用、属性访问 |

## 验收测试

```yaoxiang
# test_basic_parsing.yx

# 变量声明
x = 42
y: Int = 100

# 赋值
x = 10
result = x + y

# 条件表达式
status = if x > 0 { "positive" } else { "negative" }

print("Basic parsing tests passed!")
```

## 相关文件

- **mod.rs**: parse_program(), parse_stmt()
- **ast.rs**: Stmt, Expr 定义
