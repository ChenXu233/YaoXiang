# YaoXiang（爻象）编程语言规范

> 版本：v1.0.0-draft
> 状态：初稿
> 作者：晨煦
> 日期：2024-12-31

---

## 第一章：引言

### 1.1 语言概述

YaoXiang（爻象）是一门通用编程语言，其设计理念源于《易经》中「爻」与「象」的核心概念。「爻」是组成卦象的基本符号，象征着阴阳变化；「象」是事物本质的外在表现，代表万象万物。YaoXiang 将这一哲学思想融入编程语言的类型系统之中，提出「一切皆类型」的核心理念。

在 YaoXiang 的世界观中，值、函数、模块、泛型都是类型的具体表现。类型不再仅仅是数据的静态标签，而是可以作为数据在运行时被操作、被组合、被反射的一等公民。这种设计使得编程语言能够更直接地表达领域概念，同时保持数学上的严谨性。

YaoXiang 融合了多种编程范式的精华。它继承了 Rust 的所有权模型和零成本抽象思想，确保内存安全和运行效率；它采纳了 Python 的语法风格，追求代码的可读性和自然语言感；它借鉴了 Idris 和 Agda 的依赖类型理论，提供强大的类型表达能力；它学习了 TypeScript 的渐进式类型设计，让新手能够平滑过渡。

### 1.2 设计目标

YaoXiang 的设计目标可以概括为以下几个方面。

**第一，统一的类型抽象**。在 YaoXiang 中，类型是最高层的抽象单元。值是类型的实例，类型本身也是类型的实例（元类型），函数是输入类型到输出类型的映射，模块是类型的命名空间组合。这种统一的抽象框架简化了语言的语义，使得编译器能够进行更有效的优化。

**第二，自然的编程体验**。YaoXiang 采用 Python 风格的语法，强调可读性和表达力。变量无需显式声明类型，编译器能够根据上下文自动推断。控制流结构清晰直观，代码块使用明确的缩进和花括号标记。这种设计降低了学习门槛，让新手能够快速上手。

**第三，安全的内存管理**。YaoXiang 采用了 Rust 风格的所有权模型，每个值有唯一的所有者，当所有者离开作用域时，值被自动销毁。这种机制在编译期消除悬挂指针和数据竞争，同时不需要垃圾回收器的介入，保证了高性能和可预测的内存使用。

**第四，无感的异步编程**。传统的异步编程需要程序员显式地使用 async、await 关键字或回调函数。YaoXiang 采用了创新的无感异步机制：使用 spawn 标记的函数会自动获得异步能力，调用这些函数的代码块由编译器自动插入等待逻辑。程序员可以像写同步代码一样写并发程序。

**第五，完整的类型反射**。YaoXiang 的类型系统在运行时完全可用。程序可以在运行时检查任意值的类型、访问类型结构、进行类型转换和组合。这种能力使得元编程、序列化、测试等场景的实现变得更加简单。

**第六，AI 友好的语法**。YaoXiang 的语法设计考虑了 AI 代码生成和修改的需求。代码结构严格规范，没有歧义；AST 清晰明了，关键位置容易定位；类型信息完整明确，AI 能够准确理解代码意图。

### 1.3 语言特性速览

以下代码展示了 YaoXiang 的一些核心特性，让读者对语言风格有一个初步印象。

```yaoxiang
# 自动类型推断
x = 42                           # 推断为 Int
name = "YaoXiang"                # 推断为 String
pi = 3.14159                     # 推断为 Float

# 类型注解（可选）
count: Int = 100

# 默认不可变
x = 10
x = 20                           # 编译错误！

# 可变变量需要显式声明
mut counter = 0
counter = counter + 1            # OK

# 函数定义
fn add(a: Int, b: Int) -> Int {
    a + b                        # 最后表达式作为返回值
}

# 泛型函数
fn identity[T](x: T) -> T {
    x
}

# 类型定义
type Point = struct {
    x: Float
    y: Float
}

# 无感异步
fn fetch_data(url: String) -> JSON spawn {
    HTTP.get(url).json()
}

fn main() {
    data = fetch_data("https://api.example.com")
    # 自动等待，无需 await
    print(data.name)
}

# 模式匹配
fn classify(n: Int) -> String {
    match n {
        0 -> "zero"
        1 -> "one"
        _ if n < 0 -> "negative"
        _ -> "many"
    }
}

# 模块系统
mod Math {
    pub fn sqrt(x: Float) -> Float { ... }
    pub pi = 3.14159
}

use Math
```

---

## 第二章：词法结构

### 2.1 源文件编码

YaoXiang 源文件必须使用 UTF-8 编码。源文件通常以 `.yx` 为扩展名。一个 YaoXiang 程序由一个或多个源文件组成，每个源文件包含一系列词法单元。

### 2.2 词法单元

YaoXiang 的词法单元包括以下几类：标识符、关键字、字面量、运算符、分隔符和空白符。

**标识符**用于命名变量、函数、类型等。标识符以字母或下划线开头，后续字符可以是字母、数字或下划线。标识符是大小写敏感的，`foo`、`Foo` 和 `FOO` 是三个不同的标识符。以下是有效的标识符示例：`x`、`count`、`my_variable`、`_private`、`type1`、`_`。

YaoXiang 还支持以下划线开头的特殊标识符。下划线 `_` 用作占位符，表示忽略某个值，这在模式匹配和解构时非常有用。以下划线开头的标识符如 `_private` 和 `_unused` 表示私有成员或未使用的变量。

**关键字**是语言预定义的保留词，不能用作标识符。YaoXiang 共定义了 18 个关键字，具体列表和用途将在后续章节详细介绍。

**字面量**用于表示固定值，包括整数、浮点数、布尔值、字符、字符串和集合字面量。

整数字面量可以用十进制、八进制、十六进制或二进制表示。十进制整数如 `42`、`0`、`123456`。八进制整数以 `0o` 开头如 `0o755`。十六进制整数以 `0x` 开头如 `0xFF`、`0x1A3B`。二进制整数以 `0b` 开头如 `0b1010`。整数可以添加下划线作为分隔符以提高可读性，如 `1_000_000`、`0xFF_FF`。

浮点数字面量由整数部分、小数点、小数部分和可选的指数部分组成。标准形式如 `3.14`、`0.5`、`123.456`。科学计数法如 `1.5e10`、`2.5E-3`。添加下划线的形式如 `3.141_592`。

布尔值只有两个：`true` 表示真，`false` 表示假。

字符字面量用单引号包围，表示一个 Unicode 字符。普通字符如 `'a'`、`'中'`。转义字符如 `'\n'`（换行）、`'\t'`（制表符）、`'\\'`（反斜杠）、`'\''`（单引号）、`'\"'`（双引码）。Unicode 转义如 `'\u0041'`（大写 A）。

字符串字面量用双引号包围，支持转义字符和字符串插值。普通字符串如 `"Hello, YaoXiang!"`。包含转义的字符串如 `"Line1\nLine2"`。原始字符串以 `r"` 开头，以 `"` 结尾，其中的内容不会被转义，如 `r"C:\path\to\file"`。字符串插值使用花括号，如 `"Hello, {name}!"`。

字节串字面量用于表示二进制数据，以 `b"` 开头，如 `b"hello"`、`b"\x00\x01\x02"`。

集合字面量包括列表、字典和集合。列表用方括号如 `[1, 2, 3]`、`["a", "b", "c"]`。字典用花括号，键值对用冒号分隔如 `{"a": 1, "b": 2}`。集合用花括号但只包含值如 `{1, 2, 3}`。空列表写为 `[]`，空字典写为 `{}`。

**运算符**包括算术运算符、比较运算符、逻辑运算符、位运算符和赋值运算符。完整的运算符列表将在表达式章节详细介绍。

**分隔符**用于分隔语法单元。圆括号 `(` 和 `)` 用于分组和函数调用。方括号 `[` 和 `]` 用于列表索引和类型参数。花括号 `{` 和 `}` 用于代码块和集合字面量。逗号 `,` 用于分隔列表元素、函数参数等。分号 `;` 用于分隔同一行的多个语句（可选）。冒号 `:` 用于类型注解和字典。竖线 `|` 用于联合类型和模式匹配。箭头 `->` 用于函数返回类型和 lambda 表达式。

### 2.3 空白符与注释

**空白符**包括空格、制表符、换行符和回车符。在大多数情况下，空白符被忽略，用于分隔词法单元和美化代码。但在字符串字面量中，空白符是字面量的一部分。

YaoXiang 规定代码必须使用 4 个空格缩进，禁止使用 Tab 字符。这是一项强制语法规则，不遵守将导致编译错误。这一规定确保了代码格式的一致性，便于团队协作和 AI 处理。

**注释**用于在代码中添加说明和文档。YaoXiang 支持单行注释和多行注释。单行注释以 `#` 开头，直到行尾，如 `# 这是一个注释`。多行注释以 `#!` 开头，以 `!#` 结尾，如 `#! 这是一个多行注释，可以跨越多行 !#`。

文档注释使用特殊的注释语法，以 `##` 开头。文档注释可以附加在类型定义、函数定义等之前，会被文档工具处理。示例如下：

```yaoxiang
## 计算两个整数的和
##
## 参数:
##   a: 第一个加数
##   b: 第二个加数
##
## 返回: 两个数的和
fn add(a: Int, b: Int) -> Int {
    a + b
}
```

### 2.4 关键字详解

YaoXiang 的 18 个关键字及其用途如下表所示。

| 关键字 | 作用 | 示例 |
|--------|------|------|
| `type` | 定义新类型 | `type Point = struct { x: Int, y: Int }` |
| `fn` | 定义函数 | `fn add(a, b) -> a + b` |
| `pub` | 标记公共导出 | `pub fn main() { ... }` |
| `mod` | 定义模块 | `mod Math { ... }` |
| `use` | 导入模块 | `use std.io` |
| `spawn` | 异步标记 | `fn fetch() -> T spawn { ... }` |
| `ref` | 不可变引用 | `fn foo(x: ref T) { ... }` |
| `mut` | 可变引用 | `fn bar(x: mut T) { ... }` |
| `if` | 条件分支 | `if x > 0 { ... }` |
| `elif` | 多重条件 | `elif x == 0 { ... }` |
| `else` | 默认分支 | `else { ... }` |
| `match` | 模式匹配 | `match x { 0 -> "zero" }` |
| `while` | 条件循环 | `while i < 10 { ... }` |
| `for` | 迭代循环 | `for item in items { ... }` |
| `return` | 返回值 | `return result` |
| `break` | 跳出循环 | `break` |
| `continue` | 继续循环 | `continue` |
| `as` | 类型转换 | `x as Float` |

这些关键字都有特定的语法位置和语义，不能被用作标识符。

**无限循环语法：** 使用 `while True` 实现无限循环，无需单独的 `loop` 关键字。

```yaoxiang
# 无限循环示例
while True {
    input = read_line()
    if input == "quit" {
        break
    }
    process(input)
}
```

### 2.5 保留字

除了关键字外，YaoXiang 还有一些保留字，它们是语言预定义的特殊值，不能用作标识符，但它们不是关键字（不能用于语法结构）。

| 保留字 | 类型 | 说明 |
|--------|------|------|
| `true` | Bool | 布尔值真 |
| `false` | Bool | 布尔值假 |
| `null` | Void | 空值 |
| `none` | Option | Option 类型的无值变体 |
| `some(T)` | Option | Option 类型的值变体（构造函数） |
| `ok(T)` | Result | Result 类型的成功变体（构造函数） |
| `err(E)` | Result | Result 类型的错误变体（构造函数） |

```yaoxiang
# 布尔值
flag = true
flag = false

# Option 类型使用
maybe_value: option[String] = none
maybe_value = some("hello")

# Result 类型使用
result: result[Int, String] = ok(42)
result = err("error message")
```

---

## 第三章：类型系统

### 3.1 类型概述

YaoXiang 的类型系统是其最核心的特性，体现「一切皆类型」的设计理念。在 YaoXiang 中，类型不仅是数据的静态描述，还是可以在运行时操作的一等公民。

类型层次结构可以用一句话概括：所有类型都是 `type`（元类型）的实例。这意味着类型本身可以作为值被传递、存储和操作。这种设计借鉴了依赖类型语言的理念，但进行了简化和优化以支持高效编译。

### 3.2 类型分类

YaoXiang 的类型可以分为以下几个大类。

**原类型**是语言内置的基本类型，不需要定义即可使用。原类型包括：`Void` 表示空值，类型只有一个值 `null`；`Bool` 表示布尔值，类型有两个值 `true` 和 `false`；`Int` 表示有符号整数，默认为 64 位；`Uint` 表示无符号整数，默认为 64 位；`Float` 表示浮点数，默认为 64 位双精度；`Char` 表示单个 Unicode 字符；`String` 表示 UTF-8 字符串；`Bytes` 表示原始字节序列。

整数类型可以指定位宽：`Int8`、`Int16`、`Int32`、`Int64`、`Int128` 和对应的 `Uint8`、`Uint16`、`Uint32`、`Uint64`、`Uint128`。浮点数类型可以指定位宽：`Float32` 和 `Float64`。

**复合类型**是由其他类型组合而成的类型。结构体类型 `struct` 包含命名字段的固定集合；联合类型 `union` 表示可以是多种类型之一；枚举类型 `enum` 是联合类型的特殊形式，每个变体有唯一标识；元组类型 `tuple` 是匿名复合类型；列表类型 `list` 是同质可变长集合；字典类型 `dict` 是键值对映射；集合类型 `set` 是同质无重复元素的集合。

**函数类型**表示输入类型到输出类型的映射。函数类型的基本形式是 `fn(Param1, Param2, ...) -> ReturnType`。

**泛型类型**是参数化的类型模板。泛型类型接受类型参数来生成具体的类型，如 `List[Int]`、`Map[String, Int]`。

**依赖类型**是类型依赖于值的高级类型机制。依赖类型允许类型表达式中包含值，如 `Vector[T, n: Nat]` 表示长度为 n 的向量。

**模块类型**是模块的类型表示。模块可以看作是包含函数和类型的命名空间，模块类型描述了模块的导出接口。

### 3.3 类型定义

**结构体类型**的定义语法如下：

```yaoxiang
type Point = struct {
    x: Float
    y: Float
}

type Person = struct {
    name: String
    age: Int
    email: option[String]     # option[T] 是内置的泛型类型
}
```

结构体的字段可以通过点运算符访问：`point.x`、`person.name`。

**联合类型**的定义语法如下：

```yaoxiang
# 简单的联合类型
type Result[T, E] = union {
    ok: T
    err: E
}

# 变体也可以是命名的
type Shape = union {
    circle: struct { radius: Float }
    rectangle: struct { width: Float, height: Float }
    triangle: struct { a: Float, b: Float, c: Float }
}
```

联合类型的值必须明确是哪个变体。

**枚举类型**是联合类型的特殊形式：

```yaoxiang
type Color = enum {
    red
    green
    blue
}

type Direction = enum {
    north
    south
    east
    west
}
```

**元组类型**用括号表示：

```yaoxiang
# 元组类型
Point2D = (Float, Float)
Triple = (Int, String, Bool)

# 使用元组
point = (3.14, 2.71)
first = point.0          # 访问第一个元素
second = point.1         # 访问第二个元素
```

**泛型类型**的定义语法如下：

```yaoxiang
# 泛型列表
type List[T] = struct {
    elements: [T]
    length: Int
}

# 泛型字典
type Map[K, V] = struct {
    keys: [K]
    values: [V]
    size: Int
}

# 泛型选项类型
type Option[T] = union {
    some: T
    none
}

# 依赖类型：固定长度向量
type Vector[T, n: Nat] = struct {
    data: [T; n]         # 固定长度数组
}

# 使用泛型类型
numbers: List[Int] = [1, 2, 3]
maybe: Option[String] = some("hello")
vector: Vector[Float, 3] = [1.0, 2.0, 3.0]
```

**类型别名**允许为现有类型创建新名称：

```yaoxiang
type Integer = Int
type Text = String
type Number = Int | Float
type NumericList = List[Number]
```

### 3.4 类型操作

**类型作为值**是 YaoXiang 的核心特性之一。类型本身可以作为值被操作：

```yaoxiang
# 类型赋值
MyInt = Int
MyList = List(Int)

# 类型作为函数参数
fn print_type_name(t: type) {
    print(t.name)
}

print_type_name(Int)           # 输出: Int
print_type_name(List(Int))     # 输出: List(Int)

# 类型组合
type Pair[T, U] = struct {
    first: T
    second: U
}

# 类型联合
type Number = Int | Float

# 类型交集
type Printable = struct { to_string: fn() -> String }
type Serializable = struct { to_json: fn() -> String }
type Versatile = Printable & Serializable
```

**类型反射**允许在运行时检查类型信息：

```yaoxiang
# 获取类型
x = 42
t = type_of(x)         # 返回 Int 类型

# 检查类型
if t == Int {
    print("It's an integer")
}

# 类型结构
fn describe_type(t: type) -> String {
    match t {
        struct { fields } -> "Struct with " + fields.length + " fields"
        union { variants } -> "Union with " + variants.length + " variants"
        enum { variants } -> "Enum with " + variants.length + " cases"
        list { element } -> "List of " + element.name
        fn { params, ret } -> "Function type"
        primitive { name } -> "Primitive: " + name
    }
}

# 类型实例检查
value: type = ...
if value has_type Int {
    print("It's an integer")
}
```

**类型转换**使用 `as` 运算符：

```yaoxiang
# 数值类型转换
x = 42
y = x as Float          # Int -> Float

# 类型强制转换（需要运行时检查）
maybe_string: option[String] = some("hello")
string_value = maybe_string as String  # Option[String] -> String，可能panic

# 安全转换
fn safe_cast[T, U](value: T, target: type) -> option[U] {
    if value has_type target {
        some(value as U)
    } else {
        none
    }
}
```

### 3.5 类型推断

YaoXiang 强大的类型推断能力使得大多数情况下无需显式写类型注解：

```yaoxiang
# 基本推断
x = 42                    # 推断为 Int
y = 3.14                  # 推断为 Float
z = "hello"               # 推断为 String
flag = true               # 推断为 Bool

# 函数返回值推断
fn add(a: Int, b: Int) {
    a + b                 # 推断返回类型为 Int
}

# 泛型推断
fn first[T](list: List[T]) -> option[T] {
    if list.length > 0 {
        some(list[0])
    } else {
        none
    }
}

numbers = [1, 2, 3]
first_num = first(numbers)    # 推断 T = Int
```

类型注解仍然可以在需要时显式写出，以增加代码清晰度或解决歧义：

```yaoxiang
# 显式类型注解
count: Int = 100
price: Float = 19.99

# 函数参数类型注解（必须）
fn greet(name: String) -> String {
    "Hello, " + name
}

# 类型注解用于解决歧义
fn process(x: Int | Float) {
    ...
}

process(42)           # OK，Int
process(3.14)         # OK，Float
```

### 3.6 类型约束

泛型函数可以使用类型约束来限制类型参数：

```yaoxiang
# 基本约束
fn add[T: Numeric](a: T, b: T) -> T {
    a + b
}

# 多重约束
fn print_summary[T: Printable & Serializable](obj: T) {
    print(obj.to_string())
    print(obj.to_json())
}

# 泛型约束语法
fn compare[T: Comparable](a: T, b: T) -> Int {
    if a < b { -1 }
    elif a > b { 1 }
    else { 0 }
}
```

---

## 第四章：表达式

### 4.1 表达式概述

表达式是计算值的代码片段。YaoXiang 的表达式包括字面量、变量、运算符表达式、函数调用、块表达式、条件表达式、模式匹配表达式等。

### 4.2 优先级与结合性

下表从高到低列出了运算符的优先级，同一行的运算符具有相同优先级。

| 优先级 | 运算符 | 描述 | 结合性 |
|--------|--------|------|--------|
| 1 | `()` `[]` `.` | 函数调用、索引、成员访问 | 左到右 |
| 2 | `as` | 类型转换 | 左到右 |
| 3 | `*` `/` `%` `*?` `/?` | 算术运算符 | 左到右 |
| 4 | `+` `-` | 加减法 | 左到右 |
| 5 | `<<` `>>` | 位移 | 左到右 |
| 6 | `&` `\|` `^` | 位运算 | 左到右 |
| 7 | `==` `!=` `<` `>` `<=` `>=` | 比较 | 左到右 |
| 8 | `not` | 逻辑非 | 右到左 |
| 9 | `and` `or` | 逻辑与、或 | 左到右 |
| 10 | `if...else` | 条件表达式 | 右到左 |
| 11 | `=` `+=` `-=` `*=` `/=` | 赋值 | 右到左 |

### 4.3 算术表达式

```yaoxiang
# 基本算术
a = 10 + 5          # 加法: 15
b = 10 - 3          # 减法: 7
c = 4 * 6           # 乘法: 24
d = 15 / 2          # 除法: 7.5（浮点除法）
e = 15 // 2         # 整除: 7
f = 15 % 4          # 取余: 3

# 算术赋值
mut x = 10
x += 5              # 等价于 x = x + 5
x -= 3
x *= 2
x /= 4

# 溢出检查（可选）
safe_add(a: Int, b: Int) -> option[Int] {
    result = a +? b    # ? 运算符表示溢出检查
    if result.is_ok() { some(result.unwrap()) }
    else { none }
}
```

### 4.4 比较表达式

```yaoxiang
# 比较运算符
a = 10
b = 20

equal = a == b           # false
not_equal = a != b       # true
less = a < b             # true
greater = a > b          # false
less_equal = a <= b      # true
greater_equal = a >= b   # false

# 链式比较
x = 5
result = 0 < x < 10      # true，等价于 0 < x and x < 10
```

### 4.5 逻辑表达式

```yaoxiang
# 布尔值
flag1 = true
flag2 = false

# 逻辑运算
and_result = flag1 and flag2   # false
or_result = flag1 or flag2     # true
not_result = not flag1         # false

# 短路求值
fn greet(name: option[String]) -> String {
    "Hello, " + (name or "World")   # 如果 name 是 none，使用 "World"
}
```

### 4.6 位运算

```yaoxiang
# 位运算
a = 0b1100
b = 0b1010

and_result = a & b          # 0b1000 (8)
or_result = a | b           # 0b1110 (14)
xor_result = a ^ b          # 0b0110 (6)
not_result = not a          # 按位取反

# 移位
left_shift = a << 2         # 0b110000 (48)
right_shift = a >> 1        # 0b0110 (6)
```

### 4.7 类型转换表达式

```yaoxiang
# 显式类型转换
x = 42 as Float             # Int -> Float
y = 3.14 as Int             # Float -> Int（截断）

# 类型检查与转换
value: type = ...
if value has_type Int {
    int_val = value as Int
}

# 安全转换
fn try_convert[T](value: T, target_type: type) -> option[type] {
    if value has_type target_type {
        some(value as target_type)
    } else {
        none
    }
}
```

### 4.8 块表达式

```yaoxiang
# 块表达式的值是最后一条语句的结果
result = {
    a = 5
    b = 10
    a + b                    # 块的值是 15
}

# 块中可以使用 return 提前返回
fn example(x: Int) -> Int {
    if x < 0 {
        return 0            # 提前返回
    }
    x * 2
}
```

### 4.9 条件表达式

```yaoxiang
# if 表达式
status = if code == 200 {
    "success"
} elif code == 404 {
    "not found"
} else {
    "error"
}

# if 作为语句
if x > 0 {
    print("positive")
} elif x < 0 {
    print("negative")
} else {
    print("zero")
}

# 三元表达式（if...else 的简写）
message = if age >= 18 { "adult" } else { "minor" }
```

### 4.10 模式匹配表达式

```yaoxiang
# 基本模式匹配
fn classify(x: Int | String) -> String {
    match x {
        0 -> "zero"
        1 -> "one"
        _ if x < 0 -> "negative number"
        _ -> "positive number or string"
    }
}

# 解构结构体
type Point = struct { x: Float, y: Float }

fn distance(p: Point) -> Float {
    match p {
        struct { x: 0, y: 0 } -> 0.0
        struct { x, y } -> (x * x + y * y).sqrt()
    }
}

# 解构联合类型
type Result[T, E] = union { ok: T, err: E }

fn handle_result[T](r: Result[T, String]) -> T {
    match r {
        ok: value -> value
        err: msg -> panic("Error: " + msg)
    }
}

# 匹配守卫
fn sign(x: Int) -> String {
    match x {
        n if n < 0 -> "negative"
        0 -> "zero"
        n if n > 0 -> "positive"
    }
}
```

### 4.11 函数调用表达式

```yaoxiang
# 基本函数调用
result = add(1, 2)          # 3

# 方法调用
list = [1, 2, 3]
length = list.length        # 3
reversed = list.reversed()  # [3, 2, 1]

# 链式调用
text = "  hello  "
trimmed = text.trim().uppercase()  # "HELLO"

# 命名参数
fn greet(name: String, formal: Bool) -> String {
    if formal { "Good day, " + name }
    else { "Hi, " + name }
}

greet(name: "Alice", formal: true)      # 使用命名参数
greet("Bob", false)                      # 位置参数
```

### 4.12 索引与切片

```yaoxiang
# 列表索引
numbers = [1, 2, 3, 4, 5]
first = numbers[0]         # 1
last = numbers[-1]         # 5（负索引从末尾计数）

# 字典索引
person = {"name": "Alice", "age": 30}
name = person["name"]      # "Alice"

# 切片
slice = numbers[1:3]       # [2, 3]
from_start = numbers[:3]   # [1, 2, 3]
to_end = numbers[2:]       # [3, 4, 5]
```

### 4.13 Lambda 表达式

```yaoxiang
# 匿名函数
double = fn(x: Int) -> Int { x * 2 }
result = double(5)         # 10

# 作为参数传递
numbers = [1, 2, 3, 4, 5]
doubled = numbers.map(fn(x: Int) -> Int { x * 2 })
evens = numbers.filter(fn(x: Int) -> Bool { x % 2 == 0 })

# 捕获外部变量
fn create_adder(n: Int) -> fn(Int) -> Int {
    fn(x: Int) -> Int {
        x + n               # 捕获 n
    }
}

add5 = create_adder(5)
result = add5(10)           # 15
```

---

## 第五章：语句

### 5.1 语句概述

语句是执行操作的代码片段。与表达式不同，语句不产生值（或者说其值通常被忽略）。YaoXiang 的语句包括变量声明、赋值语句、表达式语句、控制流语句等。

### 5.2 变量声明与赋值

```yaoxiang
# 自动推断声明
x = 42
name = "YaoXiang"

# 显式类型声明
count: Int = 100
price: Float = 19.99

# 不可变性
x = 10
x = 20              # 编译错误！

# 可变变量
mut counter = 0
counter = counter + 1   # OK

# 多重赋值
a, b = 1, 2
(first, second) = (a, b)

# 解构赋值
point = (3, 4)
x, y = point          # x = 3, y = 4

person = {"name": "Alice", "age": 30}
name, age = person    # 解构字典
```

### 5.3 表达式语句

任何表达式都可以作为语句使用，其值被忽略：

```yaoxiang
# 函数调用语句
print("Hello")

# 方法调用语句
numbers.append(6)

# 赋值语句
x = 10
y = f(a, b)
```

### 5.4 条件语句

```yaoxiang
# if 语句
if x > 0 {
    print("positive")
} elif x < 0 {
    print("negative")
} else {
    print("zero")
}

# 单行 if
if x > 0 { print("positive") }

# if 作为表达式
status = if code == 200 { "OK" } else { "Error" }
```

### 5.5 循环语句

**while 循环**：

```yaoxiang
mut i = 0
while i < 10 {
    print(i)
    i += 1
}
```

**for 循环**：

```yaoxiang
# 迭代列表
for item in [1, 2, 3] {
    print(item)
}

# 迭代范围
for i in 0..10 {
    print(i)           # 0 到 9
}

# 迭代字典
for key, value in {"a": 1, "b": 2} {
    print(key + ": " + value)
}

# 迭代字符串（字符）
for ch in "hello" {
    print(ch)
}
```

**loop 循环（无限循环）**：

```yaoxiang
loop {
    input = read_line()
    if input == "quit" {
        break
    }
    process(input)
}
```

### 5.6 循环控制

```yaoxiang
# break - 跳出循环
mut found = false
for item in items {
    if item == target {
        found = true
        break
    }
}

# continue - 跳过本次迭代
for i in 1..10 {
    if i % 2 == 0 {
        continue        # 跳过偶数
    }
    print(i)
}

# 带标签的 break
'outer: loop {
    for i in 0..10 {
        for j in 0..10 {
            if i * j > 50 {
                break 'outer  # 跳出外层循环
            }
        }
    }
}
```

### 5.7 return 语句

```yaoxiang
# 显式返回
fn add(a: Int, b: Int) -> Int {
    return a + b
}

# 隐式返回（块的最后表达式）
fn add(a: Int, b: Int) -> Int {
    a + b                 # 隐式返回
}

# 多重返回
fn divmod(a: Int, b: Int) -> (Int, Int) {
    (a / b, a % b)
}

# 提前返回
fn find_item(items: [Int], target: Int) -> option[Int] {
    for i in 0..items.length {
        if items[i] == target {
            return some(i)
        }
    }
    none
}
```

---

## 第六章：函数

### 6.1 函数定义

```yaoxiang
# 基本函数
fn greet(name: String) -> String {
    "Hello, " + name
}

# 返回类型推断
fn add(a: Int, b: Int) {
    a + b
}

# 无参数函数
fn get_time() -> Int {
    system_time()
}

# 无返回值函数
fn print_message(msg: String) {
    print(msg)
}

# 多返回值
fn divmod(a: Int, b: Int) -> (Int, Int) {
    (a / b, a % b)
}
```

### 6.2 参数

```yaoxiang
# 位置参数
fn add(a: Int, b: Int) -> Int {
    a + b
}

# 命名参数
fn create_point(x: Float, y: Float) -> Point {
    Point(x, y)
}

pt = create_point(x: 3.0, y: 4.0)

# 默认参数值
fn greet(name: String, formal: Bool = false) -> String {
    if formal { "Good day, " + name }
    else { "Hi, " + name }
}

greet("Alice")              # "Hi, Alice"
greet("Dr. Smith", true)    # "Good day, Dr. Smith"

# 可变参数
fn sum(numbers: [Int]) -> Int {
    total = 0
    for n in numbers {
        total += n
    }
    total
}

sum([1, 2, 3, 4, 5])        # 15

# 关键字参数收集
fn configure(options: {timeout: Int, debug: Bool}) {
    # options 是字典类型
    if options.debug {
        print("Debug mode")
    }
}

configure({timeout: 30, debug: true})
```

### 6.3 泛型函数

```yaoxiang
# 类型参数
fn identity[T](x: T) -> T {
    x
}

fn first[T](list: List[T]) -> option[T] {
    if list.length > 0 {
        some(list[0])
    } else {
        none
    }
}

# 多类型参数
fn pair[T, U](a: T, b: U) -> (T, U) {
    (a, b)
}

# 类型约束
fn max[T: Comparable](a: T, b: T) -> T {
    if a > b { a } else { b }
}

# where 子句约束
fn process_items[T](items: [T]) -> [T] where T: Printable {
    for item in items {
        print(item)
    }
    items
}
```

### 6.4 高阶函数

```yaoxiang
# 函数作为参数
fn apply[T, U](value: T, f: fn(T) -> U) -> U {
    f(value)
}

result = apply(5, fn(x: Int) -> Int { x * 2 })   # 10

# 函数作为返回值
fn create_multiplier(factor: Int) -> fn(Int) -> Int {
    fn(x: Int) -> Int {
        x * factor
    }
}

double = create_multiplier(2)
triple = create_multiplier(3)
double(5)     # 10
triple(5)     # 15

# 内联函数
inline fn square(x: Int) -> Int {
    x * x
}
```

### 6.5 闭包

```yaoxiang
# 捕获环境变量
fn create_counter() -> fn() -> Int {
    mut count = 0
    fn() -> Int {
        count += 1
        count
    }
}

counter1 = create_counter()
counter2 = create_counter()

counter1()     # 1
counter1()     # 2
counter2()     # 1（独立的计数器）

# 捕获多个变量
fn create_adder_and_subtracter(a: Int, b: Int) -> (fn() -> Int, fn() -> Int) {
    add_fn = fn() -> Int { a + b }
    sub_fn = fn() -> Int { a - b }
    (add_fn, sub_fn)
}
```

### 6.6 递归函数

```yaoxiang
# 直接递归
fn factorial(n: Int) -> Int {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

# 尾递归优化
fn factorial_tail(n: Int, acc: Int = 1) -> Int {
    if n <= 1 {
        acc
    } else {
        factorial_tail(n - 1, n * acc)
    }
}

# 相互递归
fn is_even(n: Uint) -> Bool {
    if n == 0 { true } else { is_odd(n - 1) }
}

fn is_odd(n: Uint) -> Bool {
    if n == 0 { false } else { is_even(n - 1) }
}
```

### 6.7 特殊函数

**spawn 函数**用于标记异步函数：

```yaoxiang
# 异步函数
fn fetch_data(url: String) -> JSON spawn {
    HTTP.get(url).json()
}

fn process_files(files: [String]) -> [Result] spawn {
    results = []
    for file in files {
        data = read_file(file)
        results.append(parse(data))
    }
    results
}
```

**内建函数**是语言内置的特殊函数：

```yaoxiang
# 类型操作
t = type_of(x)             # 获取值的类型
name = type_name(t)        # 获取类型名称
fields = type_fields(t)    # 获取类型字段

# 打印
print("Hello")             # 打印到 stdout
println("Hello")           # 打印并换行

# 强制转换
x = as_int(3.14)           # 转换为 Int
y = as_float("42")         # 转换为 Float

# 长度
len = [1, 2, 3].length     # 列表长度
```

---

## 第七章：模块系统

### 7.1 模块定义

```yaoxiang
# 基本模块
mod Math {
    pub fn sqrt(x: Float) -> Float {
        x.pow(0.5)
    }

    pub fn factorial(n: Int) -> Int {
        if n <= 1 { 1 }
        else { n * factorial(n - 1) }
    }

    pub pi = 3.14159

    # 私有函数和变量
    fn internal_helper() {
        ...
    }
}
```

### 7.2 模块导入

```yaoxiang
# 导入整个模块
use std.io

# 导入并重命名
use std.io as IO

# 导入特定项
use std.io.{ read_file, write_file, File }

# 导入特定项并重命名
use std.list.{ List as LinkedList }

# 重新导出
mod MyMath {
    use std.math
    pub use std.math.{ sin, cos }   # 重新导出
}
```

### 7.3 模块结构

```
myproject/
├── main.yx
├── lib.yx
├── math/
│   ├── mod.yx
│   ├── arithmetic.yx
│   └── geometry.yx
└── utils/
    ├── mod.yx
    └── string.yx
```

**主模块**（`main.yx`）：

```yaoxiang
# main.yx
use std.io
use math
use utils

fn main() {
    print("Hello, YaoXiang!")
    result = math.add(1, 2)
    print("Result: " + result)
}
```

**子模块**（`math/mod.yx`）：

```yaoxiang
# math/mod.yx
pub fn add(a: Int, b: Int) -> Int {
    a + b
}

pub fn sub(a: Int, b: Int) -> Int {
    a - b
}

use math.arithmetic
use math.geometry
```

### 7.4 可见性

```yaoxiang
# pub 关键字控制导出
mod MyModule {
    pub fn public_function() { ... }
    fn private_function() { ... }

    pub type PublicType = struct { ... }
    type PrivateType = struct { ... }
}
```

---

## 第八章：内存管理与所有权

### 8.1 所有权原则

YaoXiang 采用 Rust 风格的所有权模型，每个值有唯一的所有者。当所有者离开作用域时，值被自动销毁。这在编译期消除悬挂指针和数据竞争，同时不需要垃圾回收器。

```yaoxiang
# 基本所有权
fn take_ownership(data: Data) {
    # data 的所有权转移进来
    # 函数结束后 data 被销毁
}

fn borrow_data(data: ref Data) {
    # 只读借用
    # 不能修改 data
}

fn modify_data(data: mut Data) {
    # 可变借用
    # 可以修改 data
}
```

### 8.2 引用类型

```yaoxiang
# 不可变引用（默认）
fn process(data: ref Data) {
    # 可以读取 data
    print(data.field)
    # 不能修改 data.field
}

# 可变引用
fn mutate(data: mut Data) {
    data.field = new_value
}

# 返回引用
fn get_field(data: ref Data) -> ref Field {
    ref data.field
}
```

### 8.3 生命周期

```yaoxiang
# 显式生命周期（复杂情况）
fn longest<'a>(s1: &'a str, s2: &'a str) -> &'a str {
    if s1.length > s2.length { s1 } else { s2 }
}

# 自动生命周期推断
fn first[T](list: ref List[T]) -> ref T {
    ref list[0]
}
```

### 8.4 智能指针

```yaoxiang
# Box - 堆分配
heap_data: Box[List[Int]] = Box.new([1, 2, 3])

# Rc - 引用计数（单线程）
shared: Rc[Data] = Rc.new(data)
ref_count = shared.ref_count()

# Arc - 原子引用计数（线程安全）
thread_safe: Arc[Data] = Arc.new(data)

# RefCell - 内部可变性
internal_mut: RefCell[Data] = RefCell.new(data)
mutable_ref = internal_mut.borrow_mut()
```

### 8.5 RAII

```yaoxiang
# RAII 示例
fn with_file(path: String, f: fn(File) -> T) -> T {
    file = File.open(path)
    result = f(file)
    file.close()
    result
}

# 使用
content = with_file("test.txt", fn(file: File) -> String {
    file.read_all()
})
```

---

## 第九章：无感异步机制

### 9.1 spawn 标记函数

```yaoxiang
# 使用 spawn 标记异步函数
fn fetch_api(url: String) -> JSON spawn {
    response = HTTP.get(url)
    JSON.parse(response.body)
}

fn process_data(data: Bytes) -> Result spawn {
    parsed = parse_data(data)
    validated = validate(parsed)
    normalized = normalize(validated)
    normalized
}
```

### 9.2 自动等待

```yaoxiang
# 调用 spawn 函数自动等待结果
fn main() {
    # fetch_api 是异步的，但调用时自动等待
    user = fetch_api("https://api.example.com/user")
    posts = fetch_api("https://api.example.com/posts")

    # user 和 posts 在这里都已经就绪
    print(user.name)
    print(posts.length)
}
```

### 9.3 并发控制

```yaoxiang
# 并行执行多个异步任务
fn fetch_all(urls: [String]) -> [JSON] {
    tasks = urls.map(fn(url: String) -> JSON spawn {
        HTTP.get(url).json()
    })

    # 并行执行所有任务
    parallel(tasks)
}

# 等待所有
fn wait_all() {
    task1 = spawn compute1()
    task2 = spawn compute2()
    task3 = spawn compute3()

    results = await_all([task1, task2, task3])
}

# 等待任意一个
fn first_success() {
    task1 = spawn compute1()
    task2 = spawn compute2()
    task3 = spawn compute3()

    result = await_any([task1, task2, task3])
}
```

### 9.4 异步与同步混用

```yaoxiang
# 同步调用异步函数
fn sync_wrapper() -> String {
    # 这个函数本身是同步的
    # 调用 spawn 函数会自动等待
    result = fetch_data()  # 自动等待
    result
}

# 异步函数调用同步函数
async fn complex_operation() -> Result spawn {
    # 可以调用同步函数
    data = load_local_data()
    # 也可以调用其他 spawn 函数
    remote = fetch_remote_data()
    combine(data, remote)
}
```

---

## 第十章：标准库概览

### 10.1 核心模块

**`std.core`** 包含基本类型和操作：

```yaoxiang
use std.core

# 基本类型操作
x = 42.abs()           # 绝对值
y = 3.14.round()       # 四舍五入
z = "hello".length()   # 字符串长度
```

**`std.io`** 包含输入输出：

```yaoxiang
use std.io

# 打印
print("Hello")
println("World")

# 文件操作
content = read_file("test.txt")
write_file("output.txt", content)

# 标准输入
line = read_line()
```

**`std.list`** 包含列表操作：

```yaoxiang
use std.list

numbers = [1, 2, 3, 4, 5]
doubled = numbers.map(fn(x) { x * 2 })
filtered = numbers.filter(fn(x) { x > 2 })
sum = numbers.reduce(fn(acc, x) { acc + x }, 0)
```

**`std.dict`** 包含字典操作：

```yaoxiang
use std.dict

data = {"name": "Alice", "age": 30}
keys = data.keys()
values = data.values()
has_key = data.contains("name")
```

**`std.math`** 包含数学函数：

```yaoxiang
use std.math

pi = math.pi
sqrt4 = math.sqrt(4)
sin_pi_2 = math.sin(math.pi / 2)
random = math.random()
```

### 10.2 网络模块

```yaoxiang
use std.net

# HTTP 客户端
response = HTTP.get("https://api.example.com")
json = response.json()
status = response.status()

# HTTP 服务器
server = HTTP.server(8080)
server.route("/hello", fn(req) {
    "Hello, World!"
})
```

### 10.3 并发模块

```yaoxiang
use std.concurrent

# 线程
thread = spawn {
    for i in 0..1000 {
        print(i)
    }
}
thread.join()

# 通道
sender, receiver = channel[String]()
spawn {
    sender.send("message")
}
msg = receiver.recv()
```

---

## 第十一章：示例程序

### 11.1 Hello World

```yaoxiang
# hello.yx
use std.io

fn main() {
    println("Hello, YaoXiang!")
}
```

运行方式：`yaoxiang hello.yx`

输出：

```
Hello, YaoXiang!
```

### 11.2 计算器

```yaoxiang
# calculator.yx
use std.io

type Expr = union {
    number: Float
    add: struct { left: Expr, right: Expr }
    sub: struct { left: Expr, right: Expr }
    mul: struct { left: Expr, right: Expr }
    div: struct { left: Expr, right: Expr }
}

fn eval(e: Expr) -> Float {
    match e {
        number: n -> n
        add: { left, right } -> eval(left) + eval(right)
        sub: { left, right } -> eval(left) - eval(right)
        mul: { left, right } -> eval(left) * eval(right)
        div: { left, right } -> eval(left) / eval(right)
    }
}

fn main() {
    expr = add(
        number: 1,
        mul: {
            left: number: 2
            right: add(number: 3, number: 4)
        }
    )
    result = eval(expr)
    println("Result: " + result)
}
```

### 11.3 异步数据处理

```yaoxiang
# async_example.yx
use std.io
use std.net

type User = struct {
    id: Int
    name: String
    email: String
}

fn fetch_users() -> [User] spawn {
    response = HTTP.get("https://jsonplaceholder.typicode.com/users")
    users_data = response.json()
    users_data.map(fn(u: dict) -> User {
        User(
            id: u["id"],
            name: u["name"],
            email: u["email"]
        )
    })
}

fn fetch_posts(user_id: Int) -> [dict] spawn {
    url = "https://jsonplaceholder.typicode.com/posts?userId=" + user_id
    response = HTTP.get(url)
    response.json()
}

fn main() {
    # 获取所有用户
    users = fetch_users()
    println("Found " + users.length + " users")

    # 获取第一个用户的文章
    first_user = users[0]
    posts = fetch_posts(first_user.id)
    println(first_user.name + " has " + posts.length + " posts")

    # 获取所有用户的文章（并行）
    all_posts = parallel(
        users.map(fn(u: User) -> [dict] spawn {
            fetch_posts(u.id)
        })
    )
    total = all_posts.flatten().length
    println("Total posts: " + total)
}
```

### 11.4 类型系统示例

```yaoxiang
# types_example.yx
use std.io

# 类型即值
IntType = Int
FloatType = Float
ListType = List(Int)

# 类型组合
type Point3D = struct {
    x: Float
    y: Float
    z: Float
}

# 泛型类型
type Container[T] = struct {
    value: T
    label: String
}

# 依赖类型
type Vector[T, n: Nat] = struct {
    data: [T; n]
}

fn describe_type(t: type) -> String {
    match t {
        struct { fields } -> "Struct with fields: " + fields.join(", ")
        union { variants } -> "Union with variants: " + variants.join(" | ")
        list { element } -> "List[" + element.name + "]"
        fn { params, ret } -> "Function: (" + params.join(", ") + ") -> " + ret.name
        _ -> "Other type"
    }
}

fn main() {
    # 类型作为值
    println("Type of Int: " + IntType.name)
    println("Type of List(Int): " + ListType.name)

    # 类型反射
    point_type = type_of(Point3D)
    println(describe_type(point_type))

    # 运行时类型检查
    value: type = 42
    if value has_type Int {
        println("It's an integer!")
    }

    # 使用泛型
    int_container = Container(value: 42, label: "answer")
    float_container = Container(value: 3.14, label: "pi")
}
```

---

## 附录A：语法速查

### A.1 类型定义语法

```
TypeDef ::= 'type' Identifier '=' TypeExpr

TypeExpr ::= PrimitiveType
           | StructType
           | UnionType
           | EnumType
           | FnType
           | GenericType
           | TypeRef
           | TypeUnion
           | TypeIntersection

StructType ::= 'struct' '{' FieldList '}'
FieldList  ::= Field (',' Field)* ','?
Field      ::= Identifier ':' TypeExpr

UnionType  ::= 'union' '{' VariantList '}'
VariantList::= Variant (',' Variant)* ','?
Variant    ::= Identifier (':' TypeExpr)?

EnumType   ::= 'enum' '{' Identifier (',' Identifier)* ','? '}'

FnType     ::= 'fn' '(' ParamList? ')' '->' TypeExpr
ParamList  ::= Param (',' Param)*
Param      ::= Identifier ':' TypeExpr

GenericType::= Identifier '[' TypeArgList ']'
TypeArgList::= TypeExpr (',' TypeExpr)* ','?
```

### A.2 函数定义语法

```
FunctionDef ::= 'fn' Identifier Params? ('->' TypeExpr)? Block

Params      ::= '(' ParamList? ')'
ParamList   ::= Param (',' Param)*
Param       ::= Identifier (':' TypeExpr)?
```

### A.3 表达式语法

```
Expr ::= Literal
       | Identifier
       | Expr '(' ArgList? ')'         # 函数调用
       | Expr '.' Identifier           # 成员访问
       | Expr '[' Expr ']'             # 索引
       | Expr 'as' TypeExpr            # 类型转换
       | UnaryOp Expr
       | Expr BinaryOp Expr
       | 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
       | 'match' Expr '{' MatchCase+ '}'
       | 'fn' Params? ('->' TypeExpr)? Block   # 匿名函数
       | '(' Expr ')'                  # 分组
```

### A.4 语句语法

```
Stmt ::= SimpleStmt
       | CompoundStmt

SimpleStmt::= Identifier '=' Expr
            | Identifier ('.' Identifier | '[' Expr ']') '=' Expr
            | Expr                         # 表达式语句
            | 'return' Expr?
            | 'break' Identifier?
            | 'continue' Identifier?

CompoundStmt::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
              | 'match' Expr '{' MatchCase+ '}'
              | 'loop' Block
              | 'while' Expr Block
              | 'for' Pattern 'in' Expr Block
```

---

## 第十二章：类型系统增强

### 12.1 错误处理机制

YaoXiang 采用 `Result[T, E]` 类型进行错误处理，将成功值和错误值统一建模。

```yaoxiang
# Result 类型定义
type Result[T, E] = union {
    ok: T      # 成功情况，包含成功值
    err: E     # 错误情况，包含错误值
}

# 返回 Result 的函数
fn divide(a: Float, b: Float) -> Result[Float, String] {
    if b == 0.0 {
        err("Division by zero")
    } else {
        ok(a / b)
    }
}

# 使用 Result
result = divide(10.0, 2.0)
match result {
    ok: value -> print("Result: " + value)
    err: msg -> print("Error: " + msg)
}
```

**Result 组合子**提供便捷的错误处理方式：

```yaoxiang
# ? 运算符（错误传播）
fn process() -> Result[Int, String] {
    # 如果任何一个步骤返回 err，? 立即返回该错误
    a = read_number()?
    b = read_number()?
    c = divide(a, b)?
    ok(c * 2)
}

# map - 转换成功值
doubled = result.map(fn(x) { x * 2 })

# map_err - 转换错误值
mapped_err = result.map_err(fn(e) { "Error: " + e })

# and_then - 链式处理
fn validate(n: Int) -> Result[Int, String] {
    if n > 0 { ok(n) } else { err("Must be positive") }
}

chained = result.and_then(validate)

# or_else - 错误恢复
recovered = result.or_else(fn(e) { ok(default_value) })
```

### 12.2 Option 类型

`Option[T]` 用于表示可能不存在的值：

```yaoxiang
# Option 类型定义
type Option[T] = union {
    some: T   # 有值
    none      # 无值
}

# 返回 Option 的函数
fn find_user(id: Int) -> Option[User] {
    if id > 0 {
        some(User(id, "Alice"))
    } else {
        none
    }
}

# 使用 Option
user = find_user(123)
match user {
    some: u -> print("Found: " + u.name)
    none -> print("User not found")
}
```

**Option 组合子**：

```yaoxiang
# ? 运算符
name = find_user(123)?.name  # 如果 user 是 none，? 返回 none

# or_else - 提供默认值
name = user.or_else(fn() { get_default_user() })

# and_then - 链式处理
email = user.and_then(fn(u) { u.email })

# map - 转换值
upper_name = user.map(fn(u) { u.name.uppercase() })
```

### 12.3 泛型约束

泛型函数可以使用约束来限制类型参数的能力：

```yaoxiang
# 基本约束语法
fn process_items[T: Printable](items: [T]) {
    for item in items {
        print(item.to_string())
    }
}

# 多重约束
fn clone_and_print[T: Cloneable & Printable](item: T) {
    clone = item.clone()
    print(clone)
}

# where 子句（复杂约束）
fn complex_operation[T](items: [T], predicate: fn(T) -> Bool) -> T
where T: Comparable & Hashable {
    # ...
}
```

**内置约束类型**：

| 约束 | 说明 | 示例 |
|------|------|------|
| `Eq` | 支持 `==` 和 `!=` | `T: Eq` |
| `Ord` | 支持比较运算 | `T: Ord` |
| `Hashable` | 可计算哈希值 | `T: Hashable` |
| `Cloneable` | 可克隆 | `T: Cloneable` |
| `Printable` | 可转换为字符串 | `T: Printable` |
| `Addable` | 支持 `+` 运算 | `T: Addable` |
| `Subtractable` | 支持 `-` 运算 | `T: Subtractable` |
| `Multipliable` | 支持 `*` 运算 | `T: Multipliable` |
| `Divisible` | 支持 `/` 运算 | `T: Divisible` |

**自定义约束**：

```yaoxiang
# 定义约束类型
type Comparable = struct {
    cmp: fn(Self, Self) -> Int
}

# 实现约束
type Point = struct { x: Float, y: Float }

impl Point: Comparable {
    fn cmp(a: Point, b: Point) -> Int {
        distance_a = a.x * a.x + a.y * a.y
        distance_b = b.x * b.x + b.y * b.y
        if distance_a < distance_b { -1 }
        elif distance_a > distance_b { 1 }
        else { 0 }
    }
}
```

### 12.4 高级模式匹配

模式匹配是 YaoXiang 的核心特性，支持丰富的匹配模式。

**基本模式**：

```yaoxiang
# 字面量模式
match value {
    0 -> "zero"
    1 -> "one"
    true -> "true"
    "hello" -> "greeting"
}

# 标识符模式
match point {
    x -> print("Got: " + x)  # 匹配任意值，绑定到 x
}

# 通配符模式
match value {
    _ -> print("Something else")  # 匹配任意值
}
```

**结构体模式**：

```yaoxiang
type Point = struct { x: Float, y: Float }
type Rectangle = struct { width: Float, height: Float }

# 解构结构体
match shape {
    struct { x: 0, y: 0 } -> print("Origin")
    struct { x, y } -> print("Point at (" + x + ", " + y + ")")
}

# 嵌套解构
match window {
    struct {
        position: struct { x, y },
        size: struct { width, height }
    } -> print("Window at " + x + ", " + y)
}
```

**联合类型模式**：

```yaoxiang
type Result[T, E] = union { ok: T, err: E }
type Option[T] = union { some: T, none }

# 匹配联合类型
match result {
    ok: value -> print("Success: " + value)
    err: error -> print("Error: " + error)
}

# 匹配 Option
match maybe_value {
    some: v -> print("Value: " + v)
    none -> print("No value")
}
```

**模式守卫**：

```yaoxiang
match number {
    n if n < 0 -> "negative"
    0 -> "zero"
    n if n > 0 -> "positive"
}

match person {
    struct { age } if age >= 18 -> "adult"
    struct { age } -> "minor"
}
```

**或模式**：

```yaoxiang
match status {
    200 | 201 | 202 -> "Success"
    400 | 401 | 403 -> "Client error"
    500 | 503 | 504 -> "Server error"
    _ -> "Unknown"
}
```

**绑定模式**：

```yaoxiang
# 在模式中绑定新名称
match user {
    struct { name: "Alice" } -> print("Hello Alice!")  # 匹配特定值
    struct { name } -> print("Hello " + name)           # 绑定 name
}

# 嵌套绑定
match data {
    struct {
        user: struct { name },
        posts: [first_post, ...rest]
    } -> print("First post by " + name)
}
```

**范围模式**：

```yaoxiang
match age {
    0..18 -> "minor"
    19..60 -> "adult"
    61.. -> "senior"
}

match char {
    'a'..'z' -> "lowercase letter"
    'A'..'Z' -> "uppercase letter"
    '0'..'9' -> "digit"
    _ -> "other"
}
```

**类型模式**：

```yaoxiang
# 匹配值的类型
match value {
    _: Int -> "integer"
    _: Float -> "floating point"
    _: String -> "string"
    _: Bool -> "boolean"
    _ -> "unknown type"
}
```

### 12.5 类型强制与转换

```yaoxiang
# as 运算符 - 显式类型转换
int_val = 42 as Float          # Int -> Float
str_val = "123" as Int         # String -> Int（运行时检查）

# Try 转换 - 安全转换
fn try_parse_int(s: String) -> Int? {
    try {
        some(s as Int)  # 如果转换失败，返回 none
    } catch {
        _ -> none
    }
}

# 类型检查
if value has_type Int {
    int_value = value as Int
}

# 类型判断
t = type_of(value)
match t {
    Int -> "integer"
    Float -> "float"
    String -> "string"
    _ -> "other"
}
```

---

## 附录B：内置类型参考

### B.1 原类型

| 类型 | 描述 | 大小 | 取值范围 |
|------|------|------|----------|
| `Void` | 空值 | 0 字节 | `null` |
| `Bool` | 布尔值 | 1 字节 | `true`, `false` |
| `Int` | 有符号整数 | 8 字节 | 64 位 |
| `Int8` | 8 位整数 | 1 字节 | -128 ~ 127 |
| `Int16` | 16 位整数 | 2 字节 | -32768 ~ 32767 |
| `Int32` | 32 位整数 | 4 字节 | -21亿 ~ 21亿 |
| `Int64` | 64 位整数 | 8 字节 | 很大 |
| `Uint` | 无符号整数 | 8 字节 | 0 ~ 2^64-1 |
| `Float` | 浮点数 | 8 字节 | 双精度 |
| `Float32` | 单精度浮点 | 4 字节 | 单精度 |
| `String` | UTF-8 字符串 | 可变 | Unicode |
| `Char` | Unicode 字符 | 4 字节 | Unicode |

### B.2 泛型类型

| 类型 | 描述 |
|------|------|
| `List[T]` | 同质列表 |
| `Dict[K, V]` | 键值对映射 |
| `Set[T]` | 无重复集合 |
| `Option[T]` | 可空类型 |
| `Result[T, E]` | 结果类型 |

---

> 「爻象之学，通天人之理，明变化之道。」
>
> 本规范定义了 YaoXiang 编程语言的核心语法和语义。随着语言的发展，本规范将持续更新完善。
