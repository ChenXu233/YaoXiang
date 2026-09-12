# yx check

YaoXiang のソースコードに対して静的なチェック（型チェック、所有権チェック）を行い、コードを生成しません。

## 使い方

```
yx check [OPTIONS] [PATH]...
```

## 引数

| 引数   | 説明                                                                                             |
| ------ | ------------------------------------------------------------------------------------------------ |
| `PATH` | 一つ以上のファイルまたはディレクトリのパス。指定しない場合は現在のプロジェクトをチェックします。 |

## オプション

| オプション         | 説明                                             | デフォルト |
| ------------------ | ------------------------------------------------ | ---------- |
| `--json`           | 診断情報を JSON 形式で出力します                 | いいえ     |
| `-w`, `--watch`    | ファイルの変更を監視し、自動的に再チェックします | いいえ     |
| `--color <MODE>`   | カラー出力モード：`auto`、`always`、`never`      | `auto`     |
| `--exclude <PATH>` | 指定したパスを除外します（複数回使用可能）       | なし       |
| `--no-progress`    | 進捗とサマリーメッセージを抑制します             | いいえ     |

## 終了コード

| 終了コード | 説明                             |
| ---------- | -------------------------------- |
| `0`        | エラーなし                       |
| `1`        | チェックでエラーが発見されました |
| `2`        | `.yx` ファイルが見つかりません   |

## ファイル間分析

`yx check` はファイル間の型チェックをサポートします。複数のファイルをチェックする場合：

1. すべての `.yx` ファイルを並列に解析します
2. モジュール依存グラフを構築します
3. 循環依存を検出します（エラーとして報告）
4. トポロジカルソート順でチェックします
5. 共有型環境を使用して、ファイル間の参照を正しく検出します

```bash
# プロジェクト全体をチェック（ファイル間参照を自動検出）
yx check src/

# 指定したファイルをチェック
yx check src/main.yx src/lib.yx
```

## 増分チェック（watch モード）

`-w` または `--watch`
を使用してファイル監視モードを有効にします。ファイルが変更されると自動的に再チェックされます。

```bash
yx check --watch
```

## JSON 出力形式

`--json` を使用する場合、出力形式は次のとおりです：

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
yx check

# 指定したファイルをチェック
yx check src/main.yx

# ディレクトリをチェックして JSON 出力
yx check src/ --json

# 監視モード
yx check --watch

# CI モード（色なし、進捗なし）
yx check --color never --no-progress

# テストディレクトリを除外
yx check src/ --exclude tests/
```

## CI との統合

```yaml
# GitHub Actions
- name: Type check
  run: yx check --color never --no-progress
```

詳細な CI 設定については [CI 統合ガイド](../guide/ci-integration.md) を参照してください。

## 関連項目

- [`yx format`](./format-command.md) -- コードフォーマット
- [エラーコードリファレンス](./error-codes.md) -- 完全なエラーコードリスト
- [CI 統合ガイド](../guide/ci-integration.md) -- CI/CD 統合
- [診断システム設計](../design/check/diagnostic-system.md) -- アーキテクチャ設計ドキュメント
