# 类型系统

> 版本：v1.0.0
> 状态：编写中

---

## 类型分类

YaoXiang 的类型分为以下几类：

```
TypeExpr
├── 原类型 (Primitive)
├── 结构体类型 (Struct)
├── 枚举类型 (Enum)
├── 元组类型 (Tuple)
├── 函数类型 (Fn)
├── 泛型类型 (Generic)
└── 复合类型 (Union/Intersection)
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

## 结构体类型

使用构造器语法定义结构体：

```yaoxiang
# 定义
type Point = Point(x: Float, y: Float)

# 构造值
p1 = Point(3.0, 4.0)
p2 = Point(x: 1.0, y: 2.0)

# 访问成员
x_coord = p1.x              # 3.0
y_coord = p1.y              # 4.0
```

### 嵌套结构体

```yaoxiang
type Rectangle = Rectangle(width: Float, height: Float)
type Circle = Circle(radius: Float)
type Shape = Shape Rectangle | Circle

# 使用
rect = Rectangle(10.0, 20.0)
circle = Circle(5.0)
```

---

## 枚举类型

使用 `|` 定义枚举变体：

```yaoxiang
# 简单枚举
type Color = red | green | blue

# 带值的变体
type Result[T, E] = ok(T) | err(E)
type Option[T] = some(T) | none

# 使用
success = ok(42)
failure = err("not found")
value = some(100)
empty = none
```

### 模式匹配

```yaoxiang
type Status = pending | processing | completed | failed(String)

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

## 元组类型

```yaoxiang
# 定义
Point2D = (Float, Float)
Triple = (Int, String, Bool)

# 使用
p = (3.0, 4.0)
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
type List[T] = { elements: [T], length: Int }
type Map[K, V] = { keys: [K], values: [V] }
type Result[T, E] = ok(T) | err(E)

# 使用
numbers: List[Int] = [1, 2, 3]
names: List[String] = ["Alice", "Bob"]
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
type Printable = { print: () -> Void }
type Serializable = { to_json: () -> String }

# 交集类型
type Document = Printable & Serializable

# 实现
type MyDoc = MyDoc(content: String) & {
    print: () -> Void = () => print(self.content)
    to_json: () -> String = () => '{"content": "' + self.content + '"}'
}
```

---

## 类型转换

```yaoxiang
# 使用 as
int_to_float = 42 as Float        # 42.0
float_to_int = 3.14 as Int        # 3
string_to_int = "123" as Int      # 123
```

---

## 下一步

- [函数与闭包](functions.md) - 学习函数定义和高阶函数
- [控制流](control-flow.md) - 条件与循环
- [错误处理](error-handling.md) - Result 和 Option
