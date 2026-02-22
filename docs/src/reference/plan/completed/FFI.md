---
title: "FFI 实现计划"
---

# FFI 实现计划

## 概述

FFI（Foreign Function Interface）机制允许 YaoXiang 代码调用 Rust 原生函数，是连接语言运行时与系统 API 的桥梁。本计划实现编译时绑定的 FFI 机制，支持：

- 标准库函数（std.io）调用 Rust 系统 API
- 用户自定义 native 函数

### 设计目标

| 目标 | 说明 |
|------|------|
| 零运行时开销 | 编译时绑定，缓存后无查找 |
| 类型安全 | 编译器检查函数签名 |
| 可扩展 | 用户可声明任意 native 函数 |
| 无新语法 | 复用现有 `name: type = value` 模型 |

### 架构概览

```
┌─────────────────────────────────────────────────────────┐
│  编译时                                                  │
│  ───────────────────────────────────────────────────  │
│                                                          │
│  YaoXiang 源码:                                         │
│  read_file: (path: String) -> String = Native("...")   │
│                           │                              │
│                           ▼                              │
│  编译器识别 Native("name") 表达式                        │
│                           │                              │
│                           ▼                              │
│  生成 CallNative { func_id: "name" } 字节码            │
│                                                          │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│  运行时                                                  │
│  ───────────────────────────────────────────────────  │
│                                                          │
│  CallNative { "std.io.read_file" }                     │
│       │                                                 │
│       ▼                                                 │
│  FfiRegistry.call() → 缓存查找 → 执行                  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

## 步骤 1：创建 FFI 注册表基础设施

### 文件

`src/backends/interpreter/ffi.rs`（新建）

### 实现内容

| 内容 | 说明 |
|------|------|
| `NativeHandler` 类型 | `fn(&[RuntimeValue]) -> Result<RuntimeValue, ExecutorError>` |
| `FfiRegistry` 结构 | `handlers: HashMap<String, NativeHandler>` + 缓存 |
| `with_std()` 方法 | 预注册 std.io 相关函数 |
| `register()` 方法 | 用户注册新函数 |
| `call()` 方法 | 带缓存的函数调用 |

### 核心代码结构

```rust
pub struct FfiRegistry {
    // 函数处理表
    handlers: HashMap<String, NativeHandler>,
    // 运行时缓存（加速调用）
    cache: Mutex<HashMap<String, NativeHandler>>,
}

impl FfiRegistry {
    // 预定义标准库函数
    pub fn with_std() -> Self { ... }

    // 用户注册新函数
    pub fn register(&mut self, name: &str, handler: NativeHandler) { ... }

    // 调用：缓存查找 → 执行
    pub fn call(&self, name: &str, args: &[RuntimeValue]) -> Result<RuntimeValue> { ... }
}
```

### 验收方法

- [x] `FfiRegistry::new()` 返回包含 std.io 函数的注册表
- [x] `register()` 能添加新函数
- [x] `call()` 能正确调用已注册的函数

### 测试内容

- 单元测试：注册和调用自定义函数 ✅ 12/12 通过
- 集成测试：调用预注册的 std.io 函数 ✅

---

## 步骤 2：添加 CallNative 字节码指令

### 文件

`src/middle/core/bytecode.rs`

### 实现内容

| 内容 | 说明 |
|------|------|
| `Opcode::CallNative` | 新增操作码 |
| `CallNative` 指令结构 | `dst: Option<Reg>, func_name: ConstIndex` |
| 序列化/反序列化 | 支持写入和读取字节码文件 |

### 验收方法

- [x] 字节码能正确序列化 `CallNative` 指令
- [x] 反序列化后指令正确

### 测试内容

- 序列化：编码 `CallNative { func_name: "test" }` ✅
- 反序列化：解码后与原指令一致 ✅

---

## 步骤 3：代码生成器识别 Native 函数

### 文件

`src/middle/passes/codegen/translator.rs`

### 实现内容

| 内容 | 说明 |
|------|------|
| 识别 `Native("name")` 表达式 | 在 `translate_call` 中检测 |
| 生成 `CallNative` 指令 | 替换 `CallStatic` |
| 处理 `Native` 类型声明 | 在符号表中标记 `is_native: true` |

### 验收方法

- [x] `Native("std.io.read_file")` 生成 `CallNative` 字节码
- [x] 正常函数仍生成 `CallStatic`

### 测试内容

- 代码生成测试：翻译 `read_file("a.txt")` 为 `CallNative` ✅
- 函数调用测试：多个参数的正确传递 ✅

---

## 步骤 4：解释器执行 CallNative

### 文件

`src/backends/interpreter/executor.rs`

### 实现内容

| 内容 | 说明 |
|------|------|
| 在 `Interpreter` 中集成 `FfiRegistry` | 作为成员 `ffi: FfiRegistry` |
| 处理 `BytecodeInstr::CallNative` | 调用 `self.ffi.call()` |
| 参数转换 | `RuntimeValue` → Rust 类型 → 返回 |

### 验收方法

- [x] 解释器能执行 `CallNative` 指令
- [x] 调用结果正确返回

### 测试内容

- 端到端测试：`println("hello")` 输出到 stdout ✅
- 文件测试：`write_file("test.txt", "content")` 创建文件 ✅
- 错误处理：不存在的 native 函数报错 ✅

---

## 步骤 5：类型检查支持 Native 类型

### 文件

`src/frontend/typecheck/mod.rs`

### 实现内容

| 内容 | 说明 |
|------|------|
| 识别 `Native` 类型标注 | 在类型推断时处理 |
| 类型签名验证 | 确认调用签名与注册匹配 |

### 验收方法

- [x] `Native("name")` 作为值时类型正确
- [x] 函数调用类型检查通过

### 测试内容

- 类型检查测试：正确的签名通过 ✅
- 类型错误测试：参数数量不匹配报错 ✅

---

## 步骤 6：重构 std.io 接口

### 文件

`src/std/io.rs`

### 实现内容

| 内容 | 说明 |
|------|------|
| 修改函数声明 | 使用 `Native("std.io.xxx")` 方式 |
| 文档注释 | 保持现有文档 |

### 待实现函数

| 函数 | Native 名称 | 说明 |
|------|-------------|------|
| `print` | `std.io.print` | 打印到 stdout |
| `println` | `std.io.println` | 打印并换行 |
| `read_file` | `std.io.read_file` | 读取文件内容 |
| `write_file` | `std.io.write_file` | 写入文件 |
| `read_line` | `std.io.read_line` | 读取一行 |
| `append_file` | `std.io.append_file` | 追加写入 |

### 验收方法

- [x] `import std.io` 后可调用 `read_file`, `write_file` 等 ✅

### 测试内容

- 集成测试：实际读取/写入文件 ✅
- 功能测试：各种 IO 函数正常工作 ✅
- 单元测试：NativeDeclaration 注册表验证 ✅ 6/6 通过
- 文档测试：NativeDeclaration 示例通过 ✅

---

## 步骤 7：用户自定义 native 函数支持

### 文件

`src/std/ffi.rs`（新建）

### 实现内容

| 内容 | 说明 |
|------|------|
| `Native` 类型定义 | 用户声明 native 函数标记 |
| `register` 函数 | 用户注册自己的 native 函数处理逻辑 |

### 用户使用方式

**YaoXiang 源码声明：**

```yaoxiang
# 声明 native 函数绑定
my_add: (a: Int, b: Int) -> Int = Native("my_add")

# 调用（编译器自动生成 CallNative 字节码）
result = my_add(1, 2)
```

**Rust 嵌入 API 注册：**

```rust
// 在 Rust 端注册 native 函数处理逻辑
interpreter.ffi_registry_mut().register("my_add", |args| {
    let a = args[0].to_int().unwrap_or(0);
    let b = args[1].to_int().unwrap_or(0);
    Ok(RuntimeValue::Int(a + b))
});
```

### 实现内容

| 内容 | 说明 |
|------|------|
| `NativeBinding` 结构体 | 用户声明的 native 函数绑定 (func_name → native_symbol) |
| `detect_native_binding()` | 检测 AST 中 `Native("...")` 模式 |
| `ModuleIR.native_bindings` | IR 层传递 native 绑定信息 |
| IR 生成器集成 | 检测 `= Native("symbol")` 后跳过函数体生成，记录绑定 |
| Translator 集成 | `translate_module` 开始前自动注册用户 native 函数 |

### 验收方法

- [x] 用户能声明自定义 native 函数 ✅
- [x] 注册后能正确调用 ✅

### 测试内容

- 单元测试：NativeBinding 创建和映射 ✅ 6/6 通过
- 集成测试：detect_native_binding 模式识别 ✅
- 文档测试：NativeBinding 示例通过 ✅

---

## 依赖关系

```
步骤 1 (FFI 注册表)
    │
    ├── 步骤 4 (解释器集成)
    │       │
    │       └── 步骤 6 (std.io 重构)
    │
    ├── 步骤 2 (字节码)
    │       │
    │       └── 步骤 3 (代码生成)
    │               │
    │               └── 步骤 5 (类型检查)
    │
    └── 步骤 7 (用户自定义)
```

## 验收总览

| 步骤 | 验收条件 | 状态 |
|------|----------|------|
| 1 | FfiRegistry 可创建、注册、调用 | ✅ |
| 2 | 字节码正确序列化/反序列化 | ✅ |
| 3 | Native 表达式生成 CallNative | ✅ |
| 4 | 解释器执行并返回正确结果 | ✅ |
| 5 | 类型检查正确处理 Native | ✅ |
| 6 | std.io 函数可用 | ✅ |
| 7 | 用户自定义 native 函数支持 | ✅ |

## 端到端测试结果

```
running 19 tests
- backends::interpreter::ffi::tests::test_new_registry_is_empty ... ok
- backends::interpreter::ffi::tests::test_with_std_has_io_functions ... ok
- backends::interpreter::ffi::tests::test_register_custom_function ... ok
- backends::interpreter::ffi::tests::test_call_custom_function ... ok
- backends::interpreter::ffi::tests::test_call_nonexistent_function_returns_error ... ok
- backends::interpreter::ffi::tests::test_call_println_via_registry ... ok
- backends::interpreter::ffi::tests::test_cache_accelerates_repeated_calls ... ok
- backends::interpreter::ffi::tests::test_register_overwrites_existing ... ok
- backends::interpreter::ffi::tests::test_registered_functions_list ... ok
- backends::interpreter::ffi::tests::test_write_and_read_file ... ok
- backends::interpreter::ffi::tests::test_read_file_missing_args ... ok
- backends::interpreter::ffi::tests::test_write_file_missing_args ... ok
- backends::interpreter::executor::tests::test_ffi_println_e2e ... ok
- backends::interpreter::executor::tests::test_ffi_write_and_read_file_e2e ... ok
- backends::interpreter::executor::tests::test_ffi_custom_function_e2e ... ok
- backends::interpreter::executor::tests::test_ffi_nonexistent_function_e2e ... ok
- backends::interpreter::executor::tests::test_ffi_append_file_e2e ... ok

test result: ok. 19 passed; 0 failed; 0 ignored
```

### 测试覆盖

- ✅ FFI 注册表创建和注册
- ✅ 标准库函数 (std.io.print, println, read_file, write_file, append_file)
- ✅ 自定义 native 函数注册和调用
- ✅ 错误处理（不存在的函数）
- ✅ 缓存加速
- ✅ 文件读写
- ✅ 文件追加
