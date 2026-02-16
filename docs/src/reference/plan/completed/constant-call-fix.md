# 常量调用问题修复

## 概述

修复 `std.math.PI` 等常量在使用时显示为 `unit` 的问题。

## 当前状态

- **问题**：使用 `PI` 时返回 `unit`，而不是预期的浮点数值
- **原因**：常量被当成无参函数调用，但 FFI handler 没有被正确执行

## 问题分析

当前代码中：

```rust
// FFI 注册
registry.register("std.math.PI", |_args| {
    Ok(RuntimeValue::Float(std::f64::consts::PI))
});
```

但常量调用（如 `PI`）可能被编译为不同的指令，而不是函数调用。

## 需要修改的模块

### 1. 编译器 - 代码生成

文件：`src/middle/passes/codegen/`

需要将常量引用（如 `PI`）正确识别为 native 函数调用，生成对应的 bytecode 指令。

### 2. 解释器/执行器

文件：`src/backends/interpreter/executor.rs`

确保常量引用能正确路由到 FFI handler。

## 实现方案

### 方案 A：在 translator 中注册常量名

```rust
// src/middle/passes/codegen/translator.rs
// 添加常量到 native_functions
native_functions.insert("std.math.PI".to_string());
native_functions.insert("std.math.E".to_string());
native_functions.insert("std.math.TAU".to_string());
```

### 方案 B：在 FFI 中使用特殊前缀

使用约定如 `__const__std.math.PI` 来区分常量和函数。

## 测试用例

```yaoxiang
use std.math.*

// 预期输出 3.14159...
println(PI)

// 预期输出 2.71828...
println(E)
```

## 相关文件

- `src/middle/passes/codegen/translator.rs` - 代码生成
- `src/backends/interpreter/executor.rs` - 解释器
- `src/backends/interpreter/ffi.rs` - FFI 注册
