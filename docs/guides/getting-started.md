# YaoXiang 快速入门

> 本指南帮助您快速上手 YaoXiang 编程语言。
>
> **注意**：本文档中的代码示例基于 YaoXiang 语言规范编写。如在实际运行中遇到语法差异，请参考 [语言规范](../YaoXiang-language-specification.md)。

[English version](./getting-started-en.md)

## 安装

### 从源码编译（推荐）

```bash
# 克隆仓库
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang

# 编译（调试版本，用于开发测试）
cargo build

# 编译（发布版本，推荐用于生产）
cargo build --release

# 运行测试
cargo test

# 查看版本
./target/debug/yaoxiang --version
# 或
./target/release/yaoxiang --version
```

**验证安装成功**：
```bash
./target/debug/yaoxiang --version
# 应输出类似: yaoxiang x.y.z
```

## 第一个程序

创建文件 `hello.yx`：

```yaoxiang
# hello.yx
use std.io

# 函数定义: name: (types) -> return_type = (params) => body
main: () -> Void = () => {
    println("Hello, YaoXiang!")
}
```

运行：

```bash
./target/debug/yaoxiang hello.yx
# 或使用 release 版本
./target/release/yaoxiang hello.yx
```

输出：

```
Hello, YaoXiang!
```

## 基本概念

### 变量与类型

```yaoxiang
# 自动类型推断
x = 42                    # 推断为 Int
name = "YaoXiang"         # 推断为 String
pi = 3.14159              # 推断为 Float
is_valid = true           # 推断为 Bool

# 显式类型注解（推荐使用类型集中约定）
count: Int = 100

# 默认不可变（安全特性）
x = 10
x = 20                    # ❌ 编译错误！不可变

# 可变变量（需要显式声明）
mut counter = 0
counter = counter + 1     # ✅ OK
```

### 函数

```yaoxiang
# 函数定义语法
add: (Int, Int) -> Int = (a, b) => a + b

# 调用
result = add(1, 2)        # result = 3

# 单参数函数
inc: Int -> Int = x => x + 1
```

### 类型定义

YaoXiang 使用统一的 `name: type = value` 语法模型：

```yaoxiang
# 变量声明
x: Int = 42
name: String = "YaoXiang"

# 函数定义
add: (Int, Int) -> Int = (a, b) => a + b

# 类型定义（使用花括号）
type Point = { x: Float, y: Float }

# 使用类型
p: Point = Point(x: 1.0, y: 2.0)
p.x  # 1.0
p.y  # 2.0
```

#### 记录类型

```yaoxiang
# 结构体类型
type Point = { x: Float, y: Float }
type Rect = { x: Float, y: Float, width: Float, height: Float }

# 使用
p = Point(x: 3.0, y: 4.0)
r = Rect(x: 0.0, y: 0.0, width: 10.0, height: 20.0)
```

#### 接口定义

接口是字段全为函数类型的记录类型：

```yaoxiang
# 定义接口
type Drawable = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

type Serializable = {
    serialize: () -> String
}

# 空接口
type EmptyInterface = {}
```

#### 类型方法

使用 `Type.method: (Type, ...) -> Return = ...` 语法定义类型方法：

```yaoxiang
# 类型定义
type Point = { x: Float, y: Float }

# 类型方法定义
Point.draw: (Point, Surface) -> Void = (self, surface) => {
    surface.plot(self.x, self.y)
}

Point.serialize: (Point) -> String = (self) => {
    "Point(${self.x}, ${self.y})"
}

# 使用方法（语法糖）
p = Point(x: 1.0, y: 2.0)
p.draw(screen)           # → Point.draw(p, screen)
str = p.serialize()      # → Point.serialize(p)
```

#### 自动绑定

使用 `pub` 关键字声明的函数会自动绑定到同文件定义的类型：

```yaoxiang
type Point = { x: Float, y: Float }

# pub 声明自动绑定到 Point
pub distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# 使用
p1 = Point(x: 3.0, y: 4.0)
p2 = Point(x: 1.0, y: 2.0)

# 函数式调用
d = distance(p1, p2)           # 3.606...

# OOP 语法糖（自动绑定到 Point.distance）
d2 = p1.distance(p2)           # → distance(p1, p2)
```

#### 枚举类型

```yaoxiang
# 简单枚举
type Color = red | green | blue

# 带数据的枚举
type Result[T, E] = ok(T) | err(E)

# 使用泛型
success: Result[Int, String] = ok(42)
failure: Result[Int, String] = err("not found")
```

#### 泛型类型

```yaoxiang
# 泛型类型定义
type List[T] = {
    data: Array[T],
    length: Int,
    push: (List[T], T) -> Void
}

# 具体实例化
type IntList = List(Int)
type StringList = List(String)
```

### 控制流

```yaoxiang
# 条件表达式
if x > 0 {
    "positive"
} elif x == 0 {
    "zero"
} else {
    "negative"
}

# 循环
for i in 0..5 {
    print(i)
}

# while 循环
mut n = 0
while n < 5 {
    print(n)
    n = n + 1
}
```

### 列表和字典

```yaoxiang
# 列表
numbers = [1, 2, 3, 4, 5]
first = numbers[0]         # 1

# 字典
scores = {"Alice": 90, "Bob": 85}
alice_score = scores["Alice"]  # 90

# 添加元素
mut list = [1, 2, 3]
list.append(4)
```

### 模式匹配

```yaoxiang
# match 表达式
result: Result[Int, String] = ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## 并作编程（异步）

YaoXiang 的独特特性：使用 `spawn` 标记的函数自动获得异步能力。

```yaoxiang
# 定义并作函数（自动异步执行）
fetch_data: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

# 调用并作函数（自动并行，无需 await）
main: () -> Void = () => {
    # 两次调用自动并行执行
    user = fetch_user(1)     # 自动并行
    posts = fetch_posts()    # 自动并行

    # 当需要结果时自动等待
    print(user.name)
    print(posts.length)
}
```

## 模块系统

```yaoxiang
# 导入标准库
use std.io
use std.math

# 使用导入的函数
result = math.sqrt(16)      # 4.0
println("Hello!")
```

## 常见问题

### Q: 变量默认不可变，如何修改变量？

```yaoxiang
# 使用 mut 关键字声明可变变量
mut x = 10
x = 20                       # ✅ OK
```

### Q: 如何定义函数？

```yaoxiang
# 完整形式（推荐）
add: (Int, Int) -> Int = (a, b) => a + b

# 简短形式（类型推断）
add = (a, b) => a + b
```

### Q: 如何处理错误？

```yaoxiang
# 使用 Result 类型
type Result[T, E] = ok(T) | err(E)

# 模式匹配处理
result = risky_operation()
match result {
    ok(value) => print("Success: " + value)
    err(error) => print("Error: " + error)
}
```

## 下一步

- 📖 阅读 [YaoXiang 指南](../YaoXiang-book.md) 了解核心特性
- 📚 查看 [语言规范](../YaoXiang-language-specification.md) 了解完整语法
- 🏗️ 浏览 [架构文档](../architecture/) 了解实现细节
- 💡 查看 [设计宣言](../YaoXiang-design-manifesto.md) 了解核心理念

## 相关资源

- [GitHub 仓库](https://github.com/yourusername/yaoxiang)
- [Issue 反馈](https://github.com/yourusername/yaoxiang/issues)
- [贡献指南](../guides/dev/)
