# yx test

YaoXiang テストファイルを実行します。テストファイルは通常の `.yx` ソースファイルであり、`std.test`
標準テストモジュールを介してアサーションと期待値を宣言します。

## 使用法

```
yx test [OPTIONS] [PATH]...
```

## テスト検出

`PATH` を指定しない場合、以下の順序でテスト範囲を決定します：

1. `./yaoxiang.toml` の `[tool.test]` の `patterns` 設定
2. 未設定の場合、デフォルトで `tests/**/*.yx` を検出

`PATH`
を指定した場合、明示的に指定されたパスのみを実行し（patterns 設定は読み取られません）、その後設定の exclude パターンと
`--filter` が適用されます。

各テストファイルは、宣言された期待値に基づいて 4 種類に分類されます（スキーマは安定しており、CI で利用されます）：

| カテゴリ        | 判定方法                                                                                       |
| --------------- | ---------------------------------------------------------------------------------------------- |
| `behavior`      | ファイルを実行し、終了コードが 0 であれば合格                                                  |
| `compile-error` | `check` が非ゼロで終了し、宣言されたエラーコードがすべて出現すれば合格（ファイルは実行しない） |
| `runtime-error` | `check` がパスし、実行が失敗する必要があり、宣言されたエラーコードがすべて出現すれば合格       |
| `invalid`       | ファイル宣言が不正（期待コードとカテゴリの矛盾など）、合格にも不合格にもカウントされない       |

期待値宣言の文法については [RFC-036](../design/rfc/accepted/036-test-framework.md)
を参照してください。

## オプション

| オプション        | 説明                                                           | デフォルト |
| ----------------- | -------------------------------------------------------------- | ---------- |
| `--filter <NAME>` | ファイル名に指定された部分文字列を含むテストファイルのみを実行 | なし       |
| `--fail-fast`     | 最初に失敗したテストファイルの実行完了後に停止                 | いいえ     |
| `-v`, `--verbose` | 各テストファイルがキャプチャした stdout/stderr を表示          | いいえ     |
| `--list`          | 検出されたテストファイルを一覧表示するだけで実行しない         | いいえ     |
| `--no-progress`   | 進捗出力を抑制（タイトルと PASS 行）；失敗とサマリーは常に表示 | いいえ     |
| `--json`          | 人間が読めるテキストの代わりに JSON レポートを出力             | いいえ     |
| `--parallel`      | テストファイルを並列実行（CPU コアごとに 1 つの worker）       | いいえ     |

## 終了コード

| 終了コード | 説明                                                 |
| ---------- | ---------------------------------------------------- |
| `0`        | すべて合格（またはテストファイルが検出されなかった） |
| `1`        | 失敗が存在する、または実行エラー                     |

## JSON 出力フォーマット

`--json` を使用する場合、出力フォーマットは次のとおりです：

```json
{
  "summary": {
    "total": 3,
    "passed": 2,
    "failed": 1,
    "skipped": 0,
    "by_kind": { "behavior": 2, "compile-error": 0, "runtime-error": 1, "invalid": 0 },
    "time_secs": 0.512
  },
  "files": [
    {
      "file": "tests/div_zero_err.yx",
      "kind": "runtime-error",
      "passed": false,
      "time_secs": 0.103,
      "exit_code": 1,
      "stderr": "..."
    }
  ]
}
```

- 失敗したファイルには `exit_code` と `stderr` が付与されます（CI の証拠収集用）；`--verbose`
  指定時はすべてのファイルに `stdout`/`stderr` が付与されます
- `files` はパスでソートされ、出力は安定しています
- `by_kind` は固定の 4 つのキー（`behavior` / `compile-error` / `runtime-error` /
  `invalid`）を持ち、ゼロカウントも出力されます

## 例

```bash
# 运行项目全部测试
yx test

# 运行指定目录
yx test tests/yaoxiang/

# 只运行文件名包含 parser 的测试文件
yx test --filter parser

# 首个失败后停止，并显示捕获输出
yx test --fail-fast -v

# 仅列出发现的测试文件
yx test --list

# CI 模式：并行、无进度
yx test --parallel --no-progress

# 输出 JSON 报告
yx test --json > report.json
```

## CI との統合

```yaml
# GitHub Actions
- name: Test
  run: yx test --parallel --no-progress
```

詳細な CI 設定については [CI 統合ガイド](../guide/ci-integration.md) を参照してください。

## 関連項目

- [`yx check`](./check-command.md) -- 静的チェック
- [`yx format`](./format-command.md) -- コードフォーマット
- [RFC-036: std.test テストフレームワーク](../design/rfc/accepted/036-test-framework.md)
  -- テストフレームワーク設計
- [CI 統合ガイド](../guide/ci-integration.md) -- CI/CD 統合
