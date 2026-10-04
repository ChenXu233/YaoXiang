---
title: '命令行接口'
description: '包管理器所有命令详细说明'
---

# 命令行接口

本页按命令列出精确参数。操作流程与场景选择见[包管理系统指南](../../guide/packaging)。

## yx init

初始化一个新的 YaoXiang 项目。

### 用法

```bash
yx init [NAME] [--lib]
```

### 参数

| 参数   | 类型   | 说明                                         |
| ------ | ------ | -------------------------------------------- |
| `NAME` | string | 项目名；**省略时取当前目录名**               |

### 选项

| 选项   | 说明                       |
| ------ | -------------------------- |
| `--lib` | 生成库项目（`src/lib.yx`） |

### 产物

`yaoxiang.toml`、`yaoxiang.lock`、`.gitignore`、`tests/`、`src/main.yx`、`.yaoxiang/vendor/std/`。

若上层目录存在 `[workspace]`，`init` 会**自动把新包登记为工作空间成员**并输出：

```bash
✓ Registered to workspace (key 'mylib')
```

::: warning `--lib` 产物是空壳
`src/lib.yx` 仅有注释、不含任何代码，需要自行填写实现。
:::

---

## yx add

添加依赖到项目。

### 用法

```bash
yx add <DEP> [--git <GIT> | --path <PATH>] [-v <VERSION>] [-D]
```

### 参数

| 参数   | 类型   | 说明         |
| ------ | ------ | ------------ |
| `DEP`  | string | 依赖名       |

### 选项

| 选项              | 说明                                    |
| ----------------- | --------------------------------------- |
| `--git <URL>`     | Git 来源；**跨仓库依赖必须显式指定**    |
| `--path <PATH>`   | 本地路径（见[已知问题](../../guide/packaging#能力现状)） |
| `-v, --version`   | 版本规范，见下表                        |
| `-D, --dev`       | 写入 `dev-dependencies`                 |
| `--trust`         | 信任该包的 `build.yx`                   |

### 版本写法

版本是**选项**，不是位置参数。`yx add foo 1.0.0` 会报 `unexpected argument`。

| 写法                | 含义                        |
| ------------------- | --------------------------- |
| `1.0.0`             | 精确匹配（**不是** caret）  |
| `*`                 | 任意版本                    |
| `^1.0.0`            | caret                       |
| `~1.0.0`            | 允许补丁升级                |
| `>=1.0.0`、`<2.0.0` | 上下界                      |
| `1.*`               | 段级通配                    |
| `>=1.2.3, <2.0.0`   | 逗号组合                    |

`1.2`、`1` 自动补齐为 `1.2.0`、`1.0.0`。

### 示例

```bash
# Git 来源（推荐）
yx add some-lib --git https://github.com/example/some-lib
yx add some-lib --git https://github.com/example/some-lib --version "^0.1.0"

# 开发依赖
yx add test-utils --git https://github.com/example/test-utils --dev
```

::: danger 不要省略来源选项
不带 `--git` / `--path` 时依赖落到官方 registry 源，该源尚未落地，`yx install` 会失败。
:::

---

## yx rm

从项目中移除依赖。

### 用法

```bash
yx rm <DEP> [-D]
```

### 选项

| 选项          | 说明                   |
| ------------- | ---------------------- |
| `-D, --dev`   | 从 `dev-dependencies` 移除 |

---

## yx install

安装项目依赖。

### 用法

```bash
yx install [--trust]
```

### 行为

1. 解析 `yaoxiang.toml` 中的依赖
2. 下载到 `.yaoxiang/vendor/`
3. 生成或更新 `yaoxiang.lock`
4. 校验 vendor 与锁文件一致

### 示例

```bash
$ yx install
✓ Resolved 1 dependencies:
  some-lib (0.1.0) [Installed]

Updated yaoxiang.lock
```

工作空间根执行 `install` 时，会合并全部成员的依赖，统一落到根锁文件与根 vendor。

---

## yx update

更新依赖。

### 用法

```bash
yx update [PKG] [--trust]
```

| 参数  | 说明                        |
| ----- | --------------------------- |
| `PKG` | 指定包名；省略则更新全部    |

---

## yx list

列出项目依赖及来源。

### 用法

```bash
yx list
```

### 示例

```bash
$ yx list
app v0.1.0

[dependencies]
  mylib = "*"
```

::: warning 工作空间根不可用
工作空间根清单没有 `[package]` 段，在根目录执行会报 `missing field package`。请在成员目录内执行。
:::

---

## yx outdated

检查依赖是否有新版本。

### 用法

```bash
yx outdated
```

### 说明

- Git 来源的默认分支会检查
- path / workspace 来源跳过
- 固定到 tag / branch / rev 的依赖报告为 pinned

```bash
$ yx outdated
All dependencies are up to date.
```

---

## yx clean

清理构建产物与 vendor 中未被锁文件引用的包。

### 用法

```bash
yx clean
```

---

## yx cache clean

清空全局包缓存。

### 用法

```bash
yx cache clean
```

```bash
$ yx cache clean
Cache cleaned (freed 29.70 KB)
```

`cache` 目前只有 `clean` 一个子命令。

---

## yx workspace

工作空间成员管理。

### 用法

```bash
yx workspace list
yx workspace add <PATH> [--as <KEY>]
yx workspace remove <KEY>
```

### 说明

- `add` 的 key 默认取成员的 `[package].name`，`--as` 可显式指定
- `remove` 只取消登记，目录保留
- 成员引用兄弟成员写 `{ workspace = "<key>" }`

### `[workspace]` 段

```toml
[workspace.members]
app = "app/yaoxiang.toml"
mylib = "mylib/yaoxiang.toml"

[workspace.dependencies]
```

::: warning members 的值是清单文件路径
写成目录（如 `"app"`）会得到误导性的 `nested workspace is not supported` 报错。
:::

---

## yx publish

打包并发布。当前可用渠道是 GitHub Release；官方 registry 尚未落地。

### 用法

```bash
yx publish --dry-run          # 本地校验 + 打包，不发布
yx publish --github           # 发布为 GitHub Release
yx publish --github --no-test # 跳过发布前测试
```

### 选项

| 选项        | 说明                                    |
| ----------- | --------------------------------------- |
| `--dry-run` | 本地校验并生成 `.yxpkg`，**零网络请求** |
| `--github`  | 发布为 GitHub Release                   |
| `--no-test` | 跳过发布前测试                          |

### 前置条件

`[package].description` 必填，否则报 `发布要求 [package] 提供 description`。

### --dry-run

流程：跑 `[tool.test]` → workspace 引用物化为 `^version` → 打包 → 计算 SHA-256。

```bash
$ yx publish --dry-run
Running pre-publish tests…
No tests found.
✓ Packed mylib/target/yxpkg/mylib-0.1.0.yxpkg (SHA-256 47a2c83…)
```

### --github

需要环境变量 `YX_GITHUB_TOKEN`。目标仓库由 `[package].repository` 优先、回退 `git remote origin`。

**publish 不会代打 tag**，须先自行推送：

```bash
git tag v0.1.0
git push origin v0.1.0
export YX_GITHUB_TOKEN=<token>
yx publish --github
```

同名 Release 已存在时会被拒绝。

### 暂不可用

`yx publish`（不带任何选项）、`yx login` / `logout`、`yank`、`--registry <url>` 均属尚未落地的官方 registry。
