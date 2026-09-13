---
title: 'データ構造のフォーマットルール'
description: 'リテラル、リストと辞書、Match 式のフォーマットルール'
---

# データ構造のフォーマットルール

---

## §8 リテラル

**§8.1 整数リテラル。** 整数リテラルは直接出力する。

```
// ✅ 正确
let x = 42;
```

**§8.2 浮動小数点リテラル。** 浮動小数点リテラルには小数点を含めなければならない。

```
// ✅ 正确
let x = 3.14;
let y = 42.0;  // 必须有小数点

// ❌ 错误
let y = 42;    // 整数，不是浮点数
```

**§8.3 文字列リテラル。** デフォルトではダブルクォートを使用する。`single_quote = true`
のときはシングルクォートを使用する。

```
// 默认（双引号）
let s = "hello";

// single_quote = true
let s = 'hello';
```

**§8.4 ブールリテラル。** ブールリテラルは小文字を使用する。

```
// ✅ 正确
let x = true;
let y = false;

// ❌ 错误
let x = True;
let y = FALSE;
```

---

## §10 リストと辞書

**§10.1 リストフォーマット。** リストは `[]` で囲み、要素間はカンマで区切る。

```
// ✅ 正确
let x = [1, 2, 3];

// ❌ 错误
let x = [1,2,3];
```

**§10.2 辞書フォーマット。** 辞書は `{}` で囲み、キーと値は `key: value` の形式を使用する。

```
// ✅ 正确
let x = {"a": 1, "b": 2};

// ❌ 错误
let x = {"a":1, "b":2};
```

**§10.3 リスト内包表記。** リスト内包表記は `[expr for var in iterable]` の形式を使用する。

```
// ✅ 正确
let x = [i * 2 for i in range(10)];

// 带条件
let x = [i for i in range(10) if i > 5];
```

---

## §11 Match 式

**§11.1 Match フォーマット。** `match` キーワードと式の間はスペースで区切る。

```
// ✅ 正确
match x { ... }

// ❌ 错误
match(x) { ... }
```

**§11.2 Pattern の整列。** 複数の pattern は整列させ、スペースで埋める。

```
// ✅ 对齐
match x {
    1    => "one",
    2    => "two",
    100  => "hundred",
    _    => "other",
}
```

**§11.3 Pattern の長すぎる場合の改行。** pattern が長すぎるときは pattern を改行し、`=>`
を body と整列させる。

```
// ✅ 换行
match x {
    VeryLongPatternName { field1, field2 }
        => handle_case(field1, field2),
    _ => default_case(),
}
```

---

## §11.4 タプル

**§11.4.1 タプルフォーマット。** タプルは `()` で囲み、要素間はカンマで区切る。

```
// ✅ 正确
let t = (1, "hello", true);
let t = (1,);  // 单元素元组

// ❌ 错误
let t = (1, "hello", true);  // 逗号后缺少空格
let t = (1,"hello",true);  // 逗号后缺少空格
```

**§11.4.2 空タプル。** 空タプルは `()` で表す。

```
// ✅ 正确
let t = ();
```

---

## §11.5 インデックスアクセス

**§11.5.1 インデックスフォーマット。** インデックスは `expr[index]` の形式を使用する。

```
// ✅ 正确
let x = arr[0];
let y = matrix[i][j];

// ❌ 错误
let x = arr [0];  // 多余空格
let y = matrix[ i ][ j ];  // 多余空格
```

---

## §11.6 フィールドアクセス

**§11.6.1 フィールドアクセスフォーマット。** フィールドアクセスは `expr.field` の形式を使用する。

```
// ✅ 正确
let x = obj.field;
let y = obj.method();

// ❌ 错误
let x = obj . field;  // 多余空格
let y = obj. field;  // 多余空格
```

**§11.6.2 チェーンフィールドアクセス。**
チェーンフィールドアクセスが行幅を超える場合は、1 行に 1 つのメソッド呼び出しとする。

```
// 超过行宽时
let result = object.method1().method2().method3().method4();

// 格式化后
let result = object
    .method1()
    .method2()
    .method3()
    .method4();
```
