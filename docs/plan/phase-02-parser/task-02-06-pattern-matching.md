# Task 2.6: 模式匹配解析

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

解析 match 表达式中的模式匹配语法。

## 模式类型

| 模式类型 | 示例 | 说明 |
|----------|------|------|
| 标识符模式 | `x` | 绑定任意值 |
| 字面量模式 | `42`, `"hello"` | 匹配具体值 |
| 构造模式 | `Some(x)`, `Point(x, y)` | 匹配变体/构造器 |
| 元组模式 | `(a, b, c)` | 匹配元组 |
| 守卫模式 | `x if x > 0` | 带条件的模式 |
| 通配符 | `_` | 忽略值 |

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
    expr: Expr::Identifier("opt"),
    arms: vec![
        MatchArm {
            pattern: Pattern::Constructor("Some", vec![Pattern::Binding("x")]),
            guard: None,
            body: Expr::Identifier("x"),
        },
        MatchArm {
            pattern: Pattern::Constructor("None", vec![]),
            guard: None,
            body: Expr::Literal(Literal::Int(0)),
        },
    ],
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

print("Pattern matching tests passed!")
```

## 相关文件

- **mod.rs**: parse_match(), parse_pattern()
- **ast.rs**: Match, MatchArm, Pattern
