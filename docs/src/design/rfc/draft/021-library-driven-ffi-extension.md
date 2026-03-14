# RFC-021: 库驱动 FFI 扩展与跨语言调用支持

> **状态**: 草案
> **作者**: 晨煦
> **创建日期**: 2026-03-14
> **最后更新**: 2026-03-14

> **参考**:
> - [RFC-001: 并作模型与错误处理系统](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 并发模型与调度器脱耦设计](./008-runtime-concurrency-model.md)
> - [FFI 实现计划](../reference/plan/completed/FFI.md)

## 摘要

本文档提出一种**库驱动**的 FFI（外部函数接口）扩展方案。不同于在语言层面引入 `extern` 块或复杂的类型声明语法，本方案通过扩展标准库提供类型安全、错误处理、动态库加载等能力。具体语言的调用绑定（如 C、Python、JavaScript）由各语言维护者编写独立的库实现，YaoXiang 核心仅提供原子能力。

## 动机

### 现有实现的不足

当前 FFI 实现（RFC-FWI）已具备以下能力：
- `Native("symbol")` 语法声明外部函数
- `FfiRegistry` 函数注册表
- 缓存优化

但功能相对单一：
- 缺乏类型安全声明机制
- 没有动态库加载支持
- 错误处理能力有限
- 缺少跨语言调用的基础设施

### 设计哲学

YaoXiang 遵循 **"核心简洁，复杂性下沉到库"** 的原则：

> **好品味 (Good Taste)**: 语言的职责是提供原子能力，而非大而全的功能集。复杂性应该通过库来解决，而不是堆积在编译器中。

因此，本方案：
- ✅ **零语法变更** - 完全向后兼容
- ✅ **库即语言** - 功能通过标准库扩展
- ✅ **生态分工** - 各语言维护者负责自己的绑定库
- ✅ **渐进增强** - 开发者按需引入功能

## 提案

### 1. 核心 FFI 库增强

扩展 `std.ffi` 模块，提供以下能力：

#### 1.1 错误处理封装

```yaoxiang
import ffi

# 带错误处理的调用
result = ffi.try_call("my_add", [1, 2])
match result {
    Ok(value) => println("Success: " ++ value),
    Err(e) => println("Error: " ++ e.message)
}

# 错误转换包装器
wrapped = ffi.wrap_error(my_func, MyError)
```

#### 1.2 类型验证

```yaoxiang
import ffi

# 验证函数签名（运行时检查）
verified_add = ffi.verify_signature(my_add, [Int, Int], Int)

# 静态类型约束（编译器插件可选）
```

#### 1.3 异步调用支持

```yaoxiang
import ffi

# 异步调用外部函数
future = ffi.async_call("compute_intensive", [arg1, arg2])
result = await(future)
```

### 2. 动态库加载

#### 2.1 库加载接口

```yaoxiang
import ffi

# 加载动态库 (.so/.dll/.dylib)
lib = ffi.load_library("./libmyext.so")

# 从库中获取函数
my_func = lib.get_symbol("my_function")
result = my_func(1, 2)
```

#### 2.2 库管理

```yaoxiang
# 列出已加载的库
loaded = ffi.loaded_libraries()

# 卸载库
ffi.unload_library(lib)

# 库版本检查
ffi.check_version(lib, "1.0.0")
```

#### 2.3 符号解析

```yaoxiang
# 按名称查找符号
sin = ffi.dlsym("libm.so", "sin")

# 按序号查找（适用于 C++ mangled names）
entry = ffi.dlsym_ordinal(lib, 0)
```

### 3. 多语言绑定生态

#### 3.1 架构设计

```
┌─────────────────────────────────────────────────────────┐
│  YaoXiang 代码                                          │
│                                                         │
│  import cffi  # C 绑定库                                 │
│  import pyffi  # Python 绑定库                          │
│  import jsffi  # JavaScript 绑定库                      │
└─────────────────────────────────────────────────────────┘
         │              │              │
         ▼              ▼              ▼
┌─────────────────────────────────────────────────────────┐
│  std.ffi 核心库                                         │
│  - 错误处理                                             │
│  - 动态库加载                                           │
│  - 类型转换                                             │
└─────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────┐
│  操作系统                                              │
│  - dlopen/dlsym (Linux/macOS)                          │
│  - LoadLibrary/GetProcAddress (Windows)                 │
└─────────────────────────────────────────────────────────┘
```

#### 3.2 C 绑定库 (cffi)

由 C 语言维护者编写，提供：

```yaoxiang
import cffi

# 声明 C 函数
c_printf: (fmt: *const c_char) -> c_int = cffi.extern("printf")

# C 类型映射
cffi.map_type("c_int", Int)
cffi.map_type("c_char", UInt8)
cffi.map_type("c_void", Void)

# 结构体支持
struct Point { x: c_int, y: c_int }
```

#### 3.3 其他语言绑定

各语言维护者自行编写库：

| 语言 | 库名 | 维护者 |
|------|------|--------|
| C | cffi | C 社区 |
| Python | pyffi | Python 社区 |
| JavaScript | jsffi | Node.js 社区 |
| Java | javaffi | Java 社区 |
| Go | goffi | Go 社区 |

**YaoXiang 核心团队不负责编写这些库**，仅提供：
- FFI 核心接口规范
- 文档模板
- 最佳实践指南

### 4. 类型转换层

#### 4.1 自动转换

```yaoxiang
# YaoXiang Int → C int (自动)
result = cffi.call("my_c_func", 42)  # 42 自动转为 c_int

# C int → YaoXiang Int (自动)
```

#### 4.2 手动转换

```yaoxiang
# 显式转换
raw_ptr = cffi.to_pointer(my_bytes)
c_string = cffi.to_c_string(my_string)
```

### 5. 安全性考虑

#### 5.1 内存安全

- 外部函数调用使用独立的内存边界
- 指针参数需要显式声明 `unsafe`
- 自动边界检查可配置

#### 5.2 错误隔离

- FFI 调用错误不会导致进程崩溃
- 错误通过 Result 类型传播
- 超时机制防止外部函数死锁

```yaoxiang
# 带超时的调用
result = ffi.call_with_timeout("blocking_func", 5000)  # 5秒超时
```

## 详细设计

### 6.1 核心数据结构

```rust
// std.ffi 模块新增类型

pub struct DynamicLibrary {
    handle: *mut std::ffi::c_void,
    path: String,
}

impl DynamicLibrary {
    pub fn load(path: &str) -> Result<Self, FfiError>;
    pub fn get_symbol(&self, name: &str) -> Result<Symbol, FfiError>;
    pub fn unload(self) -> Result<(), FfiError>;
}

pub struct Symbol {
    ptr: *mut std::ffi::c_void,
    name: String,
}

pub enum FfiError {
    LibraryNotFound(String),
    SymbolNotFound(String),
    CallFailed(String),
    TypeMismatch { expected: String, actual: String },
    Timeout,
}
```

### 6.2 编译器改动

**零语法变更** - 无需修改编译器前端。

仅在解释器/运行时增加：
- 动态库加载指令
- 超时机制

### 6.3 运行时行为

- 动态库使用 `dlopen`/`LoadLibrary` 加载
- 符号解析使用 `dlsym`/`GetProcAddress`
- 函数调用通过 FFI 注册表分发

## 权衡

### 优点

- ✅ **零语法变更** - 完全向后兼容现有代码
- ✅ **库即语言** - 功能通过标准库渐进引入
- ✅ **生态分工** - 各语言维护者负责绑定库
- ✅ **安全可控** - 错误隔离和超时保护
- ✅ **灵活扩展** - 新语言只需编写绑定库

### 缺点

- ⚠️ 编译时类型检查能力有限（依赖运行时验证）
- ⚠️ 各语言绑定库质量参差不齐（由各社区负责）
- ⚠️ 动态库版本兼容性需要使用者自行管理

## 实现策略

### 阶段 1：核心库 (v0.7)

- [ ] 扩展 `std.ffi` 模块
- [ ] 实现 `try_call` / `wrap_error`
- [ ] 添加 `call_with_timeout`
- [ ] 单元测试

### 阶段 2：动态库支持 (v0.8)

- [ ] 实现 `DynamicLibrary` 结构
- [ ] 支持 Linux/macOS (dlopen)
- [ ] 支持 Windows (LoadLibrary)
- [ ] 实现 `get_symbol` / `dlsym`
- [ ] 集成测试

### 阶段 3：文档与生态 (v0.9)

- [ ] 编写 FFI 核心库文档
- [ ] 制定绑定库编写规范
- [ ] 提供 C 绑定库示例（可选）

## 与其他 RFC 的关系

- **RFC-001**: FFI 调用作为外部函数，需考虑与并发模型的交互（默认 `@block`）
- **RFC-008**: 调度器脱耦设计，FFI 调用不参与 DAG 调度
- **RFC-020**: 动态模块与 FFI 集成设计

## 开放问题

- [ ] 是否需要提供预编译的常用 C 库绑定？（如 libcurl、sqlite）
- [ ] 如何处理 C++ name mangling？（需要外部工具如 cxx-bindgen）
- [ ] 是否需要 WASM/WebAssembly 支持？

---

## 附录 A：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| 零语法变更 | 通过库实现所有功能 | 2026-03-14 | 晨煦 |
| 动态库加载 | 使用 dlopen/dlsym | 2026-03-14 | 晨煦 |
| 语言绑定分工 | 各社区负责自己的库 | 2026-03-14 | 晨煦 |
| 错误处理 | 使用 Result 类型 | 2026-03-14 | 晨煦 |

## 附录 B：示例代码

### 完整示例：使用 C 库

```yaoxiang
# 假设 cffi 库由 C 社区维护
import cffi
import ffi

# 加载 C 数学库
libm = ffi.load_library("libm.so")

# 获取函数符号
c_sin = libm.get_symbol("sin")

# 调用（类型转换自动发生）
result = c_sin(3.14159 / 2)

# 错误处理
safe_sin = ffi.wrap_error(c_sin, FfiError)
match safe_sin(1.0) {
    Ok(v) => println("sin(1) = " ++ v),
    Err(e) => println("Error: " ++ e)
}
```

### 完整示例：异步调用

```yaoxiang
import ffi
import concurrent

# 加载外部计算库
lib = ffi.load_library("./heavy_compute.so")
compute = lib.get_symbol("heavy_compute")

# 异步执行
future = ffi.async_call(compute, [input_data])

# 等待结果
result = await(future)
```

---

## 参考文献

- [RFC-001: 并作模型与错误处理系统](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime 并发模型与调度器脱耦设计](./008-runtime-concurrency-model.md)
- [FFI 实现计划](../reference/plan/completed/FFI.md)
- [Python ctypes 文档](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading crate](https://docs.rs/libloading/latest/libloading/)
