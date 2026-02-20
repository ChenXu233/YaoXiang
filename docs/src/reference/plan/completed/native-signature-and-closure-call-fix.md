# Native 函数签名解析与闭包调用问题修复计划

> **状态**：✅ 已完成
> **日期**：2026-02-19
> **完成日期**：2026-02-19

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

输出（当前行为 - 错误）：

```
[Warning] Invalid signature 'Float': missing '->'
[Warning] Invalid signature 'Float': missing '->'
[Warning] Invalid signature 'Float': missing '->'
Error: Runtime error: Type error: Expected function value
```

修复后输出（预期行为）：

```
[Error] Invalid signature: unknown type 'Fn'
Error: (compilation failed)
```

---

## 实现目标

### 目标 1：修复签名定义

根据 RFC-010 统一类型语法，泛型函数的正确格式为：

```
函数名: [泛型参数列表](参数列表) -> 返回类型
```

其中泛型参数 `[T]` 声明在函数级别，作用于整个函数签名。

将 `map`/`filter`/`reduce` 的签名修改为（泛型参数 `[T]` 在函数名前）：

```rust
// map: 泛型 [T] 作用域为整个函数
"[T](list: List<T>, fn: (item: T) -> T) -> List<T>"

// filter: 泛型 [T] 作用域为整个函数
"[T](list: List<T>, fn: (item: T) -> Bool) -> List<T>"

// reduce: 泛型 [T] 作用域为整个函数
"[T](list: List<T>, fn: (acc: Any, item: T) -> Any, init: Any) -> Any"
```

**签名结构说明**：

```
[T](list: List<T>, fn: (item: T) -> T) -> List<T>
│  │         │      │    │        │
│  │         │      │    │        └── 返回类型（使用 T）
│  │         │      │    └── 参数类型（使用 T）
│  │         │      └── 参数名
│  │         └── 参数类型（函数类型）
│  └── 参数类型（List 泛型，使用 T）
└── 泛型参数声明（作用域为整个函数）
```

### 目标 2：泛型参数作用域规则

**泛型参数声明禁止遮蔽**（No Shadowing）：

签名中存在多个作用域层级：

```
[T](list: List[T], fn: [T](item: T) -> T) -> List[T]
│                      │
│                      └── 内层函数类型作用域（fn 的类型参数）
└── 外层函数作用域（函数的泛型参数）
```

**规则**：

1. **同级禁止遮蔽**：同一作用域内的泛型参数不能同名
2. **内层禁止遮蔽外层**：内层作用域的泛型参数不能与外层同名
3. **函数参数禁止遮蔽**：函数参数名不能与任何泛型参数同名

**有效示例**：

```yaoxiang
// ✅ 有效：泛型参数 T 作用域为整个函数
map: [T](list: List[T], fn: (item: T) -> T) -> List[T]

// ✅ 有效：内层函数类型无泛型参数
map: [T](list: List[T], fn: (item: T) -> T) -> List[T]

// ✅ 有效：多泛型参数
zip: [T, U](a: List[T], b: List[U]) -> List<(T, U)>

// ✅ 有效：函数参数名与泛型参数不同
foo: [T](x: Int, y: T) -> T
```

**无效示例**：

```yaoxiang
// ❌ 无效：内层函数泛型遮蔽外层泛型（禁止遮蔽）
"[T](list: List[T], fn: [T](item: T) -> T) -> List[T]"
# 错误：Generic parameter 'T' in function type shadows outer generic parameter 'T'

// ❌ 无效：泛型参数同名（同级禁止遮蔽）
"[T, T](x: T, y: T) -> T"
# 错误：Duplicate generic parameter 'T'

// ❌ 无效：函数参数名与泛型参数同名（禁止遮蔽）
"[T](T: Int) -> T"
# 错误：Parameter 'T' shadows generic parameter 'T'
```

### 目标 3：签名参数名检查

解析签名时，应验证参数名的合法性：
1. **参数名不能重复**（E2093）
2. **泛型参数禁止遮蔽**（E2095, E2096）

> **注意**：参数名是关键字的情况会在解析器解析签名时自动报语法错误，无需单独验证。

示例：
```
// 有效签名（符合 RFC-010）
"[T](list: List<T>, fn: (item: T) -> T) -> List<T>"

// 无效签名 - 函数参数名与泛型参数同名（禁止遮蔽）
"[T](list: List<T>, fn: (T: T) -> T) -> List<T>"
# 应报错: Parameter 'T' shadows generic parameter 'T'

// 无效签名 - 重复参数名
"[T](x: Int, x: Int) -> Int"
# 应报错: Invalid signature: duplicate parameter name 'x'

// 注意：参数名是关键字的情况会在解析器解析签名时自动报语法错误
```

### 目标 4：修复错误信息

当签名解析遇到错误时，应使用错误码系统报错（E2xxx - 语义分析阶段）：

**需要新增的错误码**：

| 错误码 | 消息模板 | 说明 |
|--------|----------|------|
| E2090 | Invalid signature: {reason} | 签名解析失败（通用） |
| E2091 | Invalid signature: unknown type '{type_name}' | 未知类型 |
| E2092 | Invalid signature: missing '->' | 缺少箭头 |
| E2093 | Invalid signature: duplicate parameter '{name}' | 重复参数名 |
| E2094 | Invalid signature: generic '{name}' shadows outer generic | 泛型参数遮蔽 |
| E2095 | Invalid signature: parameter '{name}' shadows generic | 参数名遮蔽泛型 |

> **注意**：参数名是关键字的情况不需要单独验证，因为解析器在解析签名时遇到关键字会自动报语法错误。

**错误信息格式**（符合 RFC-013）：

```
[Error] E2091: Invalid signature: unknown type 'Fn'
 --> std/list.yx:1:1
  |
1 | "(list: List, fn: Fn) -> List"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
help: Use a valid type like '(T) -> T' for function parameters
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

当使用无效签名时，应显示 **Error**（而非 Warning），使用错误码系统（E2xxx）：

```bash
# 测试无效签名
# 预期输出：
[Error] E2091: Invalid signature: unknown type 'Fn'
[Error] E2092: Invalid signature: missing '->'
[Error] E2093: Invalid signature: duplicate parameter 'x'
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
- 重复参数名应报错（E2093）
- 泛型参数遮蔽应报错（E2095, E2096）

> **注意**：参数名是关键字的情况会在解析器解析签名时自动报语法错误，无需单独验证。

```bash
# 预期错误：
[Error] E2093: Invalid signature: duplicate parameter 'x'
[Error] E2095: Invalid signature: generic 'T' shadows outer generic
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

验证错误信息是否正确显示（应为 Error 且使用错误码）：

```bash
# 预期输出：
[Error] E2091: Invalid signature: unknown type 'Fn'
# 而非：
# [Warning] Invalid signature 'Float': missing '->'
```

### 测试用例 7：重复参数名

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
| `src/std/list.rs` | Native 函数导出定义（✅ 已修改签名） |
| `src/frontend/typecheck/mod.rs` | 签名解析逻辑（✅ 已重写 parse_signature） |
| `src/backends/interpreter/executor.rs` | 运行时闭包调用（✅ 已修复 MakeClosure 查找） |
| `src/middle/core/bytecode.rs` | 字节码解码（✅ 已添加 MakeClosure 解码器） |
| `src/std/io.rs` | IO 模块（✅ 已修复列表显示格式） |
| `src/util/diagnostic/codes/e2xxx.rs` | 错误码定义（✅ 已添加 E2090-E2095） |
| `src/util/diagnostic/codes/i18n/zh.json` | 中文 i18n（✅ 已添加） |
| `src/util/diagnostic/codes/i18n/en.json` | 英文 i18n（✅ 已添加） |

### 签名解析流程

1. `TypeCheckResult::new()` 调用 `register_std_native_signatures()`
2. `register_std_native_signatures()` 遍历 std 模块的导出
3. 对每个 `Export`，调用 `parse_signature(&export.signature, env)`
4. `parse_signature` 解析签名字符串为 `MonoType::Fn`
5. 解析失败时使用错误码系统报错（E2090-E2095）

### 实际修改的代码

1. **`src/std/list.rs:71-88`**：修改三个函数的签名字符串（RFC-010 泛型函数语法）
   ```rust
   "[T](list: List<T>, fn: (item: T) -> T) -> List<T>"
   "[T](list: List<T>, fn: (item: T) -> Bool) -> List<T>"
   "[T](list: List<T>, fn: (acc: Any, item: T) -> Any, init: Any) -> Any"
   ```

2. **`src/frontend/typecheck/mod.rs`**（parse_signature 重写）：
   - 支持 `[T]` 泛型参数前缀解析
   - 支持函数类型参数 `(item: T) -> T`
   - 正确处理括号匹配（find_matching_close）
   - 参数名重复检查（E2093）
   - 泛型参数遮蔽检查（E2094、E2095）
   - 常量类型签名处理（如 `"Float"`）
   - 错误信息从 Warning 升级为 Error + 错误码

3. **`src/middle/core/bytecode.rs`**（字节码解码器）：
   - 添加 `Opcode::MakeClosure` 解码器（之前被 catch-all 分支吞掉变成 Nop）

4. **`src/backends/interpreter/executor.rs`**（闭包执行）：
   - 修复 `MakeClosure` 处理器：`FunctionRef::Index` 直接使用索引而非构造名字

5. **`src/std/io.rs`**（IO 模块）：
   - print/println 支持列表/字典的可读格式化输出（通过 heap 解析）

6. **错误码定义**：
   - `src/util/diagnostic/codes/e2xxx.rs`：添加 E2090-E2095 定义和快捷方法
   - `src/util/diagnostic/codes/i18n/zh.json`：添加中文错误信息
   - `src/util/diagnostic/codes/i18n/en.json`：添加英文错误信息

---

## 依赖关系

本任务不依赖其他任务，已独立完成。

---

## 风险与注意事项

1. **泛型支持**：✅ 类型系统支持泛型 `List<T>` 的解析，泛型参数解析为 TypeRef
2. **闭包环境捕获**：当前实现不处理闭包捕获外部变量，map/filter/reduce 用例不需要此功能
3. **额外发现并修复的问题**：
   - MakeClosure 字节码解码器缺失（导致闭包变成 Nop）
   - MakeClosure 执行器对 FunctionRef::Index 的处理错误（构造 "fn_N" 名字而非直接使用索引）
   - io.println 无法格式化显示列表内容（只显示 handle 地址）
