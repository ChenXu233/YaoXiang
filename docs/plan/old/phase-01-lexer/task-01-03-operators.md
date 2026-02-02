# Task 1.3: 运算符和分隔符

> **优先级**: P0
> **状态**: ✅ 已完成

## 功能描述

识别运算符和分隔符，生成对应的 Token。

## 运算符列表

### 算术运算符

| 运算符 | 含义 | TokenKind |
|--------|------|-----------|
| `+` | 加 | `Plus` |
| `-` | 减 | `Minus` |
| `*` | 乘 | `Star` |
| `/` | 除 | `Slash` |
| `%` | 取模 | `Percent` |

### 比较运算符

| 运算符 | 含义 | TokenKind |
|--------|------|-----------|
| `==` | 等于 | `EqEq` |
| `!=` | 不等于 | `Neq` |
| `<` | 小于 | `Lt` |
| `<=` | 小于等于 | `Le` |
| `>` | 大于 | `Gt` |
| `>=` | 大于等于 | `Ge` |

### 逻辑运算符

| 运算符 | 含义 | TokenKind |
|--------|------|-----------|
| `&&` | 逻辑与 | `And` |
| `||` | 逻辑或 | `Or` |
| `!` | 逻辑非 | `Not` |

### 其他运算符

| 运算符 | 含义 | TokenKind |
|--------|------|-----------|
| `::` | 命名空间 | `ColonColon` |
| `...` | 范围包含 | `DotDotDot` |
| `..` | 范围不包含 | `DotDot` |
| `->` | 返回类型 | `Arrow` |
| `=>` | 函数箭头 | `FatArrow` |

### 分隔符

| 符号 | 含义 | TokenKind |
|------|------|-----------|
| `(` `)` | 括号 | `LParen`, `RParen` |
| `[` `]` | 方括号 | `LBracket`, `RBracket` |
| `{` `}` | 大括号 | `LBrace`, `RBrace` |
| `,` | 逗号 | `Comma` |
| `:` | 冒号 | `Colon` |
| `;` | 分号 | `Semicolon` |
| `|` | 管道 | `Pipe` |
| `.` | 点 | `Dot` |

## 优先级解析

运算符优先级在**语法分析器**（Parser）处理，不在词法分析器。

## 验收测试

```yaoxiang
# test_operators.yx

# 算术
assert(1 + 2 == 3)
assert(5 - 3 == 2)
assert(4 * 3 == 12)
assert(10 / 2 == 5)
assert(7 % 3 == 1)

# 比较
assert(1 == 1)
assert(2 != 3)
assert(1 < 2)
assert(2 <= 2)
assert(3 > 1)
assert(3 >= 3)

# 逻辑
assert(true && true)
assert(true || false)
assert(!false)

# 混合
assert((1 + 2) * 3 == 9)
assert(1 < 2 && 3 < 4)
assert(true || false && false)

# 分隔符
result = (1, 2, 3)  # 元组
point = Point(x: 1, y: 2)  # 命名参数

# 范围操作
range = 1..10  # 不包含 10
full_range = 1...10  # 包含 10

# 函数箭头
add: (Int, Int) -> Int = (a, b) => a + b
is_even: Int -> Bool = (n) => n % 2 == 0

print("All operator tests passed!")
```

## 相关文件

- **tokens.rs**: TokenKind 运算符变体
- **mod.rs**: next_token() 中的运算符匹配
