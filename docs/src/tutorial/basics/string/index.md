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

print(f"Pi: {pi}")       // Pi: 3.14159265
print(f"Pi: {pi}")      // 实测：格式说明符暂未实现，输出 3.14159
print(f"Pi: {pi}")      // 如需四舍五入，请自行处理
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

⚠️ 0.8.2 **没有实现花括号转义**。按 Rust / Python 的习惯，想输出字面的左花括号就把
它写成两个——YaoXiang 不支持这种写法，会把两个花括号之间的内容当成变量名去解析：

<!-- docs-example: skip -->
```yaoxiang
print(f"{{literal braces}}")     // 期望 {literal braces}
print(f"Set: {{1, 2, 3}}")       // 期望 Set: {1, 2, 3}
```

> 上面两行**故意不通过检查**——报 `E1001 Unknown variable: 'literal'`。

今天要输出字面花括号，只能绕开 f-string，用普通字符串拼接：

```yaoxiang
main = () => {
    name = "YaoXiang"

    // 单写是插值
    print(f"{name} is {name}")

    // 需要字面花括号时用普通字符串拼接
    print("Set: " + "{" + "1, 2, 3" + "}")
}
```

## 多行 f-string

⚠️ 0.8.2 **不支持三引号 `f"""..."""`**——词法器不认多行字符串字面量，跨行的 f-string
报 `E0012 unterminated string`。跨行请拼多个 f-string：

```yaoxiang
name = "Alice"
age = 25
city = "Beijing"

info = f"Name: {name}" + "\n" + f"Age: {age}" + "\n" + f"City: {city}"

print(info)
// Name: Alice
// Age: 25
// City: Beijing
```

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
| 格式化   | ⚠️ 暂未实现：`{pi:.2f}` 会原样输出 |
| 转义括号 | ⚠️ 未实现：双写左花括号会被当变量名（E1001） |
| 多行     | ⚠️ 未实现：`f"""..."""` 报 E0012        |
