---
title: 'リストと辞書'
---

# リストと辞書

データ構造はプログラムの骨格です。YaoXiang のコレクション型は階層的に構築されています：`Vec(T)`
はランタイム可変長のバッファで、`List(T)`
は標準ライブラリの型（`{ data: Vec(T), length: Int }`）であり、辞書も同じ階層機構に基づいています。ユーザーは独自のコンテナ型を定義することもできます。`Index`
インターフェース（RFC-011b）を実装すれば、`[]` による添字アクセスもサポートされます。

## リスト

リストは**順序付き**の値のシーケンスであり、すべての要素は同じ型を持ちます。`[]`
を使って作成します：

```yaoxiang
// リストの作成
numbers = [1, 2, 3, 4, 5]
names = ["Alice", "Bob", "Charlie"]
empty: List(Int) = []       // 空のリストは型注釈が必要
```

### インデックスアクセス

`[]` を使って位置で要素にアクセスします。インデックスは 0 から始まります：

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

リスト内包表記はリストを作成するための強力なツールであり、既存のリストから新しいリストを生成します：

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

構文：`[式 for 変数 in リスト if 条件]` —— `if 条件` の部分はオプションです。

## 辞書

辞書は**キーと値のペア**のコレクションであり、キーは文字列、値は任意の型を取ることができます。`{}`
を使って作成します：

```yaoxiang
// 辞書の作成
scores = {"Alice": 90, "Bob": 85, "Charlie": 92}
empty: Dict(Int) = {}          // 空の辞書には型注釈が必要
```

### キーによるアクセス

`[]` を使ってキーで値にアクセスします：

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

### メンバーシップチェック

`in` を使ってキーが存在するかどうかを確認します：

```yaoxiang
config = {"host": "localhost", "port": "8080"}

has_host = "host" in config    // true
has_user = "user" in config    // false
```

## まとめ

| 型     | 構文        | 順序付き？ | 重複可能？     | キーの型         |
| ------ | ----------- | ---------- | -------------- | ---------------- |
| リスト | `[1, 2, 3]` | ✅         | ✅             | 整数インデックス |
| 辞書   | `{"a": 1}`  | ✅         | キーは重複不可 | String           |

リストは主要なコンテナであり、辞書はキーによる値検索に適しています。
