---
title: CI 統合ガイド
description: yaoxiang check と yaoxiang format を CI/CD パイプラインに統合する
---

# CI 統合ガイド

YaoXiang の静的チェックおよびフォーマットツールを CI/CD パイプラインに統合し、コード品質を保証します。

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

| 終了コード | 意味 | CI 動作 |
|--------|------|---------|
| `0` | エラーなし | 成功 |
| `1` | チェックでエラー検出 | 失敗 |
| `2` | `.yx` ファイルが見つからない | 設定による |

## JSON 出力の解析

`--json` オプションで機械可読な出力を取得できます:

```bash
yaoxiang check --json | jq '.error_count'
```

## ベストプラクティス

1. **パス引数**: `yaoxiang check` はデフォルトでカレントディレクトリをチェックしますが、`yaoxiang check src/` のようにパスを指定することもできます
2. **チェックとフォーマットを分離**: `check` と `format --dry-run` を別々に実行することで、問題の特定が容易になります
3. **`--no-progress` を使用**: CI 環境ではプログレスバーは不要です
4. **`--color never` を使用**: ANSI カラーコードがログを汚染するのを防ぎます
5. **依存関係をキャッシュ**: CI のキャッシュ機能を活用してビルドを高速化します