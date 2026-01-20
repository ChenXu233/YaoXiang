# 类型系统

> 版本：v2.0.0
> 状态：已更新（基于 RFC-010 统一类型语法）

---

## 类型分类

YaoXiang 的类型分为以下几类：

```
TypeExpr
├── 原类型 (Primitive)
├── 记录类型 (Record)
├── 枚举类型 (Enum)
├── 接口类型 (Interface)
├── 元组类型 (Tuple)
├── 函数类型 (Fn)
├── 泛型类型 (Generic)
└── 复合类型 (Union/Intersection)
```

---

## 统一语法：name: type = value

YaoXiang 采用**极简统一的类型语法**：`name: type = value`

```
├── 变量/函数：name: type = value
│   ├── x: Int = 42
│   └── add: (Int, Int) -> Int = (a, b) => a + b
│
├── 类型定义：type Name = type_expression
│   ├── type Point = { x: Float, y: Float }
│   └── type Drawable = { draw: (Surface) -> Void }
│
└── 接口：字段全为函数类型的记录类型
    └── type Serializable = { serialize: () -> String }
```

---

## 原类型

| 类型 | 描述 | 默认大小 | 示例 |
|------|------|----------|------|
| `Void` | 空值 | 0 字节 | `null` |
| `Bool` | 布尔值 | 1 字节 | `true`, `false` |
| `Int` | 有符号整数 | 8 字节 | `42`, `-10` |
| `Uint` | 无符号整数 | 8 字节 | `100u` |
| `Float` | 浮点数 | 8 字节 | `3.14`, `-0.5` |
| `String` | UTF-8 字符串 | 可变 | `"hello"` |
| `Char` | Unicode 字符 | 4 字节 | `'a'`, `'中'` |
| `Bytes` | 原始字节 | 可变 | `b"\x00\x01"` |

### 带位宽的整数

```yaoxiang
x: Int8 = 127
y: Int16 = 32000
z: Int32 = 100000
w: Int64 = 10000000000
u: Uint8 = 255
```

---

## 记录类型（Struct）

使用花括号 `{}` 定义记录类型：

```yaoxiang
# 定义
type Point = { x: Float, y: Float }

# 构造值
p1: Point = Point(3.0, 4.0)
p2: Point = Point(x: 1.0, y: 2.0)

# 访问成员
x_coord: Float = p1.x              # 3.0
y_coord: Float = p1.y              # 4.0
```

### 嵌套记录类型

```yaoxiang
type Rectangle = { width: Float, height: Float }
type Circle = { radius: Float }

# 使用
rect: Rectangle = Rectangle(10.0, 20.0)
circle: Circle = Circle(5.0)
```

---

## 枚举类型

使用 `|` 定义枚举变体：

```yaoxiang
# 简单枚举
type Color = { red | green | blue }

# 带值的变体
type Result[T, E] = { ok(T) | err(E) }
type Option[T] = { some(T) | none }

# 使用
success: Result[Int, String] = ok(42)
failure: Result[Int, String] = err("not found")
value: Option[Int] = some(100)
empty: Option[Int] = none
```

### 模式匹配

```yaoxiang
type Status = { pending | processing | completed | failed(String) }

process: Status -> String = (status) => {
    match status {
        pending => "等待中",
        processing => "处理中",
        completed => "完成",
        failed(msg) => "失败: " + msg,
    }
}
```

---

## 接口类型

**接口 = 字段全为函数类型的记录类型**

```yaoxiang
# 接口定义
type Drawable = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

type Serializable = {
    serialize: () -> String
}
```

### 类型实现接口

在类型定义中列出接口名：

```yaoxiang
type Point = {
    x: Float,
    y: Float,
    Drawable,      # 实现 Drawable 接口
    Serializable   # 实现 Serializable 接口
}

# 实现接口方法
Point.draw: (Point, Surface) -> Void = (self, surface) => {
    surface.plot(self.x, self.y)
}

Point.serialize: (Point) -> String = (self) => {
    "Point(${self.x}, ${self.y})"
}
```

### 空接口

```yaoxiang
type EmptyInterface = {}
```

---

## 方法与绑定

### 类型方法

使用 `Type.method: (Type, ...) -> ReturnType = ...` 语法：

```yaoxiang
# 类型方法：第一个参数是 self
Point.draw: (Point, Surface) -> Void = (self, surface) => {
    surface.plot(self.x, self.y)
}

Point.serialize: (Point) -> String = (self) => {
    "Point(${self.x}, ${self.y})"
}
```

### 位置绑定 [n]

将函数绑定到类型的特定参数位置：

```yaoxiang
# 定义独立函数
distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# 绑定到 Point.distance（this 绑定到第 0 位）
Point.distance: (Point, Point) -> Float = distance[0]

# 调用：函数式
d1: Float = distance(p1, p2)

# 调用：OOP 语法糖
d2: Float = p1.distance(p2)
```

### 多位置绑定

```yaoxiang
# 函数接收多个 Point 参数
transform_points: (Point, Point, Float) -> Point = (p1, p2, factor) => {
    Point(p1.x * factor, p1.y * factor)
}

# 绑定多个位置（自动柯里化）
Point.transform: (Point, Point, Float) -> Point = transform_points[0, 1]

# 调用
p1.transform(p2)(2.0)  # → transform_points(p1, p2, 2.0)
```

### 自动绑定 pub

使用 `pub` 声明的函数自动绑定到同文件定义的类型：

```yaoxiang
pub draw: (Point, Surface) -> Void = (self, surface) => {
    surface.plot(self.x, self.y)
}

# 编译器自动推断 Point.draw = draw[0]
```

---

## 元组类型

```yaoxiang
# 定义
Point2D = (Float, Float)
Triple = (Int, String, Bool)

# 使用
p: (Float, Float) = (3.0, 4.0)
(x, y) = p
```

---

## 函数类型

```yaoxiang
# 定义函数类型
type Adder = (Int, Int) -> Int
type Callback[T] = (T) -> Void
type Predicate[T] = (T) -> Bool

# 使用
add: (Int, Int) -> Int = (a, b) => a + b
```

---

## 泛型类型

```yaoxiang
# 定义泛型
type List[T] = {
    data: Array[T],
    length: Int
}
type Map[K, V] = { keys: [K], values: [V] }
type Result[T, E] = { ok(T) | err(E) }

# 使用
numbers: List[Int] = List([1, 2, 3])
names: List[String] = List(["Alice", "Bob"])
```

### 泛型函数

```yaoxiang
# 泛型函数
identity: [T](T) -> T = (x) => x

# 使用
n: Int = identity(42)
s: String = identity("hello")
```

---

## 类型联合

```yaoxiang
# 使用 |
type Number = Int | Float
type Text = String | Char

# 使用
n1: Number = 42
n2: Number = 3.14

# 模式匹配
print_number: Number -> String = (n) => {
    match n {
        i: Int => "整数: " + i.to_string(),
        f: Float => "浮点数: " + f.to_string(),
    }
}
```

---

## 类型交集

```yaoxiang
# 交集类型 = 类型组合
type Printable = { print: () -> Void }
type Serializable = { to_json: () -> String }

# 交集类型
type Document = Printable & Serializable

# 实现交集类型
type MyDoc = { content: String } & {
    print: () -> Void = () => print(self.content)
    to_json: () -> String = () => '{"content": "' + self.content + '"}'
}
```

---

## 类型转换

```yaoxiang
# 使用 as
int_to_float: Float = 42 as Float
float_to_int: Int = 3.14 as Int
string_to_int: Int = "123" as Int
```

---

## 类型推导

YaoXiang 支持类型推导，可以省略类型注解：

```yaoxiang
x = 42              # 推断为 Int
y = 3.14            # 推断为 Float
z = "hello"         # 推断为 String
p = Point(1.0, 2.0) # 推断为 Point
```

---

## 下一步

- [函数与闭包](functions.md) - 学习函数定义和高阶函数
- [控制流](control-flow.md) - 条件与循环
- [错误处理](error-handling.md) - Result 和 Option
