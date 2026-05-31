---
title: "データ構造のフォーマット規則"
description: "リテラル、リストと辞書、Match式のフォーマット規則"
---

# データ構造のフォーマット規則

---

## §8 リテラル

**§8.1 整数リテラル。** 整数リテラルはそのまま出力する。

```javascript
// ✅ 正しい
let x = 42;
```

**§8.2 浮動小数点リテラル。** 浮動小数点リテラルは小数点を含める必要がある。

```javascript
// ✅ 正しい
let x = 3.14;
let y = 42.0;  // 小数点が必要

// ❌ 間違い
let y = 42;    // 整数であり、浮動小数点数ではない
```

**§8.3 文字列リテラル。** デフォルトではダブルクォートを使用する。`single_quote = true` の場合はシングルクォートを使用する。

```javascript
// デフォルト（ダブルクォート）
let s = "hello";

// single_quote = true
let s = 'hello';
```

**§8.4 ブールリテラル。** ブールリテラルは小文字を使用する。

```javascript
// ✅ 正しい
let x = true;
let y = false;

// ❌ 間違い
let x = True;
let y = FALSE;
```

---

## §10 リストと辞書

**§10.1 リストのフォーマット。** リストは `[]` で囲み、要素間はカンマで区切る。

```javascript
// ✅ 正しい
let x = [1, 2, 3];

// ❌ 間違い
let x = [1,2,3];
```

**§10.2 辞書のフォーマット。** 辞書は `{}` で囲み、キーと値のペアは `key: value` 形式を使用する。

```javascript
// ✅ 正しい
let x = {"a": 1, "b": 2};

// ❌ 間違い
let x = {"a":1, "b":2};
```

**§10.3 リスト内包表記。** リスト内包表記は `[expr for var in iterable]` 形式を使用する。

```javascript
// ✅ 正しい
let x = [i * 2 for i in range(10)];

// 条件付き
let x = [i for i in range(10) if i > 5];
```

---

## §11 Match式

**§11.1 Matchのフォーマット。** `match` キーワードと式の間にスペースを入れる。

```javascript
// ✅ 正しい
match x { ... }

// ❌ 間違い
match(x) { ... }
```

**§11.2 Patternの整列。** 複数のpatternは整列させ、スペースで埋める。

```javascript
// ✅ 整列
match x {
    1    => "one",
    2    => "two",
    100  => "hundred",
    _    => "other",
}
```

**§11.3 Pattern过长换行。** Pattern过长换行 时，pattern换行，`=>` 与 body 对齐。

```javascript
// ✅ 换行
match x {
    VeryLongPatternName { field1, field2 }
        => handle_case(field1, field2),
    _ => default_case(),
}
```