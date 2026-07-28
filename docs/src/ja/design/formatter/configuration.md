---
title: 'フォーマット設定オプション'
description: yaoxiang fmt の設定ファイル形式、優先順位、デフォルト値
---

# 設定オプション

---

## 設定ファイル形式

設定ファイルはTOML形式を使用し、ファイル名は `yaoxiang.toml` です。

```toml
[fmt]
# 行幅の制限（デフォルト 120）
line_width = 120

# インデント幅（デフォルト 4）
indent_width = 4

# タブによるインデントを使用するか（デフォルト false）
use_tabs = false

# 単一引用符を使用するか（デフォルト false）
single_quote = false

# インポート文をソートするか（デフォルト true）
sort_imports = true
```

---

## 設定の優先順位

設定の優先順位チェーン（高い順）：

1. **CLI 引数** — コマンドライン引数が最優先
2. **プロジェクトレベルの設定** — 現在のディレクトリの `yaoxiang.toml`
3. **ユーザーレベルの設定** — `~/.config/yaoxiang/config.toml`
4. **デフォルト値** — 組み込みのデフォルト値

---

## デフォルト値

| オプション      | デフォルト値 | 説明             |
| -------------- | ------------ | --------------- |
| `line_width`   | 120          | 最大行幅         |
| `indent_width` | 4            | インデントの空白数 |
| `use_tabs`     | false        | タブを使用するか |
| `single_quote` | false        | 単一引用符を使用するか |
| `sort_imports` | true         | インポートをソートするか |
