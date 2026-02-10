# Task 2.3: 语句解析

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

解析各类语句，包括声明、赋值、控制流等。

## 语句类型

| 类型 | 示例 | AST 节点 |
|------|------|----------|
| 变量声明 | `x = 42`, `y: Int = 10`, `mut z: String` | `StmtKind::Var` |
| 表达式语句 | `foo()` | `StmtKind::Expr` |
| 块语句 | `{ a = 1; b = 2 }` | `StmtKind::Expr(Expr::Block(...))` |
| 返回语句 | `return result` | `StmtKind::Expr(Expr::Return(...))` |
| 条件语句 | `if cond { ... }` | `StmtKind::Expr(Expr::If(...))` |
| while 循环 | `while i < 10 { ... }` | `StmtKind::Expr(Expr::While(...))` |
| for 循环 | `for x in list { ... }` | `StmtKind::For` |
| break/continue | `break`, `continue` | `StmtKind::Expr(Expr::Break/Continue)` |
| 类型定义 | `type Color = red \| green \| blue` | `StmtKind::TypeDef` |
| 模块定义 | `mod my_module { ... }` | `StmtKind::Module` |
| 导入语句 | `use std.io` | `StmtKind::Use` |
| 函数定义 | `add(a, b) = a + b` | `StmtKind::Fn` |

## 输入示例

```rust
// Token 序列
KwMut, Identifier("x"), Colon, KwInt, Eq, IntLiteral(42), Semicolon
```

## 输出示例

```rust
Stmt {
    kind: StmtKind::Var {
        name: "x",
        type_annotation: Some(Type::Name("Int".to_string())),
        initializer: Some(Box::new(Expr::Lit(Literal::Int(42), span))),
        is_mut: true,
    },
    span,
}
```

## 语法变体

### 变量声明

```yaoxiang
# 简单赋值
x = 42

# 类型注解
y: Int = 100

# 可变变量
mut counter: Int = 0

# 仅声明（无初始化）
z: String
```

### 控制流

```yaoxiang
# if 表达式作为语句
if x > 0 {
    positive = true
} else {
    positive = false
}

# while 循环
i = 0
while i < 10 {
    i = i + 1
}

# for 循环
for item in list {
    print(item)
}

# break/continue
j = 0
while true {
    j = j + 1
    if j >= 10 {
        break
    }
    if j % 2 == 0 {
        continue
    }
}
```

## 验收测试

```yaoxiang
# test_statements.yx

# 变量声明和赋值
x = 42
y: Int = 100
mut z = 0
z = 50

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

# for 循环
total = 0
for n in [1, 2, 3, 4, 5] {
    total = total + n
}
assert(total == 15)

# 返回语句
add(a, b) = a + b
assert(add(3, 4) == 7)

print("Statement parsing tests passed!")
```

## 相关文件

- **[`stmt.rs`](stmt.rs:11)**: 语句解析实现
- **[`ast.rs`](ast.rs:114)**: `Stmt`, `StmtKind` 定义
- **[`nud.rs`](nud.rs:318)**: `parse_if`, `parse_while`, `parse_for`
