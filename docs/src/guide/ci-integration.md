---
title: CI 集成指南
description: 将 yaoxiang check 和 yaoxiang format 集成到 CI/CD 流水线
---

# CI 集成指南

将 YaoXiang 的静态检查和格式化工具集成到 CI/CD 流水线中，确保代码质量。

## GitHub Actions

```yaml
name: YaoXiang CI

on:
  push:
    branches: [main, dev]
  pull_request:
    branches: [main]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install YaoXiang
        run: |
          curl -fsSL https://yaoxiang.dev/install.sh | sh
          echo "$HOME/.yaoxiang/bin" >> $GITHUB_PATH

      - name: Type check
        run: yaoxiang check --color never --no-progress

      - name: Format check
        run: yaoxiang format --dry-run .
```

## GitLab CI

```yaml
yaoxiang-check:
  image: rust:latest
  script:
    - curl -fsSL https://yaoxiang.dev/install.sh | sh
    - export PATH="$HOME/.yaoxiang/bin:$PATH"
    - yaoxiang check --color never --no-progress
    - yaoxiang format --dry-run .
  rules:
    - if: $CI_MERGE_REQUEST_IID
    - if: $CI_COMMIT_BRANCH == "main"
    - if: $CI_COMMIT_BRANCH == "dev"
```

## 退出码

| 退出码 | 含义 | CI 行为 |
|--------|------|---------|
| `0` | 无错误 | 通过 |
| `1` | 检查发现错误 | 失败 |
| `2` | 未找到 `.yx` 文件 | 视配置决定 |

## JSON 输出解析

使用 `--json` 获取机器可读的输出：

```bash
yaoxiang check --json | jq '.error_count'
```

## 最佳实践

1. **路径参数**：`yaoxiang check` 默认检查当前目录，也可以指定路径：`yaoxiang check src/`
2. **分离检查和格式化**：分别运行 `check` 和 `format --dry-run`，便于定位问题
3. **使用 `--no-progress`**：CI 环境不需要进度条
4. **使用 `--color never`**：避免 ANSI 颜色码污染日志
5. **缓存依赖**：利用 CI 缓存机制加速构建
