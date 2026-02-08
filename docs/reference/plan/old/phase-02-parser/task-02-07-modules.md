# Task 2.7: 模块解析

> **优先级**: P1
> **状态**: ✅ 已实现

## 功能描述

解析模块导入和模块声明。使用 `.` 作为模块路径分隔符。

## 模块语法

### 导入语句

```yaoxiang
# 简单导入
use std.io

# 完整路径导入
use std.collections.List

# 别名导入
use std.io as io_module

# 导入特定项
use std.io.{read, write}

# 导入并重命名
use std.io.{read as r, write as w}
```

### 模块声明

```yaoxiang
math.yx

# 模块内容
add(a, b) = a + b
multiply(a, b) = a * b

# 导出（作为模块使用时）
pub add, multiply
```

### 使用模块

```yaoxiang
# 使用导入的函数
content = io.read_file("test.txt")
io.write_file("output.txt", content)

# 链式访问
result = std.collections.List.new()
result = result.push(1)
```

## Use 语句结构

```rust
pub struct Stmt {
    kind: StmtKind::Use {
        path: String,                    // 导入路径 "std.io"
        items: Option<Vec<String>>,     // 导入项 Some(["read", "write"])
        alias: Option<String>,          // 别名 Some("io_module")
    },
    span: Span,
}
```

## 语法规则

```ebnf
use_statement = "use" module_path ["as" identifier] ["{" import_items "}"]
module_path   = identifier { "." identifier }
import_items  = import_item { "," import_item }
import_item   = identifier ["as" identifier]
```

## 验收测试

```yaoxiang
# test_modules.yx

# 导入标准库
use std.io

# 使用导入的函数
content = io.read_file("test.txt")
io.write_file("output.txt", content)

# 导入并使用
use std.collections.List
my_list = List.new()
my_list = my_list.push(1)

# 导入特定项
use std.math
result = math.sqrt(16.0)

# 别名导入
use std.io as io_lib
io_lib.print("Hello via alias")

# 完整路径
use std.collections
my_set = std.collections.Set.new()

print("Module parsing tests passed!")
```

## 相关文件

- **[`stmt.rs`](stmt.rs:381)**: `parse_use_stmt()`, `parse_use_path()`
- **[`ast.rs`](ast.rs:145)**: `StmtKind::Use`
