---
title: "制御フローのフォーマット規則"
description: if/elif/else、forループ、whileループ、ループラベルのフォーマット規則
---

# 制御フローのフォーマット規則

---

## §5 制御フロー

**§5.1 if式。** `if`キーワードと条件の間にスペースを入れ、条件とコードブロックの間にスペースを入れる。

```
// ✅ 正しい
if condition { ... }

// ❌ 誤り
if(condition) { ... }
if condition{ ... }
```

**§5.2 elif/else。** `elif`と`else`は前のコードブロックとの間にスペースを入れる。

```
// ✅ 正しい
if a > 0 { ... } elif a < 0 { ... } else { ... }

// ❌ 誤り
if a > 0 { ... }elif a < 0 { ... }else { ... }
```

**§5.3 forループ。** `for`キーワード、変数、`in`キーワード、イテレータの間をスペースで区切る。

```
// ✅ 正しい
for item in collection { ... }

// ❌ 誤り
for item in(collection) { ... }
for(item) in collection { ... }
```

**§5.4 whileループ。** `while`キーワードと条件の間にスペースを入れる。

```
// ✅ 正しい
while condition { ... }

// ❌ 誤り
while(condition) { ... }
```

**§5.5 ループラベル。** ラベルとループキーワードの間に `:` とスペースで接続する。

```
// ✅ 正しい
'outer: for i in range(10) { ... }

// ❌ 誤り
'outer:for i in range(10) { ... }
'outer : for i in range(10) { ... }
```

---

## §5.6 Return文

**§5.6.1 Returnの書式。** `return`キーワードと式の間にはスペースを入れる。

```
// ✅ 正しい
return 42;
return x + y;

// ❌ 誤り
return(42);  // スペースが不足
return  42;  // 余分なスペース
```

**§5.6.2 空のReturn。** 空のreturnは`return`キーワードを直接使用する。

```
// ✅ 正しい
return;

// ❌ 誤り
return ;  // 余分なスペース
return void;  // voidは不要
```

---

## §5.7 Break文

**§5.7.1 Breakの書式。** `break`キーワードとラベルの間にはスペースを入れる。

```
// ✅ 正しい
break;
break 'outer;

// ❌ 誤り
break(outer);  // 構文エラー
break  'outer;  // 余分なスペース
```

---

## §5.8 Continue文

**§5.8.1 Continueの書式。** `continue`キーワードとラベルの間にはスペースを入れる。

```
// ✅ 正しい
continue;
continue 'outer;

// ❌ 誤り
continue(outer);  // 構文エラー
continue  'outer;  // 余分なスペース
```