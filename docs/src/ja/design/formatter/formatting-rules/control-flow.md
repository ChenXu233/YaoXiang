---
title: '制御フローのフォーマット規則'
description: 'if/else if/else、for ループ、while ループ、ループラベルのフォーマット規則'
---

# 制御フローのフォーマット規則

---

## §5 制御フロー

**§5.1 if 式。** `if`
キーワードと条件の間はスペースで区切り、条件とコードブロックの間はスペースで区切ります。

```
// ✅ 正しい
if condition { ... }

// ❌ 間違い
if(condition) { ... }
if condition{ ... }
```

**§5.2 else if/else。** `else if` および `else` は前のコードブロックとの間をスペースで区切ります。

```
// ✅ 正しい
if a > 0 { ... } else if a < 0 { ... } else { ... }

// ❌ 間違い
if a > 0 { ... }else if a < 0 { ... }else { ... }
```

**§5.3 for ループ。** `for` キーワード、変数、`in`
キーワード、イテレータの間はスペースで区切ります。

```
// ✅ 正しい
for item in collection { ... }

// ❌ 間違い
for item in(collection) { ... }
for(item) in collection { ... }
```

**§5.4 while ループ。** `while` キーワードと条件の間はスペースで区切ります。

```
// ✅ 正しい
while condition { ... }

// ❌ 間違い
while(condition) { ... }
```

**§5.5 ループラベル。** ラベルとループキーワードの間は `: ` で接続します。

```
// ✅ 正しい
'outer: for i in range(10) { ... }

// ❌ 間違い
'outer:for i in range(10) { ... }
'outer : for i in range(10) { ... }
```

---

## §5.6 Return 文

**§5.6.1 Return フォーマット。** `return` キーワードと式の間はスペースで区切ります。

```
// ✅ 正しい
return 42;
return x + y;

// ❌ 間違い
return(42);  // スペース不足
return  42;  // 余分なスペース
```

**§5.6.2 空の Return。** 空の return は `return` キーワードをそのまま使用します。

```
// ✅ 正しい
return;

// ❌ 間違い
return ;  // 余分なスペース
return void;  // void は不要
```

---

## §5.7 Break 文

**§5.7.1 Break フォーマット。** `break` キーワードとラベルの間はスペースで区切ります。

```
// ✅ 正しい
break;
break 'outer;

// ❌ 間違い
break(outer);  // 誤った構文
break  'outer;  // 余分なスペース
```

---

## §5.8 Continue 文

**§5.8.1 Continue フォーマット。** `continue` キーワードとラベルの間はスペースで区切ります。

```
// ✅ 正しい
continue;
continue 'outer;

// ❌ 間違い
continue(outer);  // 誤った構文
continue  'outer;  // 余分なスペース
```
