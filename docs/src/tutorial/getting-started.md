# YaoXiang 快速入门

> 本指南帮助您快速上手 YaoXiang 编程语言。
>
> **注意**：本文档中的代码示例基于 YaoXiang 语言规范编写。如在实际运行中遇到语法差异，请参考
> [语言规范](../reference/language-spec/index.md)。

## 安装

### 从源码编译（推荐）

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
./target/debug/yaoxiang-rs --version
# 或
./target/release/yaoxiang-rs --version
```

**验证安装成功**：

```bash
./target/debug/yaoxiang-rs --version
# 应输出类似: yaoxiang-rs 0.8.2
```

## 第一个程序

创建文件 `hello.yx`：

```yaoxiang
// hello.yx
use std.io

// 函数定义: name: (param: Type, ...) -> return_type = { return ... }  # 代码块必须显式 return
// 表达式形式: name: (param: Type, ...) -> return_type = expr           # 表达式直接返回值
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

运行：

```bash
./target/debug/yaoxiang-rs run hello.yx
# 或使用 release 版本
./target/release/yaoxiang-rs run hello.yx
```

输出：

```
Hello, YaoXiang!
```

## 基本概念

### 变量与类型

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

### 函数

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

### 类型定义

YaoXiang 使用统一的 `name: type = value` 语法模型：

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

#### 记录类型

<!-- docs-example: skip -->
```yaoxiang
// 结构体类型
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// 使用
p = Point(x=3.0, y=4.0)
r = Rect(x=0.0, y=0.0, width=10.0, height=20.0)
```

#### 接口定义

接口是字段全为函数类型的记录类型：

<!-- docs-example: skip -->
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

#### 类型方法

使用 `Type.method: (Type, ...) -> Return = ...` 语法定义类型方法：

<!-- docs-example: skip -->
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

#### 没有隐式绑定

方法不会自动挂到类型上，`pub` 关键字也已删除（RFC-029g）——要加方法就按上一节的
`Type.method: (self: T, …) -> R = …` 显式写，调用点用 `.`。

#### 枚举类型

```yaoxiang
// 简单枚举
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// 带数据的枚举
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 使用泛型——变体构造器必须用「类型.变体」限定
// 裸写 ok(42) / err("not found") 报 E1001
success: Result(Int, String) = Result(Int, String).ok(42)
failure: Result(Int, String) = Result(Int, String).err("not found")
```

#### 泛型类型

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

### 控制流

```yaoxiang
// 条件表达式
x = 42

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

### 列表和字典

```yaoxiang
use std.list

// 列表
numbers = [1, 2, 3, 4, 5]
first = numbers[0]  // 1

// 字典
scores = {"Alice": 90, "Bob": 85}
alice_score = scores["Alice"]  // 90

// 添加元素：列表没有实例方法，调用 std.list 的函数形式
// 注意：不要把变量命名为 list —— 那会遮蔽模块
mut items = [1, 2, 3]
items = list.push(items, 4)  // [1, 2, 3, 4]
```

### 模式匹配

```yaoxiang
// match 表达式
// 注意：Result 构造器需先 use std.result；裸写 ok(42) 报 E1001
use std.result
result: Result(Int, String) = Result(Int, String).ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## 并作编程（并发）

YaoXiang 的并发模型围绕 `spawn <expr>` 原语构建——它是唯一的并行入口。

<!-- docs-example: skip -->
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

**核心规则**：`spawn`
修饰的表达式在后台执行，外层同步阻塞等待结果。无依赖的任务自动并行，由运行时 GMP 模型调度。

## 模块系统

```yaoxiang
// 导入标准库
use std.io
use std.math

// 使用导入的函数
result = math.sqrt(16.0)  // 4.0
print("Hello!")
```

## 常见问题

### Q: 变量默认不可变，如何修改变量？

```yaoxiang
// 使用 mut 关键字声明可变变量
mut x = 10
x = 20  // ✅ OK
```

### Q: 如何定义函数？

```yaoxiang
// 完整形式（推荐）
add: (a: Int, b: Int) -> Int = a + b

// 简短形式（类型推断）
add = (a, b) => a + b
```

### Q: 如何处理错误？

```yaoxiang
// 记录式和类型的变体写成「字段名: (载荷) -> 类型」
// 注意：类型体里不能用 `|`——`{ ok(T) | err(E) }` 解析报 E0010
// 工程里直接用标准库内置的 Result
use std.result

r = Result(Int, String).ok(42)

// 模式匹配处理
match r {
    ok(value) => print("Success: {value}")
    err(e) => print("Error: " + e)
}
```

## 下一步

- 📚 查看 [语言规范](../reference/language-spec/index.md) 了解完整语法
- 🏗️ 浏览 [设计文档](../explanation/) 了解实现细节
- 💡 查看 [设计宣言](../explanation/manifesto.md) 了解核心理念

## 相关资源

- [GitHub 仓库](https://github.com/ChenXu233/YaoXiang)
- [Issue 反馈](https://github.com/ChenXu233/YaoXiang/issues)
- [贡献指南](../dev/contributing.md)
