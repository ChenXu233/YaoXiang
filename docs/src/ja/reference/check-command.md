# yx check

YaoXiang のソースコードに対して静的検査（型検査、所有権検査）を行い、コードは生成しません。

## 使用法

```
yx check [OPTIONS] [PATH]...
```

## 引数

| 引数   | 説明                                                                                             |
| ------ | ------------------------------------------------------------------------------------------------ |
| `PATH` | 1 つ以上のファイルまたはディレクトリのパス。指定しない場合は現在のプロジェクトをチェックします。 |

## オプション

| オプション         | 説明                                                                     | デフォルト値 |
| ------------------ | ------------------------------------------------------------------------ | ------------ |
| `--json`           | 診断情報を JSON 形式で出力します                                         | いいえ       |
| `--color <MODE>`   | 色出力モード：`auto`、`always`、`never`                                  | `auto`       |
| `--exclude <PATH>` | 指定したパスを除外します（複数回使用可能）                               | なし         |
| `--no-progress`    | 進捗と要約メッセージを抑制します                                         | いいえ       |
| `--deny-warnings`  | 警告をエラーとして扱います：警告が存在する場合に非ゼロコードで終了します | いいえ       |

## 終了コード

| 終了コード | 説明                                                                          |
| ---------- | ----------------------------------------------------------------------------- |
| `0`        | エラーなし                                                                    |
| `1`        | チェックでエラーが発見された、または `--deny-warnings` 使用時に警告が存在する |
| `2`        | `.yx` ファイルが見つからない                                                  |

## ファイル間解析

`yx check` はファイル間の型検査をサポートします。複数のファイルをチェックする場合：

1. すべての `.yx` ファイルを並列に解析します
2. モジュール依存グラフを構築します
3. 循環依存を検出します（エラーを報告）
4. トポロジカルソート順でチェックします
5. 共有型環境を使用して、ファイル間の参照を正しく検出します

```bash
# プロジェクト全体をチェック（ファイル間参照を自動検出）
yx check src/

# 指定したファイルをチェック
yx check src/main.yx src/lib.yx
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

# ディレクトリをチェックして JSON を出力
yx check src/ --json

# CI モード（色なし、進捗なし）
yx check --color never --no-progress

# CI 厳格モード（警告も失敗として扱われる）
yx check --deny-warnings

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

- [`yx format`](./format-command.md) -- コードフォーマッター
- [`yx test`](./test-command.md) -- テストを実行
- [エラーコードリファレンス](./error-codes.md) -- 完全なエラーコード一覧
- [CI 統合ガイド](../guide/ci-integration.md) -- CI/CD 統合
- [診断システム設計](../design/check/diagnostic-system.md) -- アーキテクチャ設計ドキュメント
