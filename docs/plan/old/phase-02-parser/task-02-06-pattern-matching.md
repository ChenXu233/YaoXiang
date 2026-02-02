# Task 2.6: 模式匹配解析

> **优先级**: P1
> **状态**: ✅ 已实现

## 功能描述

解析 match 表达式中的模式匹配语法。

## 模式类型

| 模式类型 | 示例 | 说明 |
|----------|------|------|
| 通配符 | `_` | 忽略值 |
| 标识符模式 | `x` | 绑定任意值 |
| 字面量模式 | `42`, `"hello"` | 匹配具体值 |
| 构造模式 | `Some(x)`, `Point(x, y)` | 匹配变体/构造器 |
| 元组模式 | `(a, b, c)` | 匹配元组 |
| 结构体模式 | `Point(x: 0, y: 0)` | 匹配命名结构体 |
| Or 模式 | `a \| b` | 或模式 |
| 守卫模式 | `x if x > 0` | 带条件的模式 |

## match 语法

```yaoxiang
# 基础 match
match value {
    1 => "one"
    2 => "two"
    _ => "other"
}

# 构造模式
match point {
    Point(x: 0, y: 0) => "origin"
    Point(x, y) => "other"
}

# 守卫模式
match n {
    x if x < 0 => "negative"
    0 => "zero"
    x => "positive"
}

# 嵌套模式
match result {
    Ok(value) => value
    Err(e) => panic(e)
}

# Or 模式
match status {
    200 | 201 => "success"
    404 => "not found"
    _ => "error"
}
```

## Pattern 枚举

```rust
pub enum Pattern {
    Wildcard,                    // _
    Identifier(String),          // x
    Literal(Literal),            // 42, "hello"
    Tuple(Vec<Pattern>),         // (a, b, c)
    Struct {                     // Point(x: 0, y: 0)
        name: String,
        fields: Vec<(String, Pattern)>,
    },
    Union {                      // Variant::Case(pattern)
        name: String,
        variant: String,
        pattern: Option<Box<Pattern>>,
    },
    Or(Vec<Pattern]),            // a | b
    Guard {                      // x if x > 0
        pattern: Box<Pattern>,
        condition: Expr,
    },
}
```

## MatchArm 结构

```rust
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
    pub span: Span,
}
```

## 输入示例

```rust
// Token 序列
KwMatch, Identifier("opt"), LBrace,
    Identifier("Some"), LParen, Identifier("x"), RParen, FatArrow, Identifier("x"),
    Comma,
    Identifier("None"), FatArrow, IntLiteral(0),
RBrace
```

## 输出示例

```rust
Expr::Match {
    expr: Expr::Var("opt", span),
    arms: vec![
        MatchArm {
            pattern: Pattern::Struct {
                name: "Some",
                fields: vec![("x", Pattern::Identifier("x"))],
            },
            body: Expr::Var("x", span),
            span,
        },
        MatchArm {
            pattern: Pattern::Identifier("None".to_string()),
            body: Expr::Lit(Literal::Int(0), span),
            span,
        },
    ],
    span,
}
```

## 验收测试

```yaoxiang
# test_pattern_matching.yx

# 基础模式匹配
describe(n) = match n {
    0 => "zero"
    1 => "one"
    _ => "many"
}
assert(describe(0) == "zero")
assert(describe(5) == "many")

# 构造模式
match Option::Some(42) {
    Some(x) => x
    None => 0
}

# 守卫模式
sign(n) = match n {
    x if x < 0 => "negative"
    0 => "zero"
    x => "positive"
}
assert(sign(-5) == "negative")

# 嵌套模式
result = match Result::Ok(Option::Some("value")) {
    Ok(Some(s)) => s
    _ => "default"
}
assert(result == "value")

# Or 模式
grade(score) = match score {
    90 | 91 | 92 | 93 | 94 | 95 | 96 | 97 | 98 | 99 | 100 => "A"
    80 | 81 | 82 | 83 | 84 | 85 | 86 | 87 | 88 | 89 => "B"
    _ => "C"
}
assert(grade(95) == "A")

print("Pattern matching tests passed!")
```

## 相关文件

- **[`nud.rs`](nud.rs:388)**: `parse_match()`, `parse_pattern()`
- **[`ast.rs`](ast.rs:220)**: `Pattern`, `MatchArm` 定义
