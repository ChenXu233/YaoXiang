---
title: 快速开始
description: 在 5 分钟内安装并运行第一个 YaoXiang 程序
---

# YaoXiang 快速入门

本指南帮助您快速上手 YaoXiang 编程语言。

## 安装

### 从源码编译（推荐）

```bash
# 克隆仓库
git clone https://github.com/yaoxiang-lang/yaoxiang.git
cd yaoxiang

# 编译（调试版本，用于开发测试）
cargo build

# 编译（发布版本，推荐用于生产）
cargo build --release

# 运行测试
cargo test

# 查看版本
./target/debug/yaoxiang --version
```

**验证安装成功**：
```bash
./target/debug/yaoxiang --version
# 应输出类似: yaoxiang x.y.z
```

## 第一个程序

创建文件 `hello.yx`：

```yaoxiang
# hello.yx
use std.io

# 函数定义: name: (param: Type, ...) -> return_type = { ... }
main: () -> Void = {
    println("Hello, YaoXiang!")
}
```

运行：

```bash
./target/debug/yaoxiang hello.yx
# 或使用 release 版本
./target/release/yaoxiang hello.yx
```

输出：

```
Hello, YaoXiang!
```

## 基本概念

### 变量与类型

```yaoxiang
# 自动类型推断
x = 42                    # 推断为 Int
name = "YaoXiang"         # 推断为 String
pi = 3.14159              # 推断为 Float
is_valid = true           # 推断为 Bool

# 显式类型注解（推荐使用类型集中约定）
count: Int = 100

# 默认不可变（安全特性）
x = 10
x = 20                    # ❌ 编译错误！不可变

# 可变变量（需要显式声明）
mut counter = 0
counter = counter + 1     # ✅ OK
```

### 函数

```yaoxiang
# 函数定义语法
add: (a: Int, b: Int) -> Int = a + b

# 调用
result = add(1, 2)        # result = 3

# 单参数函数
inc: (x: Int) -> Int = x + 1
```

## 下一步

- 📖 阅读 [教程](/zh/tutorial/) 了解核心特性
- 📚 查看 [参考文档](/zh/reference/) 了解完整 API
- 💡 查看 [设计文档](/zh/design/) 了解核心理念
