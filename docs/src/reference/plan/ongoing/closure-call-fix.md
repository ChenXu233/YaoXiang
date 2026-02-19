# 块内函数定义与闭包相关问题

> **状态**: ✅ 已完成
>
> **创建日期**: 2026-02-19
> **完成日期**: 2026-02-19

## 一、问题总结

### 1.1 问题 1：块内函数定义找不到变量

**症状**：
```yao
main = {
  add = (a, b) => a + b;      // 无类型注解的函数定义
  result = add(1, 2);         // ❌ Unknown variable: 'add'
}
```

**根因**：`src/frontend/typecheck/checking/mod.rs` 第 548-563 行

```rust
// 只有当有类型注解时才添加函数到作用域！
if let Some(crate::frontend::core::parser::ast::Type::Fn { ... }) = type_annotation {
    // ... 构建函数类型
    self.add_var(name.to_string(), PolyType::mono(fn_type));  // ❌ 没有执行
}
```

当使用最简形式 `add = (a, b) => ...` 时，`type_annotation = None`，函数名根本不会被添加到作用域。

### 1.2 问题 2：模块级函数能正常工作

模块级函数（如 `main = { ... }`）能正常工作是因为它们走的是不同的代码路径：

```
check_module
  → collect_function_signature (第379-382行)
    → 为没有类型注解的函数也添加类型变量
```

这表明类型推断的基础设施是存在的，只是 `check_fn_stmt` 没有正确使用它。

### 1.3 问题 3：use std.{io} 字段访问错误

**症状**：
```yao
use std.{io}
add: (a: Int, b: Int) -> Int = (a, b) => a + b;
main = {
  result = add(1, 2);
  io.println(result);  // ❌ Cannot access field on non-struct type 'fn(t113) -> void'
}
```

**相关但不同的问题**：`io` 被识别为函数类型而不是模块。

### 1.4 函数定义的四种形式测试结果

| 形式 | 代码 | 模块级 | 块内部 |
|------|------|--------|--------|
| 完整形式 | `add: (a: Int, b: Int) -> Int = (a, b) => a + b` | ✅ | ✅ |
| 简写（省略Lambda头） | `add: (a: Int, b: Int) -> Int = { return a + b }` | ✅ | ✅ |
| 简写（省略参数类型） | `add: (a, b) -> Int = (a, b) => { return a + b }` | ✅ | ❌ |
| 最简形式 | `add = (a, b) => { return a + b }` | ✅ | ❌ |

---

## 二、修复方案

### 2.1 问题 1 修复：块内函数定义

**状态**：✅ 已修复

修复分两部分：

#### 2.1.1 类型检查修复

修改了 `src/frontend/typecheck/checking/mod.rs` 的 `check_fn_stmt` 函数（第 546-583 行）：
- 无论是否有类型注解，都添加函数到作用域
- 如果有类型注解，使用注解的类型
- 否则从参数创建类型变量

#### 2.1.2 IR 生成修复

修改了 `src/middle/core/ir_gen.rs`：
1. 添加了 `nested_functions` 字段来存储嵌套函数（第 152 行）
2. 修改 `generate_local_stmt_ir` 来生成嵌套函数的 IR（第 1013-1032 行）
3. 修改 `generate_module_ir` 来将嵌套函数添加到模块函数列表（第 416-417 行）

**验证结果**：
- ✅ 编译阶段通过（不再报 `Unknown variable` 错误）
- ✅ 运行时正常执行

### 2.3 问题 3 修复：use std.{io}

这需要单独调查，可能是：
1. `use` 语句解析后没有正确设置模块类型
2. 字段访问检查时没有正确处理模块

---

## 三、验收标准

### 3.1 编译验收

- [x] `cargo check` 通过
- [x] 块内完整形式函数调用正常（编译阶段）
- [x] 块内最简形式函数调用正常（编译阶段）

### 3.2 功能验收

- [x] `main = { add = (a,b) => a + b; add(1,2) }` 正常执行
- [x] `main = { add: (a:Int,b:Int)->Int = (a,b)=>a+b; add(1,2) }` 正常执行

### 3.3 当前状态

| 阶段 | 模块级函数 | 块内函数 |
|------|-----------|----------|
| 词法/语法分析 | ✅ | ✅ |
| 类型检查 | ✅ | ✅ 已修复 |
| 代码生成 | ✅ | ✅ 已修复 |

## 四、待处理问题

### 4.1 use std.{io} 字段访问错误

**状态**：✅ 已修复

**修复内容**：
修改了 `src/frontend/typecheck/mod.rs` 的 `collect_use_statement` 函数（第 645-678 行）：
- 为子模块创建包含导出函数的 StructType，而不是错误的 Fn 类型
- 从模块注册表中获取子模块的导出信息

**验证结果**：
```yao
use std.{io}
main = {
  add = (a, b) => a + b;
  result = add(100, 200);
  io.println(result)  // ✅ 正常工作
}
```

---

## 四、测试用例

### 4.1 块内函数定义测试

```yao
// test_block_fn.yx
main = {
  // 完整形式
  add1: (a: Int, b: Int) -> Int = (a, b) => a + b;

  // 最简形式（无类型注解）
  add2 = (a, b) => a + b;

  result1 = add1(1, 2);
  result2 = add2(3, 4);
  result1 + result2  // 返回 10
}
```

---

## 五、相关文件

| 文件 | 行号 | 描述 |
|------|------|------|
| `src/frontend/typecheck/checking/mod.rs` | 548-584 | `check_fn_stmt` 函数 |
| `src/frontend/typecheck/mod.rs` | 379-382 | `collect_function_signature` |
| `src/frontend/typecheck/mod.rs` | 546-590 | 模块级函数签名收集逻辑 |
