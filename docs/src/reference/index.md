# YaoXiang 参考文档

> 本文档正在建设中...

YaoXiang 目前处于 **实验验证阶段**，标准库和 API 正在逐步完善。

## 当前状态

| 模块 | 状态 | 描述 |
|------|------|------|
| `std.io` | 🔨 施工中 | 输入输出 |
| `std.string` | 🔨 施工中 | 字符串操作 |
| `std.list` | 🔨 施工中 | 列表操作 |
| `std.dict` | 📋 计划中 | 字典操作 |
| `std.math` | 🔨 施工中 | 数学函数 |
| `std.net` | 📋 计划中 | 网络操作 |
| `std.concurrent` | 📋 计划中 | 并发原语 |

## 内置类型

### 原始类型

| 类型 | 描述 | 示例 |
|------|------|------|
| `Void` | 空值/无返回值 | `()` |
| `Bool` | 布尔值 | `true`, `false` |
| `Int` | 整数 | `42`, `-10` |
| `Float` | 浮点数 | `3.14`, `-0.5` |
| `Char` | 字符 | `'a'`, `'中'` |
| `String` | 字符串 | `"hello"` |

### 复合类型

| 类型 | 描述 | 示例 |
|------|------|------|
| `List[T]` | 同类元素列表 | `[1, 2, 3]` |
| `Tuple(T1, T2, ...)` | 异类元素元组 | `(1, "hello")` |
| `Dict[K, V]` | 键值对映射 | `{"a": 1}` |
| `Fn(Args) -> Ret` | 函数类型 | `(Int) -> Int` |

### 用户定义类型

```yaoxiang
# 记录类型（结构体）
type Point = { x: Float, y: Float }

# 枚举类型
type Result[T, E] = ok(T) | err(E)

# 接口类型（所有字段为函数）
type Callable = { call: (String) -> Void }
```

## 内置函数

### 输出

```yaoxiang
print(value)           # 打印，无换行
println(value)         # 打印，有换行
```

### 转换

```yaoxiang
to_string(value)       # 转换为字符串
to_int(value)          # 转换为整数
to_float(value)        # 转换为浮点数
```

### 类型检查

```yaoxiang
typeof(value)         # 返回类型名称
is_type(value, type)  # 检查类型
```

## 关键字

| 关键字 | 描述 |
|--------|------|
| `type` | 定义类型 |
| `spawn` | 标记并作函数 |
| `spawn for` | 并行循环 |
| `spawn {}` | 并作块 |
| `if` / `elif` / `else` | 条件分支 |
| `match` | 模式匹配 |
| `while` / `for` | 循环 |
| `return` | 返回值 |
| `ref` | 创建引用 |
| `mut` | 可变标记 |

## 语法速查

### 变量声明

```yaoxiang
# 不可变变量（默认）
x: Int = 42
y = 42                 # 类型推断

# 可变变量
mut count: Int = 0
count = count + 1
```

### 函数定义

```yaoxiang
# 普通函数
add: (a: Int, b: Int) -> Int = a + b

# 并作函数（自动并发）
fetch: (url: String) -> JSON spawn = HTTP.get(url).json()

# 泛型函数
identity: [T](x: T) -> T = x
```

### 控制流

```yaoxiang
# 条件
if x > 0 {
    println("positive")
} elif x < 0 {
    println("negative")
} else {
    println("zero")
}

# 模式匹配
match result {
    ok(value) => println("success: " + value),
    err(error) => println("error: " + error),
}

# 循环
for i in 0..10 {
    print(i)
}
```

### 错误处理

```yaoxiang
# ? 运算符传播错误
data = fetch_file(path)?
```

## 运算符优先级

| 优先级 | 运算符 |
|--------|--------|
| 最高 | `( )` 函数调用 |
| | `.` 字段访问 |
| | `[ ]` 索引 |
| | `unary -` 一元负号 |
| | `* / %` 乘除取模 |
| | `+ -` 加减 |
| | `== != < > <= >=` 比较 |
| | `and or` 逻辑运算 |
| 最低 | `=` 赋值 |

## 标准库使用示例

```yaoxiang
# 导入标准库
from std.io import print, println

# 列表操作
from std.list import list_push, list_pop, list_len

# 数学函数
from std.math import sqrt, sin, cos, PI

# 使用
println("Hello, YaoXiang!")
result = sqrt(16.0)  # 4.0
```

## 命令行工具

```bash
# 运行脚本
yaoxiang run hello.yx

# 构建字节码
yaoxiang build hello.yx -o hello.42

# 解释执行
yaoxiang eval 'println("Hello")'

# 查看帮助
yaoxiang --help
```

## 完整示例

```yaoxiang
# 计算斐波那契数列
fib: (n: Int) -> Int = if n <= 1 {
    n
} else {
    fib(n - 1) + fib(n - 2)
}

# 主函数
main: () -> Void = {
    println("Fibonacci(10) = " + fib(10).to_string())
}
```

## 相关资源

- [教程](../tutorial/) - 学习 YaoXiang
- [设计文档](../design/) - 语言设计决策
- [GitHub](https://github.com/ChenXu233/YaoXiang)

## 贡献指南

标准库正在建设中，欢迎贡献！

1. 选择一个模块（如 `std.io`, `std.net`）
2. 在 `src/std/` 中实现函数
3. 添加文档注释
4. 提交 PR
