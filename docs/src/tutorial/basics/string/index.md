---
title: F-string
---

# F-string

f-string 是 YaoXiang 中的**模板字符串**——你可以在字符串里直接嵌入变量和表达式，编译器自动完成类型转换和拼接。

## 基本用法

在字符串前加 `f` 前缀，用 `{表达式}` 插入值：

```yaoxiang
name = "Alice"
age = 25

greeting = f"Hello {name}, you are {age} years old"
println(greeting)  # Hello Alice, you are 25 years old
```

对比传统拼接方式，f-string 的差异一目了然：

```yaoxiang
# ❌ 传统拼接：冗长且容易出错
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())

# ✅ f-string：直观、简洁
message = f"Hello {name}, age: {age}"
```

## 表达式插值

`{}` 里不限于变量——可以放任意表达式：

```yaoxiang
x = 10
y = 20

println(f"Sum: {x + y}")         # Sum: 30
println(f"Product: {x * y}")     # Product: 200
println(f"Is positive? {x > 0}") # Is positive? true
```

## 格式化说明符

在表达式后加 `:` 和格式说明符，控制输出格式：

```yaoxiang
pi = 3.14159265

println(f"Pi: {pi}")       # Pi: 3.14159265
println(f"Pi: {pi:.2f}")   # Pi: 3.14（保留2位小数）
println(f"Pi: {pi:.4f}")   # Pi: 3.1416（保留4位小数）
```

常用格式化说明符：

| 说明符 | 含义 | 示例 | 输出 |
|--------|------|------|------|
| `:.2f` | 浮点，2位小数 | `f"{3.14159:.2f}"` | `3.14` |
| `:d` | 十进制整数 | `f"{42:d}"` | `42` |
| `:x` | 十六进制 | `f"{255:x}"` | `ff` |
| `:e` | 科学计数法 | `f"{1000:e}"` | `1.000000e+03` |
| `:s` | 字符串 | `f"{name:s}"` | `hello` |

## 调用方法

可以在 `{}` 里调用方法：

```yaoxiang
name = "alice"

println(f"Upper: {name.uppercase()}")   # Upper: ALICE
println(f"Length: {name.len()}")        # Length: 5
```

## 转义大括号

如果需要输出字面的 `{` 或 `}`，**双写**即可：

```yaoxiang
println(f"{{literal braces}}")     # {literal braces}
println(f"Set: {{1, 2, 3}}")       # Set: {1, 2, 3}

# 混合：双写输出字面量 {，单写是插值
name = "YaoXiang"
println(f"{{name}} is {name}")     # {name} is YaoXiang
```

## 多行 f-string

f-string 可以跨多行：

```yaoxiang
name = "Alice"
age = 25
city = "Beijing"

info = f"""
Name: {name}
Age: {age}
City: {city}
"""

println(info)
# Name: Alice
# Age: 25
# City: Beijing
```

## f-string 的工作原理

编译器看到 f-string 时，会把它转换为高效的字符串拼接：

```yaoxiang
# 你写的
f"Hello {name}, age: {age}"

# 编译器转换结果
"Hello ".concat(name.to_string()).concat(", age: ").concat(age.to_string())
```

这意味 f-string 不仅写起来更简洁，运行时性能也和手写拼接相当——**零额外开销**。

## 小结

::: v-pre
| 要点 | 语法 |
|------|------|
| 基本插值 | `f"text {var}"` |
| 表达式 | `f"result: {x + y}"` |
| 格式化 | `f"value: {pi:.2f}"` |
| 转义括号 | `f"{{not interpolation}}"` |
| 多行 | `f"""..."""` |
:::
