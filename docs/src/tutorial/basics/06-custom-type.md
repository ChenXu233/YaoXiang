---
title: 第六章：自定义类型
---

# 第六章：自定义类型

> 本章目标：学会创建自己的类型，理解记录类型、接口和方法的用法


## 6.1 为什么要自定义类型？

基本类型（Int、Float、String）就像乐高中的基本块。但我们需要更复杂的"形状"来描述现实世界：

| 现实事物 | 对应的自定义类型 |
|----------|------------------|
| 屏幕上的一个点 | `Point` |
| 一本书 | `Book` |
| 一个图形 | `Shape` |
| 一个用户 | `User` |

## 6.2 记录类型（Record Type）

**记录类型**，就是把多个数据打包在一起：

```yaoxiang
# 定义一个 Point 类型（记录类型）
Point: Type = {
    x: Float,    # x 坐标
    y: Float     # y 坐标
}

# 创建 Point 的值
p: Point = Point(1.0, 2.0)

# 访问字段
print(p.x)      # 1.0
print(p.y)      # 2.0
```

**结构图**：

```
Point 类型
┌─────────────────────┐
│        Point         │
├─────────────────────┤
│  ┌─────┬─────┐     │
│  │  x  │  y  │     │
│  │Float│Float│     │
│  └─────┴─────┘     │
└─────────────────────┘
```


## 6.3 更复杂的记录类型

```yaoxiang
# 一个人
Person: Type = {
    name: String,      # 姓名
    age: Int,          # 年龄
    is_student: Bool   # 是否是学生
}

# 创建和使用
person: Person = Person("小明", 18, true)
print(person.name)        # 小明
print(person.age)         # 18

# 修改可变字段
mut p: Person = Person("小红", 20, false)
p.age = 21               # ✅ 可以修改
```

## 6.4 接口（Interface）

**接口**，就是"只有方法的类型"——它定义**能做什么**，但不定义**怎么做**。

### 6.4.1 接口定义

```yaoxiang
# 定义一个可绘制的接口
Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}
```

### 6.4.2 接口内置实现

接口可以**内置实现**，直接把方法写在里面：

```yaoxiang
# 接口内置实现（完整定义）
Drawable: Type = {
    # 定义方法 + 实现
    draw: (self: Self, surface: Surface) -> Void = {
        print("绘制图形")
    },
    bounding_box: (self: Self) -> Rect = {
        return Rect(0, 0, 100, 100)
    }
}

# 使用
surface: Surface = Surface()
d: Drawable = Drawable()
d.draw(surface)
```

### 6.4.3 实现接口

```yaoxiang
# Circle 实现了 Drawable 接口
Circle: Type = {
    x: Float,
    y: Float,
    radius: Float,
    Drawable    # 实现 Drawable 接口
}

# Rectangle 也实现了 Drawable 接口
Rectangle: Type = {
    x: Float,
    y: Float,
    width: Float,
    height: Float,
    Drawable    # 实现 Drawable 接口
}
```

**接口的用途**：

```yaoxiang
# 无论是什么图形，只要实现了 Drawable，就可以统一处理
draw_all: (drawables: List[Drawable]) -> Void = {
    for d in drawables {
        d.draw(screen)
    }
}

# List[Drawable] 可以包含 Circle、Rectangle 任何实现
```

## 6.5 方法（Method）

### 6.5.1 定义方法

```yaoxiang
# 给 Point 类型添加方法
Point.move: (self: Point, dx: Float, dy: Float) -> Point = {
    return Point(self.x + dx, self.y + dy)
}

Point.distance: (self: Point, other: Point) -> Float = {
    dx = self.x - other.x
    dy = self.y - other.y
    return (dx * dx + dy * dy).sqrt()
}
```

### 6.5.2 调用方法

```yaoxiang
p1: Point = Point(0.0, 0.0)
p2: Point = Point(3.0, 4.0)

# 方法调用语法（语法糖）
moved = p1.move(1.0, 2.0)        # Point(1.0, 2.0)
dist = p1.distance(p2)            # 5.0

# 真正的函数调用（真相）
dist = Point.distance(p1, p2)    # 等价！
```


### 6.5.3 self 参数

在方法中，`self` 只是**第一个参数的名称**，不代表特殊含义：

```yaoxiang
# self 的使用（self 只是参数名）
Point.translate: (self: Point, dx: Float, dy: Float) -> Point = {
    # self 就是第一个参数的值
    return Point(self.x + dx, self.y + dy)
}

# 使用
p: Point = Point(10.0, 20.0)
p2 = p.translate(5.0, 5.0)   # self = p
# p2 = Point(15.0, 25.0)

# self 可以换成任何名字
Point.translate: (this: Point, dx: Float, dy: Float) -> Point = {
    return Point(this.x + dx, this.y + dy)
}
```


## 6.6 方法的本质：函数式 vs OOP

**重要**：YaoXiang 是**函数式语言**，不是 OOP 语言！

### 6.6.1 我们不是 OOP

| 特性 | OOP 语言（如 Java、C++） | YaoXiang |
|------|---------------------------|-----------|
| 思维方式 | 对象是核心，方法属于对象 | 函数是核心，方法是语法糖 |
| 继承 | 有继承关系 | 没有继承，只有接口实现 |
| self | self 是隐含的，必须有 | self 不是必须，只是一个参数名 |
| 方法调用 | 语法不同 | 实际上还是函数调用 |

**OOP 的方法**：
```java
// Java：方法属于对象
point.move(1.0, 2.0)
```

**YaoXiang 的"方法"**（本质是函数）：
```yaoxiang
# 仍然是函数调用，只是语法像方法
point.move(1.0, 2.0)
# 等价于
Point.move(point, 1.0, 2.0)
```

### 6.6.2 为什么看起来像 OOP？

YaoXiang 通过**统一语法** + **柯里化绑定**实现了类似 OOP 的语法糖：

```yaoxiang
# 1. 统一语法：所有东西都是 name: type = value
# 2. 柯里化：函数可以"绑定"第一个参数

# 原始函数（真正的东西）
calculate_distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# 绑定：把第一个参数绑定到 Point 类型
Point.distance = calculate_distance[0]

# 调用：看起来像 OOP
p1: Point = Point(0.0, 0.0)
p2: Point = Point(3.0, 4.0)
d = p1.distance(p2)        # 语法糖
# 等价于
d = calculate_distance(p1, p2)  # 真相！
```

### 6.6.3 self 不是必须的

在 YaoXiang 中，`self` 只是一个**普通的参数名**，不是关键字！

```yaoxiang
# self 可以换成任何名字
Point.move: (self: Point, dx: Float, dy: Float) -> Point = {
    return Point(self.x + dx, self.y + dy)
}

# 完全等价
Point.move: (p: Point, dx: Float, dy: Float) -> Point = {
    return Point(p.x + dx, p.y + dy)
}

# 使用
p: Point = Point(1.0, 2.0)
p2 = p.move(3.0, 4.0)   # self = p
```

### 6.6.4 自动绑定规则

编译器自动绑定的规则：

```
绑定点 = 第一个参数的类型 = 类型名

例如：
- Point.move: (p: Point, ...) → p 是第一个参数，类型是 Point
- 所以 p 被绑定到 Point
- 调用 p.move(other) 等价于 Point.move(p, other)
```

```yaoxiang
# 理解自动绑定
# 1. 定义函数，第一个参数是 Point
add_points: (p1: Point, p2: Point) -> Point = {
    return Point(p1.x + p2.x, p1.y + p2.y)
}

# 2. 编译器自动绑定为 Point.add
Point.add = add_points[0]

# 3. 使用
p1: Point = Point(1.0, 2.0)
p2: Point = Point(3.0, 4.0)
p3 = p1.add(p2)          # ✅ 语法糖
# 等价于
p3 = add_points(p1, p2)  # ✅ 真相
```

## 6.7 手动绑定与自动绑定

### 6.7.1 手动绑定

```yaoxiang
# 原函数
calculate_distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# 手动绑定：Point.distance = calculate_distance[0]
# [0] 表示绑定第一个参数
Point.distance = calculate_distance[0]

# 使用
p1: Point = Point(0.0, 0.0)
p2: Point = Point(3.0, 4.0)
d = p1.distance(p2)        # ✅ 语法糖
```

### 6.7.2 自动绑定

当函数定义在**模块文件**中，且第一个参数是模块类型时，编译器**自动绑定**：

```yaoxiang
# ===== 文件：Point.yx =====
# 这个文件定义 Point 类型
type Point = { x: Float, y: Float }

# 定义函数，第一个参数是 Point
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# ===== main.yx =====
use Point

p1: Point = Point(0.0, 0.0)
p2: Point = Point(3.0, 4.0)

# 编译器自动绑定为 Point.distance
d = p1.distance(p2)        # ✅ 自动绑定！
```


## 6.8 综合示例

```yaoxiang
# 定义 Shape 接口
Shape: Type = {
    area: () -> Float,
    perimeter: () -> Float
}

# Circle 实现 Shape
Circle: Type = {
    radius: Float,
    Shape    # 实现 Shape 接口
}

Circle.area: (self: Circle) -> Float = {
    return 3.14159 * self.radius * self.radius
}

Circle.perimeter: (self: Circle) -> Float = {
    return 2 * 3.14159 * self.radius
}

# Rectangle 实现 Shape
Rectangle: Type = {
    width: Float,
    height: Float,
    Shape    # 实现 Shape 接口
}

Rectangle.area: (self: Rectangle) -> Float = {
    return self.width * self.height
}

Rectangle.perimeter: (self: Rectangle) -> Float = {
    return 2 * (self.width + self.height)
}

# 使用
circle: Circle = Circle(5.0)
rect: Rectangle = Rectangle(4.0, 6.0)

print(circle.area())       # 78.53975
print(rect.area())         # 24.0

# 计算总和
total: Float = circle.area() + rect.area()  # 102.53975
```


## 6.9 本章小结

| 概念 | 说明 | 例子 |
|------|------|------|
| 记录类型 | 打包多个数据 | `Point: Type = { x: Float, y: Float }` |
| 接口 | 定义方法集合 | `Drawable: Type = { draw: (Surface) -> Void }` |
| 接口内置实现 | 接口内直接定义方法 | `Drawable: Type = { draw: (Self) -> Void = {...} }` |
| 方法绑定 | 函数绑定到类型 | `Point.move = func[0]` |
| 语法糖 | `p.move()` 等价于 `Point.move(p)` | `p.distance(other)` |
| 函数式本质 | 方法只是语法糖，本质是函数 | `Point.distance(p1, p2)` |
| self | 只是参数名，不是关键字 | 可以换成任何名字 |

**关键理解**：
- ✅ YaoXiang 是函数式语言
- ✅ 方法是语法糖，本质是函数
- ✅ self 不是必须的，只是命名习惯
- ✅ 统一语法让一切皆可表达


## 6.10 易经引言

> 「形而上者谓之道，形而下者谓之器。」
> —— 《周易·系辞上传》
>
> 器，是具体的形态——记录类型如同容器，装载数据。
> 道，是抽象的规则——接口如同契约，定义行为。
>
> 自定义类型，便是程序员"制器"的过程：
> - 记录类型是"器"，装载数据
> - 接口是"道"，定义规范
> - 方法是"用"，让类型发挥作用
>
> **以道御器，方能成事。**
