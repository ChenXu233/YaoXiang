# 函数与闭包

> 版本：v2.0.0
> 状态：已更新（基于 RFC-010 统一类型语法 + RFC-004 位置绑定）

---

## 统一语法：name: type = value

YaoXiang 所有声明都使用统一语法：

```yaoxiang
# 变量
x: Int = 42

# 函数
add: (Int, Int) -> Int = (a, b) => a + b

# 类型方法
Point.distance: (Point, Point) -> Float = (p1, p2) => ...
```

---

## 函数定义

### 完整形式（推荐）

```yaoxiang
# 基本函数
greet: (String) -> String = (name) => "Hello, " + name

# 多参数函数
add: (Int, Int) -> Int = (a, b) => a + b

# 单参数简写
inc: Int -> Int = x => x + 1

# 多行函数
fact: (Int) -> Int = (n) => {
    if n == 0 { 1 } else { n * fact(n - 1) }
}
```

### 简写形式

```yaoxiang
# 简写形式
add(Int, Int) -> Int = (a, b) => a + b

greet(String) -> String = (name) => "Hello, " + name
```

---

## 泛型函数

```yaoxiang
# 泛型函数
identity: [T](T) -> T = (x) => x

# 使用
n: Int = identity(42)              # Int
s: String = identity("hello")       # String
b: Bool = identity(true)            # Bool

# 泛型高阶函数
map: [T, U]((T) -> U, List[T]) -> List[U] = (f, list) => {
    result: List[U] = List()
    for item in list {
        result.append(f(item))
    }
    result
}

# 使用
doubled: List[Int] = map((x) => x * 2, List([1, 2, 3]))  # [2, 4, 6]
```

---

## 高阶函数

### 接受函数作为参数

```yaoxiang
# 高阶函数
apply: ((Int) -> Int, Int) -> Int = (f, x) => f(x)

# 使用
double: (Int) -> Int = x => x * 2
result: Int = apply(double, 5)     # 10

# 简写
result2: Int = apply((x) => x + 1, 5)  # 6
```

### 返回函数

```yaoxiang
# 返回函数
create_multiplier: (Int) -> (Int) -> Int = (factor) => (x) => x * factor

# 使用
double: (Int) -> Int = create_multiplier(2)
triple: (Int) -> Int = create_multiplier(3)
result1: Int = double(5)           # 10
result2: Int = triple(5)           # 15
```

---

## 闭包

### 捕获外部变量

```yaoxiang
# 创建闭包
create_counter: () -> () -> Int = () => {
    mut count: Int = 0
    () => {
        count = count + 1
        count
    }
}

# 使用
counter: () -> Int = create_counter()
c1: Int = counter()                # 1
c2: Int = counter()                # 2
c3: Int = counter()                # 3
```

### 捕获多个变量

```yaoxiang
create_adder: (Int) -> (Int) -> Int = (base) => {
    add_to_base: (Int) -> Int = (x) => base + x
    add_to_base
}

add5: (Int) -> Int = create_adder(5)
result: Int = add5(10)             # 15
```

---

## 柯里化

YaoXiang 支持自动柯里化：

```yaoxiang
# 多参数函数可以部分应用
add: (Int, Int) -> Int = (a, b) => a + b

# 完全调用
result1: Int = add(3, 5)           # 8

# 部分应用
add5: (Int) -> Int = add(5)
result2: Int = add5(10)            # 15

# 链式部分应用
curried_add: (Int) -> (Int) -> Int = add
add3: (Int) -> Int = curried_add(3)
add5_more: Int = add3(5)           # 8
```

---

## 方法与绑定

### 类型方法定义

使用 `Type.method: (Type, ...) -> ReturnType = ...` 语法：

```yaoxiang
type Point = { x: Float, y: Float }

# 类型方法：第一个参数是 self（调用者）
Point.distance: (Point, Point) -> Float = (self, other) => {
    dx = self.x - other.x
    dy = self.y - other.y
    (dx * dx + dy * dy).sqrt()
}

# 使用
p1: Point = Point(3.0, 4.0)
p2: Point = Point(1.0, 1.0)
d: Float = p1.distance(p2)
```

### 位置绑定 [n]

将独立函数绑定到类型的特定参数位置：

```yaoxiang
type Point = { x: Float, y: Float }

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

### 指定绑定位置

```yaoxiang
# 函数签名是 transform(Vector, Point)
transform: (Vector, Point) -> Point = (v, p) => {
    Point(p.x + v.x, p.y + v.y)
}

# 绑定 Point.transform，将 this 绑定到第 1 位
Point.transform: (Point, Vector) -> Point = transform[1]

# 调用：p.transform(v) → transform(v, p)
result: Point = p1.transform(v1)
```

### 多位置绑定

```yaoxiang
type Point = { x: Float, y: Float }

# 函数接收多个 Point 参数
scale_points: (Point, Point, Float) -> Point = (p1, p2, factor) => {
    Point(p1.x * factor, p1.y * factor)
}

# 绑定多个位置（自动柯里化）
Point.scale: (Point, Point, Float) -> Point = scale_points[0, 1]

# 调用
p1.scale(p2)(2.0)  # → scale_points(p1, p2, 2.0)
```

### 占位符 _

跳过某些位置：

```yaoxiang
# 只绑定第 1 参数，保留第 0、2 参数
Point.custom_op: (Point, Point, Float) -> Float = func[1, _]

# 调用：p1.custom_op(p2, 0.5) → func(p1, p2, 0.5)
```

### 自动绑定 pub

使用 `pub` 声明的函数**自动绑定**到同文件定义的类型：

```yaoxiang
type Point = { x: Float, y: Float }

# 使用 pub 声明，编译器自动绑定到 Point
pub distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# 编译器自动推断：Point.distance = distance[0]

# 现在可以这样调用：
d1: Float = distance(p1, p2)      # 函数式
d2: Float = p1.distance(p2)       # OOP 语法糖
```

### 自动绑定规则

| 函数声明 | 自动绑定结果 |
|---------|-------------|
| `pub distance: (Point, Point) -> Float = ...` | `Point.distance = distance[0]` |
| `pub draw: (Point, Surface) -> Void = ...` | `Point.draw = draw[0]` |
| `pub transform: (Vector, Point) -> Point = ...` | 需要手动指定位置 |

---

## 内置方法

### 字符串函数

```yaoxiang
len: Int = "hello".length          # 5
upper: String = "hello".to_upper()    # "HELLO"
lower: String = "HELLO".to_lower()    # "hello"
```

### 列表函数

```yaoxiang
numbers: List[Int] = List([1, 2, 3, 4, 5])

length: Int = numbers.length       # 5
first: Int = numbers[0]            # 1
last: Int = numbers[-1]            # 5
reversed: List[Int] = numbers.reversed() # [5, 4, 3, 2, 1]
```

---

## 递归函数

```yaoxiang
# 阶乘
fact: (Int) -> Int = (n) => {
    if n <= 1 { 1 } else { n * fact(n - 1) }
}

# 斐波那契
fib: (Int) -> Int = (n) => {
    if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
}

# 列表求和
sum_list: (List[Int]) -> Int = (list) => {
    if list.length == 0 { 0 } else { list[0] + sum_list(list.tail()) }
}
```

---

## 下一步

- [控制流](control-flow.md) - 条件、循环和模式匹配
- [错误处理](error-handling.md) - Result 和 Option
- [泛型编程](generics.md) - 更复杂的泛型模式
