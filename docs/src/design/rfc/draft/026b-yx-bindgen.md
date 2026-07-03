---
title: "RFC-026b: yx-bindgen 工具链"
status: "草案"
author: "晨煦"
created: "2026-06-05"
updated: "2026-07-03"
group: "rfc-026"
---

# RFC-026b: yx-bindgen 工具链

> **父 RFC**: [RFC-026: FFI 核心机制](../accepted/026-ffi-core-mechanism.md)
>
> **依赖**: 本 RFC 的实现依赖 RFC-026 的核心 FFI 机制先落地。

## 摘要

yx-bindgen 将 C 头文件机械转换为 `.yx` FFI 绑定文件，生成库绑定、类型两分（不透明句柄 / 透明类型）、函数声明和方法绑定。

**两个核心原则**：
1. **机械输出草稿，不猜所有权**——所有权由 YaoXiang 类型赋予，用户根据 C 文档确认
2. **平台正确的布局担保**——透明类型的字段大小、对齐按目标平台计算，保证与 C struct 二进制匹配

## 动机

手工编写 FFI 绑定枯燥且易错——C 库有几十上百个函数。更危险的是**透明类型的布局**：手写 `Timespec: Type = { tv_sec: Int64, tv_nsec: Int64 }` 时，字段大小、对齐、padding 必须精确匹配目标平台的 C `struct timespec`，否则 C 按错误布局读写会越界（RFC-026 §2.2 的信任担保）。yx-bindgen 从 `.h` 机械计算布局，消除这个手工担保的风险。

## 提案

### 1. 用法

```bash
yx-bindgen --header /usr/include/sqlite3.h --lib sqlite3 --output sqlite3_bindings.yx
```

`--lib` 指定链接的库名，生成 `Native.c("libsqlite3")` 头。

### 2. 生成内容

yx-bindgen 生成四类内容：
- **库绑定头**：`lib = Native.c("libxxx")`
- **类型两分**：黑盒指针 → 不透明句柄；数据结构 → 透明类型（带布局）
- **函数声明**：`lib("symbol")` 绑定
- **方法绑定**：`Type.method` 或 `[N]`

### 3. 类型两分的自动判定

yx-bindgen 按 C 类型的使用方式判定归属（RFC-026 §2）：

| C 类型 | 判定 | yx-bindgen 输出 |
|--------|------|----------------|
| `typedef struct T T;`（不完整类型/仅指针使用） | 黑盒 → 不透明句柄 | `T: Type = lib("T")` |
| `struct T { fields };`（字段可见、被读写） | 数据 → 透明类型 | `T: Type = { ...布局 }` |
| `int/long/float/double` | 值 | `Int32/Int64/Float32/Float64` |
| `char*`（参数/返回） | 值（复制） | `String` |
| `void*` | 无类型 | `*Void`（系统级） |

不完整类型（`typedef struct sqlite3 sqlite3;` 无定义体）→ 必然是黑盒句柄。完整定义的 struct → 透明类型，字段布局机械计算。

### 4. 布局计算（透明类型的关键）

对完整定义的 struct，yx-bindgen 按目标平台的 ABI 计算每个字段的偏移、大小、对齐：

```c
struct timespec {
    time_t tv_sec;    // 平台相关：Linux x86_64 = 8 字节
    long   tv_nsec;   // 8 字节
};
```

```yaoxiang
// 目标平台 Linux x86_64
Timespec: Type = {
    tv_sec: Int64,    // offset 0, size 8
    tv_nsec: Int64    // offset 8, size 8
}
// 总大小 16，对齐 8 —— 与 C struct 二进制一致
```

**平台差异**：`time_t`、`long`、`size_t` 大小随平台变化。yx-bindgen 按 `--target` 选择正确映射，保证生成的透明类型与目标平台 C struct 逐字节匹配。这是消除 RFC-026 §2.2 手工布局担保风险的核心价值。

### 5. 生成示例

输入（`sqlite3.h` 抽象）：

```c
typedef struct sqlite3 sqlite3;          // 不完整 → 黑盒句柄
typedef struct sqlite3_stmt sqlite3_stmt;

int sqlite3_open(const char *filename, sqlite3 **ppDb);
int sqlite3_close(sqlite3 *db);
int sqlite3_exec(sqlite3 *db, const char *sql, ...);
```

输出（`sqlite3_bindings.yx`）：

```yaoxiang
// sqlite3_bindings.yx —— 自动生成，不要手动编辑

// ============================================================================
// 库绑定
// ============================================================================
sqlite3 = Native.c("libsqlite3")

// ============================================================================
// 类型（不完整类型 → 不透明句柄）
// ============================================================================
SqliteDb: Type = sqlite3("sqlite3")
SqliteStmt: Type = sqlite3("sqlite3_stmt")

// ============================================================================
// 函数 + 方法绑定
// ============================================================================
SqliteDb.open: (filename: String) -> ?SqliteDb = sqlite3("sqlite3_open")
SqliteDb.exec: (sql: String) -> Int32 = sqlite3("sqlite3_exec")
SqliteDb.close: () -> Int32 = sqlite3("sqlite3_close")

// ============================================================================
// 析构（可选，用户确认）
// ============================================================================
SqliteDb.drop = SqliteDb.close
```

### 6. 用户调整

生成的绑定是草稿，用户根据 C 库文档确认所有权语义（yx-bindgen 不猜）：

```yaoxiang
// 生成：默认 String（复制）—— 大多数情况正确
SqliteDb.errmsg: () -> String = sqlite3("sqlite3_errmsg")

// getenv 返回静态区，不该复制也不该接管 —— 用户改为裸指针
getenv: (name: String) -> *const U8 = Native.c("libc")("getenv")

// 库拥有的句柄，用户确认是否绑定 .drop
SqliteDb.drop = SqliteDb.close   // 生成建议，用户确认
```

**关键**：布局由 yx-bindgen 机械保证（平台正确），但所有权语义（是否 `.drop`、`char*` 是复制还是裸指针）由用户根据 C 文档确认。工具负责机械正确，用户负责语义正确。

---

## 权衡

### 优点

1. **布局担保自动化**：透明类型布局按平台机械计算，消除手工 padding/对齐错误
2. **类型两分自动判定**：不完整类型 → 句柄，完整 struct → 透明类型
3. **可审计**：输出是普通 `.yx`，用户可读、改、提交版本控制

### 缺点

1. **所有权仍需人工确认**：yx-bindgen 不猜 `.drop`、不猜 `char*` 语义
2. **C 头解析依赖**：需要 libclang 或 tree-sitter-c
3. **平台特化**：不同 `--target` 生成不同布局，跨平台包需多份或运行期选择

---

## 实现策略

- [ ] C 头文件解析（libclang）
- [ ] 类型两分判定（不完整类型 vs 完整 struct）
- [ ] 平台 ABI 布局计算（offset/size/align，按 --target）
- [ ] 代码生成（库绑定 + 类型 + 函数 + 方法）
- [ ] 集成测试（sqlite3、libcurl，多平台布局验证）

---

## 与其他 RFC 的关系

- **RFC-026**（父）：FFI 核心机制——生成的绑定使用其 `Native.c("lib")("sym")` 语法和类型两分
- **RFC-026a**：可扩展 FFI 机制——未来可扩展生成 `Native.wasm` 等其他机制的绑定

---

## 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| **定位** | 机械输出草稿，不猜所有权 | 所有权由 YaoXiang 类型赋予，用户根据 C 文档确认 | 2026-07-03 |
| **布局担保** | 按 --target 机械计算 offset/size/align | 消除手工布局的越界风险（RFC-026 §2.2） | 2026-07-03 |
| **类型判定** | 不完整类型→句柄，完整 struct→透明类型 | 对齐 RFC-026 类型两分 | 2026-07-03 |

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/draft/` | 依赖 RFC-026 先落地 |
| **已接受** | `docs/design/rfc/accepted/` | 正式设计文档 |
