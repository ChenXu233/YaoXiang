# Task 15.3: IO 操作

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

提供文件 IO、控制台 IO、缓冲区 IO 等功能。

## IO 模块

```yaoxiang
# 文件 IO
use std::io

# 读取文件
content = io::read_file("test.txt")

# 写入文件
io::write_file("output.txt", "hello")

# 追加写入
io::append_file("log.txt", "new line\n")

# 读取行
lines = io::read_lines("input.txt")

# 控制台 IO
io::print("Hello")
io::println("World")
io::read_line()  # 读取一行输入

# 格式化输出
io::format("Value: {}", 42)
```

## 验收测试

```yaoxiang
# test_io.yx

use std::io

# 写入测试文件
io::write_file("test.txt", "hello\nworld")

# 读取文件
content = io::read_file("test.txt")
assert(content.contains("hello"))

# 读取行
lines = io::read_lines("test.txt")
assert(lines.length == 2)

# 清理
io::delete_file("test.txt")

print("IO tests passed!")
```

## 相关文件

- **io/mod.rs**
- **io/file.rs**
- **io/console.rs**
