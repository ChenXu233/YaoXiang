---
title: '包管理器'
description: 'YaoXiang 包管理器参考文档'
---

# 包管理器

YaoXiang 内置包管理器，提供项目初始化、依赖管理、版本锁定与工作空间支持。

面向使用者的完整操作流程（含可复制粘贴的建仓步骤）见[包管理系统指南](../../guide/packaging)。本页是参考索引。

## 设计原则

- **声明式依赖**：在 `yaoxiang.toml` 中声明所需依赖
- **确定性构建**：通过 `yaoxiang.lock` 锁定版本，确保可重现构建
- **本地缓存**：依赖下载到 `.yaoxiang/vendor/`，支持离线复用

## 依赖来源现状

| 来源                 | 清单写法                       | 状态                                  |
| -------------------- | ------------------------------ | ------------------------------------- |
| 工作空间成员         | `{ workspace = "key" }`        | ✅ 可用                               |
| Git 仓库             | `{ git = "<url>" }`             | ✅ 可用                               |
| 官方 registry        | `name = "1.0.0"`                | ⛔ 尚未落地（RFC-014a 无限期后置）    |
| 本地路径             | `{ path = "../lib" }`           | ⚠️ 已知问题，见指南                   |

`yx add` 不带 `--git` / `--path` 时会落到尚未落地的 registry 源，`yx install` 随后失败。

## 项目结构

`yx init` 的实际产物：

```
my-project/
├── yaoxiang.toml            # 项目清单
├── yaoxiang.lock            # 依赖锁定文件
├── .gitignore               # 已包含 .yaoxiang/
├── tests/                   # 测试目录
├── src/
│   └── main.yx              # 入口文件（--lib 时为 lib.yx）
└── .yaoxiang/
    └── vendor/
        ├── std/              # 标准库接口文件
        └── <包名>-<版本>/    # 第三方依赖
```

## 命令总览

`yx` 提供 21 个子命令，其中与包管理相关的有 11 个：

| 命令                              | 说明                         |
| --------------------------------- | ---------------------------- |
| [`yx init`](./commands#yx-init)   | 初始化项目，支持 `--lib`     |
| [`yx add`](./commands#yx-add)     | 添加依赖                     |
| [`yx install`](./commands#yx-install) | 安装依赖                |
| [`yx update`](./commands#yx-update) | 更新依赖                   |
| [`yx list`](./commands#yx-list)   | 列出依赖及来源               |
| [`yx rm`](./commands#yx-rm)       | 移除依赖                     |
| [`yx outdated`](./commands#yx-outdated) | 检查可升级依赖         |
| [`yx clean`](./commands#yx-clean) | 清理构建产物与冗余 vendor 包 |
| [`yx cache clean`](./commands#yx-cache-clean) | 清空全局缓存       |
| [`yx workspace`](./commands#yx-workspace) | 工作空间成员管理    |
| [`yx publish`](./commands#yx-publish) | 打包与发布             |

其余子命令（`run`、`check`、`test`、`build`、`format`、`lsp`、`repl`、`eval`、`explain`、`dump`）与包管理无关。

## 文档索引

- [命令行接口](./commands) - 所有包管理命令的详细说明
- [yaoxiang.toml 格式](./manifest) - 项目配置文件格式
- [yaoxiang.lock 格式](./lock) - 锁文件格式说明
- [错误码](./error-codes) - 包管理相关错误及处理方式
- [包管理系统指南](../../guide/packaging) - 面向使用者的完整流程
