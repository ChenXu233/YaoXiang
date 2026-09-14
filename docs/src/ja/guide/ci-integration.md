---
title: 'CI 統合ガイド'
description: 'yx check と yx format を CI/CD パイプラインに統合する'
---

# CI 統合ガイド

YaoXiang の静的検査とフォーマットツールを CI/CD パイプラインに統合し、コード品質を確保します。

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
        run: yx check --color never --no-progress

      - name: Format check
        run: yx format --dry-run .
```

## GitLab CI

```yaml
yaoxiang-check:
  image: rust:latest
  script:
    - curl -fsSL https://yaoxiang.dev/install.sh | sh
    - export PATH="$HOME/.yaoxiang/bin:$PATH"
    - yx check --color never --no-progress
    - yx format --dry-run .
  rules:
    - if: $CI_MERGE_REQUEST_IID
    - if: $CI_COMMIT_BRANCH == "main"
    - if: $CI_COMMIT_BRANCH == "dev"
```

## 終了コード

| 終了コード | 意味                         | CI の動作        |
| ---------- | ---------------------------- | ---------------- |
| `0`        | エラーなし                   | 成功             |
| `1`        | 検査でエラーを発見           | 失敗             |
| `2`        | `.yx` ファイルが見つからない | 設定に応じて判断 |

## JSON 出力の解析

`--json` を使用して機械可読の出力を取得します：

```bash
yx check --json | jq '.error_count'
```

## ベストプラクティス

1. **パスパラメータ**：`yx check`
   はデフォルトで現在のディレクトリを検査しますが、パスを指定することもできます：`yx check src/`
2. **検査とフォーマットを分離**：`check` と `format --dry-run`
   をそれぞれ実行することで、問題を特定しやすくなります
3. **`--no-progress` を使用**：CI 環境ではプログレスバーは不要です
4. **`--color never` を使用**：ANSI カラーコードがログを汚染するのを防ぎます
5. **依存関係をキャッシュ**：CI のキャッシュ機構を利用してビルドを高速化します
