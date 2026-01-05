# Task 1.2: 字面量识别

> **优先级**: P0
> **状态**: ✅ 已完成

## 功能描述

识别数字、字符串、字符字面量，生成对应的 Token。

## 字面量类型

| 类型 | 示例 | TokenKind |
|------|------|-----------|
| 整数 | `42`, `0xFF`, `0b1010` | `IntLiteral(i128)` |
| 浮点数 | `3.14`, `1e10` | `FloatLiteral(f64)` |
| 字符串 | `"hello"` | `StringLiteral(String)` |
| 字符 | `'a'`, `'\\n'` | `CharLiteral(char)` |
| 布尔 | `true`, `false` | `BoolLiteral(bool)` |

## 整数格式支持

```rust
// 十进制
42
-123

// 十六进制
0xFF      // 255
0xDEADBEEF

// 八进制
0o755

// 二进制
0b1010    // 10
0b11110000

// 下划线分隔（可读性）
1_000_000
0xDEAD_BEEF
```

## 浮点数格式支持

```rust
// 基础形式
3.14
0.5
.5  // 0.5

// 科学计数法
1e10
1.5e-5
3.14E10
```

## 转义序列

```rust
// 字符串转义
"\n"  // 换行
"\t"  // 制表符
"\\"  // 反斜杠
"\""  // 双引号
"\'"  // 单引符
"\0"  // 空字符

// 十六进制转义
"\xFF"

// Unicode 转义
"\u{1F600}"  // 😀
```

## 验收测试

```yaoxiang
# test_literals.yx

# 整数
assert(42 == 42)
assert(0xFF == 255)
assert(0b1010 == 10)
assert(1_000_000 == 1000000)

# 浮点数
assert(3.14 == 3.14)
assert(0.5 == 0.5)
assert(1e5 == 100000.0)

# 字符串
s = "hello, world!"
assert(s.length == 13)
assert(s[0] == 'h')

# 字符
assert('a' == 'a')
assert('\n' == '\n')
assert('\x41' == 'A')  // A

# 布尔
assert(true == true)
assert(false == false)
assert(!false == true)

print("All literal tests passed!")
```

## 相关文件

- **tokens.rs**: Literal 枚举
- **mod.rs**: scan_number(), scan_string(), scan_char()
