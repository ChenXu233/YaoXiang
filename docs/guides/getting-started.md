# YaoXiang 快速入门

> 本指南帮助您快速上手 YaoXiang 编程语言。

## 安装

### 从源码编译

```bash
# 克隆仓库
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang

# 编译
cargo build --release

# 运行
./target/release/yaoxiang your_program.yx
```

## 第一个程序

创建文件 `hello.yx`：

```yaoxiang
# hello.yx
use std.io

main() -> Void = () => {
    println("Hello, YaoXiang!")
}
```

运行：

```bash
yaoxiang hello.yx
```

输出：

```
Hello, YaoXiang!
```

## 基本概念

### 变量与类型

```yaoxiang
# 自动类型推断
x = 42                    # Int
name = "YaoXiang"         # String
pi = 3.14159              # Float
is_valid = true           # Bool

# 显式类型注解
count: Int = 100

# 不可变（默认）
x = 10
x = 20                    # 编译错误！

# 可变变量
mut counter = 0
counter = counter + 1     # OK
```

### 函数

```yaoxiang
add(Int, Int) -> Int = (a, b) => a + b

# 调用
result = add(1, 2)        # result = 3
```

### 类型定义

```yaoxiang
type Point = struct {
    x: Float
    y: Float
}

# 使用
p = Point(x: 3.0, y: 4.0)
```

### 控制流

```yaoxiang
# 条件
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
```

### 列表推导式

使用 `in` 关键字可以创建简洁的列表推导式：

```yaoxiang
# 基本列表推导式
evens = [x * 2 for x in 0..10]          # [0, 4, 8, 12, 16]

# 带条件的列表推导式
squares = [x * x for x in 1..10 if x % 2 == 1]  # [1, 9, 25, 49, 81]
```

### 成员检测

使用 `in` 关键字可以检测值是否存在于集合中：

```yaoxiang
# 成员检测
if x in [1, 2, 3] {
    print("x is in the list")
}

# 与条件表达式结合
result = if name in ["Alice", "Bob"] { "known" } else { "unknown" }
```

## 下一步

- 阅读 [YaoXiang 指南](../YaoXiang-book.md) 了解核心特性
- 查看 [语言规范](../YaoXiang-language-specification.md) 了解完整语法
- 浏览 [示例代码](../examples/) 学习常用模式
