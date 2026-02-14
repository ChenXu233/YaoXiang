---
title: 第七章：泛型的魔法
---

# 第七章：泛型的魔法

> 本章目标：理解泛型，学会用类型参数编写通用代码


## 7.1 问题的产生

假设我们要写一个"获取列表第一个元素"的函数：

```yaoxiang
# 整数列表
first_int: (list: List[Int]) -> Option[Int] = {
    # ...
}

# 字符串列表
first_string: (list: List[String]) -> Option[String] = {
    # ...
}

# 小数列表
first_float: (list: List[Float]) -> Option[Float] = {
    # ...
}
```

**问题**：每种类型都要写一个函数，太麻烦了！


## 7.2 解决方案：泛型

**泛型**，就是用**类型参数**写代码，一次编写，多种类型使用：

```yaoxiang
# 泛型函数：一个函数，适用于所有类型
first: [T](list: List[T]) -> Option[T] = {
    # T 是一个"类型参数"，调用时会被替换成具体类型
    if list.length > 0 {
        return Option.some(list[0])
    } else {
        return Option.none
    }
}
```

**使用**：

```yaoxiang
# 整数列表
int_list: List[Int] = List(1, 2, 3)
first_int: Option[Int] = first(int_list)           # Option.some(1)

# 字符串列表
str_list: List[String] = List("a", "b", "c")
first_str: Option[String] = first(str_list)         # Option.some("a")

# 小数列表
float_list: List[Float] = List(1.1, 2.2, 3.3)
first_float: Option[Float] = first(float_list)       # Option.some(1.1)
```

## 7.3 泛型的语法

```
[name: ] [泛型参数] (参数) -> 返回类型 = 实现

# 泛型参数：[T] 或 [T, U, ...]
```

```yaoxiang
# 单个泛型参数
map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = {
    # T：输入元素的类型
    # R：输出元素的类型
}

# 多个泛型参数
combine: [T, U](a: T, b: U) -> (T, U) = {
    return (a, b)
}

# 带约束的泛型参数
clone: [T: Clone](value: T) -> T = {
    # T 必须实现 Clone 接口
    return value.clone()
}
```


## 7.4 泛型类型

不仅是函数，类型也可以是泛型的：

```yaoxiang
# Option 类型（可能有值，可能没有）
Option: Type[T] = {
    some: (T) -> Self,
    none: () -> Self
}

# 使用 Option
maybe_number: Option[Int] = Option.some(42)
maybe_string: Option[String] = Option.none
```

```
Option 类型
┌─────────────────────────────────────────┐
│            Option[T]                      │
├─────────────────────────────────────────┤
│  ┌───────────────────────────────────┐  │
│  │  Option.some(value: T)            │  │
│  │  Option.none()                    │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```


## 7.5 List 类型

YaoXiang 的列表是泛型的：

```yaoxiang
# List 类型定义
List: Type[T] = {
    data: Array[T],      # 存储 T 类型的数组
    length: Int,        # 列表长度

    push: [T](self: List[T], item: T) -> Void,
    get: [T](self: List[T], index: Int) -> Option[T],
    map: [T, R](self: List[T], f: Fn(T) -> R) -> List[R],
    filter: [T](self: List[T], f: Fn(T) -> Bool) -> List[T],
}

# 使用
numbers: List[Int] = List(1, 2, 3, 4, 5)
numbers.push(6)           # 添加元素

# map：转换每个元素
doubled: List[Int] = numbers.map((x) => x * 2)  # 2, 4, 6, 8, 10, 12

# filter：筛选元素
evens: List[Int] = numbers.filter((x) => x % 2 == 0)  # 2, 4, 6
```


## 7.6 为什么需要泛型？

| 好处 | 说明 |
|------|------|
| **代码复用** | 写一次代码，适用于多种类型 |
| **类型安全** | 编译器检查类型是否正确 |
| **避免重复** | 不需要为每种类型写类似代码 |
| **抽象能力** | 写出通用的算法 |

**对比**：

```yaoxiang
# ❌ 没有泛型：每种类型写一个
first_int: (List[Int]) -> Option[Int] = ...
first_string: (List[String]) -> Option[String] = ...
first_float: (List[Float]) -> Option[Float] = ...
first_person: (List[Person]) -> Option[Person] = ...

# ✅ 有泛型：一个函数，所有类型
first: [T](List[T]) -> Option[T] = ...
```


## 7.7 类型推导

编译器可以自动推导出泛型参数：

```yaoxiang
# 完整写法
numbers: List[Int] = List[Int](1, 2, 3)

# 推导写法（推荐）
numbers = List(1, 2, 3)     # 编译器推导为 List[Int]
```


## 7.8 本章小结

| 概念 | 说明 | 例子 |
|------|------|------|
| 泛型 | 用类型参数编写通用代码 | `first: [T](List[T]) -> Option[T]` |
| 类型参数 | 泛型中的占位类型 | `T`、`R`、`U` |
| 泛型类型 | 带类型参数的模板类型 | `Option[T]`、`List[T]` |
| 泛型函数 | 带类型参数的函数 | `map: [T, R](...) -> ...` |


## 7.9 易经引言

> 「天地不仁，以万物为刍狗；圣人不仁，以百姓为刍狗。」
> —— 《道德经》
>
> 泛型之道，亦是如此：
> - **道**（算法）是通用的，不偏向任何类型
> - **器**（具体类型）在使用时才确定
>
> 一个 `first[T]`，可以取 Int 的第一个、String 的第一个、Person 的第一个...
> 这便是"有生于无，一生于多"的编程诠释。
>
> **泛型，是算法的"道"；类型，是具体的"器"。**
