# Native 函数签名解析与闭包调用问题修复计划

> **状态**：待处理
> **日期**：2026-02-19

---

## 概述

### 问题背景

当前 YaoXiang 语言在使用高阶函数（如 `list.map`、`list.filter`、`list.reduce`）时存在两个相关问题：

1. **签名解析错误**：`src/std/list.rs` 中 `map`/`filter`/`reduce` 函数的签名字符串使用了无效的类型 `Fn`
2. **错误信息误导**：当签名解析失败时，错误信息显示 "Invalid signature 'Float': missing '->'"，而非更合理的错误提示

### 问题代码

`src/std/list.rs` 第 72-87 行：

```rust
NativeExport::new(
    "map",
    "std.list.map",
    "(list: List, fn: Fn) -> List",  // ❌ Fn 是无效类型（应为 (T) -> T）
    native_map as NativeHandler,
),
NativeExport::new(
    "filter",
    "std.list.filter",
    "(list: List, fn: Fn) -> List",   // ❌ Fn 是无效类型
    native_filter as NativeHandler,
),
NativeExport::new(
    "reduce",
    "std.list.reduce",
    "(list: List, fn: Fn, init: Any) -> Any",  // ❌ Fn 是无效类型
    native_reduce as NativeHandler,
),
```

**当前签名格式**（正确）：
```
(list: List, fn: Fn) -> List
  ↑       ↑     ↑
  │       │     └── 参数类型（无效）
  │       └── 参数名
  └── 参数类型
```

**问题**：`fn` 是参数名，`Fn` 是类型名。`Fn` 不是有效的基础类型名。

### 运行时错误

执行测试代码时：

```yaoxiang
main = {
    doubled = list.map([1, 2, 3], x => x * 2);
    io.println(doubled);
}
```

输出：

```
[Warning] Invalid signature 'Float': missing '->'
[Warning] Invalid signature 'Float': missing '->'
[Warning] Invalid signature 'Float': missing '->'
Error: Runtime error: Type error: Expected function value
```

---

## 实现目标

### 目标 1：修复签名定义

根据 RFC-010 统一类型语法，泛型函数的正确格式为：

```
函数名: [泛型参数列表](参数列表) -> 返回类型
```

其中泛型参数 `[T]` 声明在函数级别，作用于整个函数签名。

将 `map`/`filter`/`reduce` 的签名修改为：

```rust
// map: 泛型 [T] 作用域为整个函数
"(list: List<T>, fn: (item: T) -> T) -> List<T>"

// filter: 泛型 [T] 作用域为整个函数
"(list: List<T>, fn: (item: T) -> Bool) -> List<T>"

// reduce: 泛型 [T] 作用域为整个函数
"(list: List<T>, fn: (acc: Any, item: T) -> Any, init: Any) -> Any"
```

**注意**：泛型参数 `[T]` 需要在调用点根据传入的 List 元素类型进行实例化。

**签名结构说明**：

```
(list: List<T>, fn: (item: T) -> T) -> List<T>
 │         │      │    │        │
 │         │      │    │        └── 返回类型（使用 T）
 │         │      │    └── 参数类型（使用 T）
 │         │      └── 参数名
 │         └── 参数类型（函数类型）
 └── 参数类型（List 泛型，使用 T）
```

### 目标 2：泛型参数作用域规则

**泛型参数声明禁止遮蔽**（No Shadowing）：

1. **同级禁止遮蔽**：同一作用域内的泛型参数不能同名
2. **跨级禁止遮蔽**：函数参数名不能与泛型参数同名
3. **继承禁止遮蔽**：子作用域不能声明与父作用域同名的泛型参数

**有效示例**：

```yaoxiang
// ✅ 有效：泛型参数 T 作用域为整个函数
map: [T](list: List[T], fn: (item: T) -> T) -> List[T]

// ✅ 有效：多泛型参数
zip: [T, U](a: List[T], b: List[U]) -> List<(T, U)>

// ✅ 有效：函数参数名与泛型参数不同
foo: [T](x: Int, y: T) -> T
```

**无效示例**：

```yaoxiang
// ❌ 无效：泛型参数同名（同级禁止遮蔽）
bad1: [T, T](x: T, y: T) -> T
# 错误：Duplicate generic parameter 'T'

// ❌ 无效：函数参数名与泛型参数同名（跨级禁止遮蔽）
bad2: [T](T: Int) -> T
# 错误：Parameter 'T' shadows generic parameter 'T'

// ❌ 无效：外部参数遮蔽泛型
bad3: [T](T: Int, list: List<T>) -> Int
# 错误：Parameter 'T' shadows generic parameter 'T'
```

### 目标 3：签名参数名检查

解析签名时，应验证参数名的合法性：
1. 参数名不能是关键字（如 `fn`, `let`, `if` 等）
2. 参数名必须是有效的标识符（字母开头，可包含数字和下划线）
3. 参数名不能重复

如果参数名无效，应报错而非忽略。

示例：
```
// 有效签名（符合 RFC-010）
"(list: List<T>, fn: (item: T) -> T) -> List<T>"

// 无效签名 - 函数参数名与泛型参数同名（禁止遮蔽）
"(list: List<T>, fn: (T: T) -> T) -> List<T>"
# 应报错: Parameter 'T' shadows generic parameter 'T'

// 无效签名 - 重复参数名
"(x: Int, x: Int) -> Int"
# 应报错: Invalid signature: duplicate parameter name 'x'

// 无效签名 - fn 是关键字（当 fn 作为普通参数名时）
"(list: List, if: (Int) -> Int) -> List"
# 应报错: Invalid signature: 'if' is a reserved keyword
```

### 目标 4：修复错误信息

当签名解析遇到未知类型时，应报错 "Unknown type: xxx"，而非当前误导性的 "Float: missing '->'"。

预期错误信息：

```
[Error] Invalid signature '(list: List, fn: Fn) -> List': unknown type 'Fn'
```

---

## 验收方案

### 验收条件 1：编译通过

修改签名后，测试代码应能通过编译，不再出现 "Invalid signature" 警告：

```bash
$ cargo run -- run tests/closure_test2.yx
# 应输出：
# [Test map:]
# [2, 4, 6]
# [Test filter:]
# [3, 4, 5]
# [Test reduce:]
# 10
# [All tests passed!]
```

### 验收条件 2：错误信息正确

当使用无效签名时，应显示正确的错误信息：

```bash
# 测试无效签名
# 预期输出：
[Error] Invalid signature '(list: List, fn: Fn) -> List': unknown type 'Fn'
```

### 验收条件 3：Lambda 参数名匹配

传入的 lambda 参数名必须与签名中定义的函数参数名一致：

```yaoxiang
// 签名定义：fn: (item: T) -> T
// 传入 lambda：x => x * 2

// ❌ 错误 - 参数名不匹配
list.map([1, 2, 3], x => x * 2)
# 预期错误：Parameter name mismatch: expected 'item', got 'x'

// ✅ 正确 - 参数名匹配
list.map([1, 2, 3], item => item * 2)

// 对于 reduce，签名是 fn: (acc: Any, item: T) -> Any
list.reduce([1, 2, 3], (accumulator, item) => accumulator + item, 0)
# ✅ 正确 - 参数名匹配
```

### 验收条件 4：参数名检查（签名解析）

签名解析时应检测无效的参数名：
- 关键字作为参数名应报错
- 重复参数名应报错

```bash
# 预期错误：
[Error] Invalid signature: parameter 'fn' is a reserved keyword
[Error] Invalid signature: duplicate parameter name 'x'
```

### 验收条件 5：高阶函数功能正常

- `list.map` 对列表每个元素应用函数，返回新列表
- `list.filter` 保留满足条件的元素，返回新列表
- `list.reduce` 对元素进行累积计算

---

## 测试方案

### 测试用例 1：基本 map 功能

```yaoxiang
main = {
    // 签名定义: fn: (item: T) -> T，参数名为 item
    doubled = list.map([1, 2, 3], item => item * 2);
    io.println(doubled);  // 预期: [2, 4, 6]
}
```

### 测试用例 2：基本 filter 功能

```yaoxiang
main = {
    // 签名定义: fn: (item: T) -> Bool，参数名为 item
    filtered = list.filter([1, 2, 3, 4, 5], item => item > 2);
    io.println(filtered);  // 预期: [3, 4, 5]
}
```

### 测试用例 3：基本 reduce 功能

```yaoxiang
main = {
    // 签名定义: fn: (acc: Any, item: T) -> Any，参数名为 acc, item
    sum = list.reduce([1, 2, 3, 4], (acc, item) => acc + item, 0);
    io.println(sum);  // 预期: 10
}
```

### 测试用例 4：Lambda 参数名不匹配

```yaoxiang
main = {
    // ❌ 错误 - 参数名不匹配
    // 签名: fn: (item: T) -> T，但传入的参数名是 x
    doubled = list.map([1, 2, 3], x => x * 2);
}
# 预期编译错误：Parameter name mismatch: expected 'item', got 'x'
```

### 测试用例 5：复杂 lambda

```yaoxiang
main = {
    // 多参数 lambda
    result = list.reduce([1, 2, 3], (acc, x) => acc * x, 1);
    io.println(result);  // 预期: 6

    // 嵌套调用
    data = list.map([1, 2, 3], x => {
        y = x + 1;
        y * 2
    });
    io.println(data);  // 预期: [4, 6, 8]
}
```

### 测试用例 6：无效签名错误提示

验证错误信息是否正确显示（需要添加测试或手动验证）。

### 测试用例 7：参数名是关键字

```rust
// 假设有这样一个无效签名
// "(list: List, if: (Int) -> Int) -> List"
// 预期编译错误：
// [Error] Invalid signature: parameter 'if' is a reserved keyword
```

### 测试用例 8：重复参数名

```rust
// 假设有这样一个无效签名
// "(x: Int, x: Int) -> Int"
// 预期编译错误：
// [Error] Invalid signature: duplicate parameter name 'x'
```

---

## 技术细节

### 相关代码文件

| 文件 | 作用 |
|------|------|
| `src/std/list.rs` | Native 函数导出定义（需修改签名） |
| `src/frontend/typecheck/mod.rs` | 签名解析逻辑（parse_signature 函数） |
| `src/backends/interpreter/executor.rs` | 运行时闭包调用 |

### 签名解析流程

1. `TypeCheckResult::new()` 调用 `register_std_native_signatures()`
2. `register_std_native_signatures()` 遍历 std 模块的导出
3. 对每个 `Export`，调用 `parse_signature(&export.signature, env)`
4. `parse_signature` 解析签名字符串为 `MonoType::Fn`
5. 解析失败时打印警告并返回默认类型

### 需要修改的代码

1. **`src/std/list.rs:72-87`**：修改三个函数的签名字符串（RFC-010 泛型函数语法）
   ```rust
   "(list: List<T>, fn: (item: T) -> T) -> List<T>"
   "(list: List<T>, fn: (item: T) -> Bool) -> List<T>"
   "(list: List<T>, fn: (acc: Any, item: T) -> Any, init: Any) -> Any"
   ```

   **注意**：这里的 `<T>` 是 List 的类型参数，函数参数 `fn` 的类型 `(item: T) -> T` 使用了相同的 `T`。编译器需要根据传入的 List 元素类型自动实例化泛型。

2. **`src/frontend/typecheck/mod.rs`**：
   - 改进错误信息：显示实际解析失败的原因
   - 添加参数名检查：检测关键字冲突和重复参数名

---

## 依赖关系

本任务不依赖其他任务，可独立完成。

---

## 风险与注意事项

1. **泛型支持**：需要确认类型系统是否支持泛型 `List<T>` 的解析
2. **闭包环境捕获**：当前实现不处理闭包捕获外部变量，需确认 map/filter/reduce 用例不需要此功能
