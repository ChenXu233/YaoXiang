```yaml
---
title: CI 統合ガイド
description: yaoxiang check と yaoxiang fmt を CI/CD パイプラインに統合する
---

# CI 統合ガイド

YaoXiang の静的チェックとフォーマットツールを CI/CD パイプラインに統合し、コード品質を確保する。

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

      - name: YaoXiang のインストール
        run: |
          curl -fsSL https://yaoxiang.dev/install.sh | sh
          echo "$HOME/.yaoxiang/bin" >> $GITHUB_PATH

      - name: 型チェック
        run: yaoxiang check --color never --no-progress

      - name: フォーマットチェック
        run: yaoxiang fmt --check
```

## GitLab CI

```yaml
yaoxiang-check:
  image: rust:latest
  script:
    - curl -fsSL https://yaoxiang.dev/install.sh | sh
    - export PATH="$HOME/.yaoxiang/bin:$PATH"
    - yaoxiang check --color never --no-progress
    - yaoxiang fmt --check
  rules:
    - if: $CI_MERGE_REQUEST_IID
    - if: $CI_COMMIT_BRANCH == "main"
    - if: $CI_COMMIT_BRANCH == "dev"
```

## 終了コード

| 終了コード | 意味 | CI 動作 |
|------------|------|---------|
| `0` | エラーなし | 成功 |
| `1` | チェックでエラーが見つかった | 失敗 |
| `2` | `.yx` ファイルが見つからない | 設定に依存 |

## JSON 出力の解析

`--json` を使用して機械可読な出力を取得する：

```bash
yaoxiang check --json | jq '.error_count'
```

## ベストプラクティス

1. **チェックとフォーマットの分離**：`check` と `fmt --check` を別々に実行し、問題の特定を容易にする
2. **`--no-progress` の使用**：CI 環境では進捗バーは不要
3. **`--color never` の使用**：ANSI カラーコードをログに混入させない
4. **依存関係のキャッシュ**：CI キャッシュ機能を活用してビルドを高速化する