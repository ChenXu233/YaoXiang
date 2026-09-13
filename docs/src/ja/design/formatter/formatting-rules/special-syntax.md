---
title: '特殊な構文規則'
description: 'F-String、import文、エラーハンドリング、Unsafeブロックのフォーマット規則'
---

# 特殊な構文規則

---

## §13 F-String

**§13.1 F-Stringフォーマット。** F-Stringは`f"..."`形式を使用し、補間は`{expr}`を使用。

```
// ✅ 正确
let msg = f"Hello, {name}!";
let msg = f"Result: {x + y}";
```

**§13.2 フォーマット指定。** F-Stringはフォーマット指定`{expr:spec}`をサポート。

```
// ✅ 正确
let msg = f"{value:.2f}";
```

---

## §14 import文

**§14.1 importのソート。** `sort_imports = true`の時、import文は以下の順序でソート：

1. 標準ライブラリ（`std`, `core`, `alloc`）
2. 外部crate
3. 相対パス（`.`または`..`で始まる）

**§14.2 グループ内のソート。** 同じグループ内のimportはアルファベット順にソート。

```
// 排序前
use z_crate;
use std::collections;
use a_crate;
use ./local;

// 排序后
use std::collections;
use a_crate;
use z_crate;
use ./local;
```

---

## §17 エラーハンドリング

**§17.1 Try演算子。** `expr?`形式を使用。

```
// ✅ 正确
let x = foo()?;

// ❌ 错误
let x = foo() ?;
```

---

## §18 Unsafeブロック

**§18.1 Unsafeフォーマット。** `unsafe { ... }`形式を使用。

```
// ✅ 正确
let x = unsafe { dangerous_function() };

// ❌ 错误
let x = unsafe{ dangerous_function() };
```
