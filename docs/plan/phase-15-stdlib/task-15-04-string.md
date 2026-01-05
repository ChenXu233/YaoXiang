# Task 15.4: 字符串处理

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

提供字符串操作、编码转换、正则匹配等功能。

## 字符串 API

```yaoxiang
# 字符串方法
s = "Hello, World!"
length = s.length
upper = s.to_uppercase()
lower = s.to_lowercase()
trimmed = s.trim()
reversed = s.reverse()

# 查找
pos = s.find("World")
contains = s.contains("Hello")
starts_with = s.starts_with("Hello")
ends_with = s.ends_with("!")

# 分割和连接
parts = s.split(", ")
joined = parts.join("-")

# 替换
new_s = s.replace("World", "YaoXiang")

# 切片
sub = s.substring(0, 5)  # "Hello"

# 正则匹配（可选）
matches = regex::find_all(r"\w+", s)
```

## 验收测试

```yaoxiang
# test_string.yx

s = "  Hello, World!  "

# 基础操作
assert(s.length == 15)
assert(s.trim() == "Hello, World!")
assert(s.to_uppercase() == "  HELLO, WORLD!  ")

# 查找
assert(s.find("World") == Some(9))
assert(s.contains("Hello"))

# 分割
parts = "a,b,c".split(",")
assert(parts == ["a", "b", "c"])

# 替换
s2 = "foo bar foo".replace("foo", "baz")
assert(s2 == "baz bar baz")

print("String tests passed!")
```

## 相关文件

- **string/mod.rs**
- **string/methods.rs**
