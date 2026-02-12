# RFC 示例：增强模式匹配语法

> **注意**: 这是一个 RFC 模板示例，展示了完整 RFC 提案的写法。
> 请参考此模板来编写您自己的 RFC。
>
> **状态**: 示例（仅供参考）

> **作者**: 晨煦（示例作者）
> **创建日期**: 2025-01-05
> **最后更新**: 2025-01-05

## 摘要

为 YaoXiang 添加更强大的模式匹配能力，包括嵌套模式、卫表达式和 `let` 模式绑定。

## 动机

### 为什么需要这个特性？

当前 `match` 表达式功能有限，无法处理以下常见场景：

```yaoxiang
# 无法解构嵌套结构
type Person = Person(name: String, address: Address)
type Address = Address(city: String, zip: Int)
match person {
    Person(name: "Alice", address: Address(city: "Beijing", _)) => "Alice from Beijing"  # ❌ 不支持
}

# 无法在模式中绑定变量
match result {
    ok(value) => print(value)          # ❌ 需要显式解构
}
```

### 当前的问题

1. 嵌套模式解构不支持
2. 模式中无法使用卫表达式
3. `let` 语句不支持模式匹配

## 提案

### 核心设计

扩展 `match` 表达式语法，支持：

1. **嵌套模式解构**：任意深度的结构体解构
2. **卫表达式**：在模式后添加 `if` 条件
3. **模式变量绑定**：直接从模式中绑定变量

### 示例

```yaoxiang
# 嵌套解构
type Person = Person(name: String, address: Address)
type Address = Address(city: String, zip: Int)

match person {
    Person(name: "Alice", address: Address(city: "Beijing", _)) => "Alice from Beijing"
    Person(name: n, address: Address(city: c, _)) => n + " from " + c
}

# 卫表达式
match n {
    n if n > 0 && n < 10 => "1-9"
    n if n >= 10 => "10+"
    _ => "unknown"
}

# 模式绑定
match result {
    ok(value) => print(value)          # value 已绑定
    err(e) => log_error(e)
}

# 嵌套 + 绑定
match data {
    User(name: first, profile: Profile(age: a)) if a >= 18 => first + " is adult"
}
```

### `let` 语句模式匹配

```yaoxiang
# 新语法
let Point(x: 0, y: _) = point  # 仅当 x == 0 时绑定
let Ok(value) = result         # 解构 Result

# 多重绑定
let (a, b, c) = tuple          # 解构元组
```

## 详细设计

### 语法变化

```
MatchExpr   ::= 'match' Expr '{' MatchArm+ '}'
MatchArm    ::= Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
Pattern     ::= LiteralPattern
              | IdentifierPattern
              | StructPattern
              | TuplePattern
              | OrPattern
              | RestPattern

LiteralPattern ::= '_' | Literal
IdentifierPattern ::= Identifier (':' Pattern)?
StructPattern ::= Identifier '(' FieldPattern (',' FieldPattern)* ','? ')'
FieldPattern  ::= Identifier ':' Pattern | Identifier
TuplePattern  ::= '(' Pattern (',' Pattern)* ','? ')'
OrPattern     ::= Pattern '|' Pattern
RestPattern   ::= '...'
```

### 类型系统影响

- 模式匹配的类型检查需要扩展
- 模式变量在匹配成功时获得正确类型

### 编译器改动

| 组件 | 改动 |
|------|------|
| lexer | 新增模式相关 token |
| parser | 新增模式解析逻辑 |
| typecheck | 模式类型推断和绑定 |
| codegen | 模式匹配代码生成 |

### 向后兼容性

- ✅ 完全向后兼容
- 仅新增语法，原有 `match` 语法不变

## 权衡

### 优点

- 语法更表达力，代码更简洁
- 与主流语言模式匹配保持一致（Rust、Scala、Elixir）
- 减少运行时错误，提前捕获不匹配

### 缺点

- 编译器实现复杂度增加
- 学习曲线略微上升

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| 仅支持顶层解构 | 无法处理常见嵌套场景 |
| 使用函数式风格 | 与命令式代码混用不自然 |
| 延迟到 v2.0 | 用户已有强烈需求 |

## 实现策略

### 阶段划分

1. **阶段 1 (v0.6)**: 嵌套解构和卫表达式
2. **阶段 2 (v0.7)**: 模式变量绑定
3. **阶段 3 (v0.8)**: `let` 模式匹配

### 依赖关系

- 无外部依赖
- 需要先完成基础类型系统

### 风险

- 模式编译复杂度可能导致性能问题
- 嵌套过深可能栈溢出

## 开放问题

1. [ ] 循环模式（`@` 绑定）的语法？
2. [ ] 是否支持编译时模式穷尽检查？
3. [ ] 性能优化策略？

## 参考文献

- [Rust 模式匹配](https://doc.rust-lang.org/book/ch18-00-patterns.html)
- [Scala 模式匹配](https://docs.scala-lang.org/tour/pattern-matching.html)
- [Elixir 模式匹配](https://elixir-lang.org/getting-started/pattern-matching.html)
