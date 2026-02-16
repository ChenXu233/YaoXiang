# 命名空间调用支持

## 概述

实现 `std.module.function` 形式的命名空间调用语法，使代码可以像 `std.io.print` 或 `std.math.abs` 这样调用模块函数。

## 当前状态

- **问题**：`use std.io.*` 可以导入短名称，但 `std.io.print` 形式的调用会报 "Unknown variable: 'std'" 错误
- **预期行为**：用户可以用 `std.<module>.<function>` 形式调用函数

## 需要修改的模块

### 1. 编译器前端 - 解析器

文件：`src/frontend/parser/`

需要支持识别 `a.b.c` 形式的名字表达式，并正确解析命名空间路径。

### 2. 编译器前端 - 类型检查

文件：`src/frontend/typecheck/`

当遇到命名空间路径时，需要：
1. 识别 `std` 作为内置命名空间
2. 解析后续的模块名（如 `io`, `math`, `net`）
3. 查找模块内的函数并验证类型

### 3. IR 生成

文件：`src/middle/passes/codegen/`

生成 IR 时，需要将命名空间路径转换为目标函数引用。

### 4. 解释器/运行时

文件：`src/backends/interpreter/executor.rs`

确保执行时能正确解析命名空间路径到 FFI handler。

## 实现步骤

1. **解析器修改**：识别 `a.b` 形式的成员访问表达式
2. **语义分析**：实现命名空间解析逻辑
3. **代码生成**：生成正确的函数调用指令
4. **测试**：添加测试用例验证 `std.io.print` 等调用

## 测试用例

```yaoxiang
use std.io

// 应该能工作
std.io.println("Hello")

// 短名称也应该能工作
use std.io.*
println("World")
```

## 相关文件

- `src/frontend/parser/` - 解析器
- `src/frontend/typecheck/` - 类型检查
- `src/middle/passes/codegen/` - 代码生成
- `src/backends/interpreter/` - 解释器
