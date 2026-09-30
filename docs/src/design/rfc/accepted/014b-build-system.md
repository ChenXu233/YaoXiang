---
title: 'RFC-014b: 构建系统与二进制分发'
status: '已接受'
author: '晨煦'
created: '2026-06-11'
updated: '2026-09-30'
group: 'rfc-014'
issue: '#91'
impl: '90%'
impl_status: 'in-progress'
---

# RFC-014b: 构建系统与二进制分发

> 本 RFC 是 [RFC-014: 包管理系统设计](../accepted/014-package-manager.md) 的子 RFC。

## 2026-09-15 审核决议

以下决议由所有者于 2026-09-15 拍板：

1. **build.yx 信任门（对「沙箱」开放问题的落地）**：`custom` 策略执行任意代码是包管理器最大的供应链攻击面——`yaoxiang add` 一个恶意包即等于交出 `std.os` 全权限。最低强制线：
   - 首次执行某包的 `build.yx` 前必须交互确认；
   - `yaoxiang add --trust <pkg>` / `install --trust` 把信任记录持久化到 `~/.yaoxiang/config.toml`（`[trust] build-scripts = ["name@version"]`）；
   - 非交互环境（CI）默认拒绝 `custom` 构建，除非显式 `--trust`；
   - 完整沙箱机制继续作为开放问题研究，但信任门是不可省略的下限。
2. **阶段重排**：`5a → 5b → 5c（cargo）→ 5d（[binaries]）→ 5f（bindgen，依赖 RFC-026b）→ 5e（custom 最后，带信任门）`。声明式路径（cargo）先行，任意代码执行最后。
3. **二进制分发归一**：`[binaries]` 是唯一二进制分发机制（对应 RFC-014a 决议 3——`.yxpkg` 只含源码）。
4. **交叉编译**：初期不支持，多平台产物由 CI 多主机产出（与 RFC-037 cargo-dist 思路对齐）。
5. **构建产物大小**：不设上限，由分发渠道自管。
6. **Cargo 版本不兼容**：走 `[build.requirements]` 预检报错 + 安装指引（正文已定义，无额外机制）。

## 摘要

定义 YaoXiang 包管理系统的构建机制：声明式构建配置、构建策略（cargo/cmake/custom/none）、预编译二进制分发、系统依赖检查。

## 动机

有些包是纯 `.yx`
代码，无需构建。有些需要编译 FFI 绑定（调用 Cargo、CMake 等）。需要统一的机制让包作者声明构建需求，让包管理器自动处理。

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

**URL 格式：** 支持绝对 URL 和相对路径。相对路径相对于包的仓库地址（GitHub repo
URL 或 Registry 根 URL）。

**跳过构建的条件：**

1. `[binaries]` 中有当前平台的条目
2. SHA-256 校验通过
3. 下载成功

三个条件都满足 → 跳过构建。否则 → fallback 到源码构建。

### build.yx 构建脚本

当 `strategy = "custom"` 时执行 `build.yx`。

**执行模型（最小规范）：**

- 脚本是普通 `.yx` 代码，拥有完整 `std` 访问权限
- **信任门（2026-09-15 决议，强制）**：首次执行前须交互确认，非交互环境默认拒绝（见上方决议 1）
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

`[build].headers` 声明需要 yx-bindgen 处理的 C 头文件。构建系统自动运行 yx-bindgen 生成 `.yx`
绑定文件。

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

yx-bindgen 从 C 头文件（`.h`）解析函数签名和类型定义，自动生成 `.yx`
绑定声明。用户不需要手动运行——构建系统在检测到 `headers` 配置时自动处理。

**与 RFC-026 的关系：** RFC-026 定义了 `yx-bindgen` 的语言级语义（`native("symbol")`
语法、unsafe 类型）。RFC-014b 定义了它在构建流程中的集成方式（`headers` 配置）。两者互补。

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

| 平台                   | 标识                        |
| ---------------------- | --------------------------- |
| Linux x86_64 (glibc)   | `x86_64-unknown-linux-gnu`  |
| Linux x86_64 (musl)    | `x86_64-unknown-linux-musl` |
| Linux ARM64            | `aarch64-unknown-linux-gnu` |
| Windows x86_64 (MSVC)  | `x86_64-pc-windows-msvc`    |
| Windows x86_64 (MinGW) | `x86_64-pc-windows-gnu`     |
| macOS ARM64            | `aarch64-apple-darwin`      |
| macOS x86_64           | `x86_64-apple-darwin`       |

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

| 方案                           | 为什么没选                        |
| ------------------------------ | --------------------------------- |
| 纯源码分发                     | 用户需要安装构建工具链，门槛高    |
| 类似 Python wheel 的二进制格式 | 过于复杂，YaoXiang 生态初期不需要 |
| 不支持 FFI 构建                | 限制了语言的扩展能力              |

## 实现策略

### 阶段划分

| 阶段     | 内容                                        | 状态 |
| -------- | ------------------------------------------- | ---- |
| Phase 5a | `[build]` 配置解析 + `BuildStrategy` 枚举（含 `[binaries]` 声明、平台三元组） | ✅ 已完成 |
| Phase 5b | 系统依赖检查（`<tool> --version` 探测 + 比较器操作数补齐 + 安装指引） | ✅ 已完成 |
| Phase 5c | Cargo 构建集成（读 `[build.cargo]` 拼命令 + 平台覆盖合并 + scratch 隔离） | ✅ 已完成 |
| Phase 5d | 预编译二进制下载 + 校验（整包 SHA-256 + 安全解包 + 回退语义） | ✅ 已完成 |
| Phase 5f | yx-bindgen 集成（`headers` 字段配置管道；RFC-026b 尚为草案，执行时明确报错） | ✅ 配置管道完成（生成器随 026b） |
| Phase 5e | build.yx 脚本执行（**最后实施**，带信任门） | ✅ 已完成 |

执行顺序（2026-09-15 决议 2）：`5a → 5b → 5c → 5d → 5f → 5e`。声明式构建先行，任意代码执行殿后。

**落地说明（2026-09-30，提交见 feat/rfc014）**：

- **产物两层分离**：cargo scratch（target/）经 `CARGO_TARGET_DIR` 指到项目 `.yaoxiang/build/cargo/<pkg>/`——不落 vendor 包目录，否则目录完整性校验和被增量构建产物撑爆；FFI 可消费的库文件（.so/.dll/.dylib/.a）复制到 vendor 包目录 `build/native/<triple>/`。vendor 完整性校验语义随之明确为**源码树完整性**（`build/` 派生产物不入校验和）。
- **`[binaries]` 回退语义**：当前平台有条目但「sha256 未声明 / 下载失败 / 校验不匹配」任一 → stderr 提示 + 回退源码构建（RFC「否则 fallback」），回退前清理半成品。整包校验通过后走安全解包（路径逃逸防护 + 解压总量护栏 512 MiB，产物大小不设硬上限——决议 5）。
- **信任门（决议 1）落地**：信任记录随用户配置体系（`~/.config/yaoxiang/config.toml` 的 `[trust] build-scripts`；RFC 草拟时的 `~/.yaoxiang/config.toml` 未曾存在，与 `[cache] dir` 同款并入）。三条放行路径：已记录 / 本次 `--trust`（**放行即持久化**，兑现「add/install --trust 把信任记录持久化」）/ 交互确认（确认即持久化）。非交互环境（stdin 非终端）默认拒绝，仅 `--trust` 可过。
- **build.yx 执行模型**：普通 .yx 脚本、**顶层语句即构建逻辑**（Script/eval 语义；上方 `fn main()` 示例是草案期伪码），进程内执行，工作目录临时切到包根（全局锁串行化翻转窗口）。当前 std 无 exec/exit API——脚本可写生成/文件类逻辑，`os.exec` 式调用外部工具随 std 演进。
- **cmake 策略**：枚举与配置解析就绪，执行尚未实现（明确报错）——RFC 阶段表未单列 cmake 阶段，待有真实需求包再排期。
- **publish 发布前测试**（014a 校验 3）已随本 Phase 接线：默认运行 `[tool.test]` 发现的测试（RFC-036 机制），失败中止发布，`--no-test` 跳过。

### 依赖关系

- 依赖 RFC-014a（Registry 协议，用于下载预编译产物）
- 依赖 `sha2` crate（完整性校验）

## 开放问题

- [x] build.yx 脚本是否需要沙箱隔离？→ 信任门为强制下限（2026-09-15 决议 1）；完整沙箱继续研究
- [x] 构建产物的最大大小限制？→ 不设上限，渠道自管（2026-09-15 决议 5）
- [x] 是否支持交叉编译（在 Linux 上构建 Windows 产物）？→ 初期不支持，CI 多主机产出（2026-09-15 决议 4）
- [x] Cargo 版本不兼容时如何处理？→ `[build.requirements]` 预检报错 + 安装指引（2026-09-15 决议 6）

---

## 参考文献

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)
