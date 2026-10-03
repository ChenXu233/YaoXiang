---
title: 'F-string'
---

# F-string

f-string 是 YaoXiang 中的**模板字符串**——你可以在字符串里直接嵌入变量和表达式，编译器自动完成类型转换和拼接。

## 基本用法

在字符串前加 `f` 前缀，用 `{表达式}` 插入值：

```yaoxiang
name = "Alice"
age = 25

greeting = f"Hello {name}, you are {age} years old"
print(greeting)  // Hello Alice, you are 25 years old
```

对比传统拼接方式，f-string 的差异一目了然：

```yaoxiang
use std.string

name = "Alice"
age = 25

// ❌ 传统拼接：冗长且容易出错
// 注意：String 没有 .concat() 实例方法，也没有 Int.to_string()——两者都报 E1053
message = "Hello, age: " + string.format("{0}", age)

// ✅ f-string：直观、简洁
message2 = f"Hello {name}, age: {age}"
```

## 表达式插值

`{}` 里不限于变量——可以放任意表达式：

```yaoxiang
x = 10
y = 20

print(f"Sum: {x + y}")         // Sum: 30
print(f"Product: {x * y}")     // Product: 200
print(f"Is positive? {x > 0}") // Is positive? true
```

## 格式化说明符

在表达式后加 `:` 和格式说明符，控制输出格式：

```yaoxiang
pi = 3.14159265
name = "hello"

print(f"Pi: {pi}")       // Pi: 3.14159265
print(f"Pi: {pi:.2f}")   // Pi: 3.14（保留 2 位小数）
print(f"{255:x}")        // ff（十六进制）
print(f"{1000:e}")       // 1.000000e+03（科学计数法）
print(f"{name:s}")       // hello（字符串展示）
```

常用格式化说明符：

| 说明符 | 含义          | 示例               | 输出           |
| ------ | ------------- | ------------------ | -------------- |
| `:.2f` | 浮点，2位小数 | `f"{3.14159:.2f}"` | `3.14`         |
| `:d`   | 十进制整数    | `f"{42:d}"`        | `42`           |
| `:x`   | 十六进制      | `f"{255:x}"`       | `ff`           |
| `:e`   | 科学计数法    | `f"{1000:e}"`      | `1.000000e+03` |
| `:s`   | 字符串        | `f"{name:s}"`      | `hello`        |

## 调用方法

可以在 `{}` 里调用方法：

```yaoxiang
use std.string

// 注意：转换走 std.string 的函数形式——`name.uppercase()` / `name.len()` 报 E1053
name = "alice"

print(f"Upper: {string.upper(name)}")  // Upper: ALICE
print(f"Length: {string.len(name)}")   // Length: 5
```

## 转义大括号

想输出字面的 `{` 或 `}`，**双写**即可（与 Python 一致）：

```yaoxiang
print(f"{{literal braces}}")     // {literal braces}
print(f"Set: {{1, 2, 3}}")       // Set: {1, 2, 3}
```

混合使用：双写输出字面量 `{`，单写是插值：

```yaoxiang
name = "YaoXiang"

print(f"{{name}} is {name}")     // {name} is YaoXiang
```

## 多行 f-string

三引号 `f"""..."""` 支持跨行模板，换行是内容的一部分（与普通 `"""` 多行字符串一致）：

```yaoxiang
name = "Alice"
age = 25
city = "Beijing"

info = f"""Name: {name}
Age: {age}
City: {city}"""

print(info)
// Name: Alice
// Age: 25
// City: Beijing
```

插值与转义大括号在多行模板里照常工作，字符串转义（`\n` 等）亦然。

## f-string 的工作原理

编译器看到 f-string 时，会把它转换为字符串拼接。语义上相当于：

```yaoxiang
use std.string

// 你写的
f"Hello {name}, age: {age}"

// 等价的手写形式（注意：没有 .concat() / .to_string() 实例方法，
// 也没有隐式 Int→String 转换）
"Hello " + string.format("{0}", name) + ", age: " + string.format("{0}", age)
```

这意味 f-string 不仅写起来更简洁，运行时性能也和手写拼接相当——**零额外开销**。

## 小结

:::: v-pre

| 要点     | 语法                       |
| -------- | -------------------------- |
| 基本插值 | `f"text {var}"`            |
| 表达式   | `f"result: {x + y}"`       |
| 格式化   | `f"value: {pi:.2f}"`       |
| 转义括号 | `f"{{not interpolation}}"` |
| 多行     | `f"""..."""`               |
