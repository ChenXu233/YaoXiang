---
title: "RFC-026：FFI 核心机制"
status: "审核中"
author: "晨煦"
created: "2026-06-05"
updated: "2026-06-10"
---

# RFC-026：FFI 核心机制

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
4. **不透明类型的处理**：`unsafe {}` 块显式定义不透明类型，空体 `Type = {}` 为真空类型
5. **不透明类型的生命周期**：`.drop` 绑定析构函数，RAII 自动释放，Null 安全处理
6. **spawn 块中的 FFI 行为**：资源类型自动串行，非资源类型可并行

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

#### 1.3 不透明类型、透明类型、真空类型

三种类型的区分在**定义时**决定，不需要编译器跨文件推断：

```yaoxiang
// 透明类型：有字段
Point: Type = { x: Int32, y: Int32 }
// 用户可以创建和访问字段
p: Point = Point { x: 1, y: 2 }

// 真空类型：空体，不在 unsafe 块中
MyMarker: Type = {}
// 零大小类型，自由创建
x: MyMarker = {}

// 不透明类型：从 unsafe 块返回
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}
// SqliteDb 是不透明类型，不能直接创建，不能访问字段
```

**规则**：
- **有字段** → 透明类型
- **空体 + 不在 unsafe 块中** → 真空类型（零大小）
- **从 unsafe 块返回** → 不透明类型

`native` 函数只能引用已明确定义的类型，不改变类型的属性。类型属性在定义时决定，不在使用时推断。

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

#### 4.1 不透明类型的内部存储

类型在 `unsafe {}` 块中定义的 `handle: *Void` 由编译器自动管理：

```text
编译器处理：
    SqliteDb = unsafe {
        SqliteDb: Type = {
            handle: *Void         // ← 编译器内部存储 C 指针
        }
        return SqliteDb
    }

结果：
    SqliteDb 是不透明类型（由 unsafe 块显式定义）
    字段访问需要 unsafe 权限
    禁止用户直接创建（必须通过 native 构造函数）
```

不需要编译器反向推断——类型是否不透明由定义方式决定，清晰且可预测。

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

// ============================================================================
// 析构函数
// ============================================================================

SqliteDb.drop = sqlite3_close[0]
SqliteStmt.drop = sqlite3_finalize[0]
```

---

### 7. 不透明类型的生命周期管理

不透明类型遵循 RFC-009 的所有权模型，零新概念。

#### 7.1 核心原则

- **Move 语义**：不透明类型默认 Move，赋值/传参/返回 = 所有权转移，不可复制
- **RAII 释放**：作用域结束时自动调用析构函数
- **消费追踪**：显式析构后变量被消费，不可再用

#### 7.2 析构函数绑定

使用 `.drop` 约定绑定析构函数，语法与普通方法绑定完全一致：

```yaoxiang
// 析构函数绑定（self 在位置 0）
SqliteDb.drop = sqlite3_close[0]
SqliteStmt.drop = sqlite3_finalize[0]
```

编译器识别 `.drop` 绑定，在作用域结束时自动调用。**不引入新关键字，不引入 trait 系统**——这就是方法绑定 + RAII，RFC-009 已经承诺的语义。

#### 7.3 自动析构

```yaoxiang
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT * FROM users")
    stmt.step()
    // ← 作用域结束，逆序自动析构：
    //   stmt.drop()  → sqlite3_finalize(stmt)
    //   db.drop()    → sqlite3_close(db)
}
```

**析构顺序**：定义顺序的逆序（后定义先析构），与 RAII 语义一致。

#### 7.4 显式析构

```yaoxiang
db = SqliteDb.open("test.db")
db.close()              // 显式析构。close 即 drop——绑定什么名字就用什么名字
db.exec("...")          // ❌ 编译错误：db 已被消费（Move 后不可读）
```

没有单独的"close vs drop"概念。`SqliteDb.drop = sqlite3_close[0]` 之后，`db.close()` 和 `db.drop()` 是同一个函数。

#### 7.5 析构与 Move

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move：所有权转移给 db2
// db 在此已无效
// ← 作用域结束，自动调用 db2.drop()

// 函数消费
process_db: (db: SqliteDb) -> Void = {
    result = db.exec("...")
    // ← 函数结束，db 在此析构
}

db = SqliteDb.open("test.db")
process_db(db)          // Move 进函数，函数结束时析构
// db 在此已无效
```

#### 7.6 Null 处理

```yaoxiang
// 可能返回 null → ? 标记可选类型，用户必须处理
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")

db = SqliteDb.open("test.db")
match db {
    Some(db) => {
        db.exec("SELECT * FROM users")
        // ← 作用域结束，自动调用 db.drop()
    }
    None => print("打开失败")
}

// 不可能返回 null → 不标记，null 时 panic
// 用于 C 函数约定永远不会返回 null 的场景
sqlite3_get_global: () -> SqliteDb = native("sqlite3_get_global")
```

**设计原则**：C 返回 null 要么让用户处理（`?`），要么 panic 暴露。不存在第三种"默默忽略"的选项。

#### 7.7 析构失败处理

```yaoxiang
// 析构函数可能返回错误码
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
SqliteDb.drop = sqlite3_close[0]

// 编译器行为：
//   debug 模式：析构返回值 != 0 → panic（暴露问题）
//   release 模式：忽略返回值（C 标准允许 close 失败但不影响内存安全）
```

#### 7.8 spawn 块中的析构

```yaoxiang
// 资源类型在 spawn 块中自动串行，析构天然安全
{
    db = SqliteDb.open("test.db")
    result = db.exec("...")
}  // ← 串行保证：exec 完成后 drop，无并发竞争

// 跨 spawn 边界 Move
db = SqliteDb.open("test.db")
spawn {
    use(db)             // Move 进 spawn
    // ← spawn 结束，自动析构
}
```

#### 7.9 无需析构的类型

不透明类型不强制绑定 `.drop`。不绑定析构函数的类型在作用域结束时什么都不做——适用于包装静态数据、全局句柄等不需要清理的场景。

编译器在 debug 模式下对未绑定 `.drop` 的不透明类型给出 lint 提示（默认 warn），提醒用户确认。

#### 7.10 生命周期规则总结

| 场景 | 行为 | 来源 |
|------|------|------|
| 不透明类型赋值 | Move（不可复制） | RFC-009 |
| `.drop` 绑定 | 方法绑定语法 `[0]` | 本文档 §3 |
| 作用域结束 | 逆序自动调用 `.drop()` | RFC-009 RAII |
| 显式 `.close()` | 消费变量，之后不可用 | RFC-009 Move 语义 |
| Null 返回 | `?T` 可选 / 直接 panic | 本文档 §7.6 |
| spawn 块中 | 自动串行，析构安全 | RFC-024 |

---

## 权衡

### 优点

1. **统一语义**：所有 `{}` 块的 return 语义一致
2. **不需要新关键字**：使用现有的 unsafe 和 return
3. **显式定义**：类型属性在定义时决定，unsafe 块返回 → 不透明，空体 → 真空，不需要推断
4. **零新概念生命周期**：`.drop` = 方法绑定 + RAII，无 trait 系统，无新关键字
5. **安全**：不透明类型的字段访问需要 unsafe 权限，析构后变量不可用
6. **实用**：yx-bindgen 自动生成绑定（含析构函数）

### 缺点

1. **unsafe 块的作用域**：需要修改 `{}` 块的 return 语义
2. **yx-bindgen 维护**：需要持续更新以支持新的 C 库

---

## 实现策略

### 阶段 1：核心机制 (v0.8)

- [ ] 实现 {} 块的 return 语义
- [ ] 实现 FFI 类型定义
- [ ] 实现 FFI 函数声明
- [ ] 实现方法绑定

### 阶段 2：生命周期管理 (v0.9)

- [ ] 实现 `.drop` 析构函数绑定
- [ ] 实现作用域结束自动析构
- [ ] 实现析构后变量消费检查
- [ ] 实现 `?T` 与 FFI null 返回的集成
- [ ] 实现内部 handle 存储
- [ ] 实现禁止直接创建不透明类型

### 阶段 3：工具链 (v1.0)

- [ ] 实现 yx-bindgen
- [ ] 支持 Linux/macOS/Windows
- [ ] 集成测试

---

## 与其他 RFC 的关系

- **RFC-008**：Runtime 并发模型，FFI 调用作为独立任务
- **RFC-009**：所有权模型——Move 语义、RAII、`?` 可选类型，不透明类型的生命周期管理完全基于此
- **RFC-010**：统一类型语法，`{}` 块的 return 语义
- **RFC-024**：并发模型，spawn 块中的 FFI 行为与析构安全

---

## 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| FFI 类型定义 | unsafe 块 + return | 统一语义，不需要新关键字 | 2026-06-05 |
| 不透明类型判断 | unsafe 块显式定义 | 类型属性在定义时决定，不依赖外部推断 | 2026-06-05 |
| 方法绑定 | [0] 语法 | 明确 self 位置 | 2026-06-05 |
| 构造函数 | 普通函数绑定 | 不需要特殊语法 | 2026-06-05 |
| spawn 块行为 | 资源类型自动串行 | 安全，符合并发模型 | 2026-06-05 |
| 析构函数 | `.drop = native_fn[0]` | 方法绑定 + RAII，零新概念 | 2026-06-10 |
| Null 处理 | `?T` 可选 / 直接 panic | C 的问题不隐藏 | 2026-06-10 |

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
