---
title: "RFC-021：库驱动 FFI 扩展与跨语言调用支持"
---

# RFC-021: 库驱动 FFI 扩展与跨语言调用支持

> **状态**: 草案
> **作者**: 晨煦
> **创建日期**: 2026-03-14
> **最后更新**: 2026-05-29

> **参考**:
> - [RFC-001: 并作模型与错误处理系统](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 并发模型与调度器脱耦设计](./008-runtime-concurrency-model.md)
> - [FFI 实现计划](../reference/plan/completed/FFI.md)

## 摘要

本文档提出一种**库驱动**的 FFI（外部函数接口）扩展方案。FFI 的唯一入口是 `native("symbol")` 声明 + `FfiRegistry` 运行时注册表，核心不引入第二套机制。在此之上通过标准库提供动态库加载、跨语言调用绑定等能力。具体语言的调用绑定（如 C、Python、JavaScript）由官方工具链自动生成或由各项目按需编写。

## 动机

### 现有实现的不足

当前 FFI 实现已具备以下能力：
- `native("symbol")` 语法声明外部函数
- `FfiRegistry` 函数注册表

但功能相对单一：
- 缺乏动态库加载支持
- 没有跨语言调用的基础设施
- 缺少自动化的绑定生成工具

### 设计哲学

YaoXiang 遵循 **"核心简洁，复杂性下沉到库"** 的原则：

> **好品味 (Good Taste)**: 语言的职责是提供原子能力，而非大而全的功能集。复杂性应该通过库来解决，而不是堆积在编译器中。

因此，本方案：
- ✅ **零语法变更** — 完全向后兼容，FFI 入口只有 `native("symbol")`
- ✅ **库即语言** — 功能通过标准库扩展
- ✅ **工具链自动化** — 绑定由 `yx-bindgen` 自动生成，非手工维护
- ✅ **渐进增强** — 开发者按需引入功能

## 提案

### 1. 核心 FFI 库增强

扩展 `std.ffi` 模块。注意：所有对外部函数的调用仍然通过 `native("symbol")` 声明，`std.ffi` 只提供辅助能力。

#### 1.1 动态库加载

```yaoxiang
import ffi

# 加载动态库 (.so/.dll/.dylib)
lib = ffi.load_library("./libmyext.so")

# 从库中获取函数符号，返回 native 可用的 symbol 名
ffi.register_library_symbols(lib, [
    "my_function",
    "another_func",
])
```

`load_library` 返回 `DynamicLibrary` 句柄，`register_library_symbols` 将符号名注册到 FfiRegistry 的已知表中。之后用户仍然通过 `native` 声明使用：

```yaoxiang
my_func: (a: Int, b: Int) -> Int = native("my_function")
```

没有第二套调用语法，没有 `try_call` 包装。

#### 1.2 库管理

```yaoxiang
# 列出已加载的库
loaded = ffi.loaded_libraries()

# 卸载库
ffi.unload_library(lib)

# 库版本检查
ffi.check_version(lib, "1.0.0")
```

#### 1.3 符号解析

```yaoxiang
# 按名称查找符号（返回 Symbol 结构）
sin_sym = ffi.dlsym("libm.so", "sin")
```

跨语言调用约定和类型转换不在运行时通过通用包装处理，而是由 `yx-bindgen` 在编译时生成。

### 2. 动态库加载实现

#### 2.1 核心数据结构

```rust
pub struct DynamicLibrary {
    handle: *mut std::ffi::c_void,
    path: String,
}

impl DynamicLibrary {
    pub fn load(path: &str) -> Result<Self, FfiError>;
    pub fn get_symbol(&self, name: &str) -> Result<*mut std::ffi::c_void, FfiError>;
    pub fn unload(self) -> Result<(), FfiError>;
}
```

#### 2.2 错误类型

```rust
pub enum FfiError {
    LibraryNotFound { name: String, os_error: Option<OsError> },
    SymbolNotFound { name: String, os_error: Option<OsError> },
    CallFailed { message: String, os_error: Option<OsError> },
    Timeout,
}

pub struct OsError {
    pub code: i32,
    pub message: String,
}
```

`OsError` 携带平台原生错误码（Linux 的 `dlerror()`、Windows 的 `GetLastError()`），保证可调试。

### 3. 多语言绑定：工具链方案

放弃"社区各语言维护者编写绑定库"的幻想。改为官方工具链自动生成。

#### 3.1 架构设计

```
┌───────────────────────────────────────────────┐
│  YaoXiang 代码                                │
│                                               │
│  // 用户只写 native 声明                       │
│  my_func: (a: Int) -> Int = native("my_func") │
└───────────────────────────────────────────────┘
         ↑                          ↑
         |  编译时                   | 运行时
┌──────────────────┐   ┌────────────────────────┐
│  yx-bindgen       │   │  std.ffi + FfiRegistry  │
│  (C 头文件 → .yx) │   │  - dlopen/dlsym         │
│                   │   │  - LoadLibrary/GetProc  │
└──────────────────┘   └────────────────────────┘
```

#### 3.2 绑定生成器 (`yx-bindgen`)

`yx-bindgen` 是一个独立的 CLI 工具，从 C 头文件生成 YaoXiang FFI 绑定代码：

```bash
yx-bindgen --header /usr/include/sqlite3.h --output sqlite3.yx
```

生成结果示例：

```yaoxiang
# 自动生成，不要手动编辑
# Source: /usr/include/sqlite3.h

sqlite3_open: (filename: *const u8, ppDb: *mut *mut opaque) -> Int
    = native("sqlite3_open")

sqlite3_close: (db: *mut opaque) -> Int
    = native("sqlite3_close")

sqlite3_exec: (
    db: *mut opaque,
    sql: *const u8,
    callback: *mut opaque,
    arg: *mut opaque,
    errmsg: *mut *mut u8,
) -> Int
    = native("sqlite3_exec")
```

`yx-bindgen` 由官方维护，保证：
- 类型映射完整（`int` → `Int`、`char*` → `*const u8`、`void*` → `*mut opaque`）
- 结构体布局对齐（自动 `#[repr(C)]` 等价物）
- callback 签名转换

#### 3.3 官方维护的绑定包

YaoXiang 核心团队不承诺维护所有语言的通用绑定库，但提供一个官方的 `libc` 绑定包（POSIX + Windows API 子集），作为 FFI 最佳实践示例和基础能力。

其他语言和库的绑定：
- 使用 `yx-bindgen` 自行生成
- 可发布为 YaoXiang 包（如 `libsqlite3`、`libcurl`、`libsdl2`）
- 核心团队不负责维护，但提供包的发布和版本管理机制

### 4. 类型转换层

#### 4.1 编译时类型映射

类型转换不通过运行时包装器，而是在 `yx-bindgen` 生成时静态决定：

| C 类型 | YaoXiang 类型 | 转换方式 |
|--------|---------------|----------|
| `int` | `Int` | 直接传值 |
| `char*` | `*const u8` | 指针传递 |
| `void*` | `*mut opaque` | 不透明指针 |
| `struct T` | `extern struct T` | 内存布局匹配 |
| `int*` | `*mut Int` | 指针传递（可变） |
| `const int*` | `*const Int` | 指针传递（只读） |

#### 4.2 手动转换（标准库辅助）

```yaoxiang
# 显式转换
raw_ptr = ffi.to_pointer(my_bytes)
c_string = ffi.to_c_string(my_string)
```

### 5. 内存所有权模型

#### 5.1 基本原则

跨 FFI 边界的每笔内存分配必须明确回答两个问题：
1. **谁分配？** （C 侧 `malloc` / YaoXiang 侧运行时）
2. **谁释放？** （C 侧 `free` / YaoXiang 侧运行时）

`yx-bindgen` 生成时，对常见模式添加注解：

```yaoxiang
# C 分配，调用者释放
sqlite3_exec: (...) -> Int
    = native("sqlite3_exec")
    # memory: C-allocated, caller must free errmsg via sqlite3_free

# 调用者分配指针
read: (fd: Int, buf: *mut u8, count: Int) -> Int
    = native("read")
    # memory: caller-allocated buf
```

运行时对跨 FFI 边界的指针引用不做自动内存管理——所有权明确落在调用者身上。

#### 5.2 字符串处理

C 函数返回的 `char*` 在被转换为 YaoXiang `String` 时立即复制。原指针的所有权由 C 函数决定（通过注解声明），不自动释放。

### 6. 安全性考虑

#### 6.1 并发安全

FFI 函数调用**默认不参与 DAG 调度**，视为阻塞操作。确认为 reentrant 的 C 函数可标记为 `@concurrent`：

```yaoxiang
# 纯函数，无全局状态，可并发
sin_safe: (x: Float) -> Float = native("sin")
    # reentrant: true

# 有全局状态，不能并发
strtok: (s: *const u8, delim: *const u8) -> *const u8 = native("strtok")
    # reentrant: false
```

`yx-bindgen` 对标准 C 库函数尽可能标注 reentrancy 信息（`strtok` 的 `_r` 变体等）。

**对异步调用者的要求：** 调用 FFI 函数前，调用者必须确保目标函数是 reentrant 的。运行时不做自动检测——这是不可能静态解决的问题。

#### 6.2 错误隔离

- FFI 调用错误通过 `Result` 类型传播（如果函数声明了 Result 返回类型）
- 超时机制防止外部函数死锁

```yaoxiang
# 带超时的调用（在 FfiRegistry 层实现）
result = ffi.call_with_timeout("blocking_func", 5000)  # 5秒超时
```

#### 6.3 指针安全

- 指针参数需要 YaoXiang 侧的 `unsafe` 标记
- 跨 FFI 边界的指针生命周期由调用者保证

### 7. 编译器改动

**零语法变更** — 只需 `native("symbol")` 声明，已在当前编译器实现。

在解释器/运行时增加：
- 动态库加载指令（`DynamicLibrary` 的 FFI 绑定）
- 超时机制

### 8. 不被通过的功能

以下功能经审核后明确排除，不再纳入 RFC：

- **`ffi.try_call`**: 冗余，已有 `native` + `Result` 返回类型
- **`ffi.verify_signature`**: 运行时做编译器的事，是错误的抽象层级
- **`ffi.async_call`**: 需要等 reentrancy 契约模型明确后再考虑
- **社区维护绑定表**: 不可执行，改为 `yx-bindgen` 工具链方案

## 权衡

### 优点

- ✅ **零语法变更** — FFI 入口只有 `native("symbol")`，完全向后兼容
- ✅ **库即语言** — 功能通过标准库渐进引入
- ✅ **工具链驱动** — `yx-bindgen` 自动处理绑定生成
- ✅ **内存安全** — 所有权模型明确，无自动回收导致的 use-after-free
- ✅ **可调试** — 错误携带 OS 原生错误码

### 缺点

- ⚠️ 类型安全受限于 C 头文件的表达能力（`void*` 无法静态区分）
- ⚠️ `yx-bindgen` 需要持续维护以跟进 C 标准演进
- ⚠️ 非 C 语言（Python/JS/Java）的绑定需要各项目自行处理，无统一方案

## 实现策略

### 阶段 1：核心库 (v0.7)

- [ ] 扩展 `std.ffi` 模块
- [ ] 实现 `DynamicLibrary` 结构
- [ ] 支持 Linux/macOS (`dlopen`/`dlsym`)
- [ ] 支持 Windows (`LoadLibrary`/`GetProcAddress`)
- [ ] 在运行时添加超时机制
- [ ] 单元测试

### 阶段 2：yx-bindgen (v0.8)

- [ ] 实现 C 头文件解析器（基于已有的 Clang 绑定或手写 parser）
- [ ] 类型映射系统
- [ ] 生成 `native("symbol")` 声明
- [ ] 生成结构体布局
- [ ] 集成测试：对 SQLite3、libcurl 等真实 C 库生成绑定

### 阶段 3：生态基础 (v0.9)

- [ ] 发布官方 `libc` 绑定包（POSIX + Windows API 子集）
- [ ] 制定绑定包发布规范
- [ ] 文档：FFI 最佳实践、内存所有权、并发安全契约

## 与其他 RFC 的关系

- **RFC-001**: FFI 调用作为外部函数，默认 `@block`（不参与 DAG 调度）
- **RFC-008**: 调度器脱耦设计，FFI 调用作为独立任务
- **RFC-020**: FFI 节点在 DAG 中的调度语义、Phi 节点、循环展开等调度层面的详细设计

## 开放问题

- [ ] `yx-bindgen` 是否需要集成到构建系统（`yaoxiang build`）中？
- [ ] WASM 平台的 FFI 支持如何设计？（WASM 的导入机制与 dlopen 完全不同）
- [ ] 是否需要提供 `cxx-bindgen` 处理 C++ name mangling（可选，v1.0 后考虑）

---

## 附录 A：设计决策记录

| 决策 | 决定 | 原因 | 日期 | 记录人 |
|------|------|------|------|--------|
| FFI 入口唯一化 | 只保留 `native("symbol")` | 避免 API 分裂 | 2026-05-29 | 晨煦 |
| 排除 `try_call` | 不实现 | 冗余，已有 Result 类型 | 2026-05-29 | 晨煦 |
| 排除 `verify_signature` | 不实现 | 运行时做编译器的事 | 2026-05-29 | 晨煦 |
| 社区维护绑定 → 工具链 | `yx-bindgen` 自动生成 | 不可执行的幻想 | 2026-05-29 | 晨煦 |
| OS 错误码 | `FfiError` 必带 `os_error` | 不可调试的 API 是无用的 | 2026-05-29 | 晨煦 |
| 零语法变更 | 依赖库实现 | 核心简洁原则 | 2026-03-14 | 晨煦 |
| 动态库加载 | 使用 dlopen/dlsym | 标准 OS 接口 | 2026-03-14 | 晨煦 |
| 错误处理 | 使用 Result 类型 | 一致性 | 2026-03-14 | 晨煦 |

## 附录 B：示例代码

### 完整示例：使用 C 库

```yaoxiang
# 加载 C 数学库
libm = ffi.load_library("libm.so")

# 将 C 符号注册到运行时表（yx-bindgen 会在编译时做这个）
ffi.register_library_symbols(libm, ["sin", "cos", "sqrt"])

# 通过 native 声明使用
sin_f: (x: Float) -> Float = native("sin")
cos_f: (x: Float) -> Float = native("cos")

# 直接调用
result = sin_f(3.14159 / 2)

# 调用可能失败的 C 函数时使用 Result
file_open: (path: *const u8, mode: *const u8) -> Result(*mut opaque, Int)
    = native("fopen")
```

### 使用 yx-bindgen

```bash
# 自动生成所有声明，不需要手写
yx-bindgen --header /usr/include/math.h --output math_bindings.yx

# 在 YaoXiang 中引入
import "math_bindings.yx"
# sin_f / cos_f 等已自动声明为 native("sin") / native("cos")
```

---

## 参考文献

- [RFC-001: 并作模型与错误处理系统](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime 并发模型与调度器脱耦设计](./008-runtime-concurrency-model.md)
- [FFI 实现计划](../reference/plan/completed/FFI.md)
- [Python ctypes 文档](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading crate](https://docs.rs/libloading/latest/libloading/)
