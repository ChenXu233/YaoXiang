---
title: "RFC-026：FFI 核心机制"
status: "已接受"
author: "晨煦"
created: "2026-07-03"
updated: "2026-07-05"
issue: "#93"
---

# RFC-026：FFI 核心机制

> **参考**:
> - [RFC-007: 函数定义语法统一方案](./007-function-syntax-unification.md)
> - [RFC-009: 所有权模型设计](./009-ownership-model.md)
> - [RFC-010: 统一类型语法](./010-unified-type-syntax.md)
> - [RFC-024: 基于 spawn 块的并发模型](./024-concurrency-model.md)

> **废弃**:
> - [RFC-020: 动态模块与 FFI 集成](../deprecated/020-dynamic-modules-ffi.md) — 内容已合并到本文档
> - [RFC-021: 库驱动 FFI 扩展与跨语言调用支持](../deprecated/021-library-driven-ffi-extension.md) — 内容已合并到本文档

> **子 RFC**:
> - [RFC-026a: 可扩展 FFI 机制体系](../review/026a-extensible-ffi-system.md) — 多 ABI 机制插件、`FfiMechanism` 抽象、动态加载
> - [RFC-026b: yx-bindgen 工具链](../draft/026b-yx-bindgen.md) — C 头文件 → `.yx` 绑定代码生成

## 摘要

本文档定义 YaoXiang 的 FFI（外部函数接口）核心机制。核心思想：**外部库是编译期链接的一等值，跨界数据的内存布局归属在类型定义时钉死，YaoXiang 的堆对象与外部代码结构性隔离。**

1. **外部库即值**：`Native.c("libsqlite3")` 编译期 link 库并返回一个解析器，用柯里化承载库信息
2. **外部符号即值**：解析器应用符号名得到外部引用，通过 `name: type = value`（RFC-007/010）绑定为类型或函数
3. **类型两分**：不透明句柄（布局归外部）/ 透明类型（布局归 YaoXiang），没有第三种
4. **编组隔离**：跨界数据默认复制到调用临时区，YaoXiang 堆对象与外部代码隔离
5. **所有权安全**：句柄唯一所有权（Move）+ RAII，结构性杜绝双重释放与 use-after-free
6. **逃生舱**：`*T` 裸指针 + `unsafe {}`，用户显式接受零复制的直接内存访问风险

**核心边界——五条不可违反的契约**：

```
1. 库编译期 link，符号编译期验证存在
2. 类型布局归属在定义时确定：不透明句柄归外部，透明类型归 YaoXiang
3. 默认编组走临时区复制，YaoXiang 堆对象与外部代码隔离
4. 句柄唯一所有权 + Move，结构性防双重释放/悬垂
5. 外部代码永远读写"布局明确、所有权明确"的内存，不存在模糊地带
```

---

## 动机

### 现状与目标

当前代码库的 `native("symbol")` 只是 YaoXiang 字节码调用 Rust std 函数的分发机制（`FfiRegistry` = `HashMap<String, RustFnPtr>`），**没有任何真正的跨 ABI 边界**——没有 dlopen、没有 C ABI 编组、没有内存所有权跨界。

真正的 FFI 必须解决四个问题：

| 问题 | 本 RFC 的答案 |
|------|--------------|
| **符号解析** | 库是编译期 link 的一等值（`Native.c("lib")`），符号编译期验证 |
| **值编组** | 签名驱动，编译期为每个参数位置确定转换规则 |
| **内存所有权** | 类型两分决定归属；默认复制隔离 |
| **生命周期安全** | Move + RAII + 借用限定单次调用 |

RFC-020 和 RFC-021 分别定义了 FFI 的不同方面，两者有重叠，本文档整合成统一规范。

### 设计目标

1. **零裸指针泄漏到用户代码**：常规 FFI 使用中，`.yx` 源码里不出现裸指针
2. **布局归属显式**：用户定义类型时就决定这块内存归谁，不靠运行时推断
3. **结构性安全**：不泄漏、不双重释放、不 use-after-free 由类型系统保证，不靠约定
4. **诚实的信任边界**：C 无法提供编译期可验证的类型契约，信任局部化在绑定声明处
5. **自举兼容**：不做宿主语言独有的过度抽象

### 不在范围内

- **多 ABI 机制插件体系**（Wasm/Python/自定义 ABI）：见 RFC-026a
- **yx-bindgen 工具链**：见 RFC-026b
- **YaoXiang 导出函数给 C 调用（反向 FFI）**：后续 RFC，本文档只声明原则
- **内联汇编、SIMD intrinsics**：不在此 RFC 范围内

---

## 提案

### 1. 外部库与符号：柯里化的一等值

FFI 的信息缺口——"链接哪个库、哪个符号"——通过让库成为一等值来填补，不引入任何新关键字。

#### 1.1 库即值

```yaoxiang
// Native.c 应用库名 → 编译期 link 该库，返回一个符号解析器
sqlite3 = Native.c("libsqlite3")
```

`Native.c("libsqlite3")` 是一个**编译期动作 + 运行期值**：

- **编译期**：链接器 `-lsqlite3`，库进入符号表，符号存在性可验证
- **值**：`sqlite3` 是一个解析器，应用符号名得到该库的外部引用

`.c` 是 ABI 机制标签（C ABI）。核心只内置 `.c`；其他机制（`.wasm` 等）见 RFC-026a。

#### 1.2 符号即值，绑定即 `name: type = value`

解析器应用符号名，得到的外部引用通过 RFC-007/010 的统一语法绑定。**LHS 的类型注解决定这个引用是类型还是函数**：

```yaoxiang
sqlite3 = Native.c("libsqlite3")

// LHS 是 Type → 绑定为不透明类型
SqliteDb: Type = sqlite3("sqlite3")

// LHS 是函数签名 → 绑定为函数
SqliteDb.open: (file: String) -> ?SqliteDb = sqlite3("sqlite3_open")
SqliteDb.exec: (sql: String) -> Int32 = sqlite3("sqlite3_exec")
SqliteDb.close: () -> Int32 = sqlite3("sqlite3_close")

// .drop 是普通方法绑定（RFC-009 RAII 约定）
SqliteDb.drop = SqliteDb.close
```

编译期验证：`sqlite3("sqlite3_open")` 中 `sqlite3_open` 必须在 `libsqlite3` 符号表中，否则编译错误。

#### 1.3 方法绑定与 self 位置

`Type.method: (...) -> ...` 的写法里，`self` 隐式在第一位——`db.exec("SELECT")` 调用时，`db` 作为 C 函数 `sqlite3_exec` 的第 0 个参数。

若需要绑定一个已声明的独立函数为方法，用 `[N]` 语法指定 self 位置（RFC-004 柯里化多位置绑定）：

```yaoxiang
// 独立函数
sqlite3_close_v2: (db: SqliteDb) -> Int32 = sqlite3("sqlite3_close_v2")

// 绑定为方法，[0] 指 db 是 self
SqliteDb.soft_close = sqlite3_close_v2[0]
```

`Native.c(...)` 直接方法绑定与 `[N]` 手动绑定都是 `name: type = value`，都是把一个函数值放到 `=` 右边，没有两套机制。

#### 1.4 用户使用：零 unsafe、零裸指针

```yaoxiang
import sqlite3_bindings

db = SqliteDb.open("test.db")
db.exec("SELECT * FROM users")
// ← 作用域结束，RAII 自动调 SqliteDb.drop → sqlite3_close(db)
```

---

### 2. 类型两分：布局归属在定义时钉死

外部数据进入 YaoXiang，只问一个问题：**这块内存的布局，谁说了算？**

```
├─ 布局是外部的黑盒（sqlite3、FILE*、socket fd）
│   → 不透明句柄  =  lib("symbol")
│   → YaoXiang 只持有指针，永不解引用，只在库函数间传递
│   → 外部代码读它自己的内存，YaoXiang 不碰
│
└─ 布局是 YaoXiang 定义的（timespec、point、要读字段的 struct）
    → 透明类型  =  { field: Type, ... }
    → YaoXiang 拥有内存、定义布局、读写字段
    → 外部代码往 YaoXiang 定义好布局的内存里填/读
```

**没有第三种。** 之前设计中的"三层内存模式（复制/接管/系统级）"是打补丁思维——真相是布局归属的二分。

#### 2.1 不透明句柄：布局归外部

```yaoxiang
SqliteDb: Type = sqlite3("sqlite3")
```

- YaoXiang 内部只持有一个指针大小的句柄
- 用户不能构造（`SqliteDb {}` → 编译错误）、不能访问字段（没有字段可访问）
- 唯一来源：返回 `SqliteDb` 的外部函数
- 调用方法时把句柄借回给库，库读它**自己的**内存（`sqlite3` 结构体在库的堆上）

外部代码"读内部"读的是它自己分配的结构，YaoXiang 只是搬运句柄。无内存冲突。

#### 2.2 透明类型：布局归 YaoXiang

```yaoxiang
// 字段有意义、需要读写 → 透明类型，布局由 YaoXiang 声明
Timespec: Type = {
    tv_sec: Int64,
    tv_nsec: Int64
}
clock_gettime: (clk: Int32, ts: *Timespec) -> Int32 = Native.c("librt")("clock_gettime")

ts = clock_gettime(CLOCK_REALTIME)   // 见 §3，编组走临时区
print(ts.tv_sec)                      // YaoXiang 按自己的字段定义读
```

外部代码往一块**YaoXiang 定义布局、YaoXiang 拥有**的内存里读写。布局是 YaoXiang 的契约，不是外部的。

#### 2.3 判断规则

用户只需判断一件事：**这个类型的字段我要不要读？**

| 判断 | 类型 | 布局归属 |
|------|------|---------|
| 不读字段，只在库函数间传句柄 | 不透明句柄 `= lib("sym")` | 外部 |
| 要读写字段 | 透明类型 `{ ... }` | YaoXiang |

---

### 3. 编组：签名驱动，临时区隔离

跨界数据转换由**签名驱动**，编译期为每个参数位置确定转换规则。**核心安全保证：外部代码读写的是编组临时区，不是 YaoXiang 的堆对象。**

#### 3.1 默认走临时区复制

```
YaoXiang → C（入参）：
    复制数据到调用临时区 → 传临时区指针给 C
    → C 越界/写坏只伤临时区，YaoXiang 堆对象隔离

C → YaoXiang（返回/输出参数）：
    C 写临时区 → YaoXiang memcpy 回自己的对象
    → C 碰不到 YaoXiang 的最终对象
```

**外部代码永远读写编组临时区，与 YaoXiang 堆对象完全隔离。** 布局声明错、C 存了指针悬垂、C 越界——都只伤临时区，YaoXiang 的对象完好。代价是一次 memcpy。

#### 3.2 编组规则表

**入参方向（YaoXiang → C）**：

| YaoXiang 类型 | C 表示 | 编组动作 | 所有权 |
|--------------|--------|---------|--------|
| `Int32/Int64/Float` | `int/long/double` | 直接放寄存器，零转换 | 值语义 |
| `String` | `const char*` | 借出只读视图（临时，调用期间有效） | YaoXiang 保留，C 只读 |
| 透明类型 | `struct T*` | 复制到临时区，传临时区指针 | YaoXiang 拥有对象，C 读副本 |
| 不透明句柄 | `void*` | 取出内部句柄指针 | YaoXiang 持有，借给 C |
| `*T` | `T*` | 直接传裸指针（unsafe） | 用户负责 |

**返回方向（C → YaoXiang）**：

| C 返回 | YaoXiang 类型 | 编组动作 | 所有权 |
|--------|--------------|---------|--------|
| `int/double` | `Int32/Float` | 直接读寄存器 | 值语义 |
| `char*` | `String` | strlen + memcpy 到 YaoXiang String | YaoXiang 拥有副本，原内存不碰 |
| `struct T*`（新建句柄） | 不透明句柄 | 句柄存入 YaoXiang 对象 | YaoXiang 接管 |
| `struct T`（值/输出参数） | 透明类型 | C 写临时区 → memcpy 回 YaoXiang | YaoXiang 拥有 |
| `char*`（静态区） | `*const U8` | 存裸指针，不复制（unsafe 读） | 不接管，用户负责 |

#### 3.3 借用生命周期：严格限定单次调用

YaoXiang 借给外部代码的指针（String 只读视图、透明类型临时区、句柄），**生命周期严格限定在单次调用内**：

- 调用期间：指针有效，外部代码可读写
- 调用返回后：借用立即失效

外部代码若存下指针留到调用后使用，是外部代码违反 FFI 标准契约（等同库 bug），YaoXiang 不为此负责。这与所有语言的 C FFI 契约一致（Rust 的 `&T` 传给 C 同样约束）。

#### 3.4 String 永不交出持久指针

`String` 是"C 不插手 YaoXiang 内存"的关键：

- 进 C：借出**临时只读视图**，调用期间有效
- 出 C：strlen + memcpy 进 YaoXiang 拥有的**副本**

C 永远拿不到 YaoXiang String 的持久指针，YaoXiang 永远不持有 C `char*` 的长期引用。结构上隔离。

---

### 4. 所有权与生命周期：Move + RAII

不透明句柄遵循 RFC-009 的所有权模型，零新概念。

#### 4.1 核心原则

- **Move 语义**：不透明句柄默认 Move，赋值/传参/返回 = 所有权转移，不可复制
- **句柄唯一所有权**：任何时刻一个句柄只有一个所有者 → 结构性杜绝双重释放
- **RAII 释放**：作用域结束时，若绑定了 `.drop`，自动调用
- **消费追踪**：显式析构或 Move 后变量被消费，不可再用 → 杜绝 use-after-free

#### 4.2 `.drop` 是可选的外部副作用

```yaoxiang
SqliteDb.drop = SqliteDb.close     // 作用域结束调 sqlite3_close
```

**`.drop` 不是防 YaoXiang 泄漏的机制**——YaoXiang 侧的句柄存储（一个指针大小的值）自动回收，与 `.drop` 无关。`.drop` 是**作用域结束时顺带调一个外部函数**的可选副作用：

- 绑了 `.drop` → 作用域结束调它（清理外部资源）
- 不绑 `.drop` → 什么都不做，**不报错、不警告**

外部资源是否需要清理，是外部库的规范问题（`getenv` 返回静态区不该释放、全局单例不该释放），YaoXiang 不越权强制。防泄漏靠 Move + 唯一所有权（无条件、结构性），不靠 `.drop`。

#### 4.3 自动析构与顺序

```yaoxiang
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT * FROM users")
    // ← 作用域结束，逆序自动析构（有 .drop 才调）：
    //   stmt.drop()  → sqlite3_finalize(stmt)
    //   db.drop()    → sqlite3_close(db)
}
```

析构顺序：定义顺序的逆序，与 RAII 一致。

#### 4.4 Move 与消费

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move：所有权转移
db.exec("...")          // ❌ 编译错误：db 已被 Move，消费后不可读

process_db: (db: SqliteDb) -> Void = {
    db.exec("...")
    // ← 函数结束，db 在此析构
}
process_db(some_db)     // Move 进函数
// some_db 在此已无效
```

#### 4.5 Null 处理

```yaoxiang
// 可能返回 null → ?T，用户必须处理
SqliteDb.open: (file: String) -> ?SqliteDb = sqlite3("sqlite3_open")

db = SqliteDb.open("test.db")
match db {
    Some(db) => db.exec("SELECT 1"),
    None => print("打开失败")
}

// 约定不返回 null → 不标记，null 时 panic 暴露
```

C 返回 null 要么用户处理（`?T`），要么 panic 暴露。没有第三种"默默忽略"。

#### 4.6 析构失败处理

`.drop` 绑定的函数返回值决定行为：

| `.drop` 返回类型 | 行为 |
|----------------|------|
| `Void` | 无失败 |
| `Int32`（错误码） | 非 0 时 panic——析构失败意味状态异常，暴露优于静默 |
| `?Error` | 非 None 时 panic——同上 |

析构失败不可静默。需要忽略特定错误时，在 `.drop` 绑定的包装函数中显式处理。

---

### 5. spawn 块中的 FFI 行为

资源类型判定由 `.drop` 绑定决定（RFC-024），零额外标记：

| 判定 | 行为 |
|------|------|
| 不透明句柄绑定了 `.drop` | 资源类型——spawn 块中同一实例操作自动串行 |
| 不透明句柄未绑定 `.drop` | 非资源类型——可并行（纯数据句柄，无释放副作用） |
| 透明类型 / 值类型 | 非资源类型——可并行 |

```yaoxiang
SqliteDb.drop = SqliteDb.close   // → 资源类型

(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // 同一实例，自动串行
    r2 = db.exec("INSERT ...")    // 等待 r1
}

(x, y) = spawn {
    db1 = SqliteDb.open("a.db"),   // 不同实例，可并行
    db2 = SqliteDb.open("b.db")
}
```

有 `.drop` 的类型在 spawn 中自动串行化同一实例操作，保证析构无并发竞争。

---

### 6. 逃生舱：裸指针 + unsafe

默认编组走临时区复制，安全但有 memcpy 开销。性能敏感场景（大结构、高频调用）需要零复制时，用户显式走裸指针逃生舱：

```yaoxiang
// C 直接读 YaoXiang 内存，零复制——用户显式接受风险
ptr: *const U8 = Native.c("libc")("getenv")("HOME")
unsafe {
    value = read_c_string(ptr)   // 用户担保 ptr 有效
}
```

**`unsafe` 只用于裸指针操作，与不透明句柄、透明类型完全正交。** 常规 FFI（句柄 + 透明类型）不需要 unsafe。写 `unsafe {}` = 用户明确签字接受直接内存访问的风险。

**信任边界**：C 无法提供编译期可验证的类型契约（`.h` 不是 ABI 契约，符号表只有名字没签名）。所以 C 签名的正确性无法自动验证——由绑定作者在写 `Native.c(...)` + 签名时担保。**信任局部化在绑定声明处**：绑定作者担保，包用户拿到安全 API。这与 Rust 的 `extern "C"` 一致（写 extern 是信任行为，安全 wrapper 包起来后调用安全）。

---

## 权衡

### 优点

1. **信息完整**：库编译期 link，符号编译期验证，无运行期"找不到库"的模糊
2. **布局归属显式**：类型两分，定义时钉死，无运行时推断
3. **结构性安全**：临时区隔离 + Move + RAII，外部代码碰不到 YaoXiang 堆对象
4. **零新关键字**：`Native.c` 柯里化 + `name: type = value`，全部复用现有语法
5. **诚实的边界**：不假装能验证 C 签名，把信任局部化在声明处

### 缺点

1. **memcpy 开销**：默认编组复制，大结构高频调用需显式走逃生舱
2. **布局担保是手工的**：透明类型布局与 C struct 匹配由绑定作者/yx-bindgen 保证
3. **C 签名不可编译期验证**：FFI 的根本限制，YaoXiang 无法从 C 消除

---

## 实现策略

### 阶段 1：外部库与符号 (v0.8)

- [ ] 实现 `Native.c("lib")` 编译期 link + 返回解析器值
- [ ] 实现符号解析器应用（`lib("symbol")`）+ 编译期符号表验证
- [ ] 实现类型两分（不透明句柄 / 透明类型）
- [ ] 实现方法绑定（直接绑定 + `[N]` 位置绑定）

### 阶段 2：编组与安全 (v0.8)

- [ ] 实现签名驱动的编组代码生成
- [ ] 实现临时区复制隔离（入参复制、返回 memcpy）
- [ ] 实现 String 临时只读视图 + 返回复制
- [ ] 实现借用生命周期限定单次调用

### 阶段 3：所有权与生命周期 (v0.9)

- [ ] 实现不透明句柄 Move + 唯一所有权
- [ ] 实现 `.drop` RAII 自动析构（可选，缺失不报错）
- [ ] 实现消费追踪（Move 后禁用）
- [ ] 实现 `?T` 与 null 返回集成
- [ ] 实现 spawn 资源类型串行

### 后续工作

- **可扩展 FFI 机制**（RFC-026a）：`FfiMechanism` 抽象、`.wasm`/`.python` 等插件、动态加载
- **yx-bindgen**（RFC-026b）：C 头文件 → `.yx` 绑定 + 平台正确的布局生成

---

## 与其他 RFC 的关系

- **RFC-004**：柯里化多位置绑定——`[N]` 方法绑定语法的来源
- **RFC-007**：函数定义语法统一——`Native.c(...)` 绑定即 `name: type = value`
- **RFC-009**：所有权模型——Move、RAII、`?T`，句柄生命周期完全基于此
- **RFC-010**：统一类型语法——LHS 类型注解决定绑定为类型还是函数
- **RFC-024**：并发模型——spawn 中资源类型判定基于 `.drop`
- **RFC-020/021**（已废弃）：内容合并到本文档
- **RFC-026a**：可扩展 FFI 机制体系
- **RFC-026b**：yx-bindgen 工具链

---

## 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| **库即值** | `Native.c("lib")` 柯里化返回解析器 | 库信息成为编译期可见的一等值，填补"链哪个库"缺口，零新关键字 | 2026-07-03 |
| **编译期 link** | `Native.c("lib")` 触发 `-llib` | 符号表编译期可读，符号存在性可验证，类型是实的 | 2026-07-03 |
| **类型两分** | 不透明句柄 / 透明类型 | 布局归属二分覆盖全部；删掉"三层内存模式"补丁 | 2026-07-03 |
| **编组临时区隔离** | 默认复制，堆对象与外部隔离 | 外部越界/悬垂只伤临时区，YaoXiang 对象完好；零复制需显式逃生舱 | 2026-07-03 |
| **`.drop` 可选** | 缺失什么都不做，不报错 | YaoXiang 句柄存储自动回收；外部资源清理是外部规范，不越权强制 | 2026-07-03 |
| **防泄漏机制** | Move + 句柄唯一所有权（无条件） | 结构性保证，与 `.drop` 无关 | 2026-07-03 |
| **信任边界** | 在 `Native.c(...)` 声明处 | C 签名不可编译期验证，信任局部化，unsafe 只用于裸指针 | 2026-07-03 |
| **Null 处理** | `?T` 或 panic | C 的问题不隐藏，无"默默忽略"选项 | 2026-07-03 |
| **析构失败** | `.drop` 返回类型决定，统一 panic | 析构失败不可静默 | 2026-07-03 |

---

## 参考文献

### YaoXiang 官方文档

- [RFC-004 柯里化多位置绑定](./004-curry-multi-position-binding.md)
- [RFC-007 函数定义语法统一](./007-function-syntax-unification.md)
- [RFC-009 所有权模型](./009-ownership-model.md)
- [RFC-010 统一类型语法](./010-unified-type-syntax.md)
- [RFC-024 并发模型](./024-concurrency-model.md)
- [RFC-026a 可扩展 FFI 机制体系](../review/026a-extensible-ffi-system.md)
- [RFC-026b yx-bindgen 工具链](../draft/026b-yx-bindgen.md)

### 外部参考

- [Rust FFI (Nomicon)](https://doc.rust-lang.org/nomicon/ffi.html)
- [Python ctypes](https://docs.python.org/3/library/ctypes.html)
- [LuaJIT FFI](https://luajit.org/ext_ffi.html)

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **审核中** | `docs/design/rfc/review/` | 开放社区讨论 |
| **已接受** | `docs/design/rfc/accepted/` | 正式设计文档 |
