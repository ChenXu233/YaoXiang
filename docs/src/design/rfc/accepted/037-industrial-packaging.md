---
title: 'RFC-037: 工业化分发方案 — 基于 cargo-dist 的编译器/工具链打包'
author: 'ChenXu233'
created: '2026-07-26'
updated: '2026-09-09'
accepted: '2026-09-09'
issue: '#230'
status: '已接受'
---

# RFC-037: 工业化分发方案 — 基于 cargo-dist 的编译器/工具链打包

> 本 RFC 与 [RFC-014b: 构建系统与二进制分发](../review/014b-build-system.md) 互补。RFC-014b 定义了
> **YaoXiang 包管理器**如何构建和分发第三方包；本 RFC 定义
> **YaoXiang 编译器/工具链本身**如何打包和分发。

## 摘要

用 `cargo-dist`（Rust 生态的二进制分发工具）承担跨平台构建编排，自有脚本负责发行包结构。核心承诺两条：发行包**物理携带标准库源码目录**（用户像读 Python `Lib/` 一样直接阅读），以及全平台动态链接的 Z3 共享库随包分发。命令模型为**前门/引擎分离**：常用命令 `yx`（小前门，内建版本管理——rustup/Go GOTOOLCHAIN 对位），引擎 `yaoxiang-rs`（现 `yaoxiang` 单体改名），避免 Python/Node 事后靠 nvm/pdm 补课的生态分裂。安装方式双层：标准渠道对齐 Go/Zig——**发行包即产品**，解压 + PATH；傻瓜渠道一行命令安装（Linux `apt` / `curl | sh`，Windows `irm | iex` / Inno exe 向导）。解决 `libz3.dll` 缺失、标准库对用户不可见、CI 脚本重复维护等问题。

## 动机

### 为什么需要这个特性？

下载 YaoXiang 的用户应该能**开箱即用**，不需要任何额外步骤；标准库对用户**直接可读**，而不是藏在二进制里的黑盒。

### 当前的问题

#### 问题 1：Windows 用户下载后跑不了

当前 Release 只上传 `yaoxiang.exe`，但 `libz3.dll` 没有打包进去。用户在 Windows 上双击运行会报错：

```
The code execution cannot proceed because libz3.dll was not found.
```

这是 **中断性 bug** — 用户连第一步都走不过去。

#### 问题 2：Release 制品只有单文件 exe，标准库对用户不可见

现状是三重断裂：

- Release 制品只有裸二进制，标准库不随发行
- LSP 的接口文件查找链近乎失联：调用 `find_std_interface_file` 时不传项目目录（只查全局 `~/.yaoxiang/std/`，而无任何流程填充它）；`package init` 写入的是 `.yaoxiang/std`，不在查找链上
- 标准库源码（`.yx` 层）与接口视图（native 层）对用户完全黑盒

工业化的做法：用户像读 Python 的 `Lib/` 一样直接打开标准库目录读源码——**发行包物理携带 std 目录是本方案的硬需求**（已裁决）。

#### 问题 3：CI 手写脚本重复维护

当前维护着多套构建流水线：

| 文件                      | 职责              | 行数        |
| ------------------------- | ----------------- | ----------- |
| `_build-platforms.yml`    | 跨平台构建        | ~255 行     |
| `release.yml`             | 版本发布          | ~189 行     |
| `nightly.yml`             | 每日构建          | ~173 行     |
| `scripts/build/setup.iss` | Inno Setup 安装器 | ~250 行     |
| **合计**                  |                   | **~870 行** |

大部分是重复的（安装 Rust → 缓存 → 构建 → 重命名 → 上传），每个平台都要写一次。

#### 问题 4：Inno Setup 版本号硬编码

`setup.iss` 里 `MyAppVersion` 写死 `0.7.0`，构建时靠 `sed` 替换。迟早会翻车。

#### 问题 5：与 RFC-014b 的边界模糊

RFC-014b 定义了"YaoXiang 包的构建和分发机制"（即 `yaoxiang.toml` 中的 `[build]` 和 `[binaries]` 配置），但**没有覆盖"YaoXiang 编译器本身怎么发布"**。本 RFC 填补这个空白。

## 提案

### 核心设计

cargo-dist 只承担**构建编排层**；包结构、安装器全部自有。职责划分：

```
cargo-dist 职责（构建编排层）:
  ├── 跨平台编译（5 个 target）
  └── 生成压缩包与 checksum
  （原生安装器与 npm wrapper 弃用——其扁平二进制假设与 bin/+lib/ 结构冲突）

build.rs 继续负责:
  └── Z3 下载/链接（全平台动态 + rpath）

YaoXiang 自有脚本:
  ├── package-dist.sh — 重组包结构（bin/ + lib/）、附带共享库、
  │   填充 std 目录（gen-std 接口视图 + .yx 层源码）、重算 checksum
  └── Inno Setup — Windows 安装向导（既有资产；铺设完整目录结构）

命令模型（前门/引擎分离）:
  ├── yx — 前门（新小 crate）：版本解析 + 派发；保留动词仅 toolchain/self，其余透传
  └── yaoxiang-rs — 引擎（现 yaoxiang 单体改名）：编译/运行/包管理/fmt/lsp 子命令

安装方式（双层）:
  ├── 标准渠道（Go/Zig 模式）: 发行包即产品，解压 + PATH
  └── 傻瓜渠道（Rust 模式）: 一行命令安装 + 版本管理（内建于 yx 前门）
      ├── Linux: apt（自建 deb 仓库，系统级平装）/ curl … | sh（装前门）
      ├── Windows: irm … | iex（装前门）/ Inno Setup 向导（既有资产，系统级平装）
      └── macOS: curl … | sh（brew 留待 homebrew-core 社区）
```

### 发布目录结构（已裁决：物理携带标准库源码）

用户必须能像读 Python 的 `Lib/` 一样直接阅读标准库——发行包自带 std 目录是硬需求，不是打包细节。Z3 同理：作为外部系统，目录式共享库分发就是它的自然形态——把 `.so` 塞进 exe 与放在外面动态链接，在"都要随发行包走"上等价，后者还保留了可替换性。

每个平台的发行包，由 `package-dist.sh` 在 cargo-dist 构建后重组：

```
yaoxiang-{version}-{target}.tar.gz / .zip     （便携即用：解压后 bin/ 内直接运行）
├── bin/
│   ├── yx                            # 前门（或 yx.exe）
│   ├── yaoxiang-rs                   # 引擎（或 yaoxiang-rs.exe）
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # 用户可直接阅读（Python Lib/ 模式）
│           ├── io.yx                 # native 模块：gen-std 生成的接口视图
│           ├── math.yx
│           ├── test.yx               # .yx 层：仓库内真实源码原样复制
│           └── ...
├── README.md
└── LICENSE
```

安装 = 解压到任意目录 + 把 `bin/` 加入 PATH（Go 的 `/usr/local/go/bin` 模式；解压到 `~/.yaoxiang/` 是常见选择）。Windows 上 Inno Setup 向导做同一件事（默认 Program Files）。便携解压时 `yx` 前门无 `~/.yaoxiang` 状态，回退到相邻的 `yaoxiang-rs`——与托管安装行为一致；引擎的 rpath 与 exe 相对 std 查找不因前门存在而改变。

### 平台支持

| 平台           | target triple               | 说明          |
| -------------- | --------------------------- | ------------- |
| Linux x86_64   | `x86_64-unknown-linux-gnu`  | 主平台        |
| Linux ARM64    | `aarch64-unknown-linux-gnu` | CI 上交叉编译 |
| macOS x86_64   | `x86_64-apple-darwin`       | Intel Mac     |
| macOS ARM64    | `aarch64-apple-darwin`      | Apple Silicon |
| Windows x86_64 | `x86_64-pc-windows-msvc`    | 主平台        |

共 5 个 target。暂不支持 Windows ARM64（Z3 官方无预编译 ARM64 包）。

### Z3 分发策略

**全平台动态链接**（已复核维持）：

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

动态链接有一个**必要配套**：Linux/macOS 的动态链接器默认不搜二进制所在目录，必须注入 rpath，否则"解压即用"不成立（Windows 默认搜 exe 目录，无需处理）。对应的 `build.rs` 修改：

```rust
// 统一动态链接 + rpath
fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    println!("cargo:rustc-link-lib=z3");     // 不再区分 Windows/非 Windows
    // 运行期在二进制所在目录查找 libz3（bin/ 内 exe 与共享库同目录）
    match target_os.as_str() {
        "linux" => println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN"),
        "macos" => println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path"),
        _ => {}                                // Windows 默认搜 exe 目录
    }
    // 保持 C++ 标准库链接不变
    let cxx = if target_os == "macos" { "c++" } else { "stdc++" };
    println!("cargo:rustc-link-lib={}", cxx);
}
```

**"全平台静态链接"不作为目标。**这不是消除特殊情况，是用错误的方式消除一个合理的情况。共享库是外部库的正常分发方式。

### 安装器支持

对照主流语言工具链的分发方式（2026-09 调研）：

| 语言    | 官方发行物                 | 官方安装方式                        | 安装器维护方            |
| ------- | -------------------------- | ----------------------------------- | ----------------------- |
| Go      | `go/{bin,src,pkg}` tarball | 官方文档即"下载 → 解压 → PATH"     | 无（brew/apt 为社区）   |
| Zig     | `zig/{bin,lib/std}` tarball | 同上，无官方安装脚本               | 无（homebrew-core 社区） |
| Node    | `{bin,lib,include}` tarball | tar + 官方 pkg/msi                 | 团队自写                |
| Rust    | 多组件 tarball             | rustup                              | 团队自写                |
| Crystal | `{bin,src,embedded}` tarball | deb/rpm/tar                        | 团队 + brew 社区        |
| Deno/Bun | 单二进制 zip              | 官方 curl 脚本                      | 团队自写（脚本极小）    |
| Gleam   | cargo-dist 单二进制        | cargo-dist 生成脚本                 | cargo-dist              |

三条规律：

- **多文件工具链没有一家用第三方生成器做安装器**——cargo-dist 的安装器只适配单二进制场景（Gleam 可用正因它是无外部依赖的单一 Rust 二进制）
- 最简模型是 **Go/Zig 的"发行包即产品"**：官方安装指引就是解压 + PATH，零安装器代码；发行包自带可读 std 源码（Go 的 `src/`、Zig 的 `lib/std/`、Crystal 的 `src/`）是常态
- 想要 curl 一键装的（Deno/Bun/rustup）都是**自写脚本**且几乎不演化；brew 公式一律社区维护在 homebrew-core，语言团队不自建 tap（Crystal 团队明确说 formula 是社区的）

YaoXiang 采用双层模型：

| 渠道                                 | 层   | 状态 | 说明                                                       |
| ------------------------------------ | ---- | ---- | ---------------------------------------------------------- |
| zip / tar.gz                         | 标准 | ✅   | 解压即用（rpath + 同目录共享库），extract + PATH 即官方指引 |
| `yx`（前门，内建版本管理）           | 傻瓜 | ✅   | rustup/Go GOTOOLCHAIN 对位：多版本安装/切换/更新 + 项目 pin（阶段五交付） |
| `curl ... \| sh`（install.sh）       | 傻瓜 | ✅   | Linux / macOS：装 yx 前门 → 前门装默认引擎（~40 行）        |
| `irm ... \| iex`（install.ps1）      | 傻瓜 | ✅   | Windows：同逻辑（~40 行）                                  |
| `apt install yaoxiang`               | 傻瓜 | ✅   | `.deb`（amd64/arm64）+ GitHub Pages 静态 apt 仓库；系统级平装，`apt upgrade` 跟版本 |
| Inno Setup exe                       | 傻瓜 | ✅   | Windows 向导（既有资产），系统级平装，铺设完整 bin/+lib/ 结构 |
| winget / `.rpm` / homebrew-core / npm | —  | ⏸   | 可选跟进：winget 与 brew-core 皆社区维护，rpm 与 deb 同构   |
| MSI / cargo-dist 原生安装器 / 自建 brew tap | — | ❌ | 见"替代方案"                                        |

**傻瓜渠道参考 Rust，版本管理内建于前门。**Rust 的分解是"引导脚本（sh.rustup.rs）→ rustup → 工具链包"，rustup 一开始就把多版本、切换、更新管住了；Python/Node 官方安装器没做这层，生态事后长出 pyenv/nvm/pdm 各自为政。YaoXiang 采用**前门/引擎分离**（Go 的 `go` 前门 + GOTOOLCHAIN、rustup 的代理派发，两家的同构形态）：常用命令是 `yx`，引擎是 `yaoxiang-rs`——

```
~/.yaoxiang/
├── bin/yx                  # 前门：小二进制，版本解析 + 派发（项目 pin > 默认 > 相邻引擎）
├── settings.toml           # 默认版本、镜像源
└── versions/               # 版本是第一级概念；<ver>/ 即该版本发行包的解压根目录
    └── 0.7.14/
        ├── bin/
        │   ├── yaoxiang-rs      # 引擎：编译/运行/包管理/fmt/lsp 子命令
        │   └── libz3.so
        └── lib/yaoxiang/std/
```

- 安装根 `~/.yaoxiang/` 对齐业界惯例（pyenv `~/.pyenv`、nvm `~/.nvm`、deno `~/.deno`、bun `~/.bun`、volta `~/.volta` 皆单根；rustup 的 `~/.cargo`+`~/.rustup` 双根是 cargo 先于 rustup 存在的历史包袱，不效仿），且 `~/.yaoxiang` 本就是代码库既有命名空间（std 全局回退槽）；支持 `YAOXIANG_HOME` 环境变量覆盖（先例 RUSTUP_HOME / DENO_INSTALL，服务 CI 与容器场景）；Windows 为 `%USERPROFILE%\.yaoxiang`
- **版本目录 = 发行包解压根目录**：`versions/<ver>/` 与便携解压、deb 安装树完全同构，安装一个版本就是解压一个发行包——三种渠道零结构分叉
- 命令面（rustup 对位）：`yx toolchain install / default / update / list / uninstall`，含 `yx self update`；其余动词原样透传给引擎
- **版本锁定是结构性保证**：fmt 等工具与语法同版本演进（旧 fmt 不认新语法），版本解析在门前一次完成、整组切换——不存在"新引擎配旧 fmt"的组合空间；未来若把 fmt/LSP 拆成独立二进制，也落在同版本 `bin/` 内
- 项目级 pin：`yx-toolchain.toml`（先例 rust-toolchain.toml，跟随命令名；不进 `yaoxiang.toml`——包清单不该把工具链版本强加给库使用者）
- 引导入口（`curl | sh` / `irm | iex`）装的是前门，前门再装默认 stable 引擎——正是 sh.rustup.rs → rustup-init 的分解
- 镜像源可配（settings.toml），延续国内用户考量（与 Z3 下载同款网络问题）
- **版本管理不破坏自包含不变量**：每个版本就是一棵完整发行树，rpath 与 exe 相对 std 查找在树内自洽，前门只做派发不改结构

`.deb` 与 Inno 是**系统级平装**通道（root / Program Files 单版本，由 `apt upgrade` / 控制面板跟新），服务服务器、CI、纯新手场景；与管理器共存靠 PATH 顺序（先例：apt 的 rustc 与 rustup 并存）。全部渠道共享同一套产物树。

`.deb` 布局复用同一棵目录树：`/usr/lib/yaoxiang/{yx,yaoxiang-rs,libz3.so,std/}` + `/usr/bin/yx` 符号链接——`$ORIGIN` 按解析后的**真实路径**计算，链接后仍命中同目录 `libz3.so`，与解压包结构同构。品牌全名留在包名与产品名上（`apt install yaoxiang`、Inno 产品名 YaoXiang），命令面统一 `yx`——同 Go：包名 `golang-go`，命令 `go`。apt 仓库静态托管在 GitHub Pages（Packages/Release/InRelease 元数据 GPG 签名，由 release CI 发布）；远期可申请 Debian/Ubuntu 官方收录（周期长、版本滞后，非主路径）。

### 标准库目录

`lib/yaoxiang/std/` 的内容有两个来源：

| 层                        | 来源                                        | 性质                     |
| ------------------------- | ------------------------------------------- | ------------------------ |
| native 模块（io/math/…）  | `yaoxiang package gen-std`（打包期调用）    | 接口签名视图（实现在二进制内） |
| .yx 层模块（test/…增长中）| 仓库 `src/std/*.yx` 原样复制                | 真实源码                 |

子命令名：**`yaoxiang package gen-std`**（与现有 `package init`/`add`/`install` 在同一体系下）。当前 `src/std/gen_interfaces.rs` 已有完整实现（`generate_all_interfaces()`、`write_interfaces_to_dir()`），只需在 `main.rs` 新增子命令入口。

**运行时查找链**——`find_std_interface_file` 增补一级 exe 相对查找：

1. 项目 `.yaoxiang/vendor/std/<name>.yx`（项目覆盖，现状）
2. **exe 所在目录 `../lib/yaoxiang/std/<name>.yx`（新增）**：便携解压与安装到 `~/.yaoxiang/{bin,lib}` 统一命中
3. `~/.yaoxiang/std/<name>.yx`（全局回退，保留为手工覆盖位）

现状这条链近乎失联（LSP 调用不传项目目录、`package init` 写入的 `.yaoxiang/std` 不在链上），本次顺手接活，并统一 `package init` 输出到 `.yaoxiang/vendor/std`（与包管理器 vendor 目录一致）。

**编译权威不改变**：`.yx` 层仍走 `include_str!` 内嵌（RFC-036 的"std 版本与二进制严格绑定"不变量保持）。发行目录定位是**可读视图 + LSP 解析源**，不是编译输入——用户手改发行目录里的 `.yx` 不会被编译器采用（是否开放 Python 式"改 `Lib/` 即生效"语义，见开放问题）。

### Wasm 构建

**保持独立，不迁入 cargo-dist。**

cargo-dist 管的是"把编译器发给用户"，wasm 是"在线 playground 嵌入文档网站"——两套完全不同的交付物。

| 方面        | 做法                                   |
| ----------- | -------------------------------------- |
| 构建工具    | 保持 `wasm-pack build`                 |
| CI workflow | 保留 `_build-wasm.yml` 独立 job        |
| 触发时机    | 跟 release 同一次 tag push，并行的独立 job |
| 发布目标    | `docs/public/wasm/` → GitHub Pages     |

### npm 发布

| 包                     | 内容                       | 状态                                       |
| ---------------------- | -------------------------- | ------------------------------------------ |
| `@yaoxiang/cli`        | 下载发行包的 wrapper       | 顺延：cargo-dist 的 npm wrapper 同样基于扁平制品假设，随安装器一并弃用；如需 npm 渠道，自写 wrapper（下载重组包解压，与解压安装同逻辑） |
| `@yaoxiang/playground` | wasm 库（JS + .wasm）      | 可选，目前只发 docs                        |

两者不冲突，名字也不冲突。

### 与现有发版流程的整合

现状 `release.yml`：push main → check-version（`v{version}` tag 不存在才放行）→ build / build-wasm / security / test 四路 → release job（打 tag 推送 + `generate-commit-list.mjs` 生成含 @mentions 的 body + 上传制品）。

cargo-dist 生成的流水线是 tag 驱动、自带 announce/publish，且不含 fmt/clippy/test/audit 门禁，release notes 格式也无法承载 merge commit changelog。**直接整体替换会打碎现有发版仪式**（PR → CI 全绿 → bump → merge commit 即 changelog）。

整合原则：**触发与门禁保持现状，构建交给 `cargo dist build`，发布保持现状。**

1. check-version / security / test 三个 job 保持现状（push main 触发、打 tag 前置门禁）
2. 全部通过后由 release job 创建并推送 `v{version}` tag（现状不变）
3. tag push 触发新的 `dist-release.yml`：`cargo dist build`（5 target 矩阵）+ `package-dist.sh` 逐 target 重组 + `_build-wasm.yml`（并行 job）
4. publish job：`generate-commit-list.mjs` 生成 body（现状脚本复用）→ 上传重组包 + `.sha256`（阶段四起追加 `.deb` 与安装脚本，并触发 apt 仓库元数据发布）

### Nightly 发布

cargo-dist 无原生 nightly 支持（[axodotdev#1143](https://github.com/axodotdev/cargo-dist/issues/1143)，仍为 open feature request）。

保持现有 cron + tag 覆盖方案，构建部分从 `_build-platforms.yml` 换成 `cargo dist build`——它本质是一条 cargo 命令，可直接在 nightly.yml 里调用。不走 workflow 复用（设想的 `uses: ./release.yml` 不可行：被复用方需要 `workflow_call` 触发器，且 cargo-dist 工作流为 tag 驱动、构建与发布耦合）：

```yaml
# nightly.yml（迁移后）
on:
  schedule:
    - cron: '17 22 * * *'
jobs:
  build:     # cargo dist build + package-dist.sh（与正式版同一套）
  publish:   # 现状保留：打/移 nightly tag → 覆盖 GitHub Pre-release
```

### cargo-dist 配置（草案）

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
installers = []   # 安装器全部自有，cargo-dist 只做构建 + 压缩包 + checksum
```

具体配置项以 `cargo dist init` 实际生成为准；`dist-version` 锁定固定版本，生成物 vendor 进仓库接受 review，不运行时拉取。

### package-dist.sh（草案）

```bash
#!/bin/bash
# cargo-dist 构建后重组发行包结构；被 dist-release.yml / nightly.yml 调用
set -euo pipefail

VERSION="$1"
TARGET="$2"
DIST_DIR="target/distrib"
PKG_ROOT="$DIST_DIR/yaoxiang-$VERSION-$TARGET"

mkdir -p "$PKG_ROOT/bin" "$PKG_ROOT/lib/yaoxiang/std"

# 前门 + 引擎（cargo-dist 构建 workspace 双二进制）
mv "$DIST_DIR/yaoxiang-$VERSION-$TARGET"*/yaoxiang-rs "$PKG_ROOT/bin/"
mv "$DIST_DIR/yaoxiang-$VERSION-$TARGET"*/yx "$PKG_ROOT/bin/"
if [ "$TARGET" = "x86_64-pc-windows-msvc" ]; then
  mv "$PKG_ROOT/bin/yx" "$PKG_ROOT/bin/yx.exe"
  mv "$PKG_ROOT/bin/yaoxiang-rs" "$PKG_ROOT/bin/yaoxiang-rs.exe"
fi

# 共享库：目录名对齐 build.rs detect_target() 的 Z3 发行包命名
# （此映射应收敛为单源——理想做法由构建侧输出，避免脚本与 build.rs 两处维护）
Z3_VERSION="4.16.0"
case "$TARGET" in
  x86_64-windows*)  cp ".z3/z3-$Z3_VERSION-x64-win/bin/libz3.dll"           "$PKG_ROOT/bin/" ;;
  x86_64-linux*)    cp ".z3/z3-$Z3_VERSION-x64-glibc-2.39/lib/libz3.so"     "$PKG_ROOT/bin/" ;;
  aarch64-linux*)   cp ".z3/z3-$Z3_VERSION-arm64-glibc-2.38/lib/libz3.so"    "$PKG_ROOT/bin/" ;;
  x86_64-apple*)    cp ".z3/z3-$Z3_VERSION-x64-osx-15.7.3/lib/libz3.dylib"   "$PKG_ROOT/bin/" ;;
  aarch64-apple*)   cp ".z3/z3-$Z3_VERSION-arm64-osx-15.7.3/lib/libz3.dylib" "$PKG_ROOT/bin/" ;;
esac

# 标准库目录：gen-std（native 接口视图）+ .yx 层真实源码
yaoxiang package gen-std --out-dir "$PKG_ROOT/lib/yaoxiang/std/"
cp src/std/*.yx "$PKG_ROOT/lib/yaoxiang/std/"

cp README.md LICENSE "$PKG_ROOT/"

# 重新打包并重算 checksum（cargo-dist 生成的 .sha256 已因重组失效）
cd "$DIST_DIR"
if [ "$TARGET" = "x86_64-pc-windows-msvc" ]; then
  zip -r "yaoxiang-$VERSION-$TARGET.zip" "yaoxiang-$VERSION-$TARGET"
  sha256sum "yaoxiang-$VERSION-$TARGET.zip" > "yaoxiang-$VERSION-$TARGET.zip.sha256"
else
  tar czf "yaoxiang-$VERSION-$TARGET.tar.gz" "yaoxiang-$VERSION-$TARGET"
  sha256sum "yaoxiang-$VERSION-$TARGET.tar.gz" > "yaoxiang-$VERSION-$TARGET.tar.gz.sha256"
fi
```

### 废弃的手写 CI

迁移完成后调整的文件：

| 文件                                     | 行数        | 处置                                       |
| ---------------------------------------- | ----------- | ------------------------------------------ |
| `.github/workflows/_build-platforms.yml` | 254         | 删除（cargo-dist 构建矩阵替代）            |
| `.github/workflows/release.yml`          | 189         | 收缩为门禁 + 打 tag（构建/发布移入 dist-release.yml） |
| `.github/workflows/nightly.yml`          | 173         | 构建段换 `cargo dist build`，发布逻辑保留  |
| `scripts/build/setup.iss`                | ~250        | **保留并转正**（Windows 向导）         |
| **合计删减**                             | **~600 行** |                                            |

保留的：

- `ci.yml`（日常 fmt + clippy + test + MSRV，不属于发布流程）
- `_build-wasm.yml`（独立构建流，挂入 dist-release.yml 并行 job）
- `_build-z3-wasm.yml`（wasm 专用 Z3）
- `docs-deploy.yml`（文档部署）

### 验收标准

"开箱即用"是可测试的，迁移完成的判定不是"新旧产物一致"，而是以下全部通过：

- 干净机器（无 Rust / 无 Z3 / 无 `~/.yaoxiang`）解压任一平台压缩包，直接执行 `bin/yaoxiang --version` 成功——不设 `LD_LIBRARY_PATH`（rpath 生效）
- 解压目录内 `lib/yaoxiang/std/*.yx` 全部可读：native 模块为签名接口视图，`.yx` 层为真实源码
- 在解压目录下的示例项目上启动 LSP，std 成员补全 / 跳转定义可用（exe 相对查找生效）
- 按官方指引解压到 `/usr/local`（或 `~/.yaoxiang`）并加入 PATH 后，任意目录下 `yaoxiang --version` 成功
- `apt install yaoxiang`（自建仓库）后可运行、`apt upgrade` 能跟版本；`/usr/bin/yaoxiang` 符号链接下 `$ORIGIN` 命中同目录 `libz3.so`
- `curl ... | sh` 与 `irm ... | iex` 在干净环境执行后 `yaoxiang` 可运行、PATH 就位
- `yx toolchain install <ver>` / `default` / `update` 生效：多版本共存、`yx-toolchain.toml` 项目 pin 优先于默认版本、前门派发到正确版本（版本树内 rpath 与 std 查找自洽，无"新引擎配旧 fmt"组合）
- 便携解压后 `yx` 回退相邻 `yaoxiang-rs`，行为与托管安装一致
- Inno Setup 安装后目录结构完整、PATH 生效、可卸载
- Release 资产齐全：5 平台重组包 + `.sha256` 与实际内容一致
- Release body 为 `generate-commit-list.mjs` 输出（merge commit changelog 完整）
- nightly 产物为 Pre-release，不影响最新正式 tag

## 权衡

### 优点

- **开箱即用** — 便携解压即用（rpath + 同目录共享库），安装器铺设完整目录
- **标准库可读** — 用户像读 Python `Lib/` 一样直接阅读 std（硬需求达成）
- **减少维护成本** — ~600 行手写构建 YAML 换成 cargo-dist + ~80 行自有脚本
- **跨平台一致性** — 全平台动态链接 + 同目录共享库，无特例
- **安装双层** — 标准渠道零新增代码（Go/Zig 模式）；傻瓜渠道参考 Rust，apt / curl / iex / exe 四入口共享同一套产物结构
- **版本管理内建** — 前门 `yx` 对位 rustup/Go GOTOOLCHAIN，免于 Python/Node 事后靠 pyenv/nvm/pdm 补课的生态分裂；工具的版本锁定是结构保证而非约定

### 缺点与风险

- **傻瓜渠道维护面** — install.sh / install.ps1（~40 行 ×2，几乎不演化）+ `.deb` 与 apt 仓库元数据发布（release CI 自动化）+ `yx` 前门 crate（阶段五交付）
- **引擎改名波及面** — `yaoxiang` → `yaoxiang-rs` 需一次性迁移 CI 制品名、Inno、测试与文档（阶段五内完成）
- **学习成本** — 团队需要学习 cargo-dist 配置
- **cargo-dist 上游风险** — 2025 年中曾随 Axo 停摆，同年 9 月原作者复活并持续发版（0.29 → 0.32+）；以 `dist-version` 锁定 + 生成物 vendor 进仓库 review 缓解
- **cargo-dist 无原生 nightly** — nightly 发布部分仍需手写
- **Z3 目录命名两处维护** — `package-dist.sh` 与 `build.rs::detect_target()` 需保持同步（应收敛为单源）

### 与 RFC-014b 的关系

|          | RFC-014b                              | RFC-037                  |
| -------- | ------------------------------------- | ------------------------ |
| **范围** | 第三方包的构建和分发                  | 编译器本身的打包和分发   |
| **工具** | `yaoxiang build` / `yaoxiang publish` | `cargo-dist` + 自有脚本  |
| **产物** | 第三方包的 FFI 库                     | 编译器 + 标准库 + 工具链 |
| **互斥** | 否，互补                              | 否，互补                 |

## 替代方案

| 方案                       | 为什么不选                                     |
| -------------------------- | ---------------------------------------------- |
| **继续手写 CI**            | 已经手写了 ~870 行，重复劳动，容易遗漏 DLL     |
| **自己写打包工具**         | 不要重新发明轮子，cargo-dist 已经成熟          |
| **只用 tar.gz 不用安装器** | 唯一官方渠道就是解压 + PATH；Inno 仅服务 Windows 向导习惯（国内用户已裁决保留） |
| **Docker 分发**            | 编译器和语言工具链需要原生二进制，不是容器场景 |
| **自建 Homebrew tap** | tap 一律社区维护（homebrew-core），自建属超前需求；傻瓜渠道的 macOS 入口是 curl 脚本 |
| **独立 `yaoxiangup` 管理器二进制** | rustup 原样先例，可行；但制造第二个用户动词，且引出"包管理器/fmt 是否也该独立"的对称性诘问——前门/引擎分离一并消解（2026-09-09 讨论否决） |
| **只做自更新、不做多版本** | 单版本自更新解决不了多项目 pin 不同版本的需求；Python/Node 缺官方版本管理，生态被迫长出 pyenv/nvm/pdm——已裁决内建（2026-09-09） |
| **全静态链接 Z3**          | 已裁决否决——外部系统目录式共享库分发即其自然形态；塞进 exe 与放外面在"都要随包走"上等价，还失去可替换性 |
| **废弃 Inno Setup**        | 已裁决否决——保留为 Windows 向导（附加渠道）            |
| **cargo-dist 原生安装器**  | 扁平二进制假设与 bin/+lib/ 结构冲突，装出即缺库 |
| **自维护 WiX/MSI**         | 失去 cargo-dist 生成器后成本高于 Inno 已覆盖的价值 |

## 实现策略

### 阶段一：语言侧改动（P0）

1. `build.rs`：全平台统一动态链接 + rpath link-arg；`copy_dll()` 扩展为 `copy_shared_lib()`（so/dylib/dll）
2. `main.rs` 新增 `yaoxiang package gen-std` 子命令（复用 `gen_interfaces.rs`）
3. `find_std_interface_file` 增补 exe 相对查找分支；`package init` 输出路径统一到 `.yaoxiang/vendor/std`

### 阶段二：cargo-dist 接入（P0）

1. 跑 `cargo dist init` 生成初始配置（`installers = []`，锁定 dist-version）
2. 编写 `package-dist.sh`（重组 + .yx 源码复制 + checksum 重算）
3. 新建 `dist-release.yml`（tag 驱动：dist build → 重组 → wasm 并行 → 自有 publish）；`release.yml` 收缩为门禁 + 打 tag
4. 双跑新旧流水线，按验收标准逐项核验

### 阶段三：旧 CI 下线（P1）

1. 确认无误后删除 `_build-platforms.yml`
2. `nightly.yml` 构建段换 `cargo dist build`
3. `setup.iss` 接入新产物结构（Inno 转正；版本号从 Cargo.toml 注入，消灭 sed 替换）

### 阶段四：傻瓜渠道（P2）

1. `install.sh` / `install.ps1`（各 ~40 行：检测平台 → 下载 → 解压 → PATH；此阶段直装最新版，阶段五切换为安装管理器）
2. `.deb` 打包（复用 `package-dist.sh` 的同一棵目录树 + `/usr/bin` 符号链接）+ GitHub Pages 静态 apt 仓库（元数据 GPG 签名，release CI 发布）

### 阶段五：前门 yx 与引擎改名（P2）

1. 现单体改名 `yaoxiang-rs`（CI 制品名、Inno、测试与文档全量过一遍，一次性迁移）
2. 新增前门小 crate `yx`（workspace 成员）：版本解析、下载 release 制品、tar/zip 解包、settings.toml、派发（保留动词仅 toolchain/self，其余透传）；版本索引来自 GitHub Releases（或静态 versions.json 镜像）；`yx` 命令名做过撞名检查（主流发行版/Homebrew 无同名常用命令，仅一小众工具以 yx 为别名）
3. `yx toolchain install/default/update/list/uninstall` + `yx-toolchain.toml` 项目 pin + 镜像源 + `yx self update`
4. 引导脚本切换目标：`curl | sh` / `irm | iex` → 装 yx 前门 → 装默认 stable

### 阶段六：可选跟进（皆不阻塞）

1. winget 提交（指向 Inno exe，社区维护，对称 homebrew-core 模式）
2. `.rpm`（dnf 用户，与 `.deb` 同构）
3. Homebrew：知名度达 homebrew-core 准入门槛后由社区提交
4. npm `@yaoxiang/cli` 自写 wrapper（名称当前未被注册）

## 开放问题

### 未决

- **.yx 层是否提供"手改发行目录即被编译采用"的 Python 式语义？** 默认否——编译权威保持 RFC-036 内嵌（std 版本与二进制严格绑定），发行目录定位为可读视图 + LSP 解析源。若未来开放，需重新审视版本绑定不变量。

### 已关闭

以下问题在设计讨论中已解决：

- ~~Windows 上 Z3 静态链接的可行性？~~ → **不做静态链接，全平台动态**（2026-09-09 复核维持）
- ~~gen-std-interfaces 子命令命名？~~ → **`yaoxiang package gen-std`**
- ~~是否保留 Inno Setup？~~ → **保留为 Windows 向导（附加渠道）**
- ~~发行包结构是否物理携带标准库源码？~~ → **必须**（用户可读性对齐 Python `Lib/`，2026-09-09 裁决）
- ~~cargo-dist 原生安装器（shell/powershell/homebrew/msi/npm）？~~ → **全部弃用**，安装器自有（扁平假设与 bin/+lib/ 冲突）
- ~~安装器策略？~~ → **双层**（2026-09-09 终裁）：标准渠道 = 发行包即产品（Go/Zig 模式，解压 + PATH）；傻瓜渠道参考 Rust 一行命令安装——Linux `apt`（自建 deb 仓库）/ `curl | sh`，Windows `irm | iex` / Inno exe。不做：MSI、cargo-dist 原生安装器、自建 brew tap
- ~~版本管理器是否纳入？~~ → **必须，属安装器体系**（2026-09-09 裁决，推翻同日早先"远期独立 RFC"的边界）：动机是 Python/Node 缺官方版本管理导致 pyenv/nvm/pdm 补课的生态分裂
- ~~版本管理器形态：独立二进制 or 子命令？~~ → **前门/引擎分离**（2026-09-09 终裁，三轮收敛 A→C→命名反转）：常用命令 `yx` = 前门（小二进制，内建 toolchain/self 动词），引擎 `yaoxiang-rs` = 现 `yaoxiang` 单体改名；目录结构 `versions/<ver>/`——版本是第一级概念，版本目录即发行包解压根目录，工具随版本整体锁定（旧 fmt 不认新语法；曾设想的内层 `toolchains/` 因冗余取消）。先例：Go `go` 前门 + GOTOOLCHAIN、rustup 代理派发
- ~~cargo-dist extra-artifacts 条件执行？~~ → **用 `package-dist.sh` 脚本处理，走 shell case 分支**
- ~~标准库接口版本兼容性？~~ → **随编译器版本一起发布，同一压缩包内**

## 参考文献

- [cargo-dist 官方文档](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: 构建系统与二进制分发](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 构建配置 — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)
