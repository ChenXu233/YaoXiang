# YaoXiang 开发问题记录

本文档记录在开发文件批量重命名工具过程中遇到的 YaoXiang 语言问题。

---

## 问题 1：顶层全局变量赋值被忽略 ✅ 已修复

### 描述
在函数外部进行变量赋值时，变量值会变成 `unit` 而不是预期值。

### 示例代码
```yaoxiang
dir = "."           // 全局赋值
op = "prefix"       // 全局赋值
param = "NEW_"      // 全局赋值
preview = true      // 全局赋值

main: () -> Void = {
    print(dir)      // 输出: test_dir
}
```

### 修复说明
已修复 IR 生成器，正确处理顶层变量的 initializer。变量在函数内部可以正确访问。

### 影响
✅ 已修复 - 现在可以正常使用全局配置常量

---

## 问题 2：string.starts_with 返回类型错误 ✅ 已修复

### 描述
`std.string.starts_with` 函数应该返回 `Bool`，但实际返回 `void`，导致在 `if` 语句中报错。

### 示例代码
```yaoxiang
use std.string

main: () -> Void = {
    name = "test.txt"
    if string.starts_with(name, ".") {  // 错误!
        print("hidden")
    }
}
```

### 错误信息
```
error: Expected type 'bool', found type 'void'
```

### 临时解决方案
该函数暂不可用，需要等待修复

### 影响
✅ 已修复 - 现在 `string.starts_with` 和 `string.ends_with` 正确返回 `Bool` 类型

---

## 问题 3：字符串字面量比较异常 ✅ 已修复

### 描述
字符串字面量直接比较可能返回不正确的结果。

### 示例代码
```yaoxiang
main: () -> Void = {
    result = "test" == "test"
    print(result)    // 输出: true
}
```

### 修复说明
1. 在解释器中添加了字符串类型的运行时比较支持
2. 现在 `==` 和 `!=` 等比较操作可以正确处理字符串类型

### 影响
✅ 已修复 - 字符串比较现在可以正常工作

---

## 问题 4：List 遍历和索引操作复杂 ✅ 已修复

### 描述
`std.list` 模块虽然存在，但无法方便地遍历列表元素或通过索引获取元素。

### 示例代码
```yaoxiang
use std.list

main: () -> Void = {
    items = [1, 2, 3]
    for item in items {
        print(item)    // 现在可以正常工作
    }

    // 索引访问
    first = items[0]
    print(first)
}
```

### 修复说明
1. **修复类型推断**：For 循环现在可以正确推导循环变量类型
   - `List<T>` → 循环变量类型为 `T`
   - `Dict<K, V>` → 循环变量类型为 `(K, V)` 元组

2. **迭代器协议支持**：添加了 Iterable/Iterator trait 定义
   - `std.list.iter(list)` → 创建迭代器
   - `std.list.next(iterator)` → 获取下一个元素
   - `std.list.has_next(iterator)` → 检查是否有更多元素

### 影响
✅ 现在可以正常使用 List 遍历和索引访问

---

## 问题 5：顶层变量不支持函数调用 ✅ 已修复为编译错误

### 描述
在顶层（模块级）变量初始化时调用函数会导致运行时类型错误。

### 示例代码
```yaoxiang
dir = "."
is_dir = os.is_dir(dir)   // 编译错误 E2014
```

### 错误信息（编译时）
```
error[E2014]: Function calls are not allowed in top-level variable initializers
```

### 设计说明
这是**语言设计决策**，不是 bug。原因：
- `main` 函数是唯一入口点，顶层函数调用时机不明确
- 避免副作用顺序不确定导致的不可预测行为
- 顶层变量仅支持常量表达式（字面量）

### 修复说明
1. 添加了编译时错误检测（E2014）
2. 在类型检查阶段检测顶层变量初始化中的函数调用
3. 提供清晰的编译错误信息

### 影响
✅ 现在会在编译时报错，而不是静默失败

---

## 问题 6：函数调用时参数求值顺序

### 描述
在某些情况下，函数调用时参数的值可能不正确。

### 临时解决方案
将复杂表达式的结果先赋值给变量，再传递

---

### 已验证可用的功能
- `std.io.print` / `std.io.println`
- `std.os.read_dir`
- `std.os.rename`
- `std.os.is_dir` (在函数内调用)
- `std.string.split`
- `std.string.substring`
- `std.string.index_of`
- `std.string.replace`
- `std.string.len`
- `std.string.is_empty`
- `std.list.len`
- 字符串拼接 `+`
- 字符串比较 `==`

### 暂不可用或存在问题的功能
- 全局变量定义（问题 1 已修复）
- `string.starts_with` / `string.ends_with`（问题 2 已修复）
- 字符串比较 `==`（问题 3 已修复）
- 列表遍历
- 列表索引访问

---

## 建议改进

2. **修复全局变量**：支持顶层变量定义
3. **修复 string.starts_with/ends_with**：确保返回 Bool 类型
4. **增强 List 支持**：添加 `get` 方法和迭代支持
5. **文档完善**：补充标准库函数的使用示例
