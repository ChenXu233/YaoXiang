---
title: '命令行接口'
description: '包管理器所有命令详细说明'
---

# 命令行接口

## yx init

初始化一个新的 YaoXiang 项目。

### 用法

```bash
yx init <项目名称>
```

### 参数

| 参数     | 说明         |
| -------- | ------------ |
| 项目名称 | 新项目的名称 |

### 选项

| 选项     | 说明         |
| -------- | ------------ |
| `--help` | 显示帮助信息 |

### 示例

```bash
# 创建新项目
yx init my-project

# 结果：
# ✨ 项目已创建：my-project
#   my-project/yaoxiang.toml
#   my-project/yaoxiang.lock
#   my-project/src/main.yx
#   my-project/.gitignore
```

---

## yx add

添加依赖到项目。

### 用法

```bash
yx add <包名> [版本]
yx add <包名> --dev
```

### 参数

| 参数 | 说明                       |
| ---- | -------------------------- |
| 包名 | 要添加的依赖名称           |
| 版本 | 版本号（可选，默认为 `*`） |

### 选项

| 选项          | 说明           |
| ------------- | -------------- |
| `--dev`, `-D` | 添加为开发依赖 |

### 示例

```bash
# 添加最新版本
yx add http

# 添加指定版本
yx add http 1.0.0

# 添加版本范围
yx add json ">=2.0.0"

# 添加开发依赖
yx add test-utils --dev
yx add benchmark -D
```

---

## yx rm

从项目中移除依赖。

### 用法

```bash
yx rm <包名>
yx rm <包名> --dev
```

### 参数

| 参数 | 说明             |
| ---- | ---------------- |
| 包名 | 要移除的依赖名称 |

### 选项

| 选项          | 说明         |
| ------------- | ------------ |
| `--dev`, `-D` | 移除开发依赖 |

### 示例

```bash
# 移除普通依赖
yx rm http

# 移除开发依赖
yx rm test-utils --dev
```

---

## yx install

安装项目依赖。

### 用法

```bash
yx install
```

### 说明

- 读取 `yaoxiang.toml` 中的依赖声明
- 下载所有依赖到 `vendor` 目录
- 生成/更新 `yaoxiang.lock` 锁定版本
- 检测依赖版本冲突

### 示例

```bash
# 安装所有依赖
yx install

# 输出示例：
# 📦 正在解析依赖...
#   http (1.0.0) [已安装]
#   json (2.0.0) [已缓存]
# ✅ 依赖已安装，锁文件已更新
```

---

## yx update

更新项目依赖。

### 用法

```bash
yx update
yx update <包名>
```

### 参数

| 参数 | 说明                     |
| ---- | ------------------------ |
| 包名 | 要更新的特定依赖（可选） |

### 说明

- 不带参数：更新所有依赖
- 带参数：仅更新指定依赖

### 示例

```bash
# 更新所有依赖
yx update

# 输出示例：
# 📦 正在更新依赖...
1.0.#   http (0 → 1.1.0)
# ✅ 已更新 1 个依赖，锁文件已更新

# 更新单个依赖
yx update http
```

---

## yx list

列出项目的所有依赖。

### 用法

```bash
yx list
```

### 说明

显示所有运行时依赖和开发依赖，以及它们的版本和来源。

### 示例

```bash
# 列出依赖
yx list

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
