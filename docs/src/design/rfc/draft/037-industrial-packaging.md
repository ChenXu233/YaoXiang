---
title: 'RFC-037: 工业化分发方案 — 基于 cargo-dist 的编译器/工具链打包'
author: 'ChenXu233'
created: '2026-07-26'
updated: '2026-07-27'
issue: '#230'
---

# RFC-037: 工业化分发方案 — 基于 cargo-dist 的编译器/工具链打包

> 本 RFC 与 [RFC-014b: 构建系统与二进制分发](../review/014b-build-system.md) 互补。RFC-014b 定义了
> **YaoXiang 包管理器**如何构建和分发第三方包；本 RFC 定义
> **YaoXiang 编译器/工具链本身**如何打包和分发。

## 摘要

用
`cargo-dist`（Rust 生态的二进制分发标准工具）替换现有的手写 CI 构建/打包逻辑，实现跨平台自动化发布。解决
`libz3.dll` 缺失、标准库接口文件未打包、目录结构混乱、CI 脚本重复维护等问题。

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

标准库接口文件（`.yx` 文件，LSP 需要）没有包含在发行包中。用户需要运行 `yaoxiang package init`
才能生成。工业化的做法是发行包自带标准库。

#### 问题 3：CI 手写脚本重复维护

当前维护了 4 套构建流水线：

| 文件                      | 职责              | 行数        |
| ------------------------- | ----------------- | ----------- |
| `_build-platforms.yml`    | 跨平台构建        | ~255 行     |
| `release.yml`             | 版本发布          | ~176 行     |
| `nightly.yml`             | 每日构建          | ~145 行     |
| `_build-wasm.yml`         | Wasm 构建         | ~75 行      |
| `scripts/build/setup.iss` | Inno Setup 安装器 | ~250 行     |
| **合计**                  |                   | **~900 行** |

大部分是重复的（安装 Rust → 缓存 → 构建 → 重命名 → 上传），每个平台都要写一次。`cargo-dist`
一行命令就能生成同等的流水线。

#### 问题 4：Inno Setup 版本号硬编码

`setup.iss` 里 `MyAppVersion` 写死 `0.7.0`，构建时靠 `sed` 替换。迟早会翻车。

#### 问题 5：与 RFC-014b 的边界模糊

RFC-014b 定义了"YaoXiang 包的构建和分发机制"（即 `yaoxiang.toml` 中的 `[build]` 和 `[binaries]`
配置），但**没有覆盖"YaoXiang 编译器本身怎么发布"**。本 RFC 填补这个空白。

## 提案

### 核心设计

采用 **cargo-dist** 作为发布流水线骨架，配合自定义 post-build 脚本处理包结构和附加文件。

```
cargo-dist 职责:
  ├── 跨平台编译（6 个 target）
  ├── 生成 CI 流水线（取代手写 ~900 行 YAML）
  ├── 生成安装器（MSI / shell / powershell / homebrew）
  ├── npm 发布（@yaoxiang/cli — 二进制下载 wrapper）
  ├── checksum + 签名
  └── 上传到 GitHub Release

build.rs 继续负责:
  └── Z3 下载/链接（已有逻辑，改为全平台动态链接）

YaoXiang 自定义脚本（package-dist.sh）负责:
  ├── 构建后重组 zip 结构（bin/ + lib/）
  ├── 附带共享库（libz3.so / dylib / dll）
  └── 预生成标准库 .yx 接口文件
```

### 发布目录结构

每个平台 release 压缩包，由 `package-dist.sh` 在 cargo-dist 构建后重组：

```
yaoxiang-{version}-{target}.tar.gz / .zip
├── bin/
│   ├── yaoxiang                      # 或 yaoxiang.exe
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # 预生成的标准库接口文件
│           ├── io.yx
│           ├── math.yx
│           ├── string.yx
│           ├── ...
│           └── mod.yx
├── README.md
└── LICENSE
```

cargo-dist 默认的 zip 是扁平的（二进制 + 自动包含的 README/LICENSE 都在根目录）。这不构成问题 — 明确分工：cargo-dist 管编译+CI+安装器，YaoXiang 用 50 行
`package-dist.sh` 管 zip 结构。

### 平台支持

| 平台           | target triple               | 说明          |
| -------------- | --------------------------- | ------------- |
| Linux x86_64   | `x86_64-unknown-linux-gnu`  | 主平台        |
| Linux ARM64    | `aarch64-unknown-linux-gnu` | CI 上交叉编译 |
| macOS x86_64   | `x86_64-apple-darwin`       | Intel Mac     |
| macOS ARM64    | `aarch64-apple-darwin`      | Apple Silicon |
| Windows x86_64 | `x86_64-pc-windows-msvc`    | 主平台        |

暂不支持 Windows ARM64（Z3 官方无预编译 ARM64 包）。

### Z3 分发策略

**统一改为全平台动态链接。**

| 平台    | 改动                | 产物          |
| ------- | ------------------- | ------------- |
| Linux   | **原静态→改为动态** | `libz3.so`    |
| macOS   | **原静态→改为动态** | `libz3.dylib` |
| Windows | 不变                | `libz3.dll`   |
| wasm32  | 不变（静态链接）    | 内嵌 `.a`     |

理由：

- **一致性** — 三个平台行为统一，不再各有特例
- **这是外部库，就该用共享库分发**。Python（`python3.dll`+`DLLs/lib*.dll`）、Node（`node`+`lib/`）都这么干
- **用户升级 Z3 不需要等编译器版本** — 换一个 `.so`/`.dylib`/`.dll` 就行
- **二进制体积更小** — Z3 不小，静态链接会让 exe 膨胀数 MB

对应的 `build.rs` 修改：

```rust
// 统一动态链接
fn link_z3(z3_dir: &Path) {
    println!("cargo:rustc-link-lib=z3");     // 不再区分 Windows/非 Windows
    // 保持 C++ 标准库链接不变
    let cxx = if target_os == "macos" { "c++" } else { "stdc++" };
    println!("cargo:rustc-link-lib={}", cxx);
}
```

**"全平台静态链接"不再作为目标。**
这不是消除特殊情况，是用错误的方式消除一个合理的情况。共享库是外部库的正常分发方式。

### 安装器支持

| 安装器           | 状态              | 说明                           |
| ---------------- | ----------------- | ------------------------------ |
| zip / tar.gz     | ✅ 默认           | 所有平台，手动下载             |
| shell 脚本       | ✅ cargo-dist     | Unix: `curl ... \| sh`         |
| powershell 脚本  | ✅ cargo-dist     | Windows: `irm ... \| iex`      |
| Homebrew formula | ✅ cargo-dist     | macOS: `brew install yaoxiang` |
| Windows MSI      | ✅ cargo-dist     | 基于 WiX，主 Windows 安装器    |
| **Inno Setup**   | **✅ 保留为辅助** | 国内用户备选，不删除           |

**Inno Setup 保留理由：**

- 国内 Windows 用户更习惯 exe 安装向导（下一步 → 下一步 → 完成）
- MSI 在某些企业/学校网络环境中被屏蔽
- 多维护一个 `setup.iss` 的成本远低于丢掉一部分用户

### 标准库接口文件生成

子命令名：**`yaoxiang package gen-std`**（与现有 `package init`/`add`/`install` 在同一体系下）

当前 `src/std/gen_interfaces.rs`
已有完整实现（`generate_all_interfaces()`、`write_interfaces_to_dir()`），只需在 `main.rs`
新增子命令入口，然后在 `package-dist.sh` 中调用：

```bash
yaoxiang package gen-std --out-dir "$PKG_ROOT/lib/yaoxiang/std/"
```

### Wasm 构建

**保持独立，不迁入 cargo-dist。**

cargo-dist 管的是"把编译器发给用户"，wasm 是"在线 playground 嵌入文档网站"——两套完全不同的交付物。

| 方面        | 做法                                   |
| ----------- | -------------------------------------- |
| 构建工具    | 保持 `wasm-pack build`                 |
| CI workflow | 保留 `_build-wasm.yml` 独立 job        |
| 触发时机    | 跟 release 同一次 push，并行的独立 job |
| 发布目标    | `docs/public/wasm/` → GitHub Pages     |

### npm 发布

两个不同的 npm 包，各自独立：

| 包                     | 内容                       | 工具                      | 状态                |
| ---------------------- | -------------------------- | ------------------------- | ------------------- |
| `@yaoxiang/cli`        | 下载 CLI 二进制（wrapper） | cargo-dist 原生生成       | cargo-dist 配置即用 |
| `@yaoxiang/playground` | wasm 库（JS + .wasm）      | wasm-pack + `npm publish` | 可选，目前只发 docs |

两者不冲突，名字也不冲突。

### Nightly 发布

cargo-dist 无原生 nightly 支持（[#1143](https://github.com/axodotdev/cargo-dist/issues/1143)，仍为 open
feature request）。

**保持现有 cron + tag 方案**，构建部分换成 cargo-dist：

```yaml
# nightly.yml（迁移后，约 50 行）
on: schedule: "17 22 * * *"
jobs:
  build:
    # 复用 cargo-dist 构建能力，但不走它的 release 流程
    uses: ./.github/workflows/release.yml  # cargo-dist 生成的构建 job
  publish:
    # 沿用现有：打 nightly tag → 覆盖 GitHub Pre-release
```

### cargo-dist 配置（草案）

跑 `cargo dist init` 后生成的初始配置，预期核心部分：

```toml
[workspace]
members = ["cargo:."]

[dist]
targets = [
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-pc-windows-msvc",
]
installers = [
  "shell",
  "powershell",
  "homebrew",
  "msi",
]
```

具体配置项以 `cargo dist init` 实际生成为准。

### package-dist.sh（草案）

```bash
#!/bin/bash
# 在 cargo-dist 构建后执行，重组发行包结构
# 被 cargo-dist 的 extra-artifacts 或独立 CI step 调用
set -euo pipefail

VERSION="$1"
TARGET="$2"
DIST_DIR="target/distrib"
PKG_ROOT="$DIST_DIR/yaoxiang-$VERSION-$TARGET"

mkdir -p "$PKG_ROOT/bin" "$PKG_ROOT/lib/yaoxiang/std"

# binary
mv "$DIST_DIR/yaoxiang" "$PKG_ROOT/bin/"

# 共享库
Z3_DIR=".z3/z3-4.16.0-..."
case "$TARGET" in
  *windows*)   cp "$Z3_DIR/bin/libz3.dll"   "$PKG_ROOT/bin/" ;;
  *linux*)     cp "$Z3_DIR/lib/libz3.so"    "$PKG_ROOT/bin/" ;;
  *apple*)     cp "$Z3_DIR/lib/libz3.dylib" "$PKG_ROOT/bin/" ;;
esac

# 标准库接口文件
yaoxiang package gen-std --out-dir "$PKG_ROOT/lib/yaoxiang/std/"

# README + LICENSE
cp README.md LICENSE "$PKG_ROOT/"

# 重新打包
cd "$DIST_DIR"
tar czf "yaoxiang-$VERSION-$TARGET.tar.gz" "yaoxiang-$VERSION-$TARGET"
```

### 标准库接口文件生成

当前 `src/std/gen_interfaces.rs` 已经实现了生成 `.yx`
接口文件的功能（`write_interfaces_to_dir`），`package init` 命令也调用了它。

只需要在 `main.rs` 中新增子命令入口，然后在打包脚本中调用。

### 废弃的手写 CI

迁移完成后删除以下文件：

| 文件                                     | 行数        | 替代                           |
| ---------------------------------------- | ----------- | ------------------------------ |
| `.github/workflows/_build-platforms.yml` | 255         | cargo-dist 自动生成            |
| `.github/workflows/release.yml`          | 176         | cargo-dist 自动生成            |
| `.github/workflows/nightly.yml`          | 145         | cargo-dist 构建 + 保留发布逻辑 |
| `scripts/build/setup.iss`                | ~250        | **保留**（国内用）             |
| **合计删减**                             | **~600 行** |                                |

保留的：

- `ci.yml`（日常 fmt + clippy + test + MSRV，不属于发布流程）
- `nightly.yml`（发布逻辑部分保留）
- `_build-wasm.yml`（独立构建流）
- `_build-z3-wasm.yml`（wasm 专用 Z3）
- `setup.iss`（国内辅助安装器）
- `docs-deploy.yml`（文档部署）

## 权衡

### 优点

- **开箱即用** — 用户下载解压后直接运行，没有缺失 DLL 的问题
- **减少维护成本** — 删除 ~600 行手写 CI YAML，cargo-dist 自动维护
- **标准化** — 业界标准工具，几百个项目验证过
- **跨平台一致性** — 全平台动态链接，行为统一
- **安装器覆盖** — shell/powershell/homebrew/msi/inno setup 全支持

### 缺点

- **学习 cargo-dist 配置** — 团队需要学习新工具
- **自定义打包脚本仍有维护成本** — 包结构和标准库接口文件的脚本需要维护
- **cargo-dist 版本迭代** — 需要关注 upstream 变更
- **cargo-dist 无原生 nightly** — nightly 发布部分仍需手写

### 与 RFC-014b 的关系

|          | RFC-014b                              | RFC-037                  |
| -------- | ------------------------------------- | ------------------------ |
| **范围** | 第三方包的构建和分发                  | 编译器本身的打包和分发   |
| **工具** | `yaoxiang build` / `yaoxiang publish` | `cargo-dist`             |
| **产物** | 第三方包的 FFI 库                     | 编译器 + 标准库 + 工具链 |
| **互斥** | 否，互补                              | 否，互补                 |

## 替代方案

| 方案                       | 为什么不选                                     |
| -------------------------- | ---------------------------------------------- |
| **继续手写 CI**            | 已经手写了 ~900 行，重复劳动，容易遗漏 DLL     |
| **自己写打包工具**         | 不要重新发明轮子，cargo-dist 已经成熟          |
| **只用 tar.gz 不用安装器** | 用户需要更友好的安装方式（Homebrew/MSI）       |
| **Docker 分发**            | 编译器和语言工具链需要原生二进制，不是容器场景 |
| **全静态链接 Z3**          | 外部库正常就该用共享库分发，不追求静态         |
| **废弃 Inno Setup**        | 国内用户习惯不同，保留成本极低                 |

## 实现策略

### 阶段一：build.rs 修改 + gen-std 子命令（P0）

1. 修改 `build.rs`：全平台统一动态链接，`copy_dll()` 扩展为 `copy_shared_lib()`
2. 在 `main.rs` 新增 `yaoxiang package gen-std` 子命令（复用 `gen_interfaces.rs`）

### 阶段二：cargo-dist 接入（P0）

1. 跑 `cargo dist init` 生成初始配置
2. 编写 `package-dist.sh` 打包脚本
3. 在 `release.yml` 中集成：cargo-dist 构建 → `package-dist.sh` 重组 → 上传
4. 验证生成的压缩包结构和内容正确

### 阶段三：旧 CI 下线（P1）

1. 并行运行新旧 CI，对比产物
2. 确认无误后删除 `_build-platforms.yml`
3. 精简 `nightly.yml`（构建部分换成 cargo-dist）
4. 确认 `setup.iss` 仍然可用

### 阶段四：安装器启用（P2）

1. 配置 Homebrew tap 自动发布
2. 配置 MSI 安装器生成
3. 配置 npm 发布（`@yaoxiang/cli`）

## 开放问题（已关闭）

以下问题在设计讨论中已解决：

- ~~Windows 上 Z3 静态链接的可行性？~~ → **不做静态链接，全平台动态**
- ~~gen-std-interfaces 子命令命名？~~ → **`yaoxiang package gen-std`**
- ~~是否保留 Inno Setup？~~ → **保留**
- ~~cargo-dist extra-artifacts 条件执行？~~ → **用 `package-dist.sh` 脚本处理，走 shell case 分支**
- ~~标准库接口版本兼容性？~~ → **随编译器版本一起发布，同一压缩包内**

## 参考文献

- [cargo-dist 官方文档](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: 构建系统与二进制分发](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 构建配置 — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)
