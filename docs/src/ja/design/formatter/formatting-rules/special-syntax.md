---
title: "特殊構文規則"
description: "F-String、インポート文、エラー処理、Unsafe ブロックのフォーマット規則"
---

# 特殊構文規則

---

## §13 F-String

**§13.1 F-String フォーマット。** F-String は `f"..."` フォーマットを使用し、補間に `{expr}` を使用します。

```
// ✅ 正しい
let msg = f"Hello, {name}!";
let msg = f"Result: {x + y}";
```

**§13.2 フォーマット仕様。** F-String はフォーマット仕様 `{expr:spec}` をサポートしています。

```
// ✅ 正しい
let msg = f"{value:.2f}";
```

---

## §14 インポート文

**§14.1 インポートのソート。** `sort_imports = true` の場合、インポート文は次の順序でソートされます：

1. 標準ライブラリ（`std`, `core`, `alloc`）
2. 外部 crate
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

**§17.1 Try 演算子。** `expr?` フォーマットを使用します。

```
// ✅ 正しい
let x = foo()?;

// ❌ 誤り
let x = foo() ?;
```

---

## §18 Unsafe ブロック

**§18.1 Unsafe フォーマット。** `unsafe { ... }` フォーマットを使用します。

```
// ✅ 正しい
let x = unsafe { dangerous_function() };

// ❌ 誤り
let x = unsafe{ dangerous_function() };
```