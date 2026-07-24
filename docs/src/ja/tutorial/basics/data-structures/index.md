---
title: リストと辞書
---

# リストと辞書

データ構造はプログラムの骨格です。YaoXiang には 2 種類の組み込みコレクション型が用意されています。リストと辞書です。

## リスト

リストは**順序付き**の値のシーケンスで、すべての要素は同じ型を持ちます。`[]` を使って作成します。

```yaoxiang
// リストの作成
numbers = [1, 2, 3, 4, 5]
names = ["Alice", "Bob", "Charlie"]
empty: List(Int) = []       // 空のリストには型注釈が必要
```

### インデックスアクセス

`[]` を使って位置で要素にアクセスします。インデックスは 0 から始まります。

```yaoxiang
scores = [95, 87, 73, 91]

first = scores[0]    // 95
second = scores[1]   // 87
last = scores[3]     // 91
```

### 一般的な操作

```yaoxiang
mut items = [1, 2, 3]

// 要素を追加
items.append(4)       // [1, 2, 3, 4]

// 長さ
count = items.len()   // 4

// スライス
slice = items[0..2]   // [1, 2]
```

### リスト内包表記

リスト内包表記はリストを作成するための強力なツールです。既存のリストから新しいリストを生成できます。

```yaoxiang
// 基本的な内包表記
squares = [x * x for x in [1, 2, 3, 4, 5]]
print(squares)  // [1, 4, 9, 16, 25]

// フィルタ条件付きの内包表記
evens = [x for x in [1, 2, 3, 4, 5, 6] if x % 2 == 0]
print(evens)  // [2, 4, 6]

// 型の変換
names = ["Alice", "Bob", "Charlie"]
lengths = [n.len() for n in names]
print(lengths)  // [5, 3, 7]
```

構文は `[式 for 変数 in リスト if 条件]` です。`if 条件` の部分はオプションです。

## 辞書

辞書は**キーと値のペア**の集合です。キーは文字列で、値には任意の型を使用できます。`{}` を使って作成します。

```yaoxiang
// 辞書の作成
scores = {"Alice": 90, "Bob": 85, "Charlie": 92}
empty: Dict(Int) = {}          // 空の辞書には型注釈が必要
```

### キーアクセス

`[]` を使ってキーで値にアクセスします。

```yaoxiang
scores = {"Alice": 90, "Bob": 85}

alice = scores["Alice"]   // 90
bob = scores["Bob"]       // 85
```

### 辞書の変更

```yaoxiang
mut data = {"name": "Alice"}

// キーと値の追加/更新
data["age"] = 25
data["name"] = "Bob"

print(data)  // {"name": "Bob", "age": 25}
```

### メンバー検出

`in` を使ってキーの存在を確認します。

```yaoxiang
config = {"host": "localhost", "port": "8080"}

has_host = "host" in config    // true
has_user = "user" in config    // false
```


## まとめ

| 型 | 構文 | 順序付き？ | 重複可能？ | キー型 |
|------|------|--------|----------|--------|
| リスト | `[1, 2, 3]` | ✅ | ✅ | 整数インデックス |
| 辞書 | `{"a": 1}` | ✅ | キーは重複不可 | String |

リストは主要なコンテナであり、辞書はキーと値の検索に適しています。