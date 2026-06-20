---
title: if-elif-else
---

# if-elif-else

`if-elif-else` 是编程中最基本的决策工具。它的逻辑非常直观——**如果条件成立，就执行某段代码；否则，检查下一个条件；都不成立，走默认路径**。

## 基本语法

语法规范中 `if` 表达式和 `if` 语句的定义完全一致：

```
if Expr Block ('elif' Expr Block)* ('else' Block)?
```

用日常语言翻译：`if` 开头，后面跟一个条件表达式和一个代码块，之后可以接零到多个 `elif 条件 代码块`，最后可以有一个可选的 `else 代码块`。

最简单的形式——只有 `if`：

```yaoxiang
if temperature > 30 {
    println("天热了，开空调吧")
}
```

加上 `else`：

```yaoxiang
if is_raining {
    println("带伞")
} else {
    println("不用带伞")
}
```

多个条件用 `elif`：

```yaoxiang
score = 85

if score >= 90 {
    println("优秀")
} elif score >= 80 {
    println("良好")
} elif score >= 60 {
    println("及格")
} else {
    println("需要努力")
}
```

注意 YaoXiang 的关键字是 `elif`，不是 `else if`。这是语言刻意保持关键字精简的一个体现。

## if 是表达式

这是 YaoXiang 控制流最重要的特性之一：**`if` 可以作为表达式使用，计算出一个值**。

```yaoxiang
# if 表达式：各分支的值会赋给 result
result = if x > 0 {
    "正数"
} elif x < 0 {
    "负数"
} else {
    "零"
}
# result 现在是 "正数"、"负数" 或 "零" 中的一个
```

当 `if` 作为表达式时，所有分支的返回值类型必须一致：

```yaoxiang
score = 88

# 所有分支都返回 String，类型一致，没问题
grade = if score >= 90 {
    "A"
} elif score >= 80 {
    "B"
} elif score >= 60 {
    "C"
} else {
    "D"
}
println(grade)  # "B"
```

在每个分支的代码块中，**最后一个表达式的值就是该分支的返回值**。你也可以用 `return` 显式返回，但在分支中通常直接写表达式就够了。

```yaoxiang
# 直接写表达式——推荐
category = if age < 18 { "未成年" } else { "成年" }

# 也可以显式 return——效果相同
category = if age < 18 {
    return "未成年"
} else {
    return "成年"
}
```

如果你只用 `if` 做条件判断而不需要值，它就是一个普通的语句——和表达式形式完全兼容。

## 嵌套 if

你可以在 `if` 内部再写 `if`，处理多层条件判断：

```yaoxiang
age = 25
has_ticket = true

if age >= 18 {
    if has_ticket {
        println("欢迎入场")
    } else {
        println("请先购票")
    }
} else {
    println("未成年人需家长陪同")
}
```

当表达式嵌套时，YaoXiang 没有 C 语言那样的"悬空 else"歧义——每个 `else` 始终归属于最近的尚未配对的 `if`。

## 使用布尔运算符组合条件

在条件中可以使用 `and`、`or`、`not` 组合多个判断：

```yaoxiang
username = "admin"
password = "123456"

# and：两个条件都成立
if username == "admin" and password == "123456" {
    println("登录成功")
}

# or：任一条件成立
if role == "admin" or role == "moderator" {
    println("有管理权限")
}

# not：取反
if not is_banned {
    println("允许发言")
}

# 组合使用
if (age >= 18 and age <= 60) or is_vip {
    println("可以参加活动")
}
```

运算符优先级上，`not` 高于 `and`，`and` 高于 `or`。不放心时加括号，让意图更清晰。

## 小结

| 要点 | 说明 |
|------|------|
| 基本结构 | `if 条件 { ... } elif 条件 { ... } else { ... }` |
| elif | YaoXiang 用 `elif`，不是 `else if` |
| 表达式 | `if` 可以返回值，所有分支类型必须一致 |
| 分支返回值 | 分支块中最后一个表达式的值即为返回值 |
| 嵌套 | `if` 内可以再写 `if`，没有悬空 else 歧义 |
| 布尔运算 | `and`、`or`、`not` 组合条件 |

下一章你将学习 `for` 循环——遍历集合和范围的标准方式。
