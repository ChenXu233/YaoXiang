# FFI 规范

本文件定义 YaoXiang 编程语言的 FFI（外部函数接口）规范，包括类型定义、函数声明、方法绑定和不透明类型的处理。

> **详细设计**：FFI 的完整设计、动机和权衡详见
> [RFC-026: FFI 核心机制](../../design/rfc/accepted/026-ffi-core-mechanism.md)。

---

## 第一章：概述

### 1.1 FFI 的核心原则

```
所有 {} 中的 return 都是将内容返回给上一作用域
默认没有 return 为返回 Void
```

### 1.2 FFI 的组成

| 组件     | 说明                          | 语法                   |
| -------- | ----------------------------- | ---------------------- |
| 类型定义 | 定义 FFI 类型（不透明或透明） | `unsafe {}` + `return` |
| 函数声明 | 声明外部函数                  | `native("symbol")`     |
| 方法绑定 | 绑定方法到类型                | `[0]` 语法             |

---

## 第二章：FFI 类型定义

### 2.1 不透明类型

不透明类型在 `unsafe {}` 块中定义，通过 `return` 返回给上一作用域：

```yaoxiang
// 在 unsafe 块中定义不透明类型
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // 裸指针
    }
    return SqliteDb
}

// SqliteDb 在 unsafe 块外可用
db = sqlite3_open("test.db")

// ❌ 编译错误：handle 字段需要 unsafe 权限
handle = db.handle

// ✅ 通过方法调用
db.close()
```

### 2.2 透明类型

透明类型直接定义，不需要 `unsafe {}` 块：

```yaoxiang
// 透明类型
Point: Type = {
    x: Int32,
    y: Int32
}

// 用户可以直接创建
p: Point = Point { x: 1, y: 2 }
```

### 2.3 不透明类型的判断

编译器自动判断不透明类型和真空类型：

```yaoxiang
// 不透明类型（被 native 函数引用）
SqliteDb: Type = {}
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
// → SqliteDb 被 native 函数引用 → 不透明类型

// 真空类型（没有被 native 函数引用）
MyType: Type = {}
// → MyType 没有被 native 函数引用 → 真空类型
```

**判断规则**：

- 如果类型被 `native` 函数引用 → 不透明类型
- 否则 → 真空类型

---

## 第三章：FFI 函数声明

### 3.1 native 语法

使用 `native("symbol")` 语法声明外部函数：

```yaoxiang
// FFI 函数声明
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
```

### 3.2 参数类型映射

FFI 函数的参数类型直接使用 YaoXiang 类型，编译器自动处理 C 类型映射：

| C 类型               | YaoXiang 类型     |
| -------------------- | ----------------- |
| `int`                | `Int32`           |
| `long`               | `Int64`           |
| `float`              | `Float32`         |
| `double`             | `Float64`         |
| `char`               | `Char`            |
| `char*`              | `String`          |
| `bool`               | `Bool`            |
| `size_t`             | **无直接映射**（见下） |
| `void*`              | `*Void`           |
| `struct T*`          | `T`（透明类型）   |
| `typedef struct T T` | `T`（不透明类型） |

> **`size_t` 没有对应的 YaoXiang 类型**：类型系统只有**有符号**整数族
> （`src/frontend/core/types/mono.rs:618-632`：`Int` / `Int8` / `Int16` / `Int32` /
> `Int64` 与 `Float` / `Float32` / `Float64`），**没有 `Uint` 或任何无符号类型**。
> 尺寸 / 长度类参数请用 `Int64` 并在你自己的一侧处理上界（`size_t` 的实际取值不会
> 超过 `i64::MAX`，但回传负数的上游约定需要你自己防守）。
>
> （`Uint` 这个名字确实出现在仓库里，但只在两处**非类型**语境：
> `src/lsp/world.rs:176` 的补全候选表，与 `src/frontend/core/types/eval/const_eval.rs:503`
> 的 `sizeof` 兜底分支。它不是可用的类型名。）

### 3.3 返回类型

FFI 函数的返回类型直接使用 YaoXiang 类型：

```yaoxiang
// 返回不透明类型
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// 返回透明类型
get_point: () -> Point = native("get_point")

// 返回基本类型
get_value: () -> Int32 = native("get_value")
```

---

## 第四章：方法绑定

### 4.1 [0] 语法

使用 `[0]` 语法指定 self 参数在函数参数元组中的位置：

```yaoxiang
// FFI 函数
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")

// 方法绑定（self 在位置 0）
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
```

**调用方式**：

```yaoxiang
db = sqlite3_open("test.db")

// 方法调用
db.close()  // 等价于 sqlite3_close(db)
db.exec("SELECT * FROM users")  // 等价于 sqlite3_exec(db, "SELECT * FROM users")
```

### 4.2 构造函数绑定

构造函数不加 `[0]`，绑定为普通函数：

```yaoxiang
// FFI 函数
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// 构造函数绑定（普通函数）
SqliteDb.open = sqlite3_open
```

**调用方式**：

```yaoxiang
// 通过构造函数创建
db = SqliteDb.open("test.db")
```

### 4.3 绑定位置

方法绑定可以在任何位置，因为类型是数据容器：

```yaoxiang
// 在类型定义后绑定
SqliteDb.close = sqlite3_close[0]

// 在其他文件中绑定
SqliteDb.exec = sqlite3_exec[0]

// 编译器最终都会检查
```

---

## 第五章：spawn 块中的 FFI 行为

### 5.1 资源类型自动串行

如果 FFI 类型是资源类型，spawn 块中自动串行化：

```yaoxiang
// SqliteDb 是资源类型
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // SqliteDb 资源
    db2 = SqliteDb.open("db2.sqlite")   // 不同实例，可并行
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // 同一 SqliteDb
    result2 = db.exec("INSERT ...")   // 自动串行
}
```

### 5.2 非资源类型可并行

如果 FFI 类型不是资源类型，spawn 块中可并行：

```yaoxiang
// Float 不是资源类型
(a, b) = spawn {
    result1 = sin(1.0),  // 可并行
    result2 = cos(1.0)   // 可并行
}
```

---

## 第六章：yx-bindgen 工具链（计划中，未实现）

> **状态：未实现。** 从 C 头文件自动生成绑定是
> [RFC-026b](../../design/rfc/accepted/026-ffi-core-mechanism.md)（本章的自动化子提案，
> 仓库里尚无独立文档）的目标，当前**没有任何可执行入口**：
>
> - `Cargo.toml` 只声明了一个二进制 `[[bin]] yaoxiang-rs`，没有 `yx-bindgen`；
> - 包清单里写 `[build].headers` 会**直接报错**——
>   `[build].headers 需要 yx-bindgen（RFC-026b，尚未实现）；移除 headers 或改用
>   [binaries] 预编译分发`（`src/package/build/mod.rs:152-159`）。

因此本节只记录**设计意图**（RFC-026b 落地后应当生成的形态），不是当前可用的 CLI。
今天要做同样的事，手写第二至第四章的内容即可——那三章描述的 `unsafe {}` 类型定义、
`native("symbol")` 声明与 `[0]` 方法绑定都是已实现的。

### 6.1 计划生成的内容

- FFI 类型定义（unsafe 块 + return）
- FFI 函数声明（native 语法）
- 方法绑定（[0] 语法）

### 6.2 计划中的调用形态与手写等价物

```bash
# 计划中，尚未实现：
# yx-bindgen --header /usr/include/sqlite3.h --output sqlite3_bindings.yx
```

生成物等价于手写：

```
// sqlite3_bindings.yx（当前需手写）
// 自动生成，不要手动编辑

// ============================================================================
// 类型定义
// ============================================================================

SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}

SqliteStmt = unsafe {
    SqliteStmt: Type = {
        handle: *Void
    }
    return SqliteStmt
}

// ============================================================================
// FFI 函数声明
// ============================================================================

sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
sqlite3_prepare_v2: (db: SqliteDb, sql: String) -> SqliteStmt = native("sqlite3_prepare_v2")
sqlite3_step: (stmt: SqliteStmt) -> Int32 = native("sqlite3_step")
sqlite3_finalize: (stmt: SqliteStmt) -> Int32 = native("sqlite3_finalize")

// ============================================================================
// 方法绑定
// ============================================================================

// 构造函数（普通函数）
SqliteDb.open = sqlite3_open

// 方法（self 在位置 0）
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
SqliteDb.prepare = sqlite3_prepare_v2[0]

// SqliteStmt 的方法
SqliteStmt.step = sqlite3_step[0]
SqliteStmt.finalize = sqlite3_finalize[0]
```

---

## 附录：FFI 语法速查

### A.1 类型定义

```yaoxiang
// 不透明类型
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}

// 透明类型
Point: Type = {
    x: Int32,
    y: Int32
}
```

### A.2 函数声明

```yaoxiang
// FFI 函数声明
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
```

### A.3 方法绑定

```yaoxiang
// 构造函数（普通函数）
SqliteDb.open = sqlite3_open

// 方法（self 在位置 0）
SqliteDb.close = sqlite3_close[0]
```

### A.4 调用方式

```yaoxiang
// 通过构造函数创建
db = SqliteDb.open("test.db")

// 通过方法调用
db.close()
db.exec("SELECT * FROM users")
```
