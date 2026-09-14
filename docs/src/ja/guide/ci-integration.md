---
title: 'CI 統合ガイド'
description: 'yx check と yx format を CI/CD パイプラインに統合する'
---

# CI 統合ガイド

YaoXiang の静的チェックとフォーマットツールを CI/CD パイプラインに統合し、コード品質を確保します。

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
          curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
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
    - curl -fsSL
      https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
    - export PATH="$HOME/.yaoxiang/bin:$PATH"
    - yx check --color never --no-progress
    - yx format --dry-run .
  rules:
    - if: $CI_MERGE_REQUEST_IID
    - if: $CI_COMMIT_BRANCH == "main"
    - if: $CI_COMMIT_BRANCH == "dev"
```

## 終了コード

| 終了コード | 意味                                                                          | CI の動作  |
| ---------- | ----------------------------------------------------------------------------- | ---------- |
| `0`        | エラーなし                                                                    | 合格       |
| `1`        | チェックでエラーが発見された、または `--deny-warnings` 指定時に警告が存在する | 失敗       |
| `2`        | `.yx` ファイルが見つからない                                                  | 設定による |

## JSON 出力のパース

機械可読の出力には `--json` を使用します：

```bash
yx check --json | jq '.error_count'
```

## ベストプラクティス

1. **パスパラメータ**：`yx check`
   はデフォルトで現在のディレクトリをチェックしますが、パスを指定することもできます：`yx check src/`
2. **チェックとフォーマットの分離**：`check` と `format --dry-run`
   をそれぞれ実行し、問題の特定を容易にします
3. **`--no-progress` を使用**：CI 環境ではプログレスバーは不要です
4. **`--color never` を使用**：ANSI カラーコードによるログの汚染を防止します
5. **厳格モード**：`--deny-warnings` を追加して、警告も CI を失敗させるようにします
6. **依存関係のキャッシュ**：CI のキャッシュ機構を利用してビルドを高速化します
