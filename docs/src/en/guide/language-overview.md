---
title: 'Syntax Cheat Sheet'
---

# Syntax Cheat Sheet

Understand YaoXiang's core syntax in 5 minutes. For deeper learning, visit the
[Tutorial](../tutorial/index.md).

## Variables

```yaoxiang
x = 42                    // 不可变（默认）
mut y = 0                 // 可变

name: String = "hello"    // 显式类型
count: Int = 100          // 类型注解

pub version = "1.0"       // 公开导出
```

## Functions

Everything is `name: type = value`. Functions are values too.

```yaoxiang
use std.io

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

## Types

No `type`, `struct`, `trait`, or `impl` keywords. A single unified declaration handles everything.

<!-- docs-example: skip -->
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
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// 注意：标准库已内置 std.option（Option + Try），不建议自行定义
// Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
// 注意：裸构造器 ok/err 当前报 E1001（实测），标准库见 std.result
// Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

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
// 泛型约束签名当前解析器尚不支持，实测报 E0011
clone: (value: T) -> T = value
// 泛型约束签名当前解析器尚不支持，实测报 E0011
sort: (list: List(T)) -> List(T)
```

## Methods

<!-- docs-example: skip -->
```yaoxiang
// 命名空间函数（Type.method 只是归属标记，不是绑定）
Point.distance: (a: &Point, b: &Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    // 注意：括号表达式不能直接挂方法（实测 E1053），需先绑定到变量
    d2 = dx * dx + dy * dy
    return d2.sqrt()
}

// 显式绑定后才有 . 调用语法
Point.distance = distance[0]
// 此后 p1.distance(p2) → distance(p1, p2)

// 快速定义 + 绑定
Point.draw: (self: &Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}
```

## Control Flow

```yaoxiang
use std.result

// if 是表达式
score = 72
value: Result(Int, String) = Result(Int, String).ok(1)
grade = if score >= 90 { "A" } else if score >= 60 { "B" } else { "C" }   // "B"

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

## Data Structures

```yaoxiang
use std.dict
use std.list

// 列表
nums = [1, 2, 3, 4, 5]
first = nums[0]           // 1

// 字典
scores = {"Alice": 90, "Bob": 85}
a = scores["Alice"]       // 90

// 列表推导式
// 注意：0.8.2 的列表推导式不支持 if 过滤（实测 E0010），改用 list.filter
evens = list.filter(nums, (x) => x % 2 == 0)
doubled = [x * 2 for x in nums]
```

## Pattern Matching

<!-- docs-example: skip -->
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

## Modules and Imports

```yaoxiang
use std.io
use std.math.{sqrt, sin, cos}
use std.{io, list}

io.println("hello")
result = sqrt(16.0)         // 4.0

// 别名
use std.math as math
use std.{io as print}

// 公开导出
pub add: (a: Int, b: Int) -> Int = a + b
pub Point: Type = { x: Float, y: Float }
```

## Ownership

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

## Concurrency

`spawn` is the only parallelism primitive. No async/await, no Send/Sync.

<!-- docs-example: skip -->
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
main: () -> Void = {
    shared = ref data
    result = spawn {
        a = shared
        return a
    }
}
```

## F-string

```yaoxiang
pi = 3.14159
name = "YaoXiang"
print(f"Hello {name}")               // Hello YaoXiang
print(f"Sum: {10 + 20}")        // Sum: 30
print(f"Pi: {pi}")                    // 实测：格式说明符暂未实现，输出 3.14159
```
