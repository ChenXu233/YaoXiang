# 快速入门

> 版本：v1.0.0
> 状态：编写中

---

## 安装 YaoXiang

```bash
cargo install yaoxiang
```

## 第一个程序

创建文件 `hello.yx`：

```yaoxiang
main: () -> Void = () => {
    print("Hello, YaoXiang!")
}
```

运行：

```bash
yaoxiang hello.yx
```

---

## 变量与类型

### 基本类型

```yaoxiang
# 整数
age: Int = 25

# 浮点数
price: Float = 19.99

# 字符串
name: String = "YaoXiang"

# 布尔值
is_active: Bool = true

# 字符
grade: Char = 'A'
```

### 类型推断

YaoXiang 支持类型推断，可以省略类型注解：

```yaoxiang
x = 42              # 推断为 Int
y = 3.14            # 推断为 Float
z = "hello"         # 推断为 String
```

### 可变变量

默认变量不可变，使用 `mut` 声明可变变量：

```yaoxiang
count: Int = 0
count = 1            # ❌ 错误：不可变

mut counter = 0
counter = counter + 1  # ✅ 正确：可变
```

---

## 运算符

### 算术运算符

```yaoxiang
a = 10 + 5          # 加法：15
b = 10 - 3          # 减法：7
c = 4 * 6           # 乘法：24
d = 15 / 2          # 除法：7.5
e = 15 // 2         # 整除：7
f = 15 % 4          # 取模：3
```

### 比较运算符

```yaoxiang
equal = a == b      # 等于
not_equal = a != b  # 不等于
less = a < b        # 小于
greater = a > b     # 大于
less_equal = a <= b # 小于等于
greater_equal = a >= b  # 大于等于
```

### 逻辑运算符

```yaoxiang
and_result = true and false   # false
or_result = true or false     # true
not_result = not true         # false
```

### 位运算符

```yaoxiang
and_result = 5 & 3            # 1 (0101 & 0011)
or_result = 5 | 3             # 7 (0101 | 0011)
xor_result = 5 ^ 3            # 6 (0101 ^ 0011)
not_result = not 5            # -6
left_shift = 5 << 1           # 10
right_shift = 5 >> 1          # 2
```

---

## 集合类型

### 列表

```yaoxiang
numbers: List[Int] = [1, 2, 3, 4, 5]
empty_list: List[String] = []

# 访问元素
first = numbers[0]             # 1
last = numbers[-1]             # 5
```

```yaoxiang
# 修改列表
mut nums = [1, 2, 3]
nums.append(4)                 # [1, 2, 3, 4]
nums.remove(0)                 # [2, 3, 4]
```

### 字典

```yaoxiang
scores = {"Alice": 95, "Bob": 87}

# 访问
alice_score = scores["Alice"]  # 95

# 修改
scores["Charlie"] = 92
scores["Bob"] = 90
```

### 元组

```yaoxiang
point = (3.0, 4.0)
coordinate = (x: Int, y: Int, z: Int) = (1, 2, 3)

# 解构
(x, y) = point
```

---

## 注释

```yaoxiang
# 单行注释

#! 多行注释
   可以跨越多行
   第二行 !#
```

---

## 下一步

- [类型系统](types.md) - 深入理解类型系统
- [函数与闭包](functions.md) - 学习函数定义
- [控制流](control-flow.md) - 条件与循环
