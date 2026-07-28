---
title: CI 統合ガイド
description: yaoxiang check と yaoxiang format を CI/CD パイプラインに統合する
---

# CI 統合ガイド

YaoXiang の静的チェックとフォーマットのツールを CI/CD パイプラインに統合して、コード品質を確保します。

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

## 終了コード

| 終了コード | 意味                         | CI の動作  |
| ---------- | ---------------------------- | ---------- |
| `0`        | エラーなし                   | 通過       |
| `1`        | チェックでエラーが検出       | 失敗       |
| `2`        | `.yx` ファイルが見つからない | 設定に依存 |

## JSON 出力の解析

`--json` を使用して機械可読な出力を取得します：

```bash
yaoxiang check --json | jq '.error_count'
```

## ベストプラクティス

1. **パス引数**：`yaoxiang check`
   はデフォルトでカレントディレクトリをチェックし、特定のパスを指定できます：`yaoxiang check src/`
2. **チェックとフォーマットの分離**：`check` と `format --dry-run`
   を別々に実行すると、問題の特定が容易になります
3. **`--no-progress` の使用**：CI 環境では進捗バーが不要です
4. **`--color never` の使用**：ANSI カラーコードをログに混入させてを防ぎます
5. **依存関係のキャッシュ**：CI のキャッシュ機能を活用してビルドを高速化します
