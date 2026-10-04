---
title: '包管理系统'
description: 'YaoXiang 包管理器使用指南：依赖管理、工作空间与发布'
---

# 包管理系统

YaoXiang 内置包管理器，提供声明式依赖管理、版本锁定与工作空间支持。

## 能力现状

依赖来源的可用性并不一致，**动手前先看这张表**：

| 来源                       | 声明方式                   | 状态                                     |
| -------------------------- | -------------------------- | ---------------------------------------- |
| **工作空间成员**           | `{ workspace = "key" }`   | ✅ 可用，本地多包的首选                  |
| **Git 仓库**               | `{ git = "<url>" }`        | ✅ 可用，跨仓库分发的当前唯一渠道        |
| 官方 registry              | `name = "1.0.0"`           | ⛔ **尚未落地**，见下方说明               |
| 本地路径                   | `{ path = "../lib" }`      | ⚠️ 已知问题，装上后无法 `use`，见下方说明 |

::: danger 官方 registry 尚未开放
RFC-014a 已决议：官方 registry 服务器、认证与 yank **无限期后置**。因此：

- `yx publish` 不带参数会直接失败
- `yx add <裸包名>` 虽然会写入清单，但随后的 `yx install` 必然失败：

```bash
$ yx add http
✓ Added dependency 'http' (*)

$ yx install
⚠ 1 dependencies failed to install:
  http - registry source not implemented (RFC-014a); use a git or path dependency
Error: Failed to install dependencies
```

**依赖外部包请一律使用 `--git`。** 发布请使用 `yx publish --github`，详见[发布](#发布)。
:::

::: warning 本地路径依赖目前不可用
`yx add --path` 能把依赖写进 `yaoxiang.toml` 与 `yaoxiang.lock`，但不会把包放进 vendor 目录，随后 `use <包名>` 会报 E5001。

**本地多包请改用[工作空间](#工作空间)**——它是为此场景设计的，功能完整。该问题已单独记录。
:::

## 从零开始：本地多包

下面这套流程是**实测可完整跑通**的，可直接复制。

### 1. 建立工作空间根

工作空间根目录需要一个 `yaoxiang.toml`：

```bash
mkdir my-project && cd my-project
```

```toml
# my-project/yaoxiang.toml
[workspace]
members = { }
```

空 `members` 是允许的——下面的 `init` 会自动往里登记。

### 2. 在根目录下创建成员

```bash
yx init mylib --lib
yx init app
```

`init` 检测到上层存在工作空间，会**自动登记**成员，无需手工编辑根清单：

```bash
✓ Registered to workspace (key 'mylib')
✓ Created library project 'mylib'
```

登记后根清单变成：

```toml
# my-project/yaoxiang.toml
[workspace.members]
app = "app/yaoxiang.toml"
mylib = "mylib/yaoxiang.toml"

[workspace.dependencies]
```

::: warning members 的值是清单文件路径
`members` 的值必须写到 `yaoxiang.toml` **文件**，不是目录。写成 `"app"` 时会得到一个误导性的 `nested workspace is not supported` 报错。
:::

确认登记结果：

```bash
$ yx workspace list
Workspace my-project (2 members)
  app 0.1.0    (app/yaoxiang.toml)
  mylib 0.1.0  (mylib/yaoxiang.toml)
```

### 3. 写库实现

`yx init --lib` 生成的 `src/lib.yx` **只有注释、没有任何代码**，需要自己填：

```yaoxiang
// mylib/src/lib.yx
greet: () -> string = "hello from mylib"
```

所有顶层绑定默认对外可见，无需 `pub`——见[模块系统](./modules#导出不需要-pub)。

### 4. 应用声明依赖

成员引用兄弟成员用 `{ workspace = "<key>" }`，key 就是登记时的名字：

```toml
# my-project/app/yaoxiang.toml
[package]
name = "app"
version = "0.1.0"
description = "demo app"

[dependencies]
mylib = { workspace = "mylib" }
```

### 5. 安装并运行

```bash
# 在工作空间根执行
yx install

$ yx install
✓ Resolved 0 dependencies:
  mylib (workspace:mylib)

Updated yaoxiang.lock
```

<!-- docs-example: skip -->

```yaoxiang
// my-project/app/src/main.yx
use std.io
use mylib

main = () => {
    io.print(mylib.greet())
}
```

```bash
$ yx run app/src/main.yx
hello from mylib
```

> `run` 需要在项目目录内执行。在项目外对单个 `.yx` 文件跑 `run` 会静默无输出。

## 从零开始：依赖外部仓库

跨仓库或任意 Git 源用 `--git`：

```bash
yx init app
cd app

# 添加依赖
yx add some-lib --git https://github.com/example/some-lib

# 安装
yx install

$ yx install
✓ Resolved 1 dependencies:
  some-lib (0.1.0) [Installed]
```

安装后包落在 `.yaoxiang/vendor/<包名>-<版本>/`，即可 `use`：

<!-- docs-example: skip -->

```yaoxiang
use some_lib
```

### 指定版本

版本用 `-v` / `--version` 选项，**不是位置参数**：

```bash
yx add some-lib --git https://github.com/example/some-lib --version "^0.1.0"
```

写作 `yx add some-lib 0.1.0` 会报 `unexpected argument`。

### 钉住分支或 tag

ref 写在 **git URL 的查询串**里，不是在清单里加 `branch` 字段：

```bash
yx add some-lib --git "https://github.com/example/some-lib?branch=dev"
yx add some-lib --git "https://github.com/example/some-lib?tag=v1.0.0"
yx add some-lib --git "https://github.com/example/some-lib?rev=abc1234"
```

这样钉住的依赖，`yx outdated` 会报 `pinned`，不参与版本比较。

::: warning 清单里的 `branch` 字段无效
写 `some-lib = { git = "...", branch = "dev" }` 不报错也不生效——依赖仍按默认分支解析。ref 必须走 URL 查询串。
:::

支持的版本规范：

| 写法              | 含义                       |
| ----------------- | -------------------------- |
| `1.0.0`           | 精确匹配（**不是** caret） |
| `*`               | 任意版本                   |
| `^1.0.0`          | caret，允许次版本内升级    |
| `~1.0.0`          | 允许补丁版本升级           |
| `>=1.0.0`、`<2.0.0` | 上下界                   |
| `1.*`             | 段级通配                   |
| `>=1.2.3, <2.0.0` | 逗号组合                   |

`1.2`、`1` 会自动补齐为 `1.2.0`、`1.0.0`。

## 清单文件

### yaoxiang.toml

```toml
[package]
name = "my-project"          # 必填
version = "0.1.0"           # 必填
description = "项目描述"     # publish 时必填
authors = ["作者 <a@b.c>"]
license = "MIT"
repository = "https://github.com/org/repo"   # --github 发布需要

[dependencies]
some-lib = { git = "https://github.com/example/some-lib", version = "^0.1.0" }

[dev-dependencies]
test-utils = { git = "https://github.com/example/test-utils" }
```

工作空间根则用 `[workspace]` 段：

```toml
[workspace.members]
app = "app/yaoxiang.toml"
mylib = "mylib/yaoxiang.toml"

[workspace.dependencies]
# 根在这里统一钉版本，成员以 { workspace = true } 继承
```

### yaoxiang.lock

由包管理器自动生成，**请提交到版本控制**：

```toml
# 由 YaoXiang 包管理器自动生成

version = 1

[package.mylib]
version = "0.1.0"
source = "workspace"
```

### vendor 目录

依赖落在项目的 `.yaoxiang/vendor/` 下：

```
.yaoxiang/
└── vendor/
    ├── std/                  # 标准库接口文件
    └── some-lib-0.1.0/       # 第三方依赖，目录名 = <包名>-<版本>
```

`init` 生成的 `.gitignore` 已包含 `.yaoxiang/`，无需再手工添加。删掉整个 `.yaoxiang/` 后重跑 `yx install` 即可重建。

## 命令参考

### yx init

```bash
yx init [NAME] [--lib]
```

`NAME` 省略时用当前目录名。`--lib` 生成库项目（`src/lib.yx`，**内容为空壳**），否则生成 `src/main.yx`。

产物：`yaoxiang.toml`、`yaoxiang.lock`、`.gitignore`、`tests/`、`.yaoxiang/vendor/std/`。

### yx add

```bash
yx add <DEP> [--git <GIT> | --path <PATH>] [-v <VERSION>] [-D]
```

| 选项          | 说明                                       |
| ------------- | ------------------------------------------ |
| `--git <URL>` | Git 来源，**跨仓库依赖必须显式指定**       |
| `--path <P>`  | 本地路径（见上方已知问题）                  |
| `-v, --version` | 版本规范                                 |
| `-D, --dev`   | 记入 `dev-dependencies`                    |
| `--trust`     | 信任该包的 `build.yx`                      |

不带 `--git` / `--path` 时会落到尚未落地的 registry 源。

### yx install / update / rm / list

```bash
yx install              # 按清单解析、下载、写锁文件
yx update [PKG]         # 不带参数更新全部；带包名只更新该包
yx rm <DEP> [-D]        # 移除依赖
yx list                 # 列出依赖及来源
```

### yx outdated

```bash
yx outdated
```

检查依赖是否有新版本。目前对 Git 来源的默认分支有效；path / workspace 来源会跳过，固定到 tag / branch / rev 的依赖会报告为 pinned。

### yx clean / yx cache clean

```bash
yx clean         # 清理构建产物与 vendor 中未被锁文件引用的包
yx cache clean   # 清空全局包缓存
```

### yx workspace

```bash
yx workspace list                  # 列出成员
yx workspace add <PATH> [--as KEY] # 登记已有包为成员
yx workspace remove <KEY>          # 取消登记（目录保留）
```

::: warning 根目录与成员目录的命令差异
工作空间**根**的清单没有 `[package]` 段，因此 `list`、`outdated`、`update` 在根目录会报 `missing field package`。这些命令请在**成员目录内**执行；`install`、`workspace`、`publish`、`cache`、`clean` 在根目录可用。
:::

## 发布

### 本地校验与打包

```bash
cd mylib
yx publish --dry-run
```

前置条件：`[package].description` 必填，否则报 `发布要求 [package] 提供 description`。

```bash
$ yx publish --dry-run
Running pre-publish tests…
No tests found.
✓ Packed mylib/target/yxpkg/mylib-0.1.0.yxpkg (SHA-256 47a2c83…)
```

流程：跑 `[tool.test]` 测试（`--no-test` 可跳过）→ 把 workspace 引用物化为 `^version` → 打包 `.yxpkg` → 计算 SHA-256。**全程零网络请求。**

### 发布到 GitHub Release

这是当前唯一可用的分发渠道：

```bash
# 1. 先打并推送 tag——publish 不会代打
git tag v0.1.0
git push origin v0.1.0

# 2. 配置仓库地址
#    在 yaoxiang.toml 写 [package].repository，或确保有 git remote origin

# 3. 提供令牌
export YX_GITHUB_TOKEN=<your-token>

# 4. 发布
yx publish --github
```

目标仓库由 `[package].repository` 优先、回退 `git remote origin`。若同名 Release 已存在会被拒绝；**tag 必须已存在**。

消费方通过 Git 源引入：

```bash
yx add mylib --git https://github.com/org/repo --version "^0.1.0"
```

### 暂不可用

以下能力属于尚未落地的官方 registry，**不要写进依赖声明**：`yx publish`（裸）、`yx login` / `logout`、`yank`、第三方 registry、`--registry <url>`。

## 排错

### `registry source not implemented`

依赖是裸包名。改用 `--git` 指定 Git 来源，见[依赖外部仓库](#从零开始依赖外部仓库)。

### E5001: module not found

1. 依赖没装——在工作空间根或成员目录跑 `yx install`
2. 用了本地路径依赖——改用工作空间
3. 是模块路径问题而非依赖问题——见[模块系统排错](./modules#排错)

### `missing field package`

在工作空间根目录执行了 `list` / `outdated` / `update`。进成员目录再执行。

### `nested workspace is not supported`

`[workspace.members]` 的值写成了目录。改成清单文件路径，如 `"app/yaoxiang.toml"`。
