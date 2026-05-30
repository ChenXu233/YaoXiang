```markdown
---
description: YaoXiang ソースコードに対して静的チェック（型チェック、所有権チェック）を実行します。
---

# yaoxiang check

YaoXiang ソースコードに対して静的チェック（型チェック、所有権チェック）を実行します。コードは生成しません。

## 使い方

```
yaoxiang check [OPTIONS] [PATH]...
```

## 引数

| 引数 | 説明 |
|------|------|
| `PATH` | 1つ以上のファイルまたはディレクトリのパス。指定しない場合は現在のプロジェクトをチェックします。 |

## オプション

| オプション | 説明 | デフォルト値 |
|------|------|------|
| `--json` | 診断情報をJSON形式で出力する | いいえ |
| `-w`, `--watch` | ファイルの変更を監視し、自動的に再チェックする | いいえ |
| `--color <MODE>` | カラー出力モード：`auto`、`always`、`never` | `auto` |
| `--exclude <PATH>` | 指定したパスを除外する（複数回使用可能） | なし |
| `--no-progress` | 進捗メッセージとサマリメッセージを表示しない | いいえ |

## 終了コード

| 終了コード | 説明 |
|--------|------|
| `0` | エラーなし |
| `1` | チェックでエラーが検出された |
| `2` | `.yx` ファイルが見つからない |

## JSON 出力形式

`--json` を使用した場合の出力形式：

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

# ディレクトリをチェックしJSON出力
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

## 関連項目

- [`yaoxiang fmt`](./format-command.md) -- コードフォーマット
- [エラーコードリファレンス](./error-codes.md) -- 完全なエラーコードリスト
```