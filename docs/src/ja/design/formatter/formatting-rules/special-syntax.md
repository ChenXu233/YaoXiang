---
title: '特殊構文規則'
description: 'F-String、インポート文、エラー処理、Unsafeブロックのフォーマット規則'
---

# 特殊構文規則

---

## §13 F-String

**§13.1 F-Stringのフォーマット。** F-Stringは `f"..."` フォーマットを使用し、補間には `{expr}` を使用します。

```
// ✅ 正しい
let msg = f"Hello, {name}!";
let msg = f"Result: {x + y}";
```

**§13.2 フォーマット仕様。** F-Stringはフォーマット仕様 `{expr:spec}` をサポートしています。

```
// ✅ 正しい
let msg = f"{value:.2f}";
```

---

## §14 インポート文

**§14.1 インポートの順序。** `sort_imports = true` の場合、インポート文は以下の順序でソートされます：

1. 標準ライブラリ（`std`, `core`, `alloc`）
2. 外部クレート
3. 相対パス（`.` または `..` で始まるもの）

**§14.2 グループ内ソート。** 同一グループ内のインポートはアルファベット順にソートされます。

```
// ソート前
use z_crate;
use std::collections;
use a_crate;
use ./local;

// ソート後
use std::collections;
use a_crate;
use z_crate;
use ./local;
```

---

## §17 エラー処理

**§17.1 Try演算子。** `expr?` フォーマットを使用します。

```
// ✅ 正しい
let x = foo()?;

// ❌ 誤り
let x = foo() ?;
```

---

## §18 Unsafeブロック

**§18.1 Unsafeのフォーマット。** `unsafe { ... }` フォーマットを使用します。

```
// ✅ 正しい
let x = unsafe { dangerous_function() };

// ❌ 誤り
let x = unsafe{ dangerous_function() };
```
