---
title: 安装 YaoXiang
description: 双层安装渠道——标准渠道解压即用（Go/Zig 模式），傻瓜渠道一行命令 + 版本管理（Rust/rustup 模式）
---

# 安装 YaoXiang

YaoXiang 的命令面是**前门/引擎分离**（RFC-037）：

- **`yx`** — 前门，日常入口。内建版本管理（`yx toolchain` / `yx self update`），其余命令透传给引擎
- **`yaoxiang-rs`** — 引擎，负责编译/运行/包管理/格式化/LSP，一般不直接调用

安装分双层渠道，全部渠道共享同一套产物结构（`bin/` + `lib/yaoxiang/std/`）。

## 标准渠道：解压即用（Go/Zig 模式）

从 [GitHub Releases](https://github.com/ChenXu233/YaoXiang/releases) 下载对应平台的压缩包，解压到任意目录，把 `bin/` 加入 PATH：

```sh
tar xzf yaoxiang-<版本>-<平台>.tar.gz
export PATH="$PWD/yaoxiang-<版本>-<平台>/bin:$PATH"
```

解压后即可直接使用：`yx --version`。压缩包内自带全部依赖（含 Z3 共享库）与可阅读的标准库源码 `lib/yaoxiang/std/`，无需任何额外步骤。

## 傻瓜渠道：一行命令（Rust/rustup 模式）

### Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.ps1 | iex
```

脚本会把工具链装进 `~/.yaoxiang/`（Windows 为 `%USERPROFILE%\.yaoxiang`），并把 `yx` 放入 PATH。

### Linux: apt 仓库

```sh
curl -fsSL <apt 仓库地址>/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/yaoxiang.gpg
echo "deb [signed-by=/usr/share/keyrings/yaoxiang.gpg] <apt 仓库地址> stable main" | sudo tee /etc/apt/sources.list.d/yaoxiang.list
sudo apt update && sudo apt install yaoxiang
```

apt 安装为系统级平装（`/usr/lib/yaoxiang/`，命令入口 `/usr/bin/yx`），`apt upgrade` 跟随版本。仓库元数据由 release CI 自动签名发布。

### Windows: Inno Setup 向导

从 [Releases](https://github.com/ChenXu233/YaoXiang/releases) 下载 `YaoXiang-Setup-<版本>.exe`，按向导安装（可选自动加入 PATH）。与 apt 相同为系统级平装。

## 版本管理（yx 前门内建）

```sh
yx toolchain install stable    # 安装最新稳定版
yx toolchain install 0.7.14    # 安装指定版本
yx toolchain default 0.7.14    # 设置默认版本
yx toolchain list              # 列出已安装版本
yx toolchain update            # 升级到最新稳定版
yx self update                 # 更新 yx 本体
```

多版本共存于 `~/.yaoxiang/versions/<版本>/`，每个版本是一棵完整的自包含工具链树（引擎、Z3、标准库同版本锁定）。

### 项目级版本锁定

在项目根目录放一个 `yx-toolchain.toml`（先例 `rust-toolchain.toml`）：

```toml
toolchain = "0.7.14"
```

在该目录下执行任何 `yx` 命令都会使用 pin 指定的版本——不同项目可以用不同版本工作。

## 环境变量

| 变量 | 说明 |
| ---- | ---- |
| `YAOXIANG_HOME` | 覆盖安装根，默认 `~/.yaoxiang`（CI 与容器场景） |
| `YAOXIANG_VERSION` | install 脚本安装指定版本而非最新版 |

## 网络与镜像（受限网络）

当前网络无法直连 GitHub 时，`yx` 支持下载镜像：在 `<安装根>/settings.toml`（缺省 `~/.yaoxiang/settings.toml`）配置 ghproxy 风格的 URL 前缀：

```toml
mirror = "https://ghproxy.example.com"
```

生效范围（`yx` 会把完整 GitHub URL 拼接到该前缀之后）：

- `yx toolchain install` / `yx toolchain update`——发行包、`.sha256` 旁证与版本查询 API
- `yx self update`——同上

注意：

- 一键安装脚本（`install.sh` / `install.ps1`）不读取镜像配置，直连 GitHub
- 下载失败时 `yx` 会在报错中提示配置镜像；镜像本身不可达同样按网络错误报告

## 验证安装

```sh
yx --version        # 前门版本
yx run main.yx      # 任意 YaoXiang 程序
```
