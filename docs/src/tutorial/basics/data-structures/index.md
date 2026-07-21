---
title: 列表和字典
---

# 列表和字典

数据结构是程序的骨架。YaoXiang 提供了两种内置集合类型：列表和字典。

## 列表

列表是一个**有序**的值的序列，所有元素类型相同。用 `[]` 创建：

```yaoxiang
// 创建列表
numbers = [1, 2, 3, 4, 5]
names = ["Alice", "Bob", "Charlie"]
empty: List(Int) = []       // 空列表需要类型注解
```

### 索引访问

用 `[]` 按位置访问元素，索引从 0 开始：

```yaoxiang
scores = [95, 87, 73, 91]

first = scores[0]    // 95
second = scores[1]   // 87
last = scores[3]     // 91
```

### 常用操作

```yaoxiang
mut items = [1, 2, 3]

// 添加元素
items.append(4)       // [1, 2, 3, 4]

// 长度
count = items.len()   // 4

// 切片
slice = items[0..2]   // [1, 2]
```

### 列表推导式

列表推导式是创建列表的强大工具——从已有列表生成新列表：

```yaoxiang
// 基本推导式
squares = [x * x for x in [1, 2, 3, 4, 5]]
print(squares)  // [1, 4, 9, 16, 25]

// 带过滤条件的推导式
evens = [x for x in [1, 2, 3, 4, 5, 6] if x % 2 == 0]
print(evens)  // [2, 4, 6]

// 转换类型
names = ["Alice", "Bob", "Charlie"]
lengths = [n.len() for n in names]
print(lengths)  // [5, 3, 7]
```

语法：`[表达式 for 变量 in 列表 if 条件]`——`if 条件` 部分是可选的。

## 字典

字典是**键值对**的集合，键是字符串，值可以是任意类型。用 `{}` 创建：

```yaoxiang
// 创建字典
scores = {"Alice": 90, "Bob": 85, "Charlie": 92}
empty: Dict(Int) = {}          // 空字典需要类型注解
```

### 键访问

用 `[]` 通过键访问值：

```yaoxiang
scores = {"Alice": 90, "Bob": 85}

alice = scores["Alice"]   // 90
bob = scores["Bob"]       // 85
```

### 修改字典

```yaoxiang
mut data = {"name": "Alice"}

// 添加/更新键值
data["age"] = 25
data["name"] = "Bob"

print(data)  // {"name": "Bob", "age": 25}
```

### 成员检测

用 `in` 检查键是否存在：

```yaoxiang
config = {"host": "localhost", "port": "8080"}

has_host = "host" in config    // true
has_user = "user" in config    // false
```


## 小结

| 类型 | 语法 | 有序？ | 可重复？ | 键类型 |
|------|------|--------|----------|--------|
| 列表 | `[1, 2, 3]` | ✅ | ✅ | 整数索引 |
| 字典 | `{"a": 1}` | ✅ | 键不重复 | String |

列表是你的主力容器，字典适合键值查找。
