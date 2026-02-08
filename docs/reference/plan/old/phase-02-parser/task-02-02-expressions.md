# Task 2.2: 表达式解析

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

解析各类表达式，包括字面量、算术、逻辑、比较、函数调用等。使用 Pratt Parser 实现完整的表达式解析。

## 表达式类型

| 类型 | 示例 | AST 节点 |
|------|------|----------|
| 字面量 | `42`, `3.14`, `"hello"` | `Expr::Lit` |
| 标识符 | `x`, `my_func` | `Expr::Var` |
| 算术/比较/逻辑 | `a + b`, `x == y`, `a && b` | `Expr::BinOp` |
| 一元运算 | `-x`, `!flag` | `Expr::UnOp` |
| 函数调用 | `foo(a, b)` | `Expr::Call` |
| 属性访问 | `point.x` | `Expr::FieldAccess` |
| 索引 | `arr[0]` | `Expr::Index` |
| 元组 | `(1, 2, 3)` | `Expr::Tuple` |
| 列表 | `[1, 2, 3]` | `Expr::List` |
| 块表达式 | `{ a = 1; b = 2 }` | `Expr::Block` |
| Lambda | `x => x + 1` | `Expr::FnDef` |
| 范围 | `1..10` | `Expr::BinOp` with `Range` |
| 列表推导 | `[x for x in list if x > 0]` | `Expr::ListComp` |
| 类型转换 | `x as Int` | `Expr::Cast` |

## 输入示例

```rust
// Token 序列
Identifier("result"), Eq, IntLiteral(10), Plus, IntLiteral(20), Semicolon
```

## 输出示例

```rust
Expr::BinOp {
    op: BinOp::Add,
    left: Box::new(Expr::Lit(Literal::Int(10), span)),
    right: Box::new(Expr::Lit(Literal::Int(20), span)),
    span,
}
```

## 二元运算符

```rust
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Le, Gt, Ge,
    And, Or,
    Range,      // ..
    Assign,     // =
}
```

## 一元运算符

```rust
pub enum UnOp {
    Neg,  // -
    Pos,  // +
    Not,  // !
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

# 一元表达式
assert(-5 < 0)
assert(!!true)

# 函数调用
add(a, b) = a + b
assert(add(1, 2) == 3)

# 属性访问
point = Point(x: 1, y: 2)
assert(point.x == 1)

# 索引
arr = [1, 2, 3]
assert(arr[0] == 1)

# 范围表达式
range = 1..5
len = 4

# 类型转换
num = 42 as Float
assert(num > 40.0)

# 列表推导
squares = [x * x for x in 1..10]
evens = [x for x in list if x % 2 == 0]

print("Expression parsing tests passed!")
```

## 列表推导式

### 语法

```yaoxiang
# 基础列表推导
squares = [x * x for x in 1..10]

# 带条件的列表推导
evens = [x for x in list if x % 2 == 0]

# 嵌套列表推导
matrix = [[x * y for y in 1..3] for x in 1..3]

# 模式匹配 + 条件
results = [value for Ok(value) in results if value > 0]
```

### ListComp 表达式

```rust
pub enum Expr {
    // ... 其他变体
    ListComp {
        element: Box<Expr>,      // 元素表达式 x * x
        var: String,             // 迭代变量名 x
        iterable: Box<Expr>,     // 可迭代对象 1..10
        condition: Option<Box<Expr>>,  // 过滤条件 if x > 0
        span: Span,
    },
}
```

### 语法规则

```ebnf
list_comp = "[" expression "for" identifier "in" expression ["if" expression] "]"
```

## 相关文件

- **[`expr.rs`](expr.rs:22)**: Pratt parser 主循环
- **[`nud.rs`](nud.rs:16)**: 前缀表达式解析
- **[`led.rs`](led.rs:17)**: 中缀表达式解析
- **[`ast.rs`](ast.rs:8)**: `Expr`, `BinOp`, `UnOp` 定义
