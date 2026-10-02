---
title: '列表和字典'
---

# 列表和字典

数据结构是程序的骨架。YaoXiang 的集合类型分层构建：`Vec(T)` 是运行时长度的原始缓冲，`List(T)` 是标准库类型（`{ data: Vec(T), length: Int }`），字典基于同样的分层机制。用户也可以定义自己的容器类型——实现 `Index` 接口（RFC-011b）后同样支持 `[]` 下标访问。

## 列表

列表是一个**有序**的值的序列，所有元素类型相同。用 `[]` 创建：

```yaoxiang
// 创建列表
numbers = [1, 2, 3, 4, 5]
names = ["Alice", "Bob", "Charlie"]

// 注意：`empty: List(Int) = []` 报 E1002——空列表的 `List(T)` 标注与
// 字面量推断出的 `Vec` 不兼容。空列表请交给 list.push 之类的函数构造。
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
use std.list

main = () => {
    mut items = [1, 2, 3]

    // 添加元素
    items = list.push(items, 4)   // [1, 2, 3, 4]

    // 长度
    count = list.len(items)   // 4

    // 切片
    // 注意：范围切片 items[0..2] 当前运行时报 E6007（实测），请用 std.list.slice
    sl = list.slice(items, 0, 2)   // [1, 2]
    print(items)
    print(count)
    print(sl)
}
```

### 列表推导式

列表推导式是创建列表的强大工具——从已有列表生成新列表：

```yaoxiang
use std.string

main = () => {
    // 基本推导式
    squares = [x * x for x in [1, 2, 3, 4, 5]]
    print(squares)  // [1, 4, 9, 16, 25]

    // 转换类型
    names = ["Alice", "Bob", "Charlie"]
    // 注意：列表推导式当前依赖 std.list.iter，运行时可能报 E6006
    lengths = [string.len(n) for n in names]
    print(lengths)  // [5, 3, 7]
}
```

语法：`[表达式 for 变量 in 列表]`。

⚠️ 0.8.2 的推导式**不支持 `if 条件` 后缀**——`[x for x in xs if x % 2 == 0]` 解析报
「Expected RBracket, found KwIf」(`E0010`)。带过滤请改用 `std.list.filter`：

```yaoxiang
use std.list

main = () => {
    evens = list.filter([1, 2, 3, 4, 5, 6], (x) => x % 2 == 0)
    print(evens)  // [2, 4, 6]
}
```

## 字典

字典是**键值对**的集合，键是字符串，值可以是任意类型。用 `{}` 创建：

```yaoxiang
use std.dict

main = () => {
    // 创建字典
    scores = {"Alice": 90, "Bob": 85, "Charlie": 92}
    // 注意：`{}` 会被解析为空块（E1108），空字典必须用 dict.new()
    empty = dict.new()
    print(scores)
    print(empty)
}
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
use std.dict

main = () => {
    // 添加/更新键值
    // 字典下标赋值可用（0.8.2 实测：`data["k"] = v` 写入后可正常读出），
    // 请用 dict.set——它返回新字典，需要重新绑定。
    data = dict.set({"name": "Alice"}, "age", 25)

    print(data["name"])  // Alice
    print(data["age"])   // 25
}
```

### 成员检测

用 `in` 检查键是否存在：

```yaoxiang
config = {"host": "localhost", "port": "8080"}

has_host = "host" in config    // true
has_user = "user" in config    // false
```

## 小结

| 类型 | 语法        | 有序？ | 可重复？ | 键类型   |
| ---- | ----------- | ------ | -------- | -------- |
| 列表 | `[1, 2, 3]` | ✅     | ✅       | 整数索引 |
| 字典 | `{"a": 1}`  | ✅     | 键不重复 | String   |

列表是你的主力容器，字典适合键值查找。
