# Task 2.3: 语句解析

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

解析各类语句，包括声明、赋值、控制流等。

## 语句类型

| 类型 | 示例 | AST 节点 |
|------|------|----------|
| 变量声明 | `x = 42`, `y: Int = 10` | `Stmt::Let` |
| 赋值 | `x = 100` | `Stmt::Assign` |
| 表达式语句 | `foo()` | `Stmt::Expr` |
| 块语句 | `{ a = 1; b = 2 }` | `Stmt::Block` |
| 返回语句 | `return result` | `Stmt::Return` |
| 条件语句 | `if cond { ... }` | `Stmt::If` |
| 循环语句 | `while i < 10 { ... }` | `Stmt::While` |
| break/continue | `break`, `continue` | `Stmt::Break/Continue` |

## 输入示例

```rust
// Token 序列
KwLet, Identifier("x"), Colon, KwInt, Eq, IntLiteral(42), Semicolon
```

## 输出示例

```rust
Stmt::Let {
    name: "x",
    ty: Some(Type::Int),
    value: Expr::Literal(Literal::Int(42)),
}
```

## 验收测试

```yaoxiang
# test_statements.yx

# 变量声明和赋值
x = 42
y: Int = 100
x = 50

# 条件语句
if x > 30 {
    result = "big"
} else {
    result = "small"
}
assert(result == "big")

# 循环语句
sum = 0
i = 0
while i < 5 {
    sum = sum + i
    i = i + 1
}
assert(sum == 10)

# 返回语句
add(a, b) = a + b
assert(add(3, 4) == 7)

print("Statement parsing tests passed!")
```

## 相关文件

- **mod.rs**: parse_stmt(), parse_if(), parse_while()
- **ast.rs**: Stmt 变体定义
