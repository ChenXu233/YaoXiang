---
title: "包管理器"
description: YaoXiang 官方包管理器使用教程
---

# 包管理器

YaoXiang 内置的包管理器，提供完整的依赖管理功能。

## 概述

YaoXiang Package Manager (YPM) 采用声明式依赖管理：

- 在 `yaoxiang.toml` 中声明项目依赖
- `yaoxiang.lock` 锁定精确版本，确保构建可重现
- 依赖下载到 `vendor` 目录

## 快速开始

```bash
# 创建新项目
yaoxiang init my-project
cd my-project

# 添加依赖
yaoxiang add http
yaoxiang add json

# 安装依赖
yaoxiang install

# 运行项目
yaoxiang run src/main.yx
```

## 项目结构

```
my-project/
├── yaoxiang.toml      # 项目清单
├── yaoxiang.lock      # 依赖锁定文件
├── vendor/            # 依赖存储
└── src/
    └── main.yx
```

---

## init

初始化一个新项目。

### 用法

```bash
yaoxiang init <name>
```

### 参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `name` | string | 项目名称 |

### 描述

在当前目录或指定路径下创建一个新的 YaoXiang 项目。

### 创建的文件

- `yaoxiang.toml` - 项目清单
- `yaoxiang.lock` - 依赖锁定文件
- `src/main.yx` - 入口文件
- `.gitignore` - Git 忽略配置

### 示例

```bash
# 在当前目录创建项目
yaoxiang init my-project

# 输出
# ✨ 项目已创建：my-project
#   my-project/yaoxiang.toml
#   my-project/yaoxiang.lock
#   my-project/src/main.yx
#   my-project/.gitignore
```

---

## add

添加依赖到项目。

### 用法

```bash
yaoxiang add <name> [version]
yaoxiang add <name> --dev
```

### 参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `name` | string | 包名称 |
| `version` | string | 版本号（可选，默认 `*`） |

### 选项

| 选项 | 说明 |
|------|------|
| `--dev`, `-D` | 添加为开发依赖 |

### 描述

将依赖添加到项目的 `yaoxiang.toml` 文件，并更新 `yaoxiang.lock`。

### 版本规范

| 规范 | 说明 | 示例 |
|------|------|------|
| `*` | 任意版本 | `http = "*"` |
| `1.0.0` | 精确版本 | `http = "1.0.0"` |
| `>=1.0.0` | 最低版本 | `http = ">1.0.0"` |
| `~1.0.0` | 兼容版本 | `http = "~1.0.0"` |
| `^1.0.0` | caret 版本 | `http = "^1.0.0"` |

### 依赖来源

#### Registry（默认）

```bash
yaoxiang add http
yaoxiang add http 1.0.0
```

#### Git 仓库

```bash
# 会在 manifest 中生成如下配置
# http = { version = "1.0.0", git = "https://github.com/example/http" }
```

#### 本地路径

```bash
# 会在 manifest 中生成如下配置
# mylib = { version = "0.1.0", path = "./mylib" }
```

### 示例

```bash
# 添加最新版本
yaoxiang add http

# 添加指定版本
yaoxiang add http 1.0.0

# 添加版本范围
yaoxiang add json ">=2.0.0"

# 添加开发依赖
yaoxiang add test-utils --dev
yaoxiang add benchmark -D
```

---

## rm

从项目中移除依赖。

### 用法

```bash
yaoxiang rm <name>
yaoxiang rm <name> --dev
```

### 参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `name` | string | 包名称 |

### 选项

| 选项 | 说明 |
|------|------|
| `--dev`, `-D` | 移除开发依赖 |

### 描述

从项目的 `yaoxiang.toml` 移除指定的依赖，并更新 `yaoxiang.lock`。

### 示例

```bash
# 移除运行时依赖
yaoxiang rm http

# 移除开发依赖
yaoxiang rm test-utils --dev
```

---

## install

安装项目依赖。

### 用法

```bash
yaoxiang install
```

### 描述

读取 `yaoxiang.toml` 中的依赖声明，执行以下操作：

1. 解析依赖版本
2. 检测版本冲突
3. 下载依赖到 `vendor` 目录
4. 生成/更新 `yaoxiang.lock`

### 行为

- 如果没有任何依赖，显示提示信息并退出
- 如果 `vendor` 目录已存在，会检查并复用缓存
- 如果检测到版本冲突，显示错误信息并退出

### 示例

```bash
# 安装所有依赖
yaoxiang install

# 输出
# 📦 正在解析依赖...
#   http (1.0.0) [已安装]
#   json (2.0.0) [已缓存]
# ✅ 依赖已安装，锁文件已更新
```

### 锁文件更新

`install` 命令会更新 `yaoxiang.lock`：

```toml
# yaoxiang.lock
[package]
version = 1

[package.http]
version = "1.0.0"
source = "registry"

[package.json]
version = "2.0.0"
source = "registry"
```

---

## update

更新项目依赖。

### 用法

```bash
yaoxiang update
yaoxiang update <name>
```

### 参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `name` | string | 包名称（可选） |

### 描述

### 全量更新

不带参数时，更新所有依赖：

1. 清空当前锁定的版本
2. 清理 `vendor` 目录中的旧版本
3. 重新下载所有依赖
4. 更新 `yaoxiang.lock`

### 单一更新

带参数时，仅更新指定依赖：

1. 从 `vendor` 中删除旧版本
2. 重新下载新版本
3. 更新 `yaoxiang.lock` 中对应条目
4. 其他依赖不受影响

### 示例

```bash
# 更新所有依赖
yaoxiang update

# 输出
# 📦 正在更新依赖...
#   http (1.0.0 → 1.1.0)
#   json (2.0.0 → 2.1.0)
# ✅ 已更新 2 个依赖，锁文件已更新

# 更新单个依赖
yaoxiang update http

# 输出
# ✅ 已更新 http (1.0.0 → 1.1.0)
```

---

## list

列出项目依赖。

### 用法

```bash
yaoxiang list
```

### 描述

显示项目中所有依赖，包括：

- 运行时依赖（来自 `[dependencies]`）
- 开发依赖（来自 `[dev-dependencies]`）
- 每个依赖的版本和来源

### 示例

```bash
yaoxiang list

# 输出
# 📦 项目依赖
#
# 运行时依赖:
#   http        1.0.0    registry
#   json        2.0.0    registry
#
# 开发依赖:
#   test-utils  0.5.0    registry
```

---

## 配置文件

### yaoxiang.toml

项目清单文件，声明项目元数据和依赖。

```toml
[package]
name = "my-project"
version = "0.1.0"
description = "项目描述"
authors = ["作者 <email@example.com>"]
license = "MIT"

[dependencies]
http = "1.0.0"
json = "*"

[dev-dependencies]
test-utils = "0.5.0"
```

### yaoxiang.lock

依赖锁定文件，由包管理器自动生成。

```toml
# 由 YaoXiang 包管理器自动生成

[package]
version = 1

[package.http]
version = "1.0.0"
source = "registry"
```

---

## 核心概念

### 运行时依赖 vs 开发依赖

- **运行时依赖** (`[dependencies]`)：项目运行时必须有的包
- **开发依赖** (`[dev-dependencies]`)：仅开发、测试时需要的包

### 依赖来源

| 类型 | 配置示例 | 说明 |
|------|----------|------|
| Registry | `http = "1.0.0"` | 从远程包仓库获取 |
| Git | `{ version = "1.0.0", git = "https://..." }` | 从 Git 仓库获取 |
| Path | `{ version = "0.1.0", path = "./lib" }` | 从本地路径获取 |

### 锁文件

`yaoxiang.lock` 由包管理器自动生成，请**务必提交到版本控制**：

- 确保团队成员使用完全相同的依赖版本
- 确保 CI 构建可重现
- 避免"在我机器上能运行"的问题

### vendor 目录

依赖下载后存储在 `vendor` 目录：

- 由 `yaoxiang install` 和 `yaoxiang update` 自动管理
- 可以删除后重新运行 `install` 重建
- 建议加入 `.gitignore`，不同团队成员独立管理

---

## 常见问题

### Q: 依赖版本冲突怎么办？

YPM 会检测依赖版本冲突并报错。解决方案：

1. 调整依赖版本要求
2. 等待依赖作者修复
3. 考虑移除冲突的依赖

### Q: 如何使用私有包？

对于私有包，可以使用 Git 来源：

```bash
# 通过 Git URL 添加
# 手动编辑 yaoxiang.toml
[dependencies]
private-pkg = { version = "1.0.0", git = "https://github.com/org/private-pkg" }
```

### Q: vendor 目录可以删除吗？

可以。删除后运行 `yaoxiang install` 会重新下载所有依赖。

### Q: 如何查看某个包的信息？

使用 `yaoxiang list` 查看所有依赖，或查看 `yaoxiang.toml`。
