# Task 2.1: 基础解析

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

实现基于 Pratt Parser 的 Token 到 AST 节点转换，包括表达式和语句的完整解析框架。

## 输入

```rust
// Token 序列
[Identifier("x"), Eq, IntLiteral(42), Semicolon]
```

## 输出

```rust
// AST 节点
Stmt {
    kind: StmtKind::Var {
        name: "x".to_string(),
        type_annotation: None,
        initializer: Some(Box::new(Expr::Lit(Literal::Int(42), span))),
        is_mut: false,
    },
    span,
}
```

## 解析规则

### 运算符优先级（从低到高）

| 优先级 | 绑定功率 | 运算符 |
|--------|----------|--------|
| 1 | BP_ASSIGN (10) | `=` |
| 2 | BP_RANGE (15) | `..` |
| 3 | BP_OR (20) | `\|\|` |
| 4 | BP_AND (30) | `&&` |
| 5 | BP_EQ (40) | `==`, `!=` |
| 6 | BP_CMP (50) | `<`, `<=`, `>`, `>=` |
| 7 | BP_ADD (60) | `+`, `-` |
| 8 | BP_MUL (70) | `*`, `/`, `%` |
| 9 | BP_UNARY (80) | `-`, `+`, `!` (一元) |
| 10 | BP_CALL (90) | 函数调用、属性访问、索引 |

### 解析器架构

```
src/frontend/parser/
├── mod.rs          # parse(), parse_expression(), ParseError
├── state.rs        # ParserState, BP_* 常量
├── ast.rs          # Expr, Stmt, Type 定义
├── nud.rs          # 前缀表达式解析
├── led.rs          # 中缀表达式解析
├── expr.rs         # Pratt parser 主循环
├── stmt.rs         # 语句解析
└── type_parser.rs  # 类型解析
```

## API

### 主要函数

```rust
// 解析完整的 Token 序列为 Module
pub fn parse(tokens: &[Token]) -> Result<Module, ParseError>

// 解析单个表达式
pub fn parse_expression(tokens: &[Token]) -> Result<Expr, ParseError>
```

### ParserState 方法

```rust
impl<'a> ParserState<'a> {
    pub fn parse_expression(&mut self, min_bp: u8) -> Option<Expr>
    pub fn parse_stmt(&mut self) -> Option<Stmt>
    pub fn parse_type_anno(&mut self) -> Option<Type>
}
```

## 验收测试

```yaoxiang
# test_basic_parsing.yx

# 变量声明
x = 42
y: Int = 100
mut z: String = "hello"

# 赋值
x = 10
result = x + y

# 运算符
sum = a + b * c
range = 1..10
check = x > 0 && y < 10

# 条件表达式
status = if x > 0 { "positive" } else { "negative" }

# 函数调用
print("Hello, World!")
result = add(1, 2)

# 属性访问
point.x

# 索引
arr[0]

print("Basic parsing tests passed!")
```

## 错误类型

```rust
pub enum ParseError {
    UnexpectedToken(TokenKind),
    ExpectedToken(TokenKind, TokenKind),
    UnterminatedBlock,
    InvalidExpression,
    InvalidPattern,
    InvalidType,
    MissingSemicolon,
    UnexpectedEof,
    Generic(String),
}
```

## 相关文件

- **[`mod.rs`](mod.rs:34)**: `parse()`, `parse_expression()`, `ParseError`
- **[`state.rs`](state.rs:35)**: `ParserState`, `BP_*` 常量
- **[`ast.rs`](ast.rs:8)**: `Expr`, `Stmt`, `Type`, `Pattern`
- **[`expr.rs`](expr.rs:22)**: Pratt parser 表达式解析
- **[`nud.rs`](nud.rs:16)**: 前缀表达式解析
- **[`led.rs`](led.rs:17)**: 中缀表达式解析
- **[`stmt.rs`](stmt.rs:11)**: 语句解析
