---
title: "命令行接口"
description: 包管理器所有命令详细说明
---

# 命令行接口

## yaoxiang init

初始化一个新的 YaoXiang 项目。

### 用法

```bash
yaoxiang init <项目名称>
```

### 参数

| 参数 | 说明 |
|------|------|
| 项目名称 | 新项目的名称 |

### 选项

| 选项 | 说明 |
|------|------|
| `--help` | 显示帮助信息 |

### 示例

```bash
# 创建新项目
yaoxiang init my-project

# 结果：
# ✨ 项目已创建：my-project
#   my-project/yaoxiang.toml
#   my-project/yaoxiang.lock
#   my-project/src/main.yx
#   my-project/.gitignore
```

---

## yaoxiang add

添加依赖到项目。

### 用法

```bash
yaoxiang add <包名> [版本]
yaoxiang add <包名> --dev
```

### 参数

| 参数 | 说明 |
|------|------|
| 包名 | 要添加的依赖名称 |
| 版本 | 版本号（可选，默认为 `*`） |

### 选项

| 选项 | 说明 |
|------|------|
| `--dev`, `-D` | 添加为开发依赖 |

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

## yaoxiang rm

从项目中移除依赖。

### 用法

```bash
yaoxiang rm <包名>
yaoxiang rm <包名> --dev
```

### 参数

| 参数 | 说明 |
|------|------|
| 包名 | 要移除的依赖名称 |

### 选项

| 选项 | 说明 |
|------|------|
| `--dev`, `-D` | 移除开发依赖 |

### 示例

```bash
# 移除普通依赖
yaoxiang rm http

# 移除开发依赖
yaoxiang rm test-utils --dev
```

---

## yaoxiang install

安装项目依赖。

### 用法

```bash
yaoxiang install
```

### 说明

- 读取 `yaoxiang.toml` 中的依赖声明
- 下载所有依赖到 `vendor` 目录
- 生成/更新 `yaoxiang.lock` 锁定版本
- 检测依赖版本冲突

### 示例

```bash
# 安装所有依赖
yaoxiang install

# 输出示例：
# 📦 正在解析依赖...
#   http (1.0.0) [已安装]
#   json (2.0.0) [已缓存]
# ✅ 依赖已安装，锁文件已更新
```

---

## yaoxiang update

更新项目依赖。

### 用法

```bash
yaoxiang update
yaoxiang update <包名>
```

### 参数

| 参数 | 说明 |
|------|------|
| 包名 | 要更新的特定依赖（可选） |

### 说明

- 不带参数：更新所有依赖
- 带参数：仅更新指定依赖

### 示例

```bash
# 更新所有依赖
yaoxiang update

# 输出示例：
# 📦 正在更新依赖...
1.0.#   http (0 → 1.1.0)
# ✅ 已更新 1 个依赖，锁文件已更新

# 更新单个依赖
yaoxiang update http
```

---

## yaoxiang list

列出项目的所有依赖。

### 用法

```bash
yaoxiang list
```

### 说明

显示所有运行时依赖和开发依赖，以及它们的版本和来源。

### 示例

```bash
# 列出依赖
yaoxiang list

# 输出示例：
# 📦 项目依赖
#
# 运行时依赖:
#   http        1.0.0    registry
#   json        2.0.0    registry
#
# 开发依赖:
#   test-utils  0.5.0    registry
```
