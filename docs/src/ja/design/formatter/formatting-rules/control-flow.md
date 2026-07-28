---
title: '制御フロー整形規則'
description: if/else if/else、for ループ、while ループ、ループラベルの整形規則
---

# 制御フロー整形規則

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

**§5.2 else if/else。** `else if` と `else` は前のコードブロックとの間にスペースを入れる。

```
// ✅ 正しい
if a > 0 { ... } else if a < 0 { ... } else { ... }

// ❌ 間違い
if a > 0 { ... }else if a < 0 { ... }else { ... }
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

---

## §5.6 return 文

**§5.6.1 return の書式。** `return` キーワードと式の間にはスペースを入れる。

```
// ✅ 正しい
return 42;
return x + y;

// ❌ 間違い
return(42);  // スペースがない
return  42;  // 余分なスペース
```

**§5.6.2 空の return。** 空の return は `return` キーワードをそのまま使用する。

```
// ✅ 正しい
return;

// ❌ 間違い
return ;  // 余分なスペース
return void;  // void は不要
```

---

## §5.7 break 文

**§5.7.1 break の書式。** `break` キーワードとラベルの間にはスペースを入れる。

```
// ✅ 正しい
break;
break 'outer;

// ❌ 間違い
break(outer);  // 構文エラー
break  'outer;  // 余分なスペース
```

---

## §5.8 continue 文

**§5.8.1 continue の書式。** `continue` キーワードとラベルの間にはスペースを入れる。

```
// ✅ 正しい
continue;
continue 'outer;

// ❌ 間違い
continue(outer);  // 構文エラー
continue  'outer;  // 余分なスペース
```
