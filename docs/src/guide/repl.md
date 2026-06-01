---
title: REPL 交互式解释器
description: YaoXiang REPL 使用指南 - 交互式代码执行环境
---

# REPL 交互式解释器

YaoXiang REPL（Read-Eval-Print Loop）是一个交互式代码执行环境，允许您逐行输入和执行 YaoXiang 代码，非常适合学习、测试和调试。

## 快速开始

### 启动 REPL

在终端中运行以下命令启动 REPL：

```bash
yaoxiang repl
```

或者直接运行 `yaoxiang`（不带任何子命令）：

```bash
yaoxiang
```

启动后，您将看到提示符：

```
YaoXiang REPL - Type :help for assistance
Press Ctrl+D or :quit to exit

>>
```

### 基本使用

在提示符 `>>` 后输入 YaoXiang 代码并按回车执行：

```rust
>> 1 + 2
3

>> "Hello, World!"
"Hello, World!"

>> let x = 10
>> x * 2
20
```

### 退出 REPL

有三种方式退出 REPL：

1. **快捷键**：按 `Ctrl+D`
2. **命令**：输入 `:quit` 或 `:q`
3. **中断**：按 `Ctrl+C` 中断当前输入

## 命令系统

REPL 提供了一系列以冒号 `:` 开头的特殊命令。

### 帮助命令

```rust
>> :help
```

显示所有可用命令的帮助信息。

### 退出命令

```rust
>> :quit
```

退出 REPL。也可以使用简写 `:q`。

### 清除命令

```rust
>> :clear
```

清除所有已定义的变量和函数，重置 REPL 状态。也可以使用简写 `:c`。

### 类型查看命令

```rust
>> :type x
```

查看符号 `x` 的类型信息。也可以使用简写 `:t`。

**示例**：

```rust
>> let name = "YaoXiang"
>> :type name
name: String

>> fn add(a: Int, b: Int) -> Int = a + b
>> :type add
add: fn(Int, Int) -> Int
```

### 符号列表命令

```rust
>> :symbols
```

列出当前 REPL 中所有已定义的符号（变量和函数）。也可以使用简写 `:i` 或 `:info`。

**示例**：

```rust
>> let x = 10
>> let y = 20
>> fn greet(name: String) -> String = "Hello, " + name
>> :symbols
x: Int
y: Int
greet: fn(String) -> String
```

### 历史命令

```rust
>> :history
```

显示命令历史记录。也可以使用简写 `:hist`。

### 统计命令

```rust
>> :stats
```

显示执行统计信息，包括评估次数和总执行时间。

**示例**：

```rust
>> :stats
Eval count: 5
Total time: 12.34ms
```

## 代码执行

### 表达式执行

REPL 可以执行任何有效的 YaoXiang 表达式：

```rust
>> 1 + 2
3

>> 10 * 5 + 3
53

>> "Hello" + " " + "World"
"Hello World"

>> true && false
false
```

### 变量定义

使用 `let` 关键字定义变量：

```rust
>> let name = "YaoXiang"
>> let age = 25
>> let pi = 3.14159
```

定义后，变量可以在后续代码中使用：

```rust
>> name
"YaoXiang"

>> age + 5
30
```

### 函数定义

使用 `fn` 关键字定义函数：

```rust
>> fn add(a: Int, b: Int) -> Int = a + b
>> fn greet(name: String) -> String = "Hello, " + name
```

调用函数：

```rust
>> add(3, 4)
7

>> greet("World")
"Hello World"
```

### 多行代码

REPL 支持多行代码输入。当检测到代码不完整时（如未闭合的括号），会自动进入续行模式：

```rust
>> fn factorial(n: Int) -> Int =
..   if n <= 1 then 1
..   else n * factorial(n - 1)
```

续行提示符为 `..`，表示当前处于多行输入模式。

### 结构体定义

```rust
>> struct Point {
..   x: Float,
..   y: Float
.. }
```

### 枚举定义

```rust
>> enum Color {
..   Red,
..   Green,
..   Blue
.. }
```

## 自动补全

REPL 提供智能自动补全功能，帮助您快速输入代码。

### 触发方式

按 `Tab` 键触发自动补全。

### 补全内容

1. **关键字补全**：YaoXiang 语言关键字
   - `let`, `fn`, `if`, `else`, `match`, `for`, `while`, `return` 等

2. **变量补全**：已定义的变量
   - 输入变量名的前几个字符，按 Tab 补全

3. **函数补全**：已定义的函数
   - 输入函数名的前几个字符，按 Tab 补全

4. **内置函数补全**：内置函数
   - `print`, `len`, `range`, `typeof`, `assert` 等

### 补全示例

```rust
>> let my_variable = 42
>> my_<Tab>
my_variable: Int

>> fn calculate_sum(a: Int, b: Int) -> Int = a + b
>> calc<Tab>
calculate_sum: fn(Int, Int) -> Int
```

## 高级功能

### 错误处理

当代码出现错误时，REPL 会显示详细的错误信息：

```rust
>> let x = 10 / 0
Error: Runtime error: DivisionByZero

>> undefined_variable
Error: Unknown symbol: undefined_variable
```

错误不会终止 REPL 会话，您可以继续输入新的代码。

### 历史记录

REPL 自动保存命令历史，支持：

- **上下箭头**：浏览历史命令
- **搜索**：输入部分内容后使用上下箭头搜索
- **历史文件**：历史记录保存在文件中，下次启动时自动加载

### 执行统计

使用 `:stats` 命令查看执行统计：

```rust
>> :stats
Eval count: 15
Total time: 45.67ms
```

这有助于监控代码性能。

## 最佳实践

### 1. 使用有意义的变量名

```rust
// 好
let user_name = "YaoXiang"
let max_retries = 3

// 不好
let x = "YaoXiang"
let n = 3
```

### 2. 定义函数复用代码

```rust
>> fn is_even(n: Int) -> Bool = n % 2 == 0
>> is_even(4)
true
>> is_even(7)
false
```

### 3. 使用 `:clear` 重置状态

当 REPL 状态混乱时，使用 `:clear` 重置：

```rust
>> :clear
Context cleared
```

### 4. 利用自动补全提高效率

输入前几个字符后按 Tab，快速补全变量和函数名。

### 5. 使用多行输入处理复杂代码

```rust
>> fn fibonacci(n: Int) -> Int =
..   if n <= 1 then n
..   else fibonacci(n - 1) + fibonacci(n - 2)
```

## 常见问题

### Q: 如何查看某个函数的定义？

A: 使用 `:type` 命令查看函数签名：

```rust
>> :type my_function
my_function: fn(Int, String) -> Bool
```

### Q: 如何清除所有定义？

A: 使用 `:clear` 命令：

```rust
>> :clear
```

### Q: 为什么我的多行代码没有执行？

A: 检查是否有未闭合的括号、引号或大括号。REPL 会等待完整的代码输入。

### Q: 如何中断长时间运行的代码？

A: 按 `Ctrl+C` 中断当前执行。

### Q: REPL 支持哪些数据类型？

A: REPL 支持所有 YaoXiang 数据类型：
- `Int`：整数
- `Float`：浮点数
- `String`：字符串
- `Bool`：布尔值
- `Unit`：单元类型
- 自定义结构体和枚举

## 示例会话

以下是一个完整的 REPL 会话示例：

```rust
YaoXiang REPL - Type :help for assistance
Press Ctrl+D or :quit to exit

>> let greeting = "Hello"
>> let name = "YaoXiang"
>> greeting + ", " + name + "!"
"Hello, YaoXiang!"

>> fn factorial(n: Int) -> Int =
..   if n <= 1 then 1
..   else n * factorial(n - 1)
..
>> factorial(5)
120

>> :symbols
greeting: String
name: String
factorial: fn(Int) -> Int

>> :stats
Eval count: 4
Total time: 2.34ms

>> :quit
```

## 相关命令

| 命令 | 简写 | 功能 |
|------|------|------|
| `:help` | `:h` | 显示帮助信息 |
| `:quit` | `:q` | 退出 REPL |
| `:clear` | `:c` | 清除所有状态 |
| `:type` | `:t` | 查看符号类型 |
| `:symbols` | `:i` | 列出所有符号 |
| `:history` | `:hist` | 显示命令历史 |
| `:stats` | - | 显示执行统计 |
