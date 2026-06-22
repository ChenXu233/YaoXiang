---
title: 语法速查
---

# 语法速查

5 分钟看懂 YaoXiang 核心语法。深入学习请访问 [教程](/tutorial/)。

## 变量

```yaoxiang
x = 42                    # 不可变（默认）
mut y = 0                 # 可变

name: String = "hello"    # 显式类型
count: Int = 100          # 类型注解
```

## 函数

```yaoxiang
# 表达式形式（直接返回值）
add: (a: Int, b: Int) -> Int = a + b

# 代码块形式（显式 return）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
```

## 类型

```yaoxiang
# 记录类型
type Point = { x: Float, y: Float }
p = Point(x: 1.0, y: 2.0)

# 枚举
type Result(T, E) = ok(T) | err(E)
type Color = red | green | blue

# 接口
type Drawable = { draw: (Surface) -> Void }

# 泛型
List: (T: Type) -> Type = { data: Array(T), length: Int }
```

## 控制流

```yaoxiang
# if 是表达式
grade = if score >= 90 { "A" } elif score >= 60 { "B" } else { "C" }

# match
result = match value {
    ok(v) => "success: ${v}",
    err(e) => "error: ${e}",
}

# 循环
for i in 0..5 { println(i) }

mut n = 0
while n < 5 { println(n); n = n + 1 }
```

## 数据结构

```yaoxiang
# 列表
nums = [1, 2, 3, 4, 5]
first = nums[0]           # 1

# 字典
scores = {"Alice": 90, "Bob": 85}
a = scores["Alice"]       # 90

# 集合
colors = {"red", "green", "blue"}

# 列表推导式
evens = [x for x in nums if x % 2 == 0]
```

## 模式匹配

```yaoxiang
match shape {
    circle(r) => pi * r * r,
    rect(w, h) => w * h,
    point => 0,
}

# 结构体模式
match p {
    { x: 0, y: 0 } => "origin",
    { x, y } => "(${x}, ${y})",
}

# 卫表达式
match age {
    adult(n) if n >= 18 => true,
    _ => false,
}
```

## Lambda

```yaoxiang
double = (x) => x * 2
add = (a, b) => a + b
apply = (list, op) => [op(x) for x in list]
```

## F-string

```yaoxiang
name = "YaoXiang"
println(f"Hello {name}")          # Hello YaoXiang
println(f"Sum: {10 + 20}")        # Sum: 30
println(f"Pi: {pi:.2f}")          # Pi: 3.14
```

## 模块

```yaoxiang
use std.io
use std.math

println("hello")
result = math.sqrt(16)    # 4.0
```

## 所有权

```yaoxiang
# Move：默认所有权转移
p1 = Point(1.0, 2.0)
p2 = p1                   # p1 被移走

# ref：共享持有
shared = ref data         # 编译器自动选 Rc/Arc

# clone：显式深拷贝
backup = data.clone()
```

## 并发

```yaoxiang
# spawn 标记的函数自动异步
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

# 自动并行，无需 await
user = fetch_user(1)
posts = fetch_posts()
```
