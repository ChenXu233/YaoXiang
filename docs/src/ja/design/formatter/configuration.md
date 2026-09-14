---
title: 'フォーマット設定オプション'
description: 'yx format の設定ファイル形式、優先順位、デフォルト値'
---

# 設定オプション

---

## 設定ファイル形式

設定ファイルは TOML 形式を使用し、ファイル名は `yaoxiang.toml` です。

```toml
[fmt]
# 行幅制限（デフォルト 120）
line_width = 120

# インデント幅（デフォルト 4）
indent_width = 4

# タブインデントを使用するか（デフォルト false）
use_tabs = false

# シングルクォートを使用するか（デフォルト false）
single_quote = false

# import 文をソートするか（デフォルト true）
sort_imports = true
```

---

## 設定の優先順位

設定の優先順位チェーン（高い順から）：

1. **CLI パラメータ** — コマンドライン引数が最優先されます
2. **プロジェクトレベル設定** — カレントディレクトリの `yaoxiang.toml`
3. **ユーザーレベル設定** — `~/.config/yaoxiang/config.toml`
4. **デフォルト値** — 内蔵のデフォルト値

---

## デフォルト値

| オプション     | デフォルト値 | 説明                         |
| -------------- | ------------ | ---------------------------- |
| `line_width`   | 120          | 最大行幅                     |
| `indent_width` | 4            | インデントのスペース数       |
| `use_tabs`     | false        | タブを使用するか             |
| `single_quote` | false        | シングルクォートを使用するか |
| `sort_imports` | true         | import をソートするか        |
