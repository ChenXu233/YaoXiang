# yaoxiang check

YaoXiang ソースコードに対して静的チェック（型チェック、所有権チェック）を実行し、コードは生成しません。

## 使い方

```
yaoxiang check [OPTIONS] [PATH]...
```

## パラメータ

| パラメータ | 説明                                                                                                |
| ---------- | --------------------------------------------------------------------------------------------------- |
| `PATH`     | 1つまたは複数のファイルまたはディレクトリパス。指定しない場合は現在のプロジェクトをチェックします。 |

## オプション

| オプション         | 説明                                        | デフォルト値 |
| ------------------ | ------------------------------------------- | ------------ |
| `--json`           | JSON 形式で診断情報を出力                   | いいえ       |
| `-w`, `--watch`    | ファイルの変更を監視し自動的に再チェック    | いいえ       |
| `--color <MODE>`   | カラー出力モード：`auto`、`always`、`never` | `auto`       |
| `--exclude <PATH>` | 指定したパスを除外（複数指定可能）          | なし         |
| `--no-progress`    | 進捗とサマリメッセージを抑制                | いいえ       |

## 終了コード

| 終了コード | 説明                           |
| ---------- | ------------------------------ |
| `0`        | エラーなし                     |
| `1`        | チェックでエラー発見           |
| `2`        | `.yx` ファイルが見つかりません |

## ファイル間分析

`yaoxiang check` はファイル間型チェックをサポートしています。複数のファイルをチェックする場合：

1. すべての `.yx` ファイルを並行解析
2. モジュール依存グラフを構築
3. 循環依存を検出（エラー報告）
4. トポロジカルソート順にチェック
5. 共有型環境を使用して、ファイル間参照を正しく検出

```bash
# プロジェクト全体をチェック（ファイル間参照を自動検出）
yaoxiang check src/

# 指定ファイルをチェック
yaoxiang check src/main.yx src/lib.yx
```

## 增量チェック（watch モード）

`-w` または `--watch`
を使用してファイル監視モードを有効にします。ファイル変更時に自動的に再チェックを行います。

```bash
yaoxiang check --watch
```

## JSON 出力形式

`--json` を使用した場合、出力形式は次のとおりです：

```json
{
  "error_count": 0,
  "warning_count": 0,
  "diagnostics": [
    {
      "file": "src/main.yx",
      "severity": "error",
      "code": "E1001",
      "message": "Unknown variable: 'x'",
      "line": 5,
      "column": 3,
      "end_line": 5,
      "end_column": 4,
      "lsp": { ... }
    }
  ]
}
```

## 例

```bash
# 現在のプロジェクトをチェック
yaoxiang check

# 指定ファイルをチェック
yaoxiang check src/main.yx

# ディレクトリをチェックしてJSON出力
yaoxiang check src/ --json

# 監視モード
yaoxiang check --watch

# CIモード（カラーなし、進捗なし）
yaoxiang check --color never --no-progress

# テストディレクトリを除外
yaoxiang check src/ --exclude tests/
```

## CI との統合

```yaml
# GitHub Actions
- name: Type check
  run: yaoxiang check --color never --no-progress
```

詳細な CI 設定については、[CI 統合ガイド](../guide/ci-integration.md)を参照してください。

## 関連項目

- [`yaoxiang format`](./format-command.md) -- コードフォーマット
- [エラーコードリファレンス](./error-codes.md) -- 完全なエラーコードリスト
- [CI 統合ガイド](../guide/ci-integration.md) -- CI/CD 統合
- [診断システム設計](../design/check/diagnostic-system.md) -- アーキテクチャ設計ドキュメント
