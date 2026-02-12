---
title: RFC-012：F-String 模板字符串
---

# RFC 012: F-String 模板字符串

> **状态**: 草案
> **作者**: Chen Xu
> **创建日期**: 2025-01-27
> **最后更新**: 2025-01-27

## 摘要

为 YaoXiang 语言添加 f-string 模板字符串特性，支持变量插值、表达式求值和格式化输出。f-string 使用 Python 风格语法（`f"..."` 前缀），在字符串中通过 `{expression}` 语法嵌入表达式，编译时转换为高效的字符串操作。

## 动机

### 为什么需要这个特性？

当前 YaoXiang 的字符串拼接方式较为繁琐：

```yaoxiang
# 现状：使用 + 拼接
name = "Alice"
age = 30
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())
print(message)

# 或者使用 format 函数
message2 = format("Hello {}, age: {}", name, age)
```

### 当前的问题

1. **可读性差**：字符串拼接和格式化需要多处调用，代码冗长
2. **易出错**：手动处理类型转换，容易遗漏 `.to_string()`
3. **性能考虑**：多次字符串拼接可能影响性能
4. **表达力不足**：无法直观地在字符串中嵌入复杂表达式

## 提案

### 核心设计

引入 f-string 作为新的字符串字面量前缀，支持：
- **变量插值**：`f"Hello {name}"`
- **表达式求值**：`f"Sum: {x + y}"`
- **格式化说明符**：`f"Pi: {pi:.2f}"`
- **类型安全**：编译时检查表达式类型

### 示例

```yaoxiang
# 基础插值
name = "Alice"
greeting = f"Hello {name}"  # "Hello Alice"

# 表达式插值
x = 10
y = 20
result = f"Sum: {x + y}"    # "Sum: 30"

# 格式化说明符
pi = 3.14159
formatted = f"Pi: {pi:.2f}"  # "Pi: 3.14"

# 复杂表达式
items = [1, 2, 3]
s = f"Count: {len(items)}, sum: {sum(items)}"  # "Count: 3, sum: 6"

# 对象方法调用
user = User("Bob", 25)
bio = f"Name: {user.name}, age: {user.get_age()}"
```

### 语法变化

| 之前 | 之后 |
|------|------|
| `"Hello ".concat(name)` | `f"Hello {name}"` |
| `format("Value: {}", value)` | `f"Value: {value}"` |
| `format("Pi: {:.2f}", pi)` | `f"Pi: {pi:.2f}"` |

### 语法规范

```
FStringLiteral ::= 'f' '"' FStringContent* '"'
FStringContent ::= FStringChar | EscapeSequence | FStringInterpolation
FStringInterpolation ::= '{' Expression (':' FormatSpec)? '}'
FormatSpec      ::= [precision][type]
```

## 详细设计

### 语法分析

编译器在词法分析阶段识别 `f` 前缀字符串字面量，解析花括号内的表达式和可选的格式化说明符。

### 转换策略

f-string 在编译时转换为高效的字符串操作：

**简单插值**：
```yaoxiang
f"Hello {name}"
```
转换为：
```yaoxiang
"Hello ".concat(name.to_string())
```

**表达式插值**：
```yaoxiang
f"Sum: {x + y}"
```
转换为：
```yaoxiang
"Sum: ".concat((x + y).to_string())
```

**格式化说明符**：
```yaoxiang
f"Pi: {pi:.2f}"
```
转换为：
```yaoxiang
format("Pi: {:.2f}", pi)
```

**多个插值**：
```yaoxiang
f"Hello {name}, you are {age} years old"
```
转换为：
```yaoxiang
"Hello ".concat(name.to_string()).concat(", you are ").concat(age.to_string()).concat(" years old")
```

### 类型系统影响

- 插值表达式必须实现 `Stringable` 接口（自动为基本类型和字符串实现）
- 格式化说明符要求类型支持相应格式化
- 编译器检查表达式类型和格式化规则的匹配

### 编译器改动

| 组件 | 改动 |
|------|------|
| lexer | 识别 f 前缀，解析字符串内插值语法 |
| parser | 新增 FStringLiteral 语法节点 |
| typecheck | 检查插值表达式类型，验证格式化规则 |
| codegen | 生成字符串拼接或格式化调用代码 |

### 向后兼容性

- ✅ 完全向后兼容
- 现有字符串字面量 `"..."` 保持不变
- f-string 是新增语法，不影响现有代码

## 权衡

### 优点

1. **语法简洁**：减少样板代码，提高可读性
2. **类型安全**：编译时检查，减少运行时错误
3. **性能优化**：编译器可优化字符串拼接
4. **表达力强**：支持任意表达式和格式化
5. **学习成本低**：与 Python 生态一致

### 缺点

1. **编译器复杂度**：需要新增语法分析和转换逻辑
2. **语法歧义**：需要与现有字符串语法区分
3. **调试挑战**：编译后代码与源代码结构不同

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| 仅支持变量插值 | 无法满足复杂格式化需求 |
| 使用函数式风格 `format(...)` | 语法不够简洁 |
| 延迟到 v2.0 | 用户对字符串便利性有明确需求 |
| 使用反引号或其他前缀 | 与 Python 生态不一致 |

## 实现策略

### 阶段划分

1. **阶段 1 (v0.9)**:
   - 基础 f-string 语法支持
   - 变量和简单表达式插值
   - 基础类型转换

2. **阶段 2 (v1.0)**:
   - 格式化说明符支持
   - 复杂表达式插值
   - 性能优化

3. **阶段 3 (v1.1)**:
   - 调试信息增强
   - 错误信息改进
   - 更多格式化选项

### 依赖关系

- 无外部依赖
- 需要基础类型系统支持
- 需要字符串库基础功能

### 风险

1. **性能风险**：多个插值可能导致过多字符串对象
   - **缓解**：编译器优化相邻字符串常量合并
2. **类型检查复杂性**：格式化说明符的类型检查
   - **缓解**：参考 Python 实现，使用简单直接的检查
3. **语法歧义**：`{` 和 `}` 的嵌套使用
   - **缓解**：明确语法规则，限制嵌套

## 开放问题

- [ ] 是否支持转义的大括号？`f"{{ literal }}" → "{"`
- [ ] 是否支持自定义格式化函数？
- [ ] 格式化说明符的完整规范？
- [ ] 性能优化的具体策略？
- [ ] 错误诊断的最佳实践？

## 附录

### 附录A：格式化说明符参考

| 类型 | 说明符 | 示例 | 输出 |
|------|--------|------|------|
| 整数 | `d` | `f"{42:d}"` | "42" |
| 浮点 | `f` | `f"{3.14:.2f}"` | "3.14" |
| 科学计数 | `e` | `f"{1000:e}"` | "1.000000e+03" |
| 字符串 | `s` | `f"{name:s}"` | "Alice" |
| 十六进制 | `x` | `f"{255:x}"` | "ff" |

### 附录B：使用场景示例

```yaoxiang
# 日志记录
log(level: String, msg: String, count: Int) = () => {
    timestamp = get_timestamp()
    print(f"[{timestamp}] {level}: {msg} (count: {count})")
}

# JSON 构建
json = "{\n    \"name\": \"".concat(user.name).concat("\",\n    \"age\": ")
    .concat(user.age.to_string()).concat(",\n    \"email\": \"")
    .concat(user.email).concat("\"\n}")

# SQL 查询构建（注意 SQL 注入风险）
query = f"SELECT * FROM users WHERE age > {min_age} AND status = '{status}'"

# 调试信息
debug_info = f"Point({x:.2f}, {y:.2f}) at {timestamp}"

# 条件格式化
status_msg = if is_active {
    f"User {name} is active"
} else {
    f"User {name} is inactive"
}
```

---

## 参考文献

- [Python f-strings](https://docs.python.org/3/tutorial/inputoutput.html#formatted-string-literals)
- [Rust format! macro](https://doc.rust-lang.org/std/macro.format.html)
- [JavaScript template literals](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Template_literals)
- [C# interpolated strings](https://docs.microsoft.com/en-us/dotnet/csharp/language-reference/tokens/interpolated)

---

## 生命周期与归宿

RFC 有以下状态流转：

```
┌─────────────┐
│   草案      │  ← 作者创建
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 社区讨论
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已接受     │    │  已拒绝     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式设计)  │    │ (保留原位)  │
└─────────────┘    └─────────────┘
```
