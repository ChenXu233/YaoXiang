---
title: "RFC-014b: 构建系统与二进制分发"
status: "草案"
author: "晨煦"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
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
    Cargo,         // 调用 cargo build
    Cmake,         // 调用 cmake
    Custom,        // 执行 build.yx 脚本
    Precompiled,   // 直接用预编译产物
}
```

### yaoxiang.toml 中的构建声明

```toml
[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"              # 构建策略
script = "build.yx"            # 仅 strategy = "custom" 时使用

[build.cargo]
features = ["ffi"]             # cargo build --features ffi
target = "release"             # cargo build --release

[build.requirements]
cargo = ">= 1.70"              # 构建时需要的工具
cmake = ">= 3.20"

[build.platforms]              # 平台特定覆盖
"linux-x86_64" = { cargo-features = ["linux-ffi"] }
"windows-x86_64" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

### 安装决策树

```
yaoxiang install foo
    │
    ├─ 1. 查 Registry/GitHub Release 有无当前平台预编译产物？
    │     → 有：下载，校验 SHA-256，直接安装（跳过构建）
    │     → 无：继续
    │
    ├─ 2. 下载源码包
    │
    ├─ 3. 读 yaoxiang.toml 的 [build] 段
    │     → strategy = "none"：直接安装
    │     → 其他：检查 requirements，执行构建
    │
    └─ 4. 安装到 vendor/
```

**预编译优先，源码兜底。** 类似 Python 的 wheel vs sdist。

### 预编译二进制声明

```toml
# yaoxiang.toml
[binaries]
"linux-x86_64" = { url = "releases/download/v1.0.0/foo-linux-x64.tar.gz", sha256 = "abc123" }
"windows-x86_64" = { url = "releases/download/v1.0.0/foo-win-x64.tar.gz", sha256 = "def456" }
"aarch64-apple-darwin" = { url = "releases/download/v1.0.0/foo-mac-arm.tar.gz", sha256 = "ghi789" }
```

**跳过构建的条件：**
1. `[binaries]` 中有当前平台的条目
2. SHA-256 校验通过
3. 下载成功

三个条件都满足 → 跳过构建。否则 → fallback 到源码构建。

### build.yx 构建脚本

当 `strategy = "custom"` 时执行：

```yx
# build.yx — 包的构建脚本
use std.os
use std.io

fn main() {
    let platform = os.platform()       # "linux", "windows", "macos"
    let arch = os.arch()               # "x86_64", "aarch64"

    if os.file_exists("Cargo.toml") {
        io.println("Building native extension via Cargo...")
        os.exec("cargo build --release")

        os.copy(
            "target/release/libfoo.so",
            "build/native/${platform}-${arch}/libfoo.so"
        )
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

使用 `{os}-{arch}` 格式：

| 平台标识 | OS | Arch |
|----------|----|------|
| `linux-x86_64` | Linux | x86_64 |
| `linux-aarch64` | Linux | ARM64 |
| `windows-x86_64` | Windows | x86_64 |
| `aarch64-apple-darwin` | macOS | ARM64 (Apple Silicon) |
| `x86_64-apple-darwin` | macOS | x86_64 |

### 构建产物目录结构

```
build/
└── native/
    ├── linux-x86_64/
    │   └── libfoo.so
    ├── windows-x86_64/
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
| Phase 5c | Cargo 构建集成 |
| Phase 5d | 预编译二进制下载 + 校验 |
| Phase 5e | build.yx 脚本执行 |

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
