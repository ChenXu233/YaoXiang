---
title: 包管理器
description: YaoXiang 包管理器参考文档
---

# 包管理器

YaoXiang 内置的包管理器，提供项目初始化、依赖管理、版本锁定等功能。

## 概述

YaoXiang 包管理器（简称 YPM）采用类似 Cargo 的设计理念：

- **声明式依赖**：在 `yaoxiang.toml` 中声明所需依赖
- **确定性构建**：通过 `yaoxiang.lock` 锁定版本，确保可重现构建
- **本地缓存**：依赖下载到 `vendor` 目录，支持离线使用

## 快速开始

```bash
# 1. 创建新项目
yaoxiang init my-project

# 2. 添加依赖
cd my-project
yaoxiang add http

# 3. 安装依赖
yaoxiang install

# 4. 运行项目
yaoxiang run src/main.yx
```

## 命令列表

| 命令 | 说明 |
|------|------|
| [`yaoxiang init`](./commands#yaoxiang-init) | 初始化新项目 |
| [`yaoxiang add`](./commands#yaoxiang-add) | 添加依赖 |
| [`yaoxiang rm`](./commands#yaoxiang-rm) | 移除依赖 |
| [`yaoxiang install`](./commands#yaoxiang-install) | 安装依赖 |
| [`yaoxiang update`](./commands#yaoxiang-update) | 更新依赖 |
| [`yaoxiang list`](./commands#yaoxiang-list) | 列出依赖 |

## 项目结构

```
my-project/
├── yaoxiang.toml      # 项目清单（必需）
├── yaoxiang.lock      # 依赖锁定文件（自动生成）
├── vendor/            # 依赖存储目录（自动生成）
└── src/
    └── main.yx       # 入口文件
```

## 文档索引

- [命令行接口](./commands) - 所有命令的详细说明
- [yaoxiang.toml 格式](./manifest) - 项目配置文件格式
- [yaoxiang.lock 格式](./lock) - 锁文件格式说明
- [错误码](./error-codes) - 常见错误及处理方式
