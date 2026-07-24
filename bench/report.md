# bench/ 编写过程中发现的 YaoXiang 语言 bug

## 1. Result 类型方法调用失败

**问题**: `result.unwrap_or()` 方法调用报 `Cannot access field on non-struct type 'Result(Int, Error)'`

**代码**:
```yaoxiang
use std.{io, string, result}
main = {
    parsed = string.parse_int("42")
    n = parsed.unwrap_or(1000)  // ← 报错
    io.println(n)
}
```

**符合规范**：`docs/src/reference/language-spec/stdlib.md §1.3` 定义 `unwrap_or: (self: Result(T, E), default: T) -> T`

**实际行为**：不能通过 `.` 方法调用语法调用 Result 类型的方法，报 `E1053`。

**影响**：`bench/src/fibonacci/fibonacci.yx` 等 4 个文件被迫改用 `result.unwrap_or(parsed, 1000)` 函数调用风格（模块名 + 函数名 + 参数）。

---

## 2. Result 枚举变体 match 失败

**问题**: `result.Result.Ok(v)` 匹配报错

**代码**:
```yaoxiang
match parsed {
    result.Result.Ok(v) => v,   // ← 报错
    result.Result.Err(_) => 0
}
```

**实际行为**：`tests/yaoxiang/02-type-system/result.yx` 标注为 `🔴 未实现 - Result 类型在运行时未完全支持`。

**影响**：无法用 match 解包 Result，只能用 `result.unwrap_or()` 函数调用方式。

---

## 3. 嵌套列表索引失败

**问题**: `c[0][0]` 报 `Cannot access field on non-struct type 'List(List(int64))'`

**代码**:
```yaoxiang
mut c = multiply(a, b, size)
val = c[0][0]  // ← 报错
```

**预期**：`c[0]` 返回 `List(Int)`，`c[0][0]` 返回 `Int`。

**实际行为**：不能链式索引嵌套列表，必须分成两步：
```yaoxiang
row0 = c[0]
val = row0[0]
```

**影响**：`bench/src/matrix/matrix.yx` 被迫使用两步索引。

---

## 4. `for` 循环变量被 move 后无法复用

**问题**: `for n in nums { filtered = filtered + [n] }` 报 `'n' has been moved`

**代码**:
```yaoxiang
for n in nums {
    filtered = filtered + [n]  // ← 第一次用 n，move 进列表
    doubled = doubled + [double(n)]  // ← 第二次用 n，报错 "n has been moved"
}
```

**预期**：`for` 循环变量应该每次迭代都重新绑定，不是永久 move。

**实际行为**：`n` 被 `[n]` 列表构造 move 后，后续无法再使用。

**影响**：`bench/src/list_ops/list_ops.yx` 被迫将 `is_even` 和 `double` 分开成两个循环，且内联了 `is_even` 避免函数调用中的 move。

---

## 5. 内置 `len()` 函数不接受 List 类型

**问题**: `len(nums)` 报 `Expected type 'string', found type 'List(int64)'`

**代码**:
```yaoxiang
nums = [1, 2, 3]
mut k = 0
while k < len(nums) {  // ← 报错
    ...
}
```

**预期**：`len()` 应该对 List、String、Dict 等类型通用。

**实际行为**：`len()` 只接受 String 类型，List 需要 `list.len(nums)`。

**影响**：`bench/src/list_ops/list_ops.yx` 被迫使用 `list.len()`。

---

## 6. `list.len()` 循环终止证明失败

**问题**: `list.len()` 在 `while` 条件中导致 `BeyondKernel("循环无法证明终止：未找到有效的递减度量")`

**代码**:
```yaoxiang
while k < list.len(nums) {
    k = k + 1
}
```

**预期**：`k` 递增、`list.len(nums)` 是常量，循环显然终止。

**实际行为**：终止检查器不能分析 `list.len()` 返回值作为边界。

**影响**：`bench/src/list_ops/list_ops.yx` 被迫弃用 `while` + `list.len()`，改用 `for` 循环。

---

## 7. `for i in 2..(n + 1)` 范围中的加法类型不匹配

**问题**: `for i in 2..(n + 1)` 报 `type mismatch in binary operation Add`

**代码**:
```yaoxiang
parsed = string.parse_int(n_str)
n = result.unwrap_or(parsed, 1000)
for i in 2..(n + 1) {  // ← 报错
    ...
}
```

**预期**：`n` 是 `Int`，`n + 1` 应该是 `Int`，可作范围上界。

**实际行为**：`n` 被推断为不应该用于算术运算的类型（可能是 `result.unwrap_or` 返回的类型不是普通 `Int`）。

**影响**：`bench/src/fibonacci/fibonacci.yx` 被迫改用 `while` 循环。

---

## 总结

| # | 问题 | 严重程度 | 回避方式 |
|---|------|----------|----------|
| 1 | Result 方法调用语法不支持 | 高 | 改用函数调用风格 |
| 2 | Result 枚举变体 match 未实现 | 高 | 改用 `result.unwrap_or()` |
| 3 | 嵌套列表索引不支持 | 中 | 分两步索引 |
| 4 | `for` 循环变量被 move | 高 | 改用 `while` + 索引 |
| 5 | `len()` 只接受 String | 中 | 改用 `list.len()` |
| 6 | 循环终止证明太保守 | 高 | 改用 `for` 循环 |
| 7 | `unwrap_or` 返回值类型不兼容 | 中 | 改 `while` 循环 |