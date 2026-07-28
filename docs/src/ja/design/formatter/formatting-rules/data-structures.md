---
title: 'データ構造のフォーマット規則'
description: 'リテラル、リストと辞書、Match式のフォーマット規則'
---

# データ構造のフォーマット規則

---

## §8 リテラル

**§8.1 整数リテラル。** 整数リテラルはそのまま出力されます。

```javascript
// ✅ 正しい
let x = 42;
```

**§8.2 浮動小数点数リテラル。** 浮動小数点数リテラルは小数点を含める必要があります。

```javascript
// ✅ 正しい
let x = 3.14;
let y = 42.0;  // 小数点が必要

// ❌ 誤り
let y = 42;    // 整数なので誤り
```

**§8.3 文字列リテラル。** デフォルトではダブルクォートを使用します。`single_quote = true` の場合はシングルクォートを使用します。

```javascript
// デフォルト（ダブルクォート）
let s = "hello";

// single_quote = true
let s = 'hello';
```

**§8.4 真偽値リテラル。** 真偽値リテラルは小文字を使用します。

```javascript
// ✅ 正しい
let x = true;
let y = false;

// ❌ 誤り
let x = True;
let y = FALSE;
```

---

## §10 リストと辞書

**§10.1 リストの形式。** リストは `[]` で囲み、要素間はカンマで区切ります。

```javascript
// ✅ 正しい
let x = [1, 2, 3];

// ❌ 誤り
let x = [1,2,3];
```

**§10.2 辞書の形式。** 辞書は `{}` で囲み、キーと値のペアは `key: value` 形式を使用します。

```javascript
// ✅ 正しい
let x = {"a": 1, "b": 2};

// ❌ 誤り
let x = {"a":1, "b":2};
```

**§10.3 リスト内包表記。** リスト内包表記は `[expr for var in iterable]` 形式を使用します。

```javascript
// ✅ 正しい
let x = [i * 2 for i in range(10)];

// 条件付き
let x = [i for i in range(10) if i > 5];
```

---

## §11 Match式

**§11.1 Matchの形式。** `match` キーワードと式の間にスペースを空けます。

```javascript
// ✅ 正しい
match x { ... }

// ❌ 誤り
match(x) { ... }
```

**§11.2 Patternの整列。** 複数のpatternは整列させ、スペースで埋めます。

```javascript
// ✅ 整列
match x {
    1    => "one",
    2    => "two",
    100  => "hundred",
    _    => "other",
}
```

**§11.3 Pattern过长换行。** Pattern过长换行 时，pattern换行，`=>` 与 body对齐。

```javascript
// ✅ 换行
match x {
    VeryLongPatternName { field1, field2 }
        => handle_case(field1, field2),
    _ => default_case(),
}
```

---

## §11.4 タプル

**§11.4.1 タプルの形式。** タプルは `()` で囲み、要素間はカンマで区切ります。

```javascript
// ✅ 正しい
let t = (1, "hello", true);
let t = (1,);  // 単一要素のタプル

// ❌ 誤り
let t = (1, "hello", true);  // カンマ後スペースなし
let t = (1,"hello",true);  // カンマ後スペースなし
```

**§11.4.2 空のタプル。** 空のタプルは `()` で表します。

```javascript
// ✅ 正しい
let t = ();
```

---

## §11.5 インデックスアクセス

**§11.5.1 インデックスの形式。** インデックスは `expr[index]` 形式を使用します。

```javascript
// ✅ 正しい
let x = arr[0];
let y = matrix[i][j];

// ❌ 誤り
let x = arr [0];  // 余分なスペース
let y = matrix[ i ][ j ];  // 余分なスペース
```

---

## §11.6 フィールドアクセス

**§11.6.1 フィールドアクセスの形式。** フィールドアクセスは `expr.field` 形式を使用します。

```javascript
// ✅ 正しい
let x = obj.field;
let y = obj.method();

// ❌ 誤り
let x = obj . field;  // 余分なスペース
let y = obj. field;  // 余分なスペース
```

**§11.6.2 チェーンされたフィールドアクセス。** チェーンされたフィールドアクセスが行幅を超える場合、各メソッド呼び出しを1行に1つずつ配置します。

```javascript
// 行幅を超える場合
let result = object.method1().method2().method3().method4();

// フォーマット後
let result = object
    .method1()
    .method2()
    .method3()
    .method4();
```
