---
title: while 循环
---

# while 循环

`for` 适合"遍历已知集合"的场景，而 `while` 适合另一种情况——**你不知道要循环多少次，只知道什么时候该停下来**。

## 基本语法

语法规范中 `while` 语句的定义：

```
while Expr Block
```

结构很简单：`while` 后面跟一个条件表达式，然后是循环体代码块。只要条件为 `true`，循环体就一直执行。

```yaoxiang
mut count = 1

while count <= 5 {
    print(count)
    count = count + 1
}
// 输出：1 2 3 4 5
```

注意我们用 `mut count` 声明变量——因为 `count` 需要在循环中被修改。如果写成 `count = 1`（不可变），循环体里的 `count = count + 1` 就会报错。

## while 的执行流程

`while` 循环的执行步骤如下：

1. 检查条件表达式
2. 如果条件是 `true`，执行循环体，然后回到步骤 1
3. 如果条件是 `false`，结束循环，继续执行后面的代码

条件在**每次迭代开始前**检查。如果一开始条件就是 `false`，循环体一次都不会执行：

```yaoxiang
mut n = 0
while n > 0 {
    print("这句话永远不会被打印")
    n = n - 1
}
// 条件 n > 0 一开始就是 false，循环体直接跳过
```

## break：提前退出循环

有时你需要在循环中途提前退出——比如找到了要搜索的目标：

```yaoxiang
numbers = [3, 7, 2, 9, 5]
mut found = false
mut index = 0

while index < 5 {
    if numbers[index] == 9 {
        found = true
        break      // 找到了，不需要继续找
    }
    index = index + 1
}

print("找到了吗？" + found.to_string())  // "找到了吗？true"
```

`break` 让程序立刻跳出当前循环，继续执行循环后面的代码。

## continue：跳过当前迭代

`continue` 和 `break` 不同——它不是退出循环，而是跳过当前迭代的剩余部分，直接进入下一次条件检查：

```yaoxiang
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue   // 跳过 3，不打印
    }
    print(n)
}
// 输出：1 2 4 5
```

这段代码中，当 `n` 等于 3 时，`continue` 跳过了 `println(n)`，直接回到 `while n < 5` 检查条件。

## 避免死循环

使用 `while` 时要特别注意——确保循环条件最终会变成 `false`，否则程序会永远卡住：

```yaoxiang
// 危险！死循环——条件永远为 true
// mut x = 1
// while x > 0 {
//     x = x + 1     // x 越来越大，永远不会 <= 0
// }

// 正确——有明确的终止条件
mut x = 1
while x <= 5 {
    print(x)
    x = x + 1        // x 逐渐增大，最终 x > 5 时循环结束
}
```

## 使用 while 读取输入

`while` 一个经典场景是处理不确定长度的输入——你不知道用户会输入多少次，只知道"输入为空时停止"：

```yaoxiang
// 伪代码示例——展示 while 的典型用法
// read_line 在读到空行时返回空字符串
mut line = read_line()
while line != "" {
    process(line)
    line = read_line()
}
```

这种"检查条件 → 处理数据 → 更新条件"的模式是 `while` 的核心用法范式。

## 小结

| 要点 | 说明 |
|------|------|
| 适用场景 | 不知道循环次数，只知道终止条件 |
| 语法 | `while 条件 { ... }` |
| 执行流程 | 先检查条件，再执行循环体 |
| `break` | 立即退出循环 |
| `continue` | 跳过当前迭代，回到条件检查 |
| 注意事项 | 确保条件最终会变成 `false`，避免死循环 |

下一章你将学习 `match` 基础——YaoXiang 最强大的分支控制工具。
