# 函数与闭包

> 版本：v1.0.0
> 状态：编写中

---

## 函数定义

### 形式一：类型集中式（推荐）

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

### 形式二：简写式

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
n = identity(42)              # Int
s = identity("hello")         # String
b = identity(true)            # Bool

# 泛型高阶函数
map: [T, U]((T) -> U, [T]) -> [U] = (f, list) => {
    result: [U] = []
    for item in list {
        result.append(f(item))
    }
    result
}

# 使用
doubled = map((x) => x * 2, [1, 2, 3])  # [2, 4, 6]
```

---

## 高阶函数

### 接受函数作为参数

```yaoxiang
# 高阶函数
apply: ((Int) -> Int, Int) -> Int = (f, x) => f(x)

# 使用
double: (Int) -> Int = x => x * 2
result = apply(double, 5)     # 10

# 简写
result2 = apply((x) => x + 1, 5)  # 6
```

### 返回函数

```yaoxiang
# 返回函数
create_multiplier: (Int) -> (Int) -> Int = (factor) => (x) => x * factor

# 使用
double = create_multiplier(2)
triple = create_multiplier(3)
result1 = double(5)           # 10
result2 = triple(5)           # 15
```

---

## 闭包

### 捕获外部变量

```yaoxiang
# 创建闭包
create_counter() -> () -> Int = () => {
    mut count = 0
    () => {
        count = count + 1
        count
    }
}

# 使用
counter = create_counter()
c1 = counter()                # 1
c2 = counter()                # 2
c3 = counter()                # 3
```

### 捕获多个变量

```yaoxiang
create_adder(base: Int) -> (Int) -> Int = (base) => {
    add_to_base: (Int) -> Int = (x) => base + x
    add_to_base
}

add5 = create_adder(5)
result = add5(10)             # 15
```

---

## 柯里化

YaoXiang 支持自动柯里化：

```yaoxiang
# 多参数函数可以部分应用
add: (Int, Int) -> Int = (a, b) => a + b

# 完全调用
result1 = add(3, 5)           # 8

# 部分应用
add5: (Int) -> Int = add(5)
result2 = add5(10)            # 15

# 链式部分应用
curried_add: (Int) -> (Int) -> Int = add
add3 = curried_add(3)
add5_more = add3(5)           # 8
```

---

## 方法绑定

### 位置绑定

```yaoxiang
type MathOps = MathOps(add: (Int, Int) -> Int, mul: (Int, Int) -> Int)

ops = MathOps(
    add: (a, b) => a + b,
    mul: (a, b) => a * b
)

# 使用
sum = ops.add(3, 5)           # 8
product = ops.mul(3, 5)       # 15
```

---

## 内置函数

### 字符串函数

```yaoxiang
len = "hello".length          # 5
upper = "hello".to_upper()    # "HELLO"
lower = "HELLO".to_lower()    # "hello"
```

### 列表函数

```yaoxiang
numbers = [1, 2, 3, 4, 5]

length = numbers.length       # 5
first = numbers[0]            # 1
last = numbers[-1]            # 5
reversed = numbers.reversed() # [5, 4, 3, 2, 1]
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
sum_list: ([Int]) -> Int = (list) => {
    if list.length == 0 { 0 } else { list[0] + sum_list(list[1..]) }
}
```

---

## 下一步

- [控制流](control-flow.md) - 条件与循环
- [错误处理](error-handling.md) - Result 和 Option
- [泛型编程](generics.md) - 更复杂的泛型模式
