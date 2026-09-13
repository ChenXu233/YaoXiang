---
title: 'フォーマット設定オプション'
description: 'yaoxiang fmt の設定ファイル形式、優先度、デフォルト値'
---

# 設定オプション

---

## 設定ファイル形式

設定ファイルはTOML形式を使用し、ファイル名は `yaoxiang.toml` です。

```toml
[fmt]
# 行幅制限（デフォルト 120）
line_width = 120

# インデント幅（デフォルト 4）
indent_width = 4

# タブインデントを使用するか（デフォルト false）
use_tabs = false

# 単一引用符を使用するか（デフォルト false）
single_quote = false

# インポート文をソートするか（デフォルト true）
sort_imports = true
```

---

## 設定の優先度

設定の優先度チェーン（高い順）：

1. **CLIパラメータ** — コマンドライン引数が最高の優先度を持つ
2. **プロジェクトレベル設定** — 現在のディレクトリの `yaoxiang.toml`
3. **ユーザーレベル設定** — `~/.config/yaoxiang/config.toml`
4. **デフォルト値** — 内蔵のデフォルト値

---

## デフォルト値

| オプション     | デフォルト値 | 説明                     |
| -------------- | ------------ | ------------------------ |
| `line_width`   | 120          | 最大行幅                 |
| `indent_width` | 4            | インデントのスペース数   |
| `use_tabs`     | false        | タブを使用するか         |
| `single_quote` | false        | 単一引用符を使用するか   |
| `sort_imports` | true         | インポートをソートするか |
