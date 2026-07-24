# yaoxiang check

YaoXiang ソースコードに対して静的検査（型検査、所有権検査）を行い、コードを生成しません。

## 使い方

```
yaoxiang check [OPTIONS] [PATH]...
```

## 引数

| 引数 | 説明 |
|------|------|
| `PATH` | 1つまたは複数のファイルまたはディレクトリのパス。指定しない場合は現在のプロジェクトを検査します。 |

## オプション

| オプション | 説明 | デフォルト値 |
|------|------|--------|
| `--json` | 診断情報を JSON 形式で出力します | いいえ |
| `-w`, `--watch` | ファイルの変更を監視し、自動的に再検査します | いいえ |
| `--color <MODE>` | カラー出力モード：`auto`、`always`、`never` | `auto` |
| `--exclude <PATH>` | 指定したパスを除外します（複数回使用可能） | なし |
| `--no-progress` | 進捗と要約メッセージを抑制します | いいえ |

## 終了コード

| 終了コード | 説明 |
|--------|------|
| `0` | エラーなし |
| `1` | 検査でエラーが発見されました |
| `2` | `.yx` ファイルが見つかりません |

## クロスファイル解析

`yaoxiang check` はクロスファイル型検査をサポートします。複数のファイルを検査する際：

1. すべての `.yx` ファイルを並列に解析
2. モジュール依存グラフを構築
3. 循環依存を検出（エラーを報告）
4. トポロジカルソート順に従って検査
5. 共有型環境を使用して、クロスファイル参照を正確に検出

```bash
# プロジェクト全体を検査（クロスファイル参照を自動検出）
yaoxiang check src/

# 指定したファイルを検査
yaoxiang check src/main.yx src/lib.yx
```

## 増分検査（watch モード）

`-w` または `--watch` を使用してファイル監視モードを有効にします。ファイルが変更されると自動的に再検査されます。

```bash
yaoxiang check --watch
```

## JSON 出力フォーマット

`--json` を使用する場合、出力フォーマットは次のとおりです：

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
# 現在のプロジェクトを検査
yaoxiang check

# 指定したファイルを検査
yaoxiang check src/main.yx

# ディレクトリを検査して JSON 出力
yaoxiang check src/ --json

# 監視モード
yaoxiang check --watch

# CI モード（色なし、進捗なし）
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

詳細な CI 設定については [CI 統合ガイド](../guide/ci-integration.md) を参照してください。

## 関連項目

- [`yaoxiang format`](./format-command.md) -- コードフォーマッタ
- [エラーコードリファレンス](./error-codes.md) -- 完全なエラーコードリスト
- [CI 統合ガイド](../guide/ci-integration.md) -- CI/CD 統合
- [診断システム設計](../design/check/diagnostic-system.md) -- アーキテクチャ設計ドキュメント