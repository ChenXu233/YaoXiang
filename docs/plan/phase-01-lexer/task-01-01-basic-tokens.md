# Task 1.1: 关键字和标识符

> **优先级**: P0
> **状态**: ✅ 已完成

## 功能描述

识别语言关键字和标识符，生成对应的 Token。

## 关键字列表（17个）

| 分类 | 关键字 |
|------|--------|
| 类型定义 | `type` |
| 可见性 | `pub` |
| 模块 | `use` |
| 并发 | `spawn` |
| 引用修饰 | `ref`, `mut` |
| 条件 | `if`, `elif`, `else` |
| 模式匹配 | `match` |
| 循环 | `while`, `for`, `in` |
| 控制流 | `return`, `break`, `continue` |
| 类型转换 | `as` |
| 字面量 | `true`, `false` |

## 输入

```rust
// 源代码字符串
"type Point = Point(x: Float, y: Float)"
```

## 输出

```rust
// Token 序列
Token {
    kind: TokenKind::KwType,
    span: Span { start: Position(1,1), end: Position(1,5) },
    literal: None,
},
Token {
    kind: TokenKind::Identifier("Point"),
    span: Span { ... },
    literal: None,
},
Token {
    kind: TokenKind::Eq,
    span: Span { ... },
    literal: None,
},
// ...
```

## 验收测试

```yaoxiang
# test_keywords.yx

# 关键字识别
type MyType = MyType(Int, Int)
pub use std::io
if x > 0 { "positive" } else { "negative" }
match value { 1 => "one", _ => "other" }
for i in 0..10 { print(i) }
spawn { compute_heavy_task() }

# 标识符
my_variable = 42
function_name(x, y) = x + y
_private_data = "secret"

# 下划线标识符
_ = compute_result()  # 丢弃结果
value_1 = 10
_value_2 = 20

print("All keyword tests passed!")
```

## 相关文件

- **tokens.rs**: TokenKind 枚举定义
- **mod.rs**: keyword_from_str() 实现
