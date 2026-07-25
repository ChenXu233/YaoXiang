---
title: "RFC-037: 工业化分发方案 — 基于 cargo-dist 的编译器/工具链打包"
author: "ChenXu233"
created: "2026-07-26"
updated: "2026-07-26"
issue: "#230"
---

# RFC-037: 工业化分发方案 — 基于 cargo-dist 的编译器/工具链打包

> 本 RFC 与 [RFC-014b: 构建系统与二进制分发](../review/014b-build-system.md) 互补。
> RFC-014b 定义了 **YaoXiang 包管理器**如何构建和分发第三方包；
> 本 RFC 定义 **YaoXiang 编译器/工具链本身**如何打包和分发。

## 摘要

用 `cargo-dist`（Rust 生态的二进制分发标准工具）替换现有的手写 CI 构建/打包逻辑，实现跨平台自动化发布。解决 `libz3.dll` 缺失、标准库接口文件未打包、目录结构混乱、CI 脚本重复维护等问题。

## 动机

### 为什么需要这个特性？

下载 YaoXiang 的用户应该能**开箱即用**，不需要任何额外步骤。

### 当前的问题

#### 问题 1：Windows 用户下载后跑不了

当前 Release 只上传 `yaoxiang.exe`，但 `libz3.dll` 没有打包进去。用户在 Windows 上双击运行会报错：

```
The code execution cannot proceed because libz3.dll was not found.
```

这是 **中断性 bug** — 用户连第一步都走不过去。

#### 问题 2：Release 制品只有单文件 exe

```
yaoxiang-v0.7.10-x86_64-pc-windows-msvc.zip
└── yaoxiang.exe
```

标准库接口文件（`.yx` 文件，LSP 需要）没有包含在发行包中，用户需要运行 `yaoxiang package init` 才能生成。工业化的做法应该是：

```
yaoxiang-0.7.10-x86_64-pc-windows-msvc.zip
├── bin/
│   ├── yaoxiang.exe
│   └── libz3.dll
└── lib/
    └── std/
        ├── io.yx
        ├── math.yx
        ├── ...
        └── mod.yx
```

#### 问题 3：CI 手写脚本重复维护

当前维护了 3 套构建流水线：

| 文件 | 职责 | 行数 |
|------|------|------|
| `_build-platforms.yml` | 跨平台构建（Linux/macOS/Windows） | ~250 行 |
| `release.yml` | 版本发布流程 | ~170 行 |
| `nightly.yml` | 每日构建 | ~170 行 |

**总计 ~600 行手写 YAML。** 这些脚本大部分是重复的（安装 Rust → 缓存 → 构建 → 重命名 → 上传），每个平台都要写一次。`cargo-dist` 一行命令就能生成同等的流水线。

#### 问题 4：Inno Setup 版本号硬编码

`setup.iss` 里 `MyAppVersion` 写死 `0.7.0`，构建时靠 `sed` 替换。迟早会翻车。

#### 问题 5：与 RFC-014b 的边界模糊

RFC-014b 定义了"YaoXiang 包的构建和分发机制"（即 `yaoxiang.toml` 中的 `[build]` 和 `[binaries]` 配置），但**没有覆盖"YaoXiang 编译器本身怎么发布"**。本 RFC 填补这个空白。

## 提案

### 核心设计

采用 **cargo-dist** 作为发布流水线，配合自定义 post-build 脚本处理 Z3 DLL 和标准库接口文件。

```
cargo-dist 负责:
  ├── 跨平台构建（6 个平台）
  ├── 生成 tar.gz/zip 压缩包
  ├── 生成安装脚本（shell/powershell）
  ├── 生成 Windows MSI 安装器
  ├── 自动发布 GitHub Release
  └── changelog 自动生成

build.rs 继续负责:
  └── Z3 下载/链接（已有逻辑，只需要微调）

自定义脚本负责:
  ├── 构建后复制 libz3.dll 到打包目录
  └── 预生成标准库 .yx 接口文件
```

### 发布目录结构

每个平台 release 压缩包：

```
yaoxiang-{version}-{target}.tar.gz / .zip
├── bin/
│   ├── yaoxiang                      # 或 yaoxiang.exe
│   └── libz3.dll                     # 仅 Windows，其他平台静态链接
├── lib/
│   └── std/                          # 预生成的标准库接口文件
│       ├── io.yx
│       ├── math.yx
│       ├── string.yx
│       ├── ...
│       └── mod.yx
└── README.md                         # 简短的安装说明
```

### 平台支持

| 平台 | target triple | 说明 |
|------|-------------|------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | 主平台 |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | CI 上交叉编译 |
| macOS x86_64 | `x86_64-apple-darwin` | Intel Mac |
| macOS ARM64 | `aarch64-apple-darwin` | Apple Silicon |
| Windows x86_64 | `x86_64-pc-windows-msvc` | 主平台 |
| Windows ARM64 | `aarch64-pc-windows-msvc` | 可选，后续支持 |

### Z3 分发策略

| 平台 | 策略 | 原因 |
|------|------|------|
| Linux | 静态链接 `libz3.a` | 已有，保持 |
| macOS | 静态链接 `libz3.a` | 已有，保持 |
| Windows | 打包 `libz3.dll` | Z3 官方 Windows 预编译只有 DLL |
| wasm32 | 静态链接 `libz3.a` | 已有，保持 |

**Windows 的 `libz3.dll` 由 build.rs 在构建时下载到 `.z3/` 目录，再由 cargo-dist 的 `extra-artifacts` 机制打包进压缩包。**

长期目标：自行编译 Z3 的 Windows 静态库（`-DZ3_BUILD_LIBZ3_SHARED=OFF`），实现全平台静态链接单文件分发。

### 安装器支持

| 安装器 | 支持 | 说明 |
|--------|------|------|
| tar.gz / zip | ✅ 默认 | 所有平台 |
| shell 安装脚本 | ✅ cargo-dist 内置 | Unix 平台 |
| powershell 安装脚本 | ✅ cargo-dist 内置 | Windows 平台 |
| Homebrew formula | ✅ cargo-dist 内置 | macOS |
| Windows MSI | ✅ cargo-dist 内置 | 替代 Inno Setup |
| Inno Setup | ❌ 废弃 | 迁到 cargo-dist 的 MSI |

## 详细设计

### cargo-dist 配置

在项目根目录创建 `dist-workspace.toml`：

```toml
[workspace]
# 指向 Cargo 工作区，所有 binary 包会自动发现
members = ["cargo:."]

[dist]
# 构建产物
package-libraries = ["cdylib"]

# 安装器
installers = [
    "shell",           # Unix shell 安装脚本
    "powershell",      # Windows powershell 安装脚本
    "homebrew",        # macOS Homebrew
    "msi",             # Windows MSI 安装器
]

# 构建后额外处理的脚本
extra-artifacts = [
    "scripts/build/package-z3.sh",   # 复制 libz3.dll + 标准库接口文件
]

# CI 配置
ci = "github"
ci.github.create-release = true
ci.github.pr-run-mode = "plan"
```

### 构建后处理脚本

`scripts/build/package-z3.sh`（跨平台，在 cargo-dist 构建后执行）：

```bash
#!/bin/bash
# 在 cargo-dist 构建后，将 libz3.dll 和标准库接口文件复制到打包目录

set -euo pipefail

# 1. 复制 Z3 DLL（仅 Windows）
if [ "$CARGO_DIST_TARGET" = "x86_64-pc-windows-msvc" ]; then
    Z3_DIR=".z3/z3-4.16.0-x64-win"
    cp "$Z3_DIR/bin/libz3.dll" "$DIST_DIR/bin/"
fi

# 2. 生成标准库接口文件
yaoxiang package gen-std-interfaces --out-dir "$DIST_DIR/lib/std/"
```

### CI 流水线变化

#### 迁移前（当前）：

```
release.yml (170 行) ──→ _build-platforms.yml (250 行) ──→ 构建 → 上传
                  └──→ wasm 构建
                  └──→ 安全审计
                  └──→ 测试
                  └──→ 发布

nightly.yml (170 行) ──→ 同上（重复）
```

#### 迁移后：

```
cargo-dist 生成的 release.yml（~100 行，自动维护）
  └──→ 6 平台并行构建
  └──→ 生成压缩包 + 安装脚本
  └──→ 创建 GitHub Release
  └──→ 上传到 Homebrew / npm

cargo-dist 生成的 pr.yml（~50 行，自动维护）
  └──→ PR 时运行 dist plan 检查
```

### 标准库接口文件生成

当前 `src/std/gen_interfaces.rs` 已经实现了生成 `.yx` 接口文件的功能（`write_interfaces_to_dir`），`package init` 命令也调用了它。

只需要：

1. 在 `main.rs` 中新增一个子命令 `yaoxiang package gen-std-interfaces`（或独立脚本）
2. 在打包脚本中调用该命令生成到 `lib/std/`

### 废弃的手写 CI

迁移完成后删除以下文件：

| 文件 | 替代 |
|------|------|
| `.github/workflows/_build-platforms.yml` | cargo-dist 自动生成 |
| `.github/workflows/release.yml` | cargo-dist 自动生成 |
| `.github/workflows/nightly.yml` | cargo-dist 的 schedule 触发 |
| `scripts/build/setup.iss` | cargo-dist 的 MSI 安装器 |
| `scripts/build/ChineseSimplified.isl` | 同上 |

## 权衡

### 优点

- **开箱即用** — 用户下载压缩包解压后直接运行，没有缺失 DLL 的问题
- **减少维护成本** — 删除 ~600 行手写 CI YAML，cargo-dist 自动维护
- **标准化** — 业界标准工具，几百个项目验证过
- **跨平台一致性** — 6 个平台使用同一套流水线
- **自动 changelog** — 内置 changelog 生成和发布说明
- **安装器覆盖** — shell/powershell/homebrew/msi 全支持

### 缺点

- **学习 cargo-dist 配置** — 团队需要学习新工具
- **自定义处理仍有维护成本** — Z3 DLL 和标准库接口文件的脚本需要维护
- **cargo-dist 版本迭代** — 需要跟随 upstream 更新
- **Windows ARM64 支持** — cargo-dist 默认支持，但 Z3 可能没有预编译 ARM64 的 DLL

### 与 RFC-014b 的关系

| | RFC-014b | RFC-037 |
|--|----------|---------|
| **范围** | 第三方包的构建和分发 | 编译器本身的打包和分发 |
| **工具** | `yaoxiang build` / `yaoxiang publish` | `cargo-dist` |
| **产物** | 第三方包的 FFI 库 | 编译器 + 标准库 + 工具链 |
| **互斥** | 否，互补 | 否，互补 |

## 替代方案

| 方案 | 为什么不选 |
|------|-----------|
| **继续手写 CI** | 已经手写了 ~600 行，重复劳动，容易遗漏 DLL |
| **自己写打包工具** | 不要重新发明轮子，cargo-dist 已经成熟 |
| **只用 tar.gz 不用安装器** | 用户需要更友好的安装方式（Homebrew/MSI） |
| **Docker 分发** | 编译器和语言工具链需要原生二进制，不是容器场景 |
| **全静态链接 Z3** | 理想方案，但 Windows 静态编译 Z3 需要额外 CI 步骤，可以后续优化 |

## 实现策略

### 阶段一：基础迁移（高优先级）

1. 调研确认 cargo-dist 的最新版本和配置格式
2. 安装 cargo-dist，运行 `dist init` 生成初始配置
3. 配置 `dist-workspace.toml`，指定目标平台
4. 使用 `cc` crate 替代 build.rs 的 Z3 外部下载逻辑（可选）

### 阶段二：自定义打包（中优先级）

1. 编写 `package-z3.sh` 构建后处理脚本
2. 在 `main.rs` 新增 `gen-std-interfaces` 子命令
3. 在打包脚本中调用生成标准库接口文件
4. 验证生成的压缩包结构正确

### 阶段三：废弃旧 CI（高优先级）

1. 在 `release.yml` 中集成 cargo-dist 流水线
2. 并行运行新旧 CI，对比产物一致性
3. 确认无误后删除旧 CI 文件
4. 删除 `setup.iss` 和相关脚本

### 阶段四：优化（低优先级）

1. 研究 Windows 静态编译 Z3 的可行性
2. 添加 Homebrew formula 自动发布
3. 添加 MSI 安装器
4. 考虑 ARM64 Windows 支持

### 依赖关系

- 无外部工具链依赖（cargo-dist 通过 cargo install 安装）
- 需要 GitHub Actions 运行 CI
- 需要 Homebrew 维护者账号（可选）

### 风险

- **cargo-dist 版本升级**：配置格式可能变化，需要关注 changelog
- **Z3 官方发布变更**：Z3 预编译包的位置或格式可能变化
- **Windows 静态链接**：Z3 的静态库在 Windows 上可能需要额外处理（如 C++ 运行时依赖）

## 开放问题

- [ ] Windows 上 Z3 静态链接的可行性？需要实测 `-DZ3_BUILD_LIBZ3_SHARED=OFF` 在 MSVC 下的表现
- [ ] `gen-std-interfaces` 子命令的具体命名和接口设计？
- [ ] 是否保留 Inno Setup 安装器作为 MSI 的补充？国内用户可能更习惯 exe 安装向导
- [ ] cargo-dist 的 `extra-artifacts` 是否支持跨平台条件执行（如仅 Windows 复制 DLL）？
- [ ] 标准库接口文件是否有版本兼容性保证？是否需要随编译器版本一起发布？

## 参考文献

- [cargo-dist 官方文档](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: 构建系统与二进制分发](../review/014b-build-system.md)
- [Rust 编译器分发流程 — bootstrap dist](https://doc.rust-lang.org/stable/nightly-rustc/bootstrap/core/build_steps/dist/index.html)
- [Go 工具链分发 — Go Toolchains](https://go.dev/doc/toolchain)
- [Z3 构建配置 — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)
- [Z3 Windows 分发脚本](https://github.com/Z3Prover/z3/blob/master/scripts/mk_win_dist_cmake.py)