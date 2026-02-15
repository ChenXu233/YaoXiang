---
title: 第五章：统一的艺术
---

# 第五章：统一的艺术

> 本章目标：理解 YaoXiang 的核心理念——`name: type = value` 统一语法模型


## 5.1 不同的"东西"

在其他编程语言中，有许多不同的"东西"：

| 语言中的概念 | 例子 |
|-------------|------|
| 变量 | `let x = 42` |
| 函数 | `fn add(a, b) { return a + b }` |
| 常量 | `const PI = 3.14` |
| 类型定义 | `struct Point { x: f64, y: f64 }` |
| 接口 | `trait Drawable { fn draw(&self); }` |
| 方法 | `impl Point { fn draw(&self) { ... } }` |

**问题**：每种"东西"都有不同的语法，需要学习很多规则。

## 5.2 YaoXiang 的答案：统一

YaoXiang 说：**所有东西都可以用同一个公式！**

```
name: type = value
```

| 概念 | YaoXiang 写法 | 公式对应 |
|------|----------------|----------|
| 变量 | `x: Int = 42` | `name: type = value` |
| 函数 | `add: (a: Int, b: Int) -> Int = a + b` | `name: type = value` |
| 常量 | `PI: Float = 3.14` | `name: type = value` |
| 类型 | `Point: Type = { x: Float, y: Float }` | `name: type = value` |
| 接口 | `Drawable: Type = { draw: (Surface) -> Void }` | `name: type = value` |

**这就是 YaoXiang 的魔法！**

## 5.3 变量：第一个例子

```yaoxiang
# 变量：名字 + 类型 = 值
age: Int = 25
name: String = "小明"
is_student: Bool = true
```

## 5.4 函数：第二个例子

```yaoxiang
# 函数：名字 + (参数类型) -> 返回类型 = 实现
add: (a: Int, b: Int) -> Int = a + b

greet: (name: String) -> String = "你好, ${name}!"

# 多行函数
max: (a: Int, b: Int) -> Int = {
    if a > b {
        return a
    } else {
        return b
    }
}
```

**注意**：
- `(a: Int, b: Int)` 是**参数列表**
- `-> Int` 是**返回类型**
- `= a + b` 是**函数体**（返回值）


## 5.5 类型定义：第三个例子

```yaoxiang
# 类型：名字 + Type = 结构
Point: Type = {
    x: Float,
    y: Float
}

# 使用类型
p: Point = Point(1.0, 2.0)
```


## 5.6 接口：第四个例子

**接口**，就是"只有方法的类型"：

```yaoxiang
# 接口：名字 + Type = 方法集合
Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

# 使用接口
circle: Circle = Circle(0.0, 0.0, 5.0)
drawable: Drawable = circle  # ✅ Circle 实现了 Drawable
```


## 5.7 方法：第五个例子

**方法**，就是"属于某个类型的函数"：

```yaoxiang
# 普通函数
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# 方法写法（属于 Point）
Point.distance: (self: Point, other: Point) -> Float = {
    dx = self.x - other.x
    dy = self.y - other.y
    return (dx * dx + dy * dy).sqrt()
}
```

**调用方式**：

```yaoxiang
p1: Point = Point(0.0, 0.0)
p2: Point = Point(3.0, 4.0)

# 两种调用方式都可以
d1 = distance(p1, p2)        # 函数式调用
d2 = p1.distance(p2)          # 方法式调用（语法糖）
```


## 5.8 统一语法的力量

| 特性 | 好处 |
|------|------|
| **只学一个规则** | 不需要记忆多种语法 |
| **代码统一** | 所有代码看起来风格一致 |
| **易于扩展** | 新特性自然融入现有模型 |
| **理论优雅** | 数学上对称美观 |


## 5.9 Lambda（匿名函数）

YaoXiang 中，**具名函数本质就是 Lambda**：

```yaoxiang
# 具名函数（推荐）
add: (a: Int, b: Int) -> Int = a + b

# Lambda 形式（完全等价）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# 简短形式
double: (x: Int) -> Int = x * 2
double = (x: Int) => x * 2
```

**理解**：具名函数就是"取了名字的 Lambda"。


## 5.10 本章小结

| 概念 | 理解 |
|------|------|
| 统一语法 | `name: type = value` 覆盖所有情况 |
| 变量 | `x: Int = 42` |
| 函数 | `add: (a: Int, b: Int) -> Int = a + b` |
| 类型 | `Point: Type = { x: Float, y: Float }` |
| 接口 | `Drawable: Type = { draw: (Surface) -> Void }` |
| 方法 | `Point.draw: (self: Point) -> Void = ...` |

**核心理念**：记住一个公式，写遍所有代码！


## 5.11 易经引言

> 「天地不仁，以万物为刍狗。」
> —— 《道德经》
>
> 在编程语言的世界里，规则不因对象而异。
> 无论是变量、函数、还是类型，
> 皆遵循同一法则：`name: type = value`。
>
> 这便是"道生一，一生二，二生三，三生万物"的编程诠释——
> 一个公式，衍生万千可能。
