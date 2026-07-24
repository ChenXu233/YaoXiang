---
title: 语法速查
---

# 语法速查

5 分钟看懂 YaoXiang 核心语法。深入学习请访问 [教程](/tutorial/)。

## 变量

```yaoxiang
x = 42                    // 不可变（默认）
mut y = 0                 // 可变

name: String = "hello"    // 显式类型
count: Int = 100          // 类型注解

pub version = "1.0"       // 公开导出
```

## 函数

一切皆 `name: type = value`。函数也是值。

```yaoxiang
// 表达式形式（直接返回值）
add: (a: Int, b: Int) -> Int = a + b

// 代码块形式（显式 return）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Lambda（签名完整时可省略参数名）
double = (x) => x * 2
add = (a, b) => a + b
inc = x => x + 1            // 单参数可省略括号

// 代码块内需要 return
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

// Void 函数不需要 return
greet: (name: String) -> Void = {
    io.println("Hello, " + name)
}
```

## 类型

没有 `type`、`struct`、`trait`、`impl` 关键字。一个统一声明搞定一切。

```yaoxiang
// 记录类型
Point: Type = { x: Float, y: Float }
p = Point(1.0, 2.0)            // 位置参数
p = Point(x=1.0, y=2.0)        // 命名参数

// 带默认值的字段
Point: Type = { x: Float = 0, y: Float = 0 }
Point()                        // OK: x=0, y=0
Point(x=1.0)                   // OK: x=1.0, y=0

// 变体类型（枚举）
Color: Type = { red | green | blue }

Option: (T: Type) -> Type = { some(T) | none }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// 接口（字段全为函数类型的记录类型）
Drawable: Type = { draw: (Surface) -> Void }

// 接口组合
DrawableSerializable: Type = Drawable & Serializable

// 类型内声明接口实现
Circle: Type = {
    radius: Float,
    Drawable,              // 实现 Drawable 接口
    Serializable,          // 实现 Serializable 接口
}

// 泛型类型
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
}

// 泛型约束
clone: (T: Clone)(value: T) -> T = value.clone()
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T)
```

## 方法

```yaoxiang
// 命名空间函数（Type.method 只是归属标记，不是绑定）
Point.distance: (a: &Point, b: &Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

// 显式绑定后才有 . 调用语法
Point.distance = distance[0]
// 此后 p1.distance(p2) → distance(p1, p2)

// 快速定义 + 绑定
Point.draw: (self: &Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}
```

## 控制流

```yaoxiang
// if 是表达式
grade = if score >= 90 { "A" } elif score >= 60 { "B" } else { "C" }

// match
result = match value {
    ok(v) => "success: {v}",
    err(e) => "error: {e}",
    _ => "unknown",
}

// 循环
for i in 0..5 { io.println(i) }
for item in items { io.println(item) }

mut n = 0
while n < 5 { io.println(n); n = n + 1 }
```

## 数据结构

```yaoxiang
// 列表
nums = [1, 2, 3, 4, 5]
first = nums[0]           // 1

// 字典
scores = {"Alice": 90, "Bob": 85}
a = scores["Alice"]       // 90

// 列表推导式
evens = [x for x in nums if x % 2 == 0]
doubled = [x * 2 for x in nums]
```

## 模式匹配

```yaoxiang
match shape {
    circle(r) => pi * r * r,
    rect(w, h) => w * h,
    point => 0,
}

// 结构体/元组模式
match p {
    { x: 0, y: 0 } => "origin",
    { x, y } => "({x}, {y})",
}
match t {
    (0, 0) => "origin",
    (x, y) => "({x}, {y})",
}

// 解构赋值
a, b = (1, 2)              // a=1, b=2

// 卫表达式
match age {
    n if n >= 18 => true,
    _ => false,
}
```

## 模块和导入

```yaoxiang
use std.io
use std.math.{sqrt, sin, cos}
use std.{io, list}

io.println("hello")
result = sqrt(16)         // 4.0

// 别名
use std.math as math
use std.{io as print}

// 公开导出
pub add: (a: Int, b: Int) -> Int = a + b
pub Point: Type = { x: Float, y: Float }
```

## 所有权

```yaoxiang
// Move：默认所有权转移
p1 = Point(1.0, 2.0)
p2 = p1                   // p1 被移走

// 借用 &：自动创建令牌（无需手动 &）
distance: (a: &Point, b: &Point) -> Float = ...
d = distance(p1, p2)      // 编译器自动创建借用令牌

// 可变借用 &mut
update: (p: &mut Point, x: Float) -> Void = { p.x = x }

// ref：共享持有（编译器自动选 Rc/Arc）
shared = ref data

// clone：显式深拷贝
backup = data.clone()
```

## 并发

spawn 是唯一并行原语。无 async/await，无 Send/Sync。

```yaoxiang
// spawn 块：子表达式自动并行
result = spawn {
    user = fetch_user(1)
    posts = fetch_posts()
    return (user, posts)
}

// spawn for：数据并行
results = spawn for item in items {
    return process(item)
}

// spawn + ref：跨任务共享
main = {
    shared = ref data
    result = spawn {
        a = shared
        return a
    }
}
```

## F-string

```yaoxiang
name = "YaoXiang"
io.println(f"Hello {name}")          // Hello YaoXiang
io.println(f"Sum: {10 + 20}")        // Sum: 30
io.println(f"Pi: {pi:.2f}")          // Pi: 3.14
```
