---
title: 第十章：完整应用
---

# 第十章：完整应用

> 本章目标：综合运用前9章知识，完成一个小型项目


## 10.1 项目目标

我们要创建一个**简单的图形计算器**，可以：

- 定义点（Point）和圆（Circle）
- 计算距离
- 判断点在圆内/圆外
- 批量处理多个图形


## 10.2 第一步：定义基本类型

```yaoxiang
# ===== 文件：geometry.yx =====

# 点类型（记录类型）
Point: Type = {
    x: Float,
    y: Float,

    # 方法：移动
    move: (self: Point, dx: Float, dy: Float) -> Point = {
        return Point(self.x + dx, self.y + dy)
    },

    # 方法：距离
    distance: (self: Point, other: Point) -> Float = {
        dx = self.x - other.x
        dy = self.y - other.y
        return (dx * dx + dy * dy).sqrt()
    }
}
```


## 10.3 第二步：定义圆类型

```yaoxiang
# ===== 文件：geometry.yx（续）=====

# 圆形类型
Circle: Type = {
    center: Point,
    radius: Float,

    # 方法：面积
    area: (self: Circle) -> Float = {
        return 3.14159 * self.radius * self.radius
    },

    # 方法：判断点是否在圆内
    contains: (self: Circle, p: Point) -> Bool = {
        distance = self.center.distance(p)
        return distance <= self.radius
    },

    # 方法：调整大小
    scale: (self: Circle, factor: Float) -> Circle = {
        return Circle(self.center, self.radius * factor)
    }
}
```


## 10.4 第三步：创建图形管理器

```yaoxiang
# ===== 文件：geometry.yx（续）=====

# 可绘制的接口
Drawable: Type = {
    draw: (self: Self) -> Void,
    bounding_box: (self: Self) -> Rect
}

# 矩形类型（实现 Drawable）
Rect: Type = {
    x: Float,
    y: Float,
    width: Float,
    height: Float,
    Drawable
}

Rect.draw: (self: Rect) -> Void = {
    print("绘制矩形: ${self.x}, ${self.y}, ${self.width}, ${self.height}")
}

Rect.bounding_box: (self: Rect) -> Rect = {
    return self
}
```


## 10.5 第四步：泛型图形集合

```yaoxiang
# ===== 文件：geometry.yx（续）=====

# 图形集合（使用泛型）
ShapeContainer: Type[T: Drawable] = {
    shapes: List[T],

    add: [T: Drawable](self: ShapeContainer[T], shape: T) -> Void = {
        self.shapes.push(shape)
    },

    draw_all: [T: Drawable](self: ShapeContainer[T]) -> Void = {
        for shape in self.shapes {
            shape.draw()
        }
    },

    count: [T: Drawable](self: ShapeContainer[T]) -> Int = {
        return self.shapes.length
    }
}
```

## 10.6 第五步：主程序

```yaoxiang
# ===== 文件：main.yx =====

use geometry.{Point, Circle, Rect, Drawable, ShapeContainer}

main: () -> Void = {
    # 创建点
    origin: Point = Point(0.0, 0.0)
    p1: Point = Point(3.0, 4.0)

    # 计算距离
    dist = origin.distance(p1)    # 5.0
    print("距离: ${dist}")

    # 创建圆
    circle: Circle = Circle(origin, 5.0)
    print("圆面积: ${circle.area()}")

    # 判断点在圆内/外
    inside = Point(1.0, 1.0)
    outside = Point(10.0, 10.0)

    print("点在圆内: ${circle.contains(inside)}")    # true
    print("点在圆外: ${circle.contains(outside)}")   # false

    # 所有权示例
    mut c1: Circle = Circle(Point(0.0, 0.0), 10.0)

    # 链式调用（所有权回流）
    c1 = c1.scale(2.0).move(5.0, 5.0)

    # 共享示例
    shared_circle = ref c1
    spawn(() => {
        print("在新任务中访问圆: ${shared_circle.area()}")
    })

    # 泛型容器示例
    mut container: ShapeContainer[Circle] = ShapeContainer(List())
    container.add(circle)
    container.add(Circle(Point(1.0, 1.0), 3.0))

    print("容器中的图形数量: ${container.count()}")
    container.draw_all()

    print("程序运行完成！")
}
```

---

## 10.7 完整代码结构

```
项目结构
┌─────────────────────────────────────────┐
│  geometry.yx                            │
│  ├── Point 类型定义                      │
│  ├── Circle 类型定义                     │
│  ├── Rect 类型定义                       │
│  ├── Drawable 接口                       │
│  └── ShapeContainer 泛型容器              │
├─────────────────────────────────────────┤
│  main.yx                               │
│  ├── use geometry.{...}                 │
│  └── main 函数                          │
└─────────────────────────────────────────┘
```


## 10.8 运行程序

```bash
# 保存文件
保存 geometry.yx 和 main.yx

# 运行
yaoxiang main.yx
```

**预期输出**：

```
距离: 5.0
圆面积: 78.53975
点在圆内: true
点在圆外: false
在新任务中访问圆: 314.159
容器中的图形数量: 2
绘制矩形: 0.0, 0.0, 10.0, 10.0
绘制矩形: 1.0, 1.0, 3.0, 3.0
程序运行完成！
```


## 10.9 本章知识回顾

| 章节 | 知识点 | 应用 |
|------|---------|------|
| 第1章 | 程序入口 main | `main: () -> Void = { ... }` |
| 第2章 | 基本类型 | `Float`、`Int`、`Bool` |
| 第3章 | 变量和作用域 | `mut`、`let` |
| 第4章 | Type 类型 | `Point: Type = { ... }` |
| 第5章 | 统一语法 | `name: type = value` |
| 第6章 | 自定义类型/方法 | `Point.move: ...` |
| 第7章 | 泛型 | `ShapeContainer[T]` |
| 第8章 | 所有权 | `mut c1 = ...` |
| 第9章 | Move/ref/clone | `shared = ref c1`、`c1.scale(...)` |


## 10.10 恭喜你！

完成这10章教程，你已经：

✅ 理解了程序的基本概念
✅ 掌握了类型系统
✅ 理解了元类型和 Type 的秘密
✅ 学会了统一语法 `name: type = value`
✅ 能够自定义类型和接口
✅ 理解了泛型编程
✅ 掌握了所有权模型
✅ 能够综合运用这些知识

**下一步**：可以继续学习高级主题，如并作模型、错误处理等。


## 10.11 易经引言

> 「大哉乾元，万物资始，乃统天。」
> —— 《周易·乾卦》
>
> 从第一行 Hello World，到完整的图形计算器，
> 这便是"万物资始"的过程。
>
> 你已经：
> - 学会了"道"（编程思想）
> - 掌握了"器"（语言语法）
> - 能够"制器"（编写程序）
>
> 愿你在编程之道上，继续探索前行。
>
> **乾卦曰：天行健，君子以自强不息。**
