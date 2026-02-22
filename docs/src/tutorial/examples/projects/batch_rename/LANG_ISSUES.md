# YaoXiang 开发问题记录

本文档记录在开发文件批量重命名工具过程中遇到的 YaoXiang 语言问题。

---

## 问题 1：List 遍历时类型标注导致的编译错误 已修复

**状态**：🔴 未解决

**错误信息**：
```
error [E1053] Cannot access field on non-struct type 't35'
help: Field access is only available on struct types
```

**代码示例（有问题）**：
```yaoxiang
// 类型定义
FileEntry: Type = {
    original: String,
    new_name: String,
    is_file: Bool
}

// 遍历时使用类型标注
for item in files {
    entry: FileEntry = item  // ← 这里报错
    if entry.is_file {
        print(entry.original)
    }
}
```

**问题分析**：
- 使用 `entry: FileEntry = item` 进行类型标注时，编译器将 item 推断为某种内部类型 t35
- 访问字段时提示 "Cannot access field on non-struct type"


## 问题 3：变量作用域实现错误 - 块级作用域无效（严重 BUG） 已修复

**状态**：🔴 未解决 - **严重 BUG**

**错误信息**：
```
error [E2013] Cannot shadow existing variable 'fp1'
help: Use a different variable name, or declare the variable in a different scope
```

**问题分析**：
- 在 for 循环内部无法使用与外部同名的变量（即使只是赋值而非声明）
- 整个函数是一个作用域，块级作用域（如 for 循环、if 语句）不独立
- 这违反了大多数编程语言的作用域规则，导致代码难以编写

**示例**：
```yaoxiang
main = {
    fp1 = "hello"  // 外部变量

    for item in list {
        fp1 = item  // ← 报错：Cannot shadow existing variable 'fp1'
    }
}
```

**影响**：
- 无法在循环中复用外部变量名
- 代码需要为每个块使用唯一变量名，导致代码冗长
- 严重影响开发体验

---

## 问题 4：std.os.is_dir 函数缺失

**状态**：🔴 未解决

**代码**：
```yaoxiang
if std.os.is_dir(dir_path) == false {
    print("Error: Path is not a directory!\n")
    return
}
```

**错误**：尝试运行时报错 `std.os.is_dir` 函数不存在

**标准库现状**（src/std/os.rs）：
- `std.os.exists(path)` ✓
- `std.os.is_file(path)` ✓
- `std.os.is_dir(path)` ✗ 不存在

---

## 问题 5：Int 转 String 的序号格式化

**状态**：🔴 未解决

**需求**：将数字转换为 3 位字符串（如 1 → "001"）

**尝试的代码**：
```yaoxiang
// 尝试使用 substring 截取数字的字符串表示
counter = 1
num_str = std.string.substring(counter, 0, 1)  // ← 可能不支持 Int 参数
```

**问题**：std.string.substring 的参数类型和 YaoXiang 的类型推断可能不支持 Int 直接转 String
