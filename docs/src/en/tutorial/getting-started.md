# YaoXiang Quick Start

> This guide helps you get up to speed quickly with the YaoXiang programming language.
>
> **Note**: The code examples in this document are written based on the YaoXiang language
> specification. If you encounter syntax differences in actual execution, please refer to the
> [language specification](../reference/language-spec/index.md).

## Installation

### Build from Source (Recommended)

```bash
# 克隆仓库
git clone https://github.com/ChenXu233/YaoXiang.git
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

**Verify successful installation**:

```bash
./target/debug/yaoxiang --version
# 应输出类似: yaoxiang x.y.z
```

## Your First Program

Create the file `hello.yx`:

```yaoxiang
// hello.yx
use std.io

// 函数定义: name: (param: Type, ...) -> return_type = { return ... }  # 代码块必须显式 return
// 表达式形式: name: (param: Type, ...) -> return_type = expr           # 表达式直接返回值
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

Run it:

```bash
./target/debug/yaoxiang hello.yx
# 或使用 release 版本
./target/release/yaoxiang hello.yx
```

Output:

```
Hello, YaoXiang!
```

## Basic Concepts

### Variables and Types

```yaoxiang
// 自动类型推断
x = 42  // 推断为 Int
name = "YaoXiang"  // 推断为 String
pi = 3.14159  // 推断为 Float
is_valid = true  // 推断为 Bool

// 显式类型注解（推荐使用类型集中约定）
count: Int = 100

// 默认不可变（安全特性）
x = 10
x = 20  // ❌ 编译错误！不可变

// 可变变量（需要显式声明）
mut counter = 0
counter = counter + 1  // ✅ OK
```

### Functions

```yaoxiang
// 函数定义语法
// 表达式形式：直接返回值，不需要 return
add: (a: Int, b: Int) -> Int = a + b

// 代码块形式：必须使用 return 返回值
// add: (a: Int, b: Int) -> Int = { return a + b }

// 调用
result = add(1, 2)  // result = 3

// 单参数函数（表达式形式）
inc: (x: Int) -> Int = x + 1
```

### Type Definitions

YaoXiang uses a unified `name: type = value` syntax model:

```yaoxiang
// 变量声明
x: Int = 42
name: String = "YaoXiang"

// 函数定义
add: (a: Int, b: Int) -> Int = a + b

// 类型定义（使用花括号）
Point: Type = { x: Float, y: Float }

// 使用类型
p: Point = Point(x=1.0, y=2.0)
p.x  // 1.0
p.y  // 2.0
```

#### Record Type

```yaoxiang
// 结构体类型
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// 使用
p = Point(x=3.0, y=4.0)
r = Rect(x=0.0, y=0.0, width=10.0, height=20.0)
```

#### Interface Definition

An interface is a record type where all fields are function types:

```yaoxiang
// 定义接口
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 空接口
EmptyInterface: Type = {}
```

#### Type Methods

Use the `Type.method: (Type, ...) -> Return = ...` syntax to define type methods:

```yaoxiang
// 类型定义
Point: Type = { x: Float, y: Float }

// 类型方法定义
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    "Point({self.x}, {self.y})"
}

// 使用方法（语法糖）
p = Point(x=1.0, y=2.0)
p.draw(screen)  // → Point.draw(p, screen)
str = p.serialize()  // → Point.serialize(p)
```

#### Automatic Binding

Functions declared with the `pub` keyword are automatically bound to types defined in the same file:

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub 声明自动绑定到 Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// 使用
p1 = Point(x=3.0, y=4.0)
p2 = Point(x=1.0, y=2.0)

// 函数式调用
d = distance(p1, p2)  // 3.606...

// OOP 语法糖（自动绑定到 Point.distance）
d2 = p1.distance(p2)  // → distance(p1, p2)
```

#### Enum Type

```yaoxiang
// 简单枚举
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// 带数据的枚举
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 使用泛型
success: Result(Int, String) = ok(42)
failure: Result(Int, String) = err("not found")
```

#### Generic Type

```yaoxiang
// 泛型类型定义
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (List(T), T) -> Void
}

// 具体实例化
IntList: Type = List(Int)
StringList: Type = List(String)
```

### Control Flow

```yaoxiang
// 条件表达式
if x > 0 {
    "positive"
} else if x == 0 {
    "zero"
} else {
    "negative"
}

// 循环
for i in 0..5 {
    print(i)
}

// while 循环
mut n = 0
while n < 5 {
    print(n)
    n = n + 1
}
```

### Lists and Dictionaries

```yaoxiang
// 列表
numbers = [1, 2, 3, 4, 5]
first = numbers[0]  // 1

// 字典
scores = {"Alice": 90, "Bob": 85}
alice_score = scores["Alice"]  // 90

// 添加元素
mut list = [1, 2, 3]
list.append(4)
```

### Pattern Matching

```yaoxiang
// match 表达式
result: Result(Int, String) = ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## Spawn Programming (Concurrency)

YaoXiang's concurrency model is built around the `spawn <expr>` primitive — it is the only entry
point for parallelism.

```yaoxiang
// spawn 修饰任意表达式，自动并行执行
main: () -> Void = {
    user = spawn fetch_user(1)   // 后台执行
    posts = spawn fetch_posts()  // 并行的另一步

    // 需要结果时自动阻塞等待
    print(user.name)
    print(posts.length)
}
```

**Core rule**: Expressions modified by `spawn` execute in the background, while the outer scope
synchronously blocks waiting for results. Tasks with no dependencies automatically run in parallel,
scheduled by the runtime GMP model.

## Module System

```yaoxiang
// 导入标准库
use std.io
use std.math

// 使用导入的函数
result = math.sqrt(16)  // 4.0
print("Hello!")
```

## FAQ

### Q: Variables are immutable by default. How do I modify a variable?

```yaoxiang
// 使用 mut 关键字声明可变变量
mut x = 10
x = 20  // ✅ OK
```

### Q: How do I define a function?

```yaoxiang
// 完整形式（推荐）
add: (a: Int, b: Int) -> Int = a + b

// 简短形式（类型推断）
add = (a, b) => a + b
```

### Q: How do I handle errors?

```yaoxiang
// 使用 Result 类型
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// 模式匹配处理
result = risky_operation()
match result {
    ok(value) => print("Success: " + value)
    err(error) => print("Error: " + error)
}
```

## Next Steps

- 📚 See the [Language Specification](../reference/language-spec/index.md) for the complete syntax
- 🏗️ Browse the [Design Documents](../design/) for implementation details
- 💡 Read the [Design Manifesto](../design/manifesto.md) for the core philosophy

## Related Resources

- [GitHub Repository](https://github.com/yourusername/yaoxiang)
- [Issue Feedback](https://github.com/yourusername/yaoxiang/issues)
- [Contributing Guide](../dev/contributing.md)
