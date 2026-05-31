# yaoxiang check

YaoXiang ソースコードに対して静的チェック（型チェック、所有権チェック）を実行します。コードの生成は一切行いません。

## 使い方

```
yaoxiang check [OPTIONS] [PATH]...
```

## 引数

| 引数 | 説明 |
|------|------|
| `PATH` | 1 つ以上のファイルまたはディレクトリのパス。指定しない場合はカレントプロジェクトをチェックします。 |

## オプション

| オプション | 説明 | デフォルト値 |
|------|------|--------|
| `--json` | 診断情報を JSON 形式で出力する | なし |
| `-w`, `--watch` | ファイルの変更を監視し、自動的に再チェックする | なし |
| `--color <MODE>` | カラー出力モード：`auto`、`always`、`never` | `auto` |
| `--exclude <PATH>` | 指定したパスを除外する（複数回使用可能） | なし |
| `--no-progress` | 進捗メッセージとサマリを抑制する | なし |

## 終了コード

| 終了コード | 説明 |
|--------|------|
| `0` | エラーなし |
| `1` | チェックによりエラーが検出された |
| `2` | `.yx` ファイルが見つからなかった |

## ファイル間分析

`yaoxiang check` はファイル間の型チェックをサポートしています。複数のファイルをチェックする場合：

1. すべての `.yx` ファイルを並行してパースする
2. モジュール依存グラフを構築する
3. 循環依存を検出する（エラーとして報告）
4. トポロジカルソート順でチェックする
5. 共有型環境を使用して、ファイル間参照を正しく検出する

```bash
# プロジェクト全体をチェック（ファイル間参照を自動検出）
yaoxiang check src/

# 指定ファイルをチェック
yaoxiang check src/main.yx src/lib.yx
```

## 增量チェック（watch モード）

`-w` または `--watch` を使用するとファイル監視モードが有効になります。ファイルが変更되면自動的に再チェックします。

```bash
yaoxiang check --watch
```

## JSON 出力形式

`--json` を使用した場合、出力形式は以下の通りです：

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

# ディレクトリをチェックして JSON 出力
yaoxiang check src/ --json

# 監視モード
yaoxiang check --watch

# CI モード（カラーなし、進捗なし）
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

詳細な CI 設定については、[CI 統合ガイド](../guide/ci-integration.md) を参照してください。

## 関連項目

- [`yaoxiang fmt`](./format-command.md) -- コードフォーマット
- [エラーコードリファレンス](./error-codes.md) -- 完全なエラーコード一覧
- [CI 統合ガイド](../guide/ci-integration.md) -- CI/CD 統合
- [診断システム設計](../design/check/diagnostic-system.md) -- アーキテクチャ設計ドキュメント