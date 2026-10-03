---
title: 'yaoxiang.toml 格式'
description: '项目配置文件格式说明'
---

# yaoxiang.toml 格式

`yaoxiang.toml` 是 YaoXiang 项目的清单文件，声明项目元数据、依赖与入口。

## 文件结构

```toml
[package]
name = "项目名称"
version = "0.1.0"
description = "项目描述"
authors = ["作者名"]
license = "MIT"
repository = "https://github.com/org/repo"

[dependencies]
# 运行时依赖

[dev-dependencies]
# 开发依赖
```

## [package]

| 字段          | 类型   | 必需 | 说明                                        |
| ------------- | ------ | ---- | ------------------------------------------- |
| `name`        | string | 是   | 项目名称                                    |
| `version`     | string | 是   | 语义化版本号                                |
| `description` | string | 否   | 项目描述；**`publish` 时必填**             |
| `authors`     | array  | 否   | 作者列表                                    |
| `license`     | string | 否   | 许可证标识符                                |
| `repository`  | string | 否   | 仓库地址；`publish --github` 的目标解析来源之一 |

## 依赖声明

### 可用来源

| 来源           | 写法                             | 状态                                  |
| -------------- | -------------------------------- | ------------------------------------- |
| 工作空间成员   | `{ workspace = "key" }`          | ✅ 可用                               |
| Git 仓库       | `{ git = "https://..." }`        | ✅ 可用                               |
| 官方 registry  | `some-lib = "1.0.0"`             | ⛔ 尚未落地（RFC-014a 无限期后置）    |
| 本地路径       | `{ path = "../lib" }`            | ⚠️ 已知问题：装上后无法 `use`         |

### 示例

```toml
[dependencies]
# Git 依赖（跨仓库分发的当前唯一渠道）
some-lib = { git = "https://github.com/example/some-lib", version = "^0.1.0" }

# 工作空间成员
utils = { workspace = "mylib" }

# 开发依赖
test-utils = { git = "https://github.com/example/test-utils" }
```

### 依赖字段

| 字段      | 类型   | 说明                                     |
| --------- | ------ | ---------------------------------------- |
| `version` | string | 版本号或版本范围                         |
| `git`     | string | Git 仓库地址                             |
| `path`    | string | 本地相对路径（见上方已知问题）           |
| `workspace` | string \| bool | 引用工作空间成员或继承根声明      |

::: warning `branch` 字段当前无效
依赖字段里**没有** `branch`。写 `branch = "main"` 不会报错，但也不会生效——依赖仍按默认分支解析。

需要钉住分支、tag 或提交时，把 ref 写进 **git URL 的查询串**：

```toml
[dependencies]
pinned-dev = { git = "https://github.com/example/lib?branch=dev" }
at-tag    = { git = "https://github.com/example/lib?tag=v1.0.0" }
at-rev    = { git = "https://github.com/example/lib?rev=abc1234" }
```

被这样钉住的依赖，`yx outdated` 会报 `pinned`，不参与版本比较。
:::

## 版本号语法

| 语法                | 说明                       | 示例                    |
| ------------------- | -------------------------- | ----------------------- |
| `1.0.0`             | **精确匹配**（不是 caret） | `"1.0.0"`               |
| `*`                 | 任意版本                   | `"*"`                   |
| `^1.0.0`            | caret，允许次版本内升级    | `"^1.0.0"`              |
| `~1.0.0`            | 允许补丁版本升级           | `"~1.0.0"`              |
| `>=1.0.0`、`<2.0.0` | 上下界                     | `">=1.0.0"`             |
| `>=1.0.0, <2.0.0`   | 逗号组合                   | `">=1.0.0, <2.0.0"`     |
| `1.*`               | 段级通配                   | `"1.*"`                 |

`1.2`、`1` 会自动补齐为 `1.2.0`、`1.0.0`。

## [workspace]

工作空间**根目录**的 `yaoxiang.toml` 用 `[workspace]` 段，不含 `[package]`：

```toml
[workspace.members]
app = "app/yaoxiang.toml"
mylib = "mylib/yaoxiang.toml"

[workspace.dependencies]
# 在根统一钉版本，成员以 { workspace = true } 继承
```

`members` 的值必须是**清单文件路径**，不是目录。成员引用兄弟成员写 `{ workspace = "<key>" }`，key 即登记时的名字。

::: warning 根目录与成员目录的差异
工作空间根没有 `[package]` 段，因此 `list`、`outdated`、`update` 在根目录会报 `missing field package`，需进成员目录执行。`install`、`workspace`、`publish`、`cache`、`clean` 在根目录可用。
:::

## 其他段

| 段           | 说明                                                     |
| ------------ | -------------------------------------------------------- |
| `[lib]`      | 库入口文件（相对清单目录）                                |
| `[[bin]]`    | 二进制目标条目                                            |
| `[run]`      | 默认入口与参数                                            |
| `[exports]`  | use 路径前缀 → 文件路径的映射（跨包导出面，RFC-015/029f）  |
| `[build]`    | 构建声明：策略、C 头文件、工具版本要求（RFC-014b）        |
| `[binaries]` | 预编译产物声明，平台三元组 → 产物                         |
| `[i18n]`     | 项目级语言配置                                            |

## 完整示例

```toml
[package]
name = "web-server"
version = "0.1.0"
description = "一个简单的 Web 服务器"
authors = ["开发者 <dev@example.com>"]
license = "MIT"
repository = "https://github.com/org/web-server"

[dependencies]
some-lib = { git = "https://github.com/example/some-lib", version = "^0.1.0" }
mylib = { workspace = "mylib" }

[dev-dependencies]
test-utils = { git = "https://github.com/example/test-utils" }
```
