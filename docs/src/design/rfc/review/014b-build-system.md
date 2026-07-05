---
title: "RFC-014b: 构建系统与二进制分发"
status: "审核中"
author: "晨煦"
created: "2026-06-11"
updated: "2026-07-05"
group: "rfc-014"
issue: "#91"
impl: "0%"
impl_status: "not-started"
---

# RFC-014b: 构建系统与二进制分发

> 本 RFC 是 [RFC-014: 包管理系统设计](../accepted/014-package-manager.md) 的子 RFC。

## 摘要

定义 YaoXiang 包管理系统的构建机制：声明式构建配置、构建策略（cargo/cmake/custom/none）、预编译二进制分发、系统依赖检查。

## 动机

有些包是纯 `.yx` 代码，无需构建。有些需要编译 FFI 绑定（调用 Cargo、CMake 等）。需要统一的机制让包作者声明构建需求，让包管理器自动处理。

### 当前的问题

- 没有构建配置声明（`yaoxiang.toml` 中没有 `[build]` 段）
- 没有预编译二进制分发机制
- FFI 包的构建完全依赖用户手动操作
- 没有系统依赖检查

## 提案

### 核心设计：声明式构建 + 预编译优先

包作者在 `yaoxiang.toml` 中声明构建需求，包管理器根据声明自动决策。

### 构建策略

```rust
enum BuildStrategy {
    None,          // 纯 .yx 包，无需构建
    Cargo,         // 调用 cargo build，读 [build.cargo] 配置
    Cmake,         // 调用 cmake
    Custom,        // 执行 build.yx 脚本
}
```

注意：`Precompiled` 变体已删除。`[binaries]` 的存在自动触发预编译优先行为，不需要显式声明 strategy。

### yaoxiang.toml 中的构建声明

```toml
[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"              # 构建策略
headers = ["include/sqlite3.h"] # 可选：yx-bindgen 自动处理的 C 头文件

[build.cargo]
features = ["ffi"]             # cargo build --features ffi
target = "release"             # cargo build --release

[build.requirements]
cargo = ">= 1.70"              # 构建时需要的工具
cmake = ">= 3.20"

[build.platforms]              # 平台特定覆盖
"x86_64-unknown-linux-gnu" = { cargo-features = ["linux-ffi"] }
"x86_64-pc-windows-msvc" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

### 安装决策树

```
yaoxiang install foo
    │
    ├─ 1. [binaries] 有当前平台条目？
    │     → 有：下载，校验 SHA-256，直接安装（跳过构建）
    │     → 无：继续
    │
    ├─ 2. 下载源码包
    │
    ├─ 3. [build].headers 有值？
    │     → 有：自动运行 yx-bindgen 生成绑定文件
    │
    ├─ 4. 读 [build].strategy
    │     → "none"：直接安装
    │     → "cargo"：读 [build.cargo] 配置，拼 cargo build 命令
    │     → "cmake"：调用 cmake
    │     → "custom"：执行 build.yx 脚本
    │
    └─ 5. 安装到 vendor/
```

**预编译优先，源码兜底。** `[binaries]` 的存在自动触发预编译检查，不需要显式 strategy。

### cargo 策略详解

`strategy = "cargo"` 时，读 `[build.cargo]` 配置拼命令：

```toml
[build]
strategy = "cargo"

[build.cargo]
features = ["ffi"]             # → cargo build --features ffi
target = "release"             # → cargo build --release

[build.platforms]              # 平台覆盖
"x86_64-unknown-linux-gnu" = { cargo-features = ["linux-ffi"] }
"x86_64-pc-windows-msvc" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

实际执行的命令：

```bash
# 基础
cargo build --release --features ffi

# 有平台覆盖时（以 linux 为例）
cargo build --release --features ffi,linux-ffi
```

### 预编译二进制声明

```toml
# yaoxiang.toml
[binaries]
"x86_64-unknown-linux-gnu" = { url = "releases/download/v1.0.0/foo-linux-x86_64.tar.gz", sha256 = "abc123" }
"x86_64-pc-windows-msvc" = { url = "https://example.com/foo-win-x86_64.tar.gz", sha256 = "def456" }
"aarch64-apple-darwin" = { url = "releases/download/v1.0.0/foo-macos-aarch64.tar.gz", sha256 = "ghi789" }
```

**URL 格式：** 支持绝对 URL 和相对路径。相对路径相对于包的仓库地址（GitHub repo URL 或 Registry 根 URL）。

**跳过构建的条件：**
1. `[binaries]` 中有当前平台的条目
2. SHA-256 校验通过
3. 下载成功

三个条件都满足 → 跳过构建。否则 → fallback 到源码构建。

### build.yx 构建脚本

当 `strategy = "custom"` 时执行 `build.yx`。

**执行模型（最小规范）：**
- 脚本是普通 `.yx` 代码，拥有完整 `std` 访问权限
- 工作目录：包根目录（`vendor/<pkg>-<ver>/`）
- 成功：退出码 0
- 失败：非 0 退出码，安装中止
- 包管理器不约束脚本行为，只检查退出码

```yx
# build.yx — 包的构建脚本
use std.os
use std.io

fn main() {
    let platform = os.platform()
    let arch = os.arch()

    if os.file_exists("Cargo.toml") {
        io.println("Building native extension via Cargo...")
        let result = os.exec("cargo build --release")
        if result.exit_code != 0 {
            io.println("Build failed!")
            os.exit(1)
        }
    }

    io.println("Build complete!")
}
```

### 系统依赖检查

安装前自动检查所有 `[build.requirements]`，不满足则报错：

```
Error: Build requirement not satisfied
  cargo >= 1.70 required, but cargo is not installed
  Install: https://rustup.rs
```

### yx-bindgen 集成（headers 字段）

`[build].headers` 声明需要 yx-bindgen 处理的 C 头文件。构建系统自动运行 yx-bindgen 生成 `.yx` 绑定文件。

```toml
[build]
strategy = "cargo"
headers = ["include/sqlite3.h", "include/json.h"]
```

构建流程：

```
1. [binaries] 有预编译？→ 跳过全部构建
2. [build].headers 有值？→ yx-bindgen 自动生成绑定
3. 执行 [build].strategy（cargo/cmake/custom）
4. 安装
```

yx-bindgen 从 C 头文件（`.h`）解析函数签名和类型定义，自动生成 `.yx` 绑定声明。用户不需要手动运行——构建系统在检测到 `headers` 配置时自动处理。

**与 RFC-026 的关系：** RFC-026 定义了 `yx-bindgen` 的语言级语义（`native("symbol")` 语法、unsafe 类型）。RFC-014b 定义了它在构建流程中的集成方式（`headers` 配置）。两者互补。

### 与 Cargo Workspace 的集成

如果包中有 FFI 代码，可以同时定义 Cargo workspace：

```
my-package/
├── yaoxiang.toml          # YaoXiang 包配置
├── Cargo.toml             # Cargo workspace（FFI 部分）
├── src/
│   └── lib.yx             # YaoXiang 代码
└── native/
    ├── Cargo.toml          # Rust FFI 代码
    └── src/
        └── lib.rs
```

`yaoxiang build` 自动检测并调用 `cargo build` 编译 native 部分。

## 详细设计

### 平台标识

使用 Rust target triple 格式（`arch-vendor-os-env`）：

| 平台 | 标识 |
|------|------|
| Linux x86_64 (glibc) | `x86_64-unknown-linux-gnu` |
| Linux x86_64 (musl) | `x86_64-unknown-linux-musl` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` |
| Windows x86_64 (MSVC) | `x86_64-pc-windows-msvc` |
| Windows x86_64 (MinGW) | `x86_64-pc-windows-gnu` |
| macOS ARM64 | `aarch64-apple-darwin` |
| macOS x86_64 | `x86_64-apple-darwin` |

使用 Rust target triple 而非简化格式，因为：
1. 区分同一 OS 上的不同 ABI（gnu vs musl，msvc vs gnu）
2. 与 Rust/Cargo 生态对齐，减少映射错误
3. 未来扩展无需改格式

### 构建产物目录结构

```
build/
└── native/
    ├── x86_64-unknown-linux-gnu/
    │   └── libfoo.so
    ├── x86_64-pc-windows-msvc/
    │   └── foo.dll
    └── aarch64-apple-darwin/
        └── libfoo.dylib
```

### 预编译包的完整生命周期

```
开发者：
  1. 写 .yx 代码 + FFI 绑定
  2. 在 yaoxiang.toml 声明 [build] + [binaries]
  3. yaoxiang publish
     → 自动在 CI 上构建多平台二进制
     → 上传源码 + 预编译产物

用户：
  yaoxiang add native-foo
    → 检测到有预编译产物 → 直接下载（秒级）
    → 没有预编译产物 → 下载源码 + 执行构建（分钟级）
```

## 权衡

### 优点

- 声明式配置，用户无需理解构建细节
- 预编译优先，安装速度极快
- 支持多平台，自动选择
- 与 Cargo 生态无缝集成

### 缺点

- 预编译产物需要 CI 支持
- 多平台构建增加发布复杂度
- build.yx 脚本需要沙箱安全机制

## 替代方案

| 方案 | 为什么没选 |
|------|-----------|
| 纯源码分发 | 用户需要安装构建工具链，门槛高 |
| 类似 Python wheel 的二进制格式 | 过于复杂，YaoXiang 生态初期不需要 |
| 不支持 FFI 构建 | 限制了语言的扩展能力 |

## 实现策略

### 阶段划分

| 阶段 | 内容 |
|------|------|
| Phase 5a | `[build]` 配置解析 + `BuildStrategy` 枚举 |
| Phase 5b | 系统依赖检查 |
| Phase 5c | Cargo 构建集成（读 `[build.cargo]` 拼命令） |
| Phase 5d | 预编译二进制下载 + 校验 |
| Phase 5e | build.yx 脚本执行 |
| Phase 5f | yx-bindgen 集成（`headers` 字段） |

### 依赖关系

- 依赖 RFC-014a（Registry 协议，用于下载预编译产物）
- 依赖 `sha2` crate（完整性校验）

## 开放问题

- [ ] build.yx 脚本是否需要沙箱隔离？
- [ ] 构建产物的最大大小限制？
- [ ] 是否支持交叉编译（在 Linux 上构建 Windows 产物）？
- [ ] Cargo 版本不兼容时如何处理？

---

## 参考文献

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)
