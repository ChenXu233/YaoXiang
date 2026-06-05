---
title: "RFC-026：FFI 核心机制"
---

# RFC-026：FFI 核心机制

> **状态**: 审核中
> **作者**: 晨煦
> **创建日期**: 2026-06-05
> **最后更新**: 2026-06-05

> **参考**:
> - [RFC-008: Runtime 并发模型与调度器脱耦设计](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有权模型设计](./009-ownership-model.md)
> - [RFC-010: 统一类型语法](./010-unified-type-syntax.md)
> - [RFC-024: 基于 spawn 块的并发模型](./024-concurrency-model.md)

> **废弃**:
> - [RFC-020: 动态模块与 FFI 集成](./020-dynamic-modules-ffi.md) — 内容已合并到本文档
> - [RFC-021: 库驱动 FFI 扩展与跨语言调用支持](./021-library-driven-ffi-extension.md) — 内容已合并到本文档

## 摘要

本文档定义 YaoXiang 的 FFI（外部函数接口）核心机制，包括：

1. **FFI 类型定义**：使用 `unsafe {}` 块定义不透明类型，通过 `return` 返回给上一作用域
2. **FFI 函数声明**：使用 `native("symbol")` 语法声明外部函数
3. **方法绑定**：使用 `[0]` 语法指定 self 参数位置
4. **不透明类型的处理**：编译器自动判断不透明类型和真空类型
5. **spawn 块中的 FFI 行为**：资源类型自动串行，非资源类型可并行

**核心设计——一个原则，统一语义**：

```
所有 {} 中的 return 都是将内容返回给上一作用域
默认没有 return 为返回 Void
```

---

## 动机

### 为什么需要这个设计？

RFC-020 和 RFC-021 分别定义了 FFI 的不同方面：
- RFC-020：动态模块与 FFI 集成
- RFC-021：库驱动 FFI 扩展

两者有重叠，需要整合成统一的 FFI 规范。

### 设计目标

1. **统一**：所有 `{}` 块的 return 语义一致
2. **安全**：不透明类型的字段访问需要 unsafe 权限
3. **简洁**：不需要新关键字或特殊标记
4. **实用**：yx-bindgen 自动生成绑定

---

## 提案

### 1. FFI 类型定义

#### 1.1 unsafe 块 + return 语义

在 unsafe 块中定义不透明类型，通过 return 返回给上一作用域：

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

#### 1.2 透明类型

透明类型直接定义，不需要 unsafe 块：

```yaoxiang
// 透明类型
Point: Type = {
    x: Int32,
    y: Int32
}

// 用户可以直接创建
p: Point = Point { x: 1, y: 2 }
```

#### 1.3 不透明类型的判断

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

### 2. FFI 函数声明

#### 2.1 native 语法

使用 `native("symbol")` 语法声明外部函数：

```yaoxiang
// FFI 函数声明
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
```

#### 2.2 参数类型映射

FFI 函数的参数类型直接使用 YaoXiang 类型，编译器自动处理 C 类型映射：

| C 类型 | YaoXiang 类型 |
|--------|---------------|
| `int` | `Int32` |
| `long` | `Int64` |
| `float` | `Float32` |
| `double` | `Float64` |
| `char` | `Char` |
| `char*` | `String` |
| `bool` | `Bool` |
| `size_t` | `Uint` |
| `void*` | `*Void` |
| `struct T*` | `T`（透明类型）|
| `typedef struct T T` | `T`（不透明类型）|

#### 2.3 返回类型

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

### 3. 方法绑定

#### 3.1 [0] 语法

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

#### 3.2 构造函数绑定

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

#### 3.3 绑定位置

方法绑定可以在任何位置，因为类型是数据容器：

```yaoxiang
// 在类型定义后绑定
SqliteDb.close = sqlite3_close[0]

// 在其他文件中绑定
SqliteDb.exec = sqlite3_exec[0]

// 编译器最终都会检查
```

---

### 4. 不透明类型的处理

#### 4.1 编译器自动处理

编译器自动判断不透明类型，内部处理 C 指针存储：

```text
编译器分析：
    SqliteDb: Type = {}
    sqlite3_open: ... -> SqliteDb = native("sqlite3_open")

推断：
    SqliteDb 是不透明类型
    内部自动添加 @internal handle: *Void
    禁止用户直接创建
```

#### 4.2 用户代码

```yaoxiang
import sqlite3_bindings

// ✅ 通过构造函数创建
db = SqliteDb.open("test.db")

// ❌ 编译错误：不能直接创建不透明类型
db: SqliteDb = {}

// ✅ 通过方法调用
result = db.exec("SELECT * FROM users")
db.close()
```

---

### 5. spawn 块中的 FFI 行为

#### 5.1 资源类型自动串行

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

#### 5.2 非资源类型可并行

如果 FFI 类型不是资源类型，spawn 块中可并行：

```yaoxiang
// Float 不是资源类型
(a, b) = spawn {
    result1 = sin(1.0),  // 可并行
    result2 = cos(1.0)   // 可并行
}
```

---

### 6. yx-bindgen 工具链

#### 6.1 生成内容

yx-bindgen 生成以下内容：
- FFI 类型定义（unsafe 块 + return）
- FFI 函数声明（native 语法）
- 方法绑定（[0] 语法）

#### 6.2 生成示例

```bash
yx-bindgen --header /usr/include/sqlite3.h --output sqlite3_bindings.yx
```

生成结果：

```yaoxiang
// sqlite3_bindings.yx
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

## 权衡

### 优点

1. **统一语义**：所有 `{}` 块的 return 语义一致
2. **不需要新关键字**：使用现有的 unsafe 和 return
3. **不需要特殊标记**：编译器自动判断不透明类型
4. **安全**：不透明类型的字段访问需要 unsafe 权限
5. **实用**：yx-bindgen 自动生成绑定

### 缺点

1. **unsafe 块的作用域**：需要修改 `{}` 块的 return 语义
2. **编译器复杂度**：需要自动判断不透明类型
3. **yx-bindgen 维护**：需要持续更新以支持新的 C 库

---

## 实现策略

### 阶段 1：核心机制 (v0.8)

- [ ] 实现 unsafe 块的 return 语义
- [ ] 实现 FFI 类型定义
- [ ] 实现 FFI 函数声明
- [ ] 实现方法绑定

### 阶段 2：不透明类型 (v0.9)

- [ ] 实现编译器自动判断不透明类型
- [ ] 实现内部 handle 存储
- [ ] 实现禁止直接创建不透明类型

### 阶段 3：工具链 (v1.0)

- [ ] 实现 yx-bindgen
- [ ] 支持 Linux/macOS/Windows
- [ ] 集成测试

---

## 与其他 RFC 的关系

- **RFC-008**：Runtime 并发模型，FFI 调用作为独立任务
- **RFC-009**：所有权模型，unsafe 块的语义
- **RFC-010**：统一类型语法，{} 块的 return 语义
- **RFC-024**：并发模型，spawn 块中的 FFI 行为

---

## 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| FFI 类型定义 | unsafe 块 + return | 统一语义，不需要新关键字 | 2026-06-05 |
| 不透明类型判断 | 编译器自动判断 | 不需要特殊标记 | 2026-06-05 |
| 方法绑定 | [0] 语法 | 明确 self 位置 | 2026-06-05 |
| 构造函数 | 普通函数绑定 | 不需要特殊语法 | 2026-06-05 |
| spawn 块行为 | 资源类型自动串行 | 安全，符合并发模型 | 2026-06-05 |

---

## 参考文献

### YaoXiang 官方文档

- [RFC-008 Runtime 并发模型](./008-runtime-concurrency-model.md)
- [RFC-009 所有权模型](./009-ownership-model.md)
- [RFC-010 统一类型语法](./010-unified-type-syntax.md)
- [RFC-024 并发模型](./024-concurrency-model.md)

### 外部参考

- [Rust FFI](https://doc.rust-lang.org/nomicon/ffi.html)
- [Python ctypes](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading](https://docs.rs/libloading/latest/libloading/)

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **审核中** | `docs/design/rfc/` | 开放社区讨论 |
| **已接受** | `docs/design/rfc/accepted/` | 正式设计文档 |
