---
title: リストと辞書
---

# リストと辞書

データ構造はプログラムの骨格です。YaoXiang は 3 種類の組み込みコレクション型を提供します：リスト、辞書、集合です。

## リスト

リストは**順序付き**の値のシーケンスで、すべての要素は同じ型です。`[]` を使って作成します：

```yaoxiang
# リストの作成
numbers = [1, 2, 3, 4, 5]
names = ["Alice", "Bob", "Charlie"]
empty: List(Int) = []       # 空のリストには型注釈が必要
```

### インデックスアクセス

`[]` を使って位置で要素にアクセスします。インデックスは 0 から始まります：

```yaoxiang
scores = [95, 87, 73, 91]

first = scores[0]    # 95
second = scores[1]   # 87
last = scores[3]     # 91
```

### 一般的な操作

```yaoxiang
mut items = [1, 2, 3]

# 要素の追加
items.append(4)       # [1, 2, 3, 4]

# 長さ
count = items.len()   # 4

# スライス
slice = items[0..2]   # [1, 2]
```

### リスト内包表記

リスト内包表記はリストを作成する強力なツールです——既存のリストから新しいリストを生成します：

```yaoxiang
# 基本的な内包表記
squares = [x * x for x in [1, 2, 3, 4, 5]]
println(squares)  # [1, 4, 9, 16, 25]

# フィルタ条件付きの内包表記
evens = [x for x in [1, 2, 3, 4, 5, 6] if x % 2 == 0]
println(evens)  # [2, 4, 6]

# 型の変換
names = ["Alice", "Bob", "Charlie"]
lengths = [n.len() for n in names]
println(lengths)  # [5, 3, 7]
```

構文：`[式 for 変数 in リスト if 条件]`——`if 条件` の部分はオプションです。

## 辞書

辞書は**キーと値のペア**のコレクションで、キーは文字列、値は任意の型を取れます。`{}` を使って作成します：

```yaoxiang
# 辞書の作成
scores = {"Alice": 90, "Bob": 85, "Charlie": 92}
empty: Dict(Int) = {}          # 空の辞書には型注釈が必要
```

### キーアクセス

`[]` を使ってキーで値にアクセスします：

```yaoxiang
scores = {"Alice": 90, "Bob": 85}

alice = scores["Alice"]   # 90
bob = scores["Bob"]       # 85
```

### 辞書の変更

```yaoxiang
mut data = {"name": "Alice"}

# キーと値の追加 / 更新
data["age"] = 25
data["name"] = "Bob"

println(data)  # {"name": "Bob", "age": 25}
```

### メンバー検査

`in` を使ってキーの存在を確認します：

```yaoxiang
config = {"host": "localhost", "port": "8080"}

has_host = "host" in config    # true
has_user = "user" in config    # false
```

## 集合

集合は**順序なし、重複なし**の値のコレクションです。`{}` を使って作成します（辞書との違い：コロンがありません）：

```yaoxiang
# 集合の作成
colors = {"red", "green", "blue"}
numbers = {1, 2, 3, 3, 2, 1}   # 重複は自動的に取り除かれる

println(numbers)  # {1, 2, 3}
```

### 集合の操作

```yaoxiang
mut tags = {"rust", "compiler"}

# 追加
tags.insert("language")

# メンバー検査
has_rust = "rust" in tags      # true
has_python = "python" in tags  # false
```

## まとめ

| 型 | 構文 | 順序付き？ | 重複可能？ | キーの型 |
|------|------|--------|----------|--------|
| リスト | `[1, 2, 3]` | ✅ | ✅ | 整数インデックス |
| 辞書 | `{"a": 1}` | ✅ | キーは重複不可 | 文字列 |
| 集合 | `{1, 2, 3}` | ❌ | ❌ | なし |

リストは主要なコンテナであり、辞書はキーと値の検索に適し、集合は重複除去とメンバー検査に適しています。