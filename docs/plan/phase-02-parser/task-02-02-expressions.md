# Task 2.2: 表达式解析

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

解析各类表达式，包括字面量、算术、逻辑、比较、函数调用等。

## 表达式类型

| 类型 | 示例 | AST 节点 |
|------|------|----------|
| 字面量 | `42`, `3.14`, `"hello"` | `Expr::Literal` |
| 标识符 | `x`, `my_func` | `Expr::Identifier` |
| 算术 | `a + b`, `x * y` | `Expr::Binary` |
| 逻辑 | `a && b`, `!flag` | `Expr::Logical` |
| 比较 | `x == y`, `a < b` | `Expr::Compare` |
| 函数调用 | `foo(a, b)` | `Expr::Call` |
| 属性访问 | `point.x` | `Expr::Field` |
| 元组 | `(1, 2, 3)` | `Expr::Tuple` |
| 分组 | `(a + b)` | `Expr::Group` |

## 输入示例

```rust
// Token 序列
Identifier("result"), Eq, IntLiteral(10), Plus, IntLiteral(20), Semicolon
```

## 输出示例

```rust
Expr::Binary {
    op: BinaryOp::Add,
    left: Expr::Literal(Literal::Int(10)),
    right: Expr::Literal(Literal::Int(20)),
}
```

## 验收测试

```yaoxiang
# test_expressions.yx

# 算术表达式
assert(1 + 2 == 3)
assert(10 - 4 == 6)
assert(3 * 5 == 15)
assert(10 / 2 == 5)
assert(7 % 3 == 1)

# 比较表达式
assert(1 < 10)
assert(5 >= 5)
assert(3 != 4)

# 逻辑表达式
assert(true && true)
assert(false || true)
assert(!false)

# 函数调用
add(a, b) = a + b
assert(add(1, 2) == 3)

# 属性访问
point = Point(x: 1, y: 2)
assert(point.x == 1)

print("Expression parsing tests passed!")
```

## 相关文件

- **mod.rs**: parse_expr(), parse_call()
- **ast.rs**: Expr, BinaryOp, LogicalOp, CompareOp
