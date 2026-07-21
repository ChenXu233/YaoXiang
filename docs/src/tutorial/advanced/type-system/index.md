---
title: 类型系统
---

# 类型系统

在基础教程中你学会了使用 `Int`、`String`、`Bool` 等内置类型。本章深入 YaoXiang 的类型系统，学会**定义自己的类型**。

## 统一语法模型

YaoXiang 的类型系统建立在 RFC-010 定义的统一语法之上：**一切皆 `name: type = value`**。

| 概念 | 写法 |
|------|------|
| 变量 | `x: Int = 42` |
| 函数 | `add: (a: Int, b: Int) -> Int = a + b` |
| 记录类型 | `Point: Type = { x: Float, y: Float }` |
| 接口 | `Drawable: Type = { draw: (Surface) -> Void }` |
| 泛型类型 | `List: (T: Type) -> Type = { ... }` |

注意：**类型定义本身也是 `name: Type = value`**。

## 记录类型

记录类型（在其他语言中叫"结构体"）是 YaoXiang 中最基本的数据组织方式：

```yaoxiang
// 定义记录类型
Point: Type = { x: Float, y: Float }

// 创建实例
origin = Point(x: 0.0, y: 0.0)
p = Point(x: 3.0, y: 4.0)

// 访问字段
print(p.x)  // 3.0
print(p.y)  // 4.0
```

### 字段默认值

字段可以指定默认值，构造时可选提供：

```yaoxiang
User: Type = {
    name: String,
    age: Int = 0,
    active: Bool = true,
}

alice = User(name: "Alice", age: 25)        // active 取默认值 true
bob = User(name: "Bob")                      // age=0, active=true
anonymous = User(name: "guest", active: false)  // age=0
```

### 方法定义

使用 `Type.method` 语法为类型定义方法：

```yaoxiang
Point: Type = { x: Float, y: Float }

// 定义方法：Point.method 语法
Point.length: (self: Point) -> Float = {
    return (self.x * self.x + self.y * self.y).sqrt()
}

p = Point(x: 3.0, y: 4.0)

// 两种调用方式等价
print(Point.length(p))  // 5.0 — 函数式调用
print(p.length())       // 5.0 — .调用语法
```

### pub 自动绑定

在同一文件中，`pub` 声明的函数会自动绑定到同文件定义的类型：

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub 函数自动绑定到 Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

p1 = Point(x: 0.0, y: 0.0)
p2 = Point(x: 3.0, y: 4.0)

// 自动绑定的方法用 . 调用
print(p1.distance(p2))  // 5.0
```

## 枚举类型

枚举定义一组互斥的变体。无数据的变体用小写，有数据的变体用函数式语法：

```yaoxiang
// 简单枚举
Color: Type = { red | green | blue }

// 带数据的枚举
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// 嵌套枚举
Shape: Type = { circle(Float) | rect(Float, Float) | point }
```

枚举的核心理念：**每个变体本身也是一个类型**。

```yaoxiang
area: (s: Shape) -> Float = match s {
    circle(r) => 3.14159 * r * r,
    rect(w, h) => w * h,
    point => 0,
}

print(area(circle(5.0)))    // 78.53975
print(area(rect(3.0, 4.0))) // 12.0
```

## 接口

接口是**字段全为函数类型的记录类型**。实现接口就是让记录包含该接口名：

```yaoxiang
// 定义接口
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

// 实现接口：在记录类型中包含接口名
Circle: Type = {
    x: Float,
    y: Float,
    radius: Float,
    Drawable,       // 实现 Drawable 接口
}

// 提供接口要求的方法
Circle.draw: (self: Circle, surface: Surface) -> Void = {
    surface.draw_circle(self.x, self.y, self.radius)
}

Circle.bounding_box: (self: Circle) -> Rect = {
    return Rect(
        x: self.x - self.radius,
        y: self.y - self.radius,
        width: self.radius * 2.0,
        height: self.radius * 2.0,
    )
}
```

接口实现了多态——任何实现了 `Drawable` 的类型都可以传给接受 `Drawable` 的函数。

## 泛型类型

泛型让你编写**不限定具体类型**的类型定义：

```yaoxiang
// 泛型 Pair
Pair: (T: Type, U: Type) -> Type = { first: T, second: U }

// 使用
string_pair = Pair(Int, String)(first: 1, second: "hello")
float_pair = Pair(Float, Float)(first: 3.14, second: 2.71)
```

泛型函数：

```yaoxiang
// 泛型 map：对列表的每个元素应用函数
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = {
    mut result: List(R) = []
    for item in list {
        result.append(f(item))
    }
    return result
}

numbers = [1, 2, 3, 4]
doubled = map(Int, Int)(numbers, (x) => x * 2)
print(doubled)  // [2, 4, 6, 8]
```

## 小结

| 概念 | 语法 | 用途 |
|------|------|------|
| 记录类型 | `Point: Type = { x: Float, y: Float }` | 组织相关数据 |
| 枚举 | `Color: Type = { red \| green \| blue }` | 多选一 |
| 接口 | `Drawable: Type = { draw: ... }` | 多态抽象 |
| 泛型 | `List: (T: Type) -> Type = { ... }` | 类型参数化 |
| 方法 | `Type.method: (self: Type, ...) -> ...` | 行为附加 |
