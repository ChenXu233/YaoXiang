---
title: "制御フローのフォーマットルール"
description: if/elif/else、for ループ、while ループ、ループラベルのフォーマットルール
---

# 制御フローのフォーマットルール

---

## §5 制御フロー

**§5.1 if 式。** `if` キーワードと条件の間、条件とコードブロックの間にはスペースを入れる。

```
// ✅ 正しい
if condition { ... }

// ❌ 間違い
if(condition) { ... }
if condition{ ... }
```

**§5.2 elif/else。** `elif` と `else` は前のコードブロックとの間にスペースを入れる。

```
// ✅ 正しい
if a > 0 { ... } elif a < 0 { ... } else { ... }

// ❌ 間違い
if a > 0 { ... }elif a < 0 { ... }else { ... }
```

**§5.3 for ループ。** `for` キーワード、変数、`in` キーワード、イテレータの間にはスペースを入れる。

```
// ✅ 正しい
for item in collection { ... }

// ❌ 間違い
for item in(collection) { ... }
for(item) in collection { ... }
```

**§5.4 while ループ。** `while` キーワードと条件の間にはスペースを入れる。

```
// ✅ 正しい
while condition { ... }

// ❌ 間違い
while(condition) { ... }
```

**§5.5 ループラベル。** ラベルとループキーワードの間は `: ` で接続する。

```
// ✅ 正しい
'outer: for i in range(10) { ... }

// ❌ 間違い
'outer:for i in range(10) { ... }
'outer : for i in range(10) { ... }
```